use crate::process::pty::{ProcessError, PtyProcess, spawn_pty};
use crate::process::types::*;
use crate::theme::types::RGBA;
use alacritty_terminal::Term;
use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::Config as TermConfig;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::time::{Duration, sleep};

/// Simple dimensions wrapper for `Term::new`.
struct TermDimensions {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// Per-process state kept by the manager.
struct ProcessState {
    pid: u32,
    status: ProcessStatus,
    pty: PtyProcess,
    terminal: Arc<Mutex<Term<VoidListener>>>,
    parser: Arc<Mutex<alacritty_terminal::vte::ansi::Processor>>,
    _scrollback: usize,
    generation_id: u64,
    project: String,
    app: String,
    subcommand: String,
}

/// Manages the lifecycle of child processes with PTY + terminal emulator.
pub struct ProcessManager {
    processes: HashMap<(String, String, String), ProcessState>,
    screen_tx: broadcast::Sender<ScreenUpdate>,
    state_tx: broadcast::Sender<StateEvent>,
    next_generation_id: u64,
}

impl ProcessManager {
    pub fn new() -> Self {
        let (screen_tx, _) = broadcast::channel(64);
        let (state_tx, _) = broadcast::channel(64);
        Self {
            processes: HashMap::new(),
            screen_tx,
            state_tx,
            next_generation_id: 1,
        }
    }

    /// Start a new process.  If one is already running for the same key it is
    /// stopped first.
    pub fn start(
        &mut self,
        project: &str,
        app: &str,
        subcommand: &str,
        command: &str,
        workdir: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<u64, ProcessError> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());

        // Stop existing process for this key, if any.
        if self.processes.contains_key(&key) {
            let _ = self.stop(project, app, subcommand);
        }

        let generation_id = self.next_generation_id;
        self.next_generation_id += 1;

        let pty = spawn_pty(command, workdir, cols, rows)?;
        let terminal = build_terminal(cols, rows);
        let parser = alacritty_terminal::vte::ansi::Processor::new();

        let state = ProcessState {
            pid: pty.pid,
            status: ProcessStatus::Starting,
            pty,
            terminal: Arc::new(Mutex::new(terminal)),
            parser: Arc::new(Mutex::new(parser)),
            _scrollback: 0,
            generation_id,
            project: project.to_string(),
            app: app.to_string(),
            subcommand: subcommand.to_string(),
        };

        self.processes.insert(key.clone(), state);

        // Spawn blocking reader task.
        let reader = {
            let st = self.processes.get(&key).unwrap();
            st.pty
                .master
                .try_clone_reader()
                .map_err(|e| ProcessError::Pty(e.to_string()))?
        };
        let term = Arc::clone(&self.processes.get(&key).unwrap().terminal);
        let parser = Arc::clone(&self.processes.get(&key).unwrap().parser);
        let screen_tx = self.screen_tx.clone();
        let state_tx = self.state_tx.clone();
        let project_owned = project.to_string();
        let app_owned = app.to_string();
        let sub_owned = subcommand.to_string();

        tokio::spawn(async move {
            read_task(
                reader,
                term,
                parser,
                generation_id,
                project_owned,
                app_owned,
                sub_owned,
                screen_tx,
                state_tx,
            )
            .await;
        });

        let _ = self.state_tx.send(StateEvent::Started {
            project: project.to_string(),
            app: app.to_string(),
            subcommand: subcommand.to_string(),
        });

        Ok(generation_id)
    }

    /// Gracefully stop a process (SIGTERM to process group).
    pub fn stop(&mut self, project: &str, app: &str, subcommand: &str) -> Result<(), ProcessError> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let state = self.processes.get_mut(&key).ok_or_else(|| {
            ProcessError::NotFound(project.to_string(), app.to_string(), subcommand.to_string())
        })?;

        state.status = ProcessStatus::Stopping;
        state.pty.kill_process_group()?;

        // 5-second grace period → SIGKILL.
        let pid = state.pid;
        let state_tx = self.state_tx.clone();
        let project_owned = project.to_string();
        let app_owned = app.to_string();
        let sub_owned = subcommand.to_string();

        tokio::spawn(async move {
            sleep(Duration::from_secs(5)).await;
            // If the process is still alive, force-kill it.
            let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
            let _ = state_tx.send(StateEvent::Stopped {
                project: project_owned,
                app: app_owned,
                subcommand: sub_owned,
            });
        });

        Ok(())
    }

    /// Stop then start.
    pub fn restart(
        &mut self,
        project: &str,
        app: &str,
        subcommand: &str,
        command: &str,
        workdir: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<u64, ProcessError> {
        let _ = self.stop(project, app, subcommand);
        self.start(project, app, subcommand, command, workdir, cols, rows)
    }

    /// Resize the PTY and terminal emulator for a running process.
    pub fn resize(
        &mut self,
        project: &str,
        app: &str,
        subcommand: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), ProcessError> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let state = self.processes.get_mut(&key).ok_or_else(|| {
            ProcessError::NotFound(project.to_string(), app.to_string(), subcommand.to_string())
        })?;

        state.pty.resize(cols, rows)?;
        let mut term = state.terminal.lock().unwrap();
        let dims = TermDimensions {
            cols: cols as usize,
            rows: rows as usize,
        };
        term.resize(dims);
        Ok(())
    }

    /// Get static metadata for a process.
    pub fn get_info(&self, project: &str, app: &str, subcommand: &str) -> Option<ProcessInfo> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        self.processes.get(&key).map(|s| ProcessInfo {
            project: s.project.clone(),
            app: s.app.clone(),
            subcommand: s.subcommand.clone(),
            status: s.status,
            pid: Some(s.pid),
            generation_id: s.generation_id,
        })
    }

    /// Extract the visible display lines from a process's terminal emulator.
    pub fn get_display_lines(
        &self,
        project: &str,
        app: &str,
        subcommand: &str,
    ) -> Option<Vec<DisplayLine>> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let state = self.processes.get(&key)?;
        let term = state.terminal.lock().unwrap();
        Some(extract_lines(&term))
    }

    /// Remove a stopped process from the manager.
    pub fn remove(&mut self, project: &str, app: &str, subcommand: &str) {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        self.processes.remove(&key);
    }

    /// List all known processes.
    pub fn list(&self) -> Vec<ProcessInfo> {
        self.processes
            .values()
            .map(|s| ProcessInfo {
                project: s.project.clone(),
                app: s.app.clone(),
                subcommand: s.subcommand.clone(),
                status: s.status,
                pid: Some(s.pid),
                generation_id: s.generation_id,
            })
            .collect()
    }

    pub fn subscribe_screen(&self) -> broadcast::Receiver<ScreenUpdate> {
        self.screen_tx.subscribe()
    }

    pub fn subscribe_state(&self) -> broadcast::Receiver<StateEvent> {
        self.state_tx.subscribe()
    }
}

/// Blocking task that reads PTY output and feeds it into the terminal emulator.
async fn read_task(
    mut reader: Box<dyn std::io::Read + Send>,
    terminal: Arc<Mutex<Term<VoidListener>>>,
    parser: Arc<Mutex<alacritty_terminal::vte::ansi::Processor>>,
    generation_id: u64,
    project: String,
    app: String,
    subcommand: String,
    screen_tx: broadcast::Sender<ScreenUpdate>,
    state_tx: broadcast::Sender<StateEvent>,
) {
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                // EOF — process exited.
                let _ = state_tx.send(StateEvent::Stopped {
                    project: project.clone(),
                    app: app.clone(),
                    subcommand: subcommand.clone(),
                });
                break;
            }
            Ok(n) => {
                let mut term = terminal.lock().unwrap();
                let mut parser = parser.lock().unwrap();
                parser.advance(&mut *term, &buf[..n]);
                let _ = screen_tx.send(ScreenUpdate {
                    project: project.clone(),
                    app: app.clone(),
                    subcommand: subcommand.clone(),
                    generation_id,
                });
            }
            Err(_) => {
                let _ = state_tx.send(StateEvent::Stopped {
                    project: project.clone(),
                    app: app.clone(),
                    subcommand: subcommand.clone(),
                });
                break;
            }
        }
    }
}

fn build_terminal(cols: u16, rows: u16) -> Term<VoidListener> {
    let config = TermConfig::default();
    let dims = TermDimensions {
        cols: cols as usize,
        rows: rows as usize,
    };
    Term::new(config, &dims, VoidListener)
}

/// Convert a `vte::ansi::Color` into our `RGBA`.
fn color_to_rgba(c: &alacritty_terminal::vte::ansi::Color) -> RGBA {
    use alacritty_terminal::vte::ansi::{Color, NamedColor};
    match c {
        Color::Named(NamedColor::Black) => RGBA::new(0.0, 0.0, 0.0, 1.0),
        Color::Named(NamedColor::Red) => RGBA::new(0.8, 0.0, 0.0, 1.0),
        Color::Named(NamedColor::Green) => RGBA::new(0.0, 0.8, 0.0, 1.0),
        Color::Named(NamedColor::Yellow) => RGBA::new(0.8, 0.8, 0.0, 1.0),
        Color::Named(NamedColor::Blue) => RGBA::new(0.0, 0.0, 0.8, 1.0),
        Color::Named(NamedColor::Magenta) => RGBA::new(0.8, 0.0, 0.8, 1.0),
        Color::Named(NamedColor::Cyan) => RGBA::new(0.0, 0.8, 0.8, 1.0),
        Color::Named(NamedColor::White) => RGBA::new(0.9, 0.9, 0.9, 1.0),
        Color::Named(NamedColor::Foreground) => RGBA::new(0.9, 0.9, 0.9, 1.0),
        Color::Named(NamedColor::Background) => RGBA::new(0.0, 0.0, 0.0, 1.0),
        Color::Named(_) => RGBA::new(0.9, 0.9, 0.9, 1.0),
        Color::Spec(rgb) => RGBA::new(
            rgb.r as f32 / 255.0,
            rgb.g as f32 / 255.0,
            rgb.b as f32 / 255.0,
            1.0,
        ),
        Color::Indexed(i) => indexed_to_rgb(*i),
    }
}

fn indexed_to_rgb(i: u8) -> RGBA {
    let (r, g, b) = if i < 16 {
        // Standard 16 colors (simplified mapping).
        match i {
            0 => (0, 0, 0),
            1 => (205, 0, 0),
            2 => (0, 205, 0),
            3 => (205, 205, 0),
            4 => (0, 0, 238),
            5 => (205, 0, 205),
            6 => (0, 205, 205),
            7 => (229, 229, 229),
            8 => (127, 127, 127),
            9 => (255, 0, 0),
            10 => (0, 255, 0),
            11 => (255, 255, 0),
            12 => (92, 92, 255),
            13 => (255, 0, 255),
            14 => (0, 255, 255),
            15 => (255, 255, 255),
            _ => (0, 0, 0),
        }
    } else if i < 232 {
        // 6×6×6 color cube.
        let n = i - 16;
        let r = (n / 36) as u8;
        let g = ((n % 36) / 6) as u8;
        let b = (n % 6) as u8;
        let scale = |v: u8| if v == 0 { 0 } else { v * 40 + 55 };
        (scale(r), scale(g), scale(b))
    } else {
        // Grayscale ramp.
        let gray = (i - 232) as u8;
        let v = gray * 10 + 8;
        (v, v, v)
    };
    RGBA::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

fn extract_lines(term: &Term<VoidListener>) -> Vec<DisplayLine> {
    let grid = term.grid();
    let cols = grid.columns();
    let lines_count = grid.screen_lines();

    let mut lines = Vec::new();
    for line_idx in 0..lines_count {
        let line = &grid[Line(line_idx as i32)];
        let mut display_line = DisplayLine::new();
        for col_idx in 0..cols {
            let cell = &line[Column(col_idx)];
            let c = cell.c;
            if c == ' ' && display_line.is_empty() {
                // Skip leading spaces on empty lines.
                continue;
            }
            let fg = color_to_rgba(&cell.fg);
            let bg = color_to_rgba(&cell.bg);
            let flags = cell.flags;
            display_line.push(TerminalCell {
                c,
                fg,
                bg,
                bold: flags.contains(alacritty_terminal::term::cell::Flags::BOLD),
                italic: flags.contains(alacritty_terminal::term::cell::Flags::ITALIC),
                underline: flags.contains(alacritty_terminal::term::cell::Flags::UNDERLINE),
            });
        }
        lines.push(display_line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_pty_spawn_terminal_emulator_and_kill() {
        let mut mgr = ProcessManager::new();
        let mut state_rx = mgr.subscribe_state();
        let mut screen_rx = mgr.subscribe_screen();

        let generation_id = mgr
            .start(
                "test-project",
                "test-app",
                "default",
                "echo hello_world",
                std::path::Path::new("/"),
                80,
                24,
            )
            .expect("start should succeed");

        assert!(generation_id > 0);

        // Wait for the process to finish (echo exits immediately).
        // We may receive Started first, so drain until we see Stopped.
        let event = loop {
            let ev = timeout(Duration::from_secs(5), state_rx.recv())
                .await
                .expect("should receive state event within 5s")
                .expect("channel should be open");
            if matches!(ev, StateEvent::Stopped { .. }) {
                break ev;
            }
        };

        match event {
            StateEvent::Stopped {
                project,
                app,
                subcommand,
            } => {
                assert_eq!(project, "test-project");
                assert_eq!(app, "test-app");
                assert_eq!(subcommand, "default");
            }
            other => panic!("expected Stopped event, got {:?}", other),
        }

        // We should also have gotten at least one screen update.
        let screen = timeout(Duration::from_secs(1), screen_rx.recv())
            .await
            .expect("should receive screen update")
            .expect("channel should be open");

        assert_eq!(screen.project, "test-project");
        assert_eq!(screen.app, "test-app");
        assert_eq!(screen.subcommand, "default");

        // Verify terminal grid contains "hello_world".
        let lines = mgr
            .get_display_lines("test-project", "test-app", "default")
            .expect("should have display lines");

        let text: String = lines
            .iter()
            .flat_map(|line| line.iter().map(|cell| cell.c))
            .collect();

        assert!(
            text.contains("hello_world"),
            "terminal grid should contain 'hello_world', got: {:?}",
            text
        );

        // Verify process info.
        let info = mgr
            .get_info("test-project", "test-app", "default")
            .expect("should have process info");
        assert_eq!(info.status, ProcessStatus::Starting);
        assert_eq!(info.generation_id, generation_id);

        mgr.remove("test-project", "test-app", "default");
        assert!(
            mgr.get_info("test-project", "test-app", "default")
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_process_manager_start_stop() {
        let mut mgr = ProcessManager::new();
        let mut state_rx = mgr.subscribe_state();

        let generation_id = mgr
            .start(
                "proj",
                "app",
                "dev",
                "sleep 30",
                std::path::Path::new("/"),
                80,
                24,
            )
            .expect("start should succeed");

        // Wait for Started event.
        let event = timeout(Duration::from_secs(2), state_rx.recv())
            .await
            .expect("should receive state event")
            .expect("channel should be open");

        match event {
            StateEvent::Started {
                project,
                app,
                subcommand,
            } => {
                assert_eq!(project, "proj");
                assert_eq!(app, "app");
                assert_eq!(subcommand, "dev");
            }
            other => panic!("expected Started event, got {:?}", other),
        }

        let info = mgr.get_info("proj", "app", "dev").unwrap();
        assert_eq!(info.status, ProcessStatus::Starting);
        assert_eq!(info.generation_id, generation_id);

        // Stop it.
        mgr.stop("proj", "app", "dev").expect("stop should succeed");

        // Wait for Stopped event (should come within 6s: 5s grace + 1s buffer).
        let event = timeout(Duration::from_secs(6), state_rx.recv())
            .await
            .expect("should receive state event")
            .expect("channel should be open");

        match event {
            StateEvent::Stopped {
                project,
                app,
                subcommand,
            } => {
                assert_eq!(project, "proj");
                assert_eq!(app, "app");
                assert_eq!(subcommand, "dev");
            }
            other => panic!("expected Stopped event, got {:?}", other),
        }

        mgr.remove("proj", "app", "dev");
    }
}
