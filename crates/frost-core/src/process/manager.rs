use crate::process::listener::FrostListener;
use crate::process::pty::{ProcessError, PtyProcess, spawn_pty};
use crate::process::types::*;
use crate::theme::types::RGBA;
use alacritty_terminal::Term;
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
    terminal: Arc<Mutex<Term<FrostListener>>>,
    parser: Arc<Mutex<alacritty_terminal::vte::ansi::Processor>>,
    /// Shared with the emulator's [`FrostListener`] — title + bell.
    terminal_state: Arc<Mutex<TerminalState>>,
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
        let terminal_state = Arc::new(Mutex::new(TerminalState::default()));
        let listener = FrostListener::new(Arc::clone(&terminal_state));
        let terminal = build_terminal_with_listener(cols, rows, listener);
        let parser = alacritty_terminal::vte::ansi::Processor::new();

        let state = ProcessState {
            pid: pty.pid,
            status: ProcessStatus::Starting,
            pty,
            terminal: Arc::new(Mutex::new(terminal)),
            parser: Arc::new(Mutex::new(parser)),
            terminal_state,
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

    /// Extract every line the emulator currently has — both scrollback
    /// history and the visible viewport — in top-to-bottom order. The
    /// returned `Vec` length equals `history_size + screen_lines`; the
    /// last `screen_lines` rows are the live viewport.
    pub fn get_lines_with_scrollback(
        &self,
        project: &str,
        app: &str,
        subcommand: &str,
    ) -> Option<Vec<DisplayLine>> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let state = self.processes.get(&key)?;
        let term = state.terminal.lock().unwrap();
        Some(extract_lines_with_scrollback(&term))
    }

    /// Latest window title the child set via OSC 0/2, if any. Cleared
    /// when the child sends an empty / reset title.
    pub fn get_title(&self, project: &str, app: &str, subcommand: &str) -> Option<String> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let state = self.processes.get(&key)?;
        state.terminal_state.lock().ok()?.title.clone()
    }

    /// How long ago the last BEL (`\x07`) was received from this child.
    /// `None` means the child has never rung the bell. The TUI uses this
    /// to flash the log-pane border for ~150 ms after a bell event.
    pub fn get_bell_age(
        &self,
        project: &str,
        app: &str,
        subcommand: &str,
    ) -> Option<std::time::Duration> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let state = self.processes.get(&key)?;
        let bell_at = state.terminal_state.lock().ok()?.bell_at?;
        Some(bell_at.elapsed())
    }

    /// Number of scrollback rows currently buffered for this process.
    /// Used by the TUI to clamp scroll offsets and to render a
    /// `(scrolled M/N)` hint.
    pub fn get_history_size(&self, project: &str, app: &str, subcommand: &str) -> usize {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let Some(state) = self.processes.get(&key) else {
            return 0;
        };
        let term = state.terminal.lock().unwrap();
        term.grid().history_size()
    }

    /// Get the cursor position (line, column) for a process's terminal emulator.
    pub fn get_cursor_position(
        &self,
        project: &str,
        app: &str,
        subcommand: &str,
    ) -> Option<(usize, usize)> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let state = self.processes.get(&key)?;
        let term = state.terminal.lock().unwrap();
        let point = term.grid().cursor.point;
        Some((point.line.0 as usize, point.column.0))
    }

    /// Return whether the emulator's cursor is currently visible (DEC private
    /// mode 25 / `\x1B[?25h` vs `\x1B[?25l`). Defaults to `true` for unknown
    /// processes so callers err on the side of showing the cursor.
    pub fn is_cursor_visible(&self, project: &str, app: &str, subcommand: &str) -> bool {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let Some(state) = self.processes.get(&key) else {
            return true;
        };
        let term = state.terminal.lock().unwrap();
        term.mode()
            .contains(alacritty_terminal::term::TermMode::SHOW_CURSOR)
    }

    /// Return whether the emulator has enabled bracketed-paste mode (DEC
    /// private mode 2004 / `\x1B[?2004h`). Used by the TUI to decide
    /// whether to wrap pasted text in `\x1B[200~ … \x1B[201~` delimiters.
    /// Defaults to `false` for unknown processes so the TUI sends raw
    /// bytes by default (safer when in doubt).
    pub fn is_bracketed_paste_active(&self, project: &str, app: &str, subcommand: &str) -> bool {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let Some(state) = self.processes.get(&key) else {
            return false;
        };
        let term = state.terminal.lock().unwrap();
        term.mode()
            .contains(alacritty_terminal::term::TermMode::BRACKETED_PASTE)
    }

    /// Snapshot of the mouse-related DEC modes the emulator has enabled
    /// for the given process. The TUI uses this to decide whether to
    /// forward a mouse event as an escape sequence or to consume it
    /// locally (scrollback, selection). Returns an all-`false` snapshot
    /// for unknown processes so callers default to local handling.
    pub fn mouse_modes(&self, project: &str, app: &str, subcommand: &str) -> MouseModes {
        use alacritty_terminal::term::TermMode;
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let Some(state) = self.processes.get(&key) else {
            return MouseModes::default();
        };
        let term = state.terminal.lock().unwrap();
        let m = term.mode();
        MouseModes {
            report_click: m.contains(TermMode::MOUSE_REPORT_CLICK),
            drag: m.contains(TermMode::MOUSE_DRAG),
            motion: m.contains(TermMode::MOUSE_MOTION),
            sgr: m.contains(TermMode::SGR_MOUSE),
            alternate_scroll: m.contains(TermMode::ALTERNATE_SCROLL),
        }
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

    /// Write raw bytes to a running process's PTY stdin.
    pub fn write_stdin(
        &mut self,
        project: &str,
        app: &str,
        subcommand: &str,
        data: &[u8],
    ) -> Result<(), ProcessError> {
        let key = (project.to_string(), app.to_string(), subcommand.to_string());
        let state = self.processes.get_mut(&key).ok_or_else(|| {
            ProcessError::NotFound(project.to_string(), app.to_string(), subcommand.to_string())
        })?;
        state.pty.write_stdin(data)
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
    terminal: Arc<Mutex<Term<FrostListener>>>,
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

/// Build a `Term` wired to the given listener. Production code in
/// `start()` uses this with a real `FrostListener` connected to the
/// shared `TerminalState`; tests use [`build_terminal`] which substitutes
/// a dummy listener.
fn build_terminal_with_listener(
    cols: u16,
    rows: u16,
    listener: FrostListener,
) -> Term<FrostListener> {
    let config = TermConfig::default();
    let dims = TermDimensions {
        cols: cols as usize,
        rows: rows as usize,
    };
    Term::new(config, &dims, listener)
}

/// Convenience for tests: a `Term` with a throwaway listener whose
/// terminal state is dropped at the end of the test.
#[cfg(test)]
fn build_terminal(cols: u16, rows: u16) -> Term<FrostListener> {
    build_terminal_with_listener(cols, rows, FrostListener::dummy())
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

/// Build a `TerminalCell` from a raw alacritty cell, applying the same
/// flag-mapping the visible-grid extractor uses. Centralised so the
/// scrollback walker and the viewport walker stay in lockstep.
fn build_terminal_cell(cell: &alacritty_terminal::term::cell::Cell) -> TerminalCell {
    use alacritty_terminal::term::cell::Flags;
    let any_underline = Flags::UNDERLINE
        | Flags::DOUBLE_UNDERLINE
        | Flags::UNDERCURL
        | Flags::DOTTED_UNDERLINE
        | Flags::DASHED_UNDERLINE;
    let flags = cell.flags;
    TerminalCell {
        c: cell.c,
        fg: color_to_rgba(&cell.fg),
        bg: color_to_rgba(&cell.bg),
        bold: flags.contains(Flags::BOLD),
        italic: flags.contains(Flags::ITALIC),
        underline: flags.intersects(any_underline),
        reverse: flags.contains(Flags::INVERSE),
        dim: flags.contains(Flags::DIM),
        hidden: flags.contains(Flags::HIDDEN),
        strikethrough: flags.contains(Flags::STRIKEOUT),
        wide: flags.contains(Flags::WIDE_CHAR),
    }
}

/// True when this cell is a placeholder for the right half of a wide
/// glyph or for a wide glyph that wrapped past the screen edge; both
/// must be skipped so the renderer doesn't double-paint.
fn is_wide_spacer(flags: alacritty_terminal::term::cell::Flags) -> bool {
    use alacritty_terminal::term::cell::Flags;
    flags.contains(Flags::WIDE_CHAR_SPACER) || flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
}

/// Same extraction as [`extract_lines`] but walks the entire grid
/// including scrollback history. Returned rows are ordered from oldest
/// (top of history) to newest (bottom of viewport).
fn extract_lines_with_scrollback(term: &Term<FrostListener>) -> Vec<DisplayLine> {
    use alacritty_terminal::grid::Dimensions;
    let grid = term.grid();
    let cols = grid.columns();
    let history = grid.history_size();
    let screen = grid.screen_lines();
    let total = history + screen;

    let mut lines = Vec::with_capacity(total);
    // Iterate from `-history` (oldest) to `screen - 1` (bottom of view).
    let start = -(history as i32);
    let end = screen as i32;
    for line_idx in start..end {
        let line = &grid[Line(line_idx)];
        let mut display_line: DisplayLine = Vec::with_capacity(cols);
        for col_idx in 0..cols {
            let cell = &line[Column(col_idx)];
            if is_wide_spacer(cell.flags) {
                continue;
            }
            display_line.push(build_terminal_cell(cell));
        }
        lines.push(display_line);
    }
    lines
}

fn extract_lines(term: &Term<FrostListener>) -> Vec<DisplayLine> {
    let grid = term.grid();
    let cols = grid.columns();
    let lines_count = grid.screen_lines();

    let mut lines = Vec::with_capacity(lines_count);
    for line_idx in 0..lines_count {
        let line = &grid[Line(line_idx as i32)];
        let mut display_line: DisplayLine = Vec::with_capacity(cols);
        for col_idx in 0..cols {
            let cell = &line[Column(col_idx)];
            if is_wide_spacer(cell.flags) {
                continue;
            }
            display_line.push(build_terminal_cell(cell));
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

    #[test]
    fn extract_lines_populates_reverse_and_bg_from_ansi() {
        // Build a fresh terminal and feed it ANSI that sets a red bg, white fg,
        // a 'A' char, then turns on reverse video, writes 'B', resets.
        let cols: u16 = 10;
        let rows: u16 = 3;
        let mut term = build_terminal(cols, rows);
        let mut parser: alacritty_terminal::vte::ansi::Processor<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        > = alacritty_terminal::vte::ansi::Processor::new();

        // ESC[37;41m → fg white (37), bg red (41) ; write 'A'
        // ESC[7m    → reverse on ; write 'B'
        // ESC[0m    → reset
        let bytes = b"\x1B[37;41mA\x1B[7mB\x1B[0m";
        parser.advance(&mut term, bytes);

        let lines = extract_lines(&term);
        assert!(!lines.is_empty());
        let row0 = &lines[0];
        assert_eq!(row0.len(), cols as usize, "row width must equal cols");

        let a = row0[0];
        assert_eq!(a.c, 'A');
        assert!(!a.reverse, "first cell should not be reversed");
        // bg should be the red we set, not the default black.
        assert!(
            a.bg.r > a.bg.g && a.bg.r > a.bg.b,
            "expected red-dominant bg, got {:?}",
            a.bg
        );

        let b = row0[1];
        assert_eq!(b.c, 'B');
        assert!(b.reverse, "second cell should be reversed");
    }

    #[test]
    fn extract_lines_populates_dim_strike_hidden() {
        let mut term = build_terminal(20, 2);
        let mut parser: alacritty_terminal::vte::ansi::Processor<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        > = alacritty_terminal::vte::ansi::Processor::new();

        // ESC[2m = dim, ESC[9m = strikethrough, ESC[8m = hidden.
        parser.advance(&mut term, b"\x1B[2mD\x1B[0m\x1B[9mS\x1B[0m\x1B[8mH\x1B[0m");

        let lines = extract_lines(&term);
        let row0 = &lines[0];
        assert_eq!(row0[0].c, 'D');
        assert!(row0[0].dim);
        assert!(!row0[0].strikethrough);
        assert!(!row0[0].hidden);

        assert_eq!(row0[1].c, 'S');
        assert!(row0[1].strikethrough);
        assert!(!row0[1].dim);

        assert_eq!(row0[2].c, 'H');
        assert!(row0[2].hidden);
    }

    #[test]
    fn extract_lines_handles_wide_char_and_skips_spacer() {
        // A wide CJK char takes 2 columns; alacritty emits one cell with
        // WIDE_CHAR set followed by a spacer cell that we must skip.
        let mut term = build_terminal(10, 2);
        let mut parser: alacritty_terminal::vte::ansi::Processor<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        > = alacritty_terminal::vte::ansi::Processor::new();

        // U+4E2D ('中') is double-wide. Follow with 'x' so we can verify
        // the spacer was skipped and 'x' lands at the correct index.
        parser.advance(&mut term, "中x".as_bytes());

        let lines = extract_lines(&term);
        let row0 = &lines[0];

        assert_eq!(row0[0].c, '中', "first emitted cell is the wide char");
        assert!(row0[0].wide, "wide flag should be set");

        assert_eq!(
            row0[1].c, 'x',
            "spacer cell should be skipped — next emitted cell is 'x'"
        );
        assert!(!row0[1].wide);
    }

    #[test]
    fn mouse_modes_default_when_no_modes_set() {
        let term = build_terminal(80, 24);
        // Mirror what the public getter does using an internal helper view.
        use alacritty_terminal::term::TermMode;
        let m = term.mode();
        assert!(!m.contains(TermMode::MOUSE_REPORT_CLICK));
        assert!(!m.contains(TermMode::MOUSE_DRAG));
        assert!(!m.contains(TermMode::MOUSE_MOTION));
        assert!(!m.contains(TermMode::SGR_MOUSE));
    }

    #[test]
    fn mouse_modes_track_dec_set_sequences() {
        // Feed `\x1B[?1000h` (report click) + `\x1B[?1006h` (SGR mouse)
        // into the emulator and verify the modes turn on.
        let mut term = build_terminal(80, 24);
        let mut parser: alacritty_terminal::vte::ansi::Processor<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        > = alacritty_terminal::vte::ansi::Processor::new();

        parser.advance(&mut term, b"\x1B[?1000h\x1B[?1006h");

        use alacritty_terminal::term::TermMode;
        let m = term.mode();
        assert!(m.contains(TermMode::MOUSE_REPORT_CLICK));
        assert!(m.contains(TermMode::SGR_MOUSE));

        // And turn them back off.
        parser.advance(&mut term, b"\x1B[?1000l\x1B[?1006l");
        let m = term.mode();
        assert!(!m.contains(TermMode::MOUSE_REPORT_CLICK));
        assert!(!m.contains(TermMode::SGR_MOUSE));
    }

    #[test]
    fn scrollback_grows_when_output_exceeds_screen() {
        // 5-row screen, push 20 line-feeds + content per line. After
        // overflowing the screen, alacritty should park lines into the
        // history buffer.
        let mut term = build_terminal(20, 5);
        let mut parser: alacritty_terminal::vte::ansi::Processor<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        > = alacritty_terminal::vte::ansi::Processor::new();
        for i in 0..20 {
            let line = format!("LINE{:02}\r\n", i);
            parser.advance(&mut term, line.as_bytes());
        }

        let visible = extract_lines(&term);
        let all = extract_lines_with_scrollback(&term);
        assert_eq!(visible.len(), 5, "viewport stays at screen size");
        assert!(
            all.len() > visible.len(),
            "scrollback should contain older lines: visible={} all={}",
            visible.len(),
            all.len()
        );

        // The very first line emitted must be reachable via scrollback.
        let all_text: String = all
            .iter()
            .flat_map(|row| row.iter().map(|c| c.c))
            .collect();
        assert!(
            all_text.contains("LINE00"),
            "scrollback missing first line: {:?}",
            all_text
        );
    }

    #[test]
    fn get_history_size_matches_total_minus_screen() {
        use alacritty_terminal::grid::Dimensions;
        let mut term = build_terminal(20, 4);
        let mut parser: alacritty_terminal::vte::ansi::Processor<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        > = alacritty_terminal::vte::ansi::Processor::new();
        for i in 0..10 {
            let line = format!("X{:02}\r\n", i);
            parser.advance(&mut term, line.as_bytes());
        }
        let history = term.grid().history_size();
        let total = term.grid().total_lines();
        let screen = term.grid().screen_lines();
        assert_eq!(history, total - screen);
        assert!(history > 0, "should have produced history");
    }

    #[test]
    fn listener_captures_osc_title_and_bel() {
        // Build a Term wired to a FrostListener whose state we hold a
        // handle to, then drive it with an OSC 0 title set and a BEL.
        let listener = FrostListener::dummy();
        let state = listener.state();
        let mut term = build_terminal_with_listener(80, 24, listener);
        let mut parser: alacritty_terminal::vte::ansi::Processor<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        > = alacritty_terminal::vte::ansi::Processor::new();

        // OSC 0 ; <title> BEL  → title "frost".
        parser.advance(&mut term, b"\x1B]0;frost\x07");
        // A second BEL with no OSC — should land as Bell event.
        parser.advance(&mut term, b"\x07");

        let snap = state.lock().unwrap();
        assert_eq!(
            snap.title.as_deref(),
            Some("frost"),
            "OSC 0/2 should populate title"
        );
        assert!(snap.bell_at.is_some(), "BEL should set bell timestamp");
    }

    #[test]
    fn extract_lines_preserves_leading_spaces() {
        // Regression: a previous implementation dropped leading spaces and
        // collapsed indented output. Ensure spaces are kept so background
        // colours and column alignment are correct.
        let cols: u16 = 10;
        let rows: u16 = 2;
        let mut term = build_terminal(cols, rows);
        let mut parser: alacritty_terminal::vte::ansi::Processor<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        > = alacritty_terminal::vte::ansi::Processor::new();

        // Two spaces then 'X'.
        parser.advance(&mut term, b"  X");

        let lines = extract_lines(&term);
        let row0 = &lines[0];
        assert_eq!(row0.len(), cols as usize);
        assert_eq!(row0[0].c, ' ');
        assert_eq!(row0[1].c, ' ');
        assert_eq!(row0[2].c, 'X');
    }

    #[tokio::test]
    async fn resize_updates_both_pty_master_and_emulator_grid() {
        // After `ProcessManager::resize`, two invariants must hold:
        //   1. The kernel's view of the master PTY's winsize must reflect
        //      the new dims (proven by `MasterPty::get_size`).
        //   2. The internal alacritty `Term` grid must be re-sized so the
        //      renderer paints the correct number of cells.
        //
        // Note: we deliberately do **not** assert on the *child shell's*
        // perceived size via `stty size`. On macOS BSD PTYs that propagation
        // path has timing edges that make a deterministic test flaky in CI;
        // for the host-side contract this `MasterPty::get_size` check is
        // sufficient and is what every callsite of `resize` actually relies
        // on.
        let mut mgr = ProcessManager::new();

        mgr.start("p", "a", "s", "sleep 1", std::path::Path::new("/"), 80, 24)
            .expect("start should succeed");

        tokio::time::sleep(Duration::from_millis(80)).await;
        mgr.resize("p", "a", "s", 100, 30)
            .expect("resize should succeed");

        // Master-fd ioctl is now in effect.
        {
            let key = ("p".to_string(), "a".to_string(), "s".to_string());
            let state = mgr.processes.get(&key).expect("state");
            let size = state.pty.master.get_size().expect("get_size");
            assert_eq!(size.cols, 100, "master cols must reflect resize");
            assert_eq!(size.rows, 30, "master rows must reflect resize");
        }

        // Emulator grid is also resized so the renderer paints 30 rows of
        // 100 cells.
        let lines = mgr
            .get_display_lines("p", "a", "s")
            .expect("display lines");
        assert_eq!(lines.len(), 30, "term grid screen_lines == new rows");
        assert_eq!(
            lines.first().map(|r| r.len()).unwrap_or(0),
            100,
            "term grid columns == new cols"
        );

        let _ = mgr.stop("p", "a", "s");
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
