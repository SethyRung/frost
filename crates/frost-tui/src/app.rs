use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use frost_core::{
    FrostConfig, ProcessManager, ProcessStatus, RuntimeCommand, ThemeRegistry, flatten_config,
};

use crate::{
    actions::Action,
    command_bar::CommandBar,
    log_viewer::LogViewer,
    palette::{Palette, PaletteAction},
    search::SearchDialog,
    selection::{GridPoint, Selection},
    sidebar::{Sidebar, TreeItemKind, build_visible_tree},
    state::{AppState, Focus, Overlay},
    theme_dialog::ThemeDialog,
};

/// Visual constants used by the layout. Kept in one place so the PTY
/// resize calculation and the renderer can never drift apart.
const SIDEBAR_WIDTH: u16 = 30;
const COMMAND_BAR_HEIGHT: u16 = 3;
const PANE_BORDER: u16 = 2;
const MIN_PTY_COLS: u16 = 20;
const MIN_PTY_ROWS: u16 = 5;

/// Top-left cell of the log pane's inner area (i.e. just inside the
/// border, where the PTY cell at (0,0) renders). Mouse events arriving
/// from crossterm are in window-absolute coords; subtract this offset to
/// get PTY-local coords.
pub(crate) const LOG_PANE_INNER_X: u16 = SIDEBAR_WIDTH + 1;
pub(crate) const LOG_PANE_INNER_Y: u16 = 1;

/// Compute the PTY dimensions (cols, rows) that match the inner area of
/// the log viewer pane for a given full terminal `(width, height)`.
///
/// The log pane sits to the right of a fixed-width sidebar and above a
/// fixed-height command bar, and has a 1-cell border on every side. The
/// usable cell area is therefore `width - sidebar - 2` by
/// `height - command_bar - 2`. Returns sane minimums so the PTY never
/// gets zero or negative dimensions on tiny terminals.
pub(crate) fn log_pane_dims(terminal_width: u16, terminal_height: u16) -> (u16, u16) {
    let cols = terminal_width
        .saturating_sub(SIDEBAR_WIDTH)
        .saturating_sub(PANE_BORDER)
        .max(MIN_PTY_COLS);
    let rows = terminal_height
        .saturating_sub(COMMAND_BAR_HEIGHT)
        .saturating_sub(PANE_BORDER)
        .max(MIN_PTY_ROWS);
    (cols, rows)
}

/// Translate a window-absolute mouse coordinate into a 1-indexed PTY
/// cell coordinate, or `None` when the mouse is outside the log pane's
/// inner area. Returned values are 1-based to match SGR mouse encoding
/// (`\x1B[<b;x;y M/m`) directly.
pub(crate) fn window_to_pty_coords(
    window_col: u16,
    window_row: u16,
    pane_cols: u16,
    pane_rows: u16,
) -> Option<(u16, u16)> {
    if window_col < LOG_PANE_INNER_X || window_row < LOG_PANE_INNER_Y {
        return None;
    }
    let pty_col = window_col - LOG_PANE_INNER_X;
    let pty_row = window_row - LOG_PANE_INNER_Y;
    if pty_col >= pane_cols || pty_row >= pane_rows {
        return None;
    }
    Some((pty_col + 1, pty_row + 1))
}

/// 0-based grid translation of a window-absolute mouse coordinate. Same
/// gate as [`window_to_pty_coords`] but returns `(row, col)` aligned
/// with how the renderer indexes `LogViewer.lines`.
pub(crate) fn window_to_grid_coords(
    window_col: u16,
    window_row: u16,
    pane_cols: u16,
    pane_rows: u16,
) -> Option<GridPoint> {
    let (x, y) = window_to_pty_coords(window_col, window_row, pane_cols, pane_rows)?;
    Some(GridPoint {
        row: (y - 1) as usize,
        col: (x - 1) as usize,
    })
}

/// Build the SGR-mouse "Cb" parameter byte (button + modifiers + drag/motion
/// bits) following the xterm extended encoding used by DEC private mode
/// 1006.
///
/// Button base values:
///   - Left = 0, Middle = 1, Right = 2
///   - Drag adds 32; bare motion uses 35 (button 3 + motion)
///   - Wheel up = 64, wheel down = 65, wheel left = 66, wheel right = 67
///
/// Modifier bits: shift +4, alt +8, ctrl +16.
fn sgr_mouse_button_code(kind: MouseEventKind, mods: KeyModifiers) -> Option<(u16, bool)> {
    let (base, is_press) = match kind {
        MouseEventKind::Down(b) => (mouse_button_base(b), true),
        MouseEventKind::Up(b) => (mouse_button_base(b), false),
        MouseEventKind::Drag(b) => (mouse_button_base(b) + 32, true),
        MouseEventKind::Moved => (35, true),
        MouseEventKind::ScrollUp => (64, true),
        MouseEventKind::ScrollDown => (65, true),
        MouseEventKind::ScrollLeft => (66, true),
        MouseEventKind::ScrollRight => (67, true),
    };
    let mut code = base as u16;
    if mods.contains(KeyModifiers::SHIFT) {
        code += 4;
    }
    if mods.contains(KeyModifiers::ALT) {
        code += 8;
    }
    if mods.contains(KeyModifiers::CONTROL) {
        code += 16;
    }
    Some((code, is_press))
}

fn mouse_button_base(b: MouseButton) -> u8 {
    match b {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

/// Encode a mouse event for the child PTY in SGR (DEC 1006) form:
/// `\x1B[<Cb;x;yM` for press / motion, `\x1B[<Cb;x;ym` for release.
/// Returns `None` for kinds the child should not see (e.g. release of a
/// scroll wheel, which has no semantic).
pub(crate) fn encode_sgr_mouse(
    kind: MouseEventKind,
    mods: KeyModifiers,
    col_1based: u16,
    row_1based: u16,
) -> Option<Vec<u8>> {
    let (code, is_press) = sgr_mouse_button_code(kind, mods)?;
    let final_byte = if is_press { 'M' } else { 'm' };
    Some(format!("\x1B[<{};{};{}{}", code, col_1based, row_1based, final_byte).into_bytes())
}

/// Top-level application container.
pub struct App {
    pub state: AppState,
    pub process_manager: Arc<Mutex<ProcessManager>>,
    pub config: FrostConfig,
    pub expanded: HashSet<String>,
    pub flattened: Vec<RuntimeCommand>,
    #[allow(dead_code)]
    pub config_path: PathBuf,
    pub theme_registry: ThemeRegistry,
    pub active_theme: String,
}

impl App {
    pub fn new(
        process_manager: ProcessManager,
        config: FrostConfig,
        config_path: PathBuf,
        initial_terminal_size: (u16, u16),
    ) -> Self {
        let flattened = flatten_config(&config, &config_path);

        let mut expanded = HashSet::new();
        for project_name in config.projects.keys() {
            expanded.insert(project_name.clone());
        }
        for rt_cmd in &flattened {
            let app_path = format!("{}/{}", rt_cmd.project_name, rt_cmd.app_name);
            expanded.insert(app_path);
        }

        // Pretend we just resized to current dims so the first real Resize
        // event at the same size is a no-op.
        let state = AppState {
            terminal_size: initial_terminal_size,
            last_pty_dims: log_pane_dims(initial_terminal_size.0, initial_terminal_size.1),
            ..AppState::default()
        };

        let mut app = Self {
            state,
            process_manager: Arc::new(Mutex::new(process_manager)),
            config,
            expanded,
            flattened,
            config_path,
            theme_registry: ThemeRegistry::with_builtin_themes(),
            active_theme: "opencode".to_string(),
        };

        app.start_all_terminals();
        app.auto_select_first();

        app
    }

    /// Apply an incoming action to mutate state.
    pub fn handle_action(&mut self, action: Action) {
        // Handle overlay-specific actions first.
        if self.state.overlay.is_some() {
            self.handle_overlay_action(action);
            return;
        }

        match action {
            Action::Quit => self.state.should_quit = true,
            Action::Tick => {
                self.state.tick_count += 1;
                self.update_running_count();
            }
            Action::Resize { width, height } => {
                self.state.terminal_size = (width, height);
                self.resize_all_processes(width, height);
            }
            Action::Up => self.nav_up(),
            Action::Down => self.nav_down(),
            Action::Toggle => self.toggle_selected(),
            Action::ScrollUp => self.scroll_up(),
            Action::ScrollDown => self.scroll_down(),
            Action::ScrollBottom => self.scroll_bottom(),
            Action::ScrollTop => self.scroll_top(),
            Action::ToggleFocus => self.toggle_focus(),
            Action::WriteInput(bytes) => self.write_to_process(&bytes),
            Action::Paste(text) => self.paste_into_process(&text),
            Action::Mouse(m) => self.handle_mouse(m),
            Action::CopySelection => self.copy_selection_to_clipboard(),
            Action::OpenPalette => {
                self.state.overlay = Some(Overlay::Palette);
                self.state.filter_text.clear();
                self.state.overlay_selected = 0;
            }
            Action::OpenSearch => {
                self.state.overlay = Some(Overlay::Search);
                self.state.filter_text.clear();
                self.state.overlay_selected = 0;
            }
            _ => {}
        }
    }

    fn handle_overlay_action(&mut self, action: Action) {
        match action {
            Action::CloseOverlay | Action::Quit => {
                self.state.overlay = None;
                self.state.filter_text.clear();
                self.state.overlay_selected = 0;
            }
            Action::Up => {
                if self.state.overlay_selected > 0 {
                    self.state.overlay_selected -= 1;
                }
            }
            Action::Down => {
                self.state.overlay_selected += 1;
            }
            Action::Confirm => {
                self.confirm_overlay();
            }
            Action::FilterChar(c) => {
                self.state.filter_text.push(c);
                self.state.overlay_selected = 0;
            }
            Action::FilterBackspace => {
                self.state.filter_text.pop();
                self.state.overlay_selected = 0;
            }
            Action::FilterClear => {
                self.state.filter_text.clear();
                self.state.overlay_selected = 0;
            }
            _ => {}
        }
    }

    fn confirm_overlay(&mut self) {
        match self.state.overlay {
            Some(Overlay::Palette) => {
                let palette = Palette {
                    items: crate::palette::default_items(),
                    selected: self.state.overlay_selected,
                    filter: self.state.filter_text.clone(),
                };
                if let Some(action) = palette.selected_action() {
                    match action {
                        PaletteAction::SwitchTheme => {
                            self.state.overlay = Some(Overlay::ThemeDialog);
                            self.state.filter_text.clear();
                            self.state.overlay_selected = 0;
                            return;
                        }
                        PaletteAction::ReloadConfig => {
                            // TODO: reload config
                        }
                        PaletteAction::Quit => {
                            self.state.should_quit = true;
                        }
                    }
                }
                self.state.overlay = None;
                self.state.filter_text.clear();
                self.state.overlay_selected = 0;
            }
            Some(Overlay::Search) => {
                let items = build_visible_tree(&self.config, &self.expanded);
                let dialog = SearchDialog {
                    items,
                    selected: self.state.overlay_selected,
                    filter: self.state.filter_text.clone(),
                };
                if let Some(item) = dialog.selected_item() {
                    // Find the index of this item in the visible tree.
                    let all_items = build_visible_tree(&self.config, &self.expanded);
                    if let Some(idx) = all_items.iter().position(|i| i.path == item.path) {
                        self.state.selected_index = idx;
                        self.update_selected_process();
                    }
                }
                self.state.overlay = None;
                self.state.filter_text.clear();
                self.state.overlay_selected = 0;
            }
            Some(Overlay::ThemeDialog) => {
                let themes = self.theme_registry.get_ids();
                let dialog = ThemeDialog {
                    themes,
                    selected: self.state.overlay_selected,
                    filter: self.state.filter_text.clone(),
                    active_theme: self.active_theme.clone(),
                };
                if let Some(theme) = dialog.selected_theme() {
                    self.active_theme = theme;
                }
                self.state.overlay = None;
                self.state.filter_text.clear();
                self.state.overlay_selected = 0;
            }
            None => {}
        }
    }

    fn nav_up(&mut self) {
        if self.state.selected_index > 0 {
            self.state.selected_index -= 1;
        }
        self.update_selected_process();
    }

    fn nav_down(&mut self) {
        let items = build_visible_tree(&self.config, &self.expanded);
        if self.state.selected_index + 1 < items.len() {
            self.state.selected_index += 1;
        }
        self.update_selected_process();
    }

    fn update_selected_process(&mut self) {
        let items = build_visible_tree(&self.config, &self.expanded);
        if let Some(item) = items.get(self.state.selected_index)
            && (item.kind == TreeItemKind::Terminal || item.kind == TreeItemKind::Subcommand)
        {
            let parts: Vec<_> = item.path.split('/').collect();
            if parts.len() == 3 {
                let new = (
                    parts[0].to_string(),
                    parts[1].to_string(),
                    parts[2].to_string(),
                );
                // Switching to a different process invalidates the
                // selection — coords no longer map to the visible grid.
                if self.state.selected_process.as_ref() != Some(&new) {
                    self.state.selection = None;
                }
                self.state.selected_process = Some(new);
                // Navigating onto a subcommand row that has never been
                // run leaves it stopped, which is confusing: the user
                // sees an empty log pane and typing does nothing. Spawn
                // a real interactive shell so the row immediately acts
                // like a terminal. Pressing Enter on the row will swap
                // the shell out for the configured command.
                self.ensure_shell_for_selected();
            }
        }
    }

    /// Spawn `$SHELL` on the selected process key if nothing is running
    /// there yet. Marks the new process as [`ProcessPurpose::Shell`] so
    /// the next Enter on the row knows to swap to the configured
    /// command. No-op when a process is already running for this key.
    fn ensure_shell_for_selected(&mut self) {
        let Some((proj, app, sub)) = self.state.selected_process.clone() else {
            return;
        };
        let already_running = {
            let pm = self.process_manager.lock().unwrap();
            pm.get_info(&proj, &app, &sub)
                .map(|info| {
                    matches!(
                        info.status,
                        ProcessStatus::Running | ProcessStatus::Starting
                    )
                })
                .unwrap_or(false)
        };
        if already_running {
            return;
        }

        // Resolve a workdir. For a subcommand row use that subcommand's
        // resolved workdir; for a "terminal" row fall back to any
        // sibling subcommand's workdir, then to the config dir.
        let workdir = self
            .flattened
            .iter()
            .find(|c| {
                c.project_name == proj && c.app_name == app && c.subcommand_name == sub
            })
            .or_else(|| {
                self.flattened
                    .iter()
                    .find(|c| c.project_name == proj && c.app_name == app)
            })
            .map(|c| c.workdir.clone())
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let (cols, rows) = self.current_pty_dims();
        let mut pm = self.process_manager.lock().unwrap();
        let _ = pm.start(&proj, &app, &sub, &shell, &workdir, cols, rows);
    }

    fn toggle_selected(&mut self) {
        let items = build_visible_tree(&self.config, &self.expanded);
        let Some(item) = items.get(self.state.selected_index).cloned() else {
            return;
        };

        match item.kind {
            TreeItemKind::Project | TreeItemKind::App => {
                if self.expanded.contains(&item.path) {
                    self.expanded.remove(&item.path);
                } else {
                    self.expanded.insert(item.path);
                }
            }
            TreeItemKind::Terminal => {
                let parts: Vec<_> = item.path.split('/').collect();
                if parts.len() != 3 {
                    return;
                }
                let (project, app, subcommand) = (parts[0], parts[1], parts[2]);

                let status = {
                    let pm = self.process_manager.lock().unwrap();
                    pm.get_info(project, app, subcommand)
                        .map(|info| info.status)
                        .unwrap_or(ProcessStatus::Stopped)
                };

                if status == ProcessStatus::Stopped || status == ProcessStatus::Crashed {
                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
                    let workdir = self
                        .flattened
                        .iter()
                        .find(|c| c.project_name == project && c.app_name == app)
                        .map(|c| c.workdir.clone())
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    let (cols, rows) = self.current_pty_dims();
                    {
                        let mut pm = self.process_manager.lock().unwrap();
                        let _ = pm.start(project, app, subcommand, &shell, &workdir, cols, rows);
                    }
                    self.update_selected_process();
                }
            }
            TreeItemKind::Subcommand => {
                let parts: Vec<_> = item.path.split('/').collect();
                if parts.len() != 3 {
                    return;
                }
                let (project, app, subcommand) = (parts[0], parts[1], parts[2]);
                self.run_command_in_shell(project, app, subcommand);
            }
        }
    }

    /// Pressing Enter on a subcommand row runs its configured command
    /// *inside* the auto-spawned interactive shell for that row, the
    /// same way a human would by typing the command at the prompt. The
    /// command therefore inherits the shell's PTY, its job-control
    /// signals, and crucially the user's ability to interact with it
    /// (Ctrl+C to stop, arrow keys for in-process navigation, etc.).
    ///
    /// First we make sure the row's shell is alive; then we write
    /// `<command>\n` to its stdin. Re-pressing Enter just runs the
    /// command again — there is no separate "stop" state because the
    /// running command is already cancellable with Ctrl+C from the log
    /// viewer (which forwards `0x03` to the controlling tty thanks to
    /// PR4's input rework).
    fn run_command_in_shell(&mut self, project: &str, app: &str, subcommand: &str) {
        let rt_cmd = self
            .flattened
            .iter()
            .find(|c| {
                c.project_name == project && c.app_name == app && c.subcommand_name == subcommand
            })
            .cloned();
        let Some(rt_cmd) = rt_cmd else {
            return;
        };

        // Guarantee the shell PTY exists before injecting anything.
        self.ensure_shell_for_selected();

        let mut line = rt_cmd.command;
        if !line.ends_with('\n') {
            line.push('\n');
        }
        let mut pm = self.process_manager.lock().unwrap();
        let _ = pm.write_stdin(project, app, subcommand, line.as_bytes());
    }

    /// Resolve the current `(cols, rows)` the log pane should render. Uses
    /// the most recent terminal size if known, otherwise falls back to a
    /// reasonable 80x24 default so processes started before the first
    /// Resize event still get sensible dims.
    fn current_pty_dims(&self) -> (u16, u16) {
        let (w, h) = self.state.terminal_size;
        if w == 0 || h == 0 {
            (80, 24)
        } else {
            log_pane_dims(w, h)
        }
    }

    fn update_running_count(&mut self) {
        let pm = self.process_manager.lock().unwrap();
        self.state.running_count = pm
            .list()
            .iter()
            .filter(|info| {
                info.status == ProcessStatus::Running || info.status == ProcessStatus::Starting
            })
            .count();
    }

    /// Recompute the log-pane PTY dimensions from the current terminal
    /// size and forward them to every running process. Dedupes against
    /// `last_pty_dims` so continuous drag-resize events don't flood the
    /// PTY with SIGWINCHes.
    fn resize_all_processes(&mut self, width: u16, height: u16) {
        let (cols, rows) = log_pane_dims(width, height);
        if (cols, rows) == self.state.last_pty_dims {
            return;
        }
        self.state.last_pty_dims = (cols, rows);

        let mut pm = self.process_manager.lock().unwrap();
        for info in pm.list() {
            if info.status == ProcessStatus::Running || info.status == ProcessStatus::Starting {
                let _ = pm.resize(&info.project, &info.app, &info.subcommand, cols, rows);
            }
        }
    }

    /// Step size for scrollback PageUp/PageDown / wheel — half the
    /// visible pane, with a 1-line minimum so tiny terminals still
    /// scroll.
    fn scroll_step(&self) -> usize {
        let (_, rows) = self.current_pty_dims();
        (rows as usize / 2).max(1)
    }

    fn max_scroll(&self) -> usize {
        let Some((proj, app, sub)) = self.state.selected_process.as_ref() else {
            return 0;
        };
        let pm = self.process_manager.lock().unwrap();
        pm.get_history_size(proj, app, sub)
    }

    fn scroll_up(&mut self) {
        let step = self.scroll_step();
        let cap = self.max_scroll();
        self.state.log_scroll = (self.state.log_scroll + step).min(cap);
    }

    fn scroll_down(&mut self) {
        let step = self.scroll_step();
        self.state.log_scroll = self.state.log_scroll.saturating_sub(step);
    }

    fn scroll_bottom(&mut self) {
        self.state.log_scroll = 0;
    }

    fn scroll_top(&mut self) {
        self.state.log_scroll = self.max_scroll();
    }

    fn toggle_focus(&mut self) {
        self.state.focus = match self.state.focus {
            Focus::Sidebar => Focus::LogViewer,
            Focus::LogViewer => Focus::Sidebar,
        };
        if self.state.focus == Focus::LogViewer {
            // `update_selected_process` already runs `ensure_shell_for_selected`
            // so focusing the log on a row with no live PTY auto-spawns the
            // interactive shell — Log (interactive) is always typeable.
            self.update_selected_process();
        }
    }

    fn write_to_process(&mut self, data: &[u8]) {
        if let Some((ref proj, ref app, ref sub)) = self.state.selected_process {
            let mut pm = self.process_manager.lock().unwrap();
            let _ = pm.write_stdin(proj, app, sub, data);
        }
    }

    /// Route a mouse event:
    ///   * If the child has any mouse-reporting mode active and the
    ///     event falls inside the log pane, encode in SGR and forward.
    ///   * Else mouse-down/drag in the pane updates the local text
    ///     selection so the user can copy log output without the child
    ///     swallowing the events.
    ///   * Wheel in the pane bumps `log_scroll` for scrollback.
    ///   * Otherwise drop.
    ///
    /// We do **not** require the log pane to be focused for forwarding —
    /// users expect mouse interactions in the log area to "just work" the
    /// moment the pointer is there, mirroring how real terminals behave.
    fn handle_mouse(&mut self, event: MouseEvent) {
        let Some((proj, app, sub)) = self.state.selected_process.clone() else {
            return;
        };
        let (cols, rows) = self.current_pty_dims();
        let in_pane_pty = window_to_pty_coords(event.column, event.row, cols, rows);
        let in_pane_screen = window_to_grid_coords(event.column, event.row, cols, rows);

        let (modes, history_size) = {
            let pm = self.process_manager.lock().unwrap();
            (
                pm.mouse_modes(&proj, &app, &sub),
                pm.get_history_size(&proj, &app, &sub),
            )
        };

        if modes.reporting()
            && modes.sgr
            && let Some((x, y)) = in_pane_pty
            && let Some(bytes) = encode_sgr_mouse(event.kind, event.modifiers, x, y)
        {
            let mut pm = self.process_manager.lock().unwrap();
            let _ = pm.write_stdin(&proj, &app, &sub, &bytes);
            return;
        }

        // Translate a screen-relative grid point into the buffer-relative
        // address that the selection model uses. With scrollback present,
        // the same window cell maps to a different absolute row depending
        // on `log_scroll`; storing buffer rows means the highlight
        // survives subsequent scrolling.
        let in_pane_buffer = in_pane_screen.map(|p| {
            let visible_height = rows as usize;
            let total = history_size + visible_height;
            let start = total.saturating_sub(visible_height + self.state.log_scroll);
            GridPoint {
                row: start + p.row,
                col: p.col,
            }
        });

        // Local handling — the child isn't capturing mouse, so the
        // events drive selection / scrollback in the TUI itself.
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(point) = in_pane_buffer {
                    if event.modifiers.contains(KeyModifiers::SHIFT) {
                        match self.state.selection.as_mut() {
                            Some(sel) => sel.extend_to(point),
                            None => self.state.selection = Some(Selection::at(point)),
                        }
                    } else {
                        self.state.selection = Some(Selection::at(point));
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(point) = in_pane_buffer
                    && let Some(sel) = self.state.selection.as_mut()
                {
                    sel.extend_to(point);
                }
            }
            MouseEventKind::ScrollUp if in_pane_pty.is_some() => self.scroll_up(),
            MouseEventKind::ScrollDown if in_pane_pty.is_some() => self.scroll_down(),
            _ => {}
        }
    }

    /// Push the current selection (if any) to the system clipboard. The
    /// selected region is read straight out of the renderer's view of the
    /// emulator grid, with trailing whitespace stripped per row and rows
    /// joined with `\n` so paste targets receive plain text.
    fn copy_selection_to_clipboard(&mut self) {
        let Some(sel) = self.state.selection else {
            return;
        };
        let Some((proj, app, sub)) = self.state.selected_process.clone() else {
            return;
        };

        let lines = {
            let pm = self.process_manager.lock().unwrap();
            pm.get_lines_with_scrollback(&proj, &app, &sub)
                .unwrap_or_default()
        };
        if lines.is_empty() {
            return;
        }
        let cols = lines[0].len();
        let mut buf = String::new();
        for (i, range) in sel.iter_row_ranges(cols).iter().enumerate() {
            if range.row >= lines.len() {
                break;
            }
            let row = &lines[range.row];
            let end = range.end_col.min(row.len().saturating_sub(1));
            let mut row_text: String = (range.start_col..=end).map(|c| row[c].c).collect();
            // Trim only the trailing fill spaces alacritty pads to the
            // grid width — leading whitespace is part of the user's
            // selection and must be preserved.
            let trimmed_end = row_text.trim_end_matches(' ').len();
            row_text.truncate(trimmed_end);
            if i > 0 {
                buf.push('\n');
            }
            buf.push_str(&row_text);
        }
        if buf.is_empty() {
            return;
        }
        // arboard returns an error on platforms where the clipboard is
        // unavailable (e.g. headless CI); swallow it so the TUI keeps
        // running.
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(buf);
        }
    }

    /// Forward pasted text to the focused process. When the child has
    /// enabled DEC private mode 2004 (bracketed paste) the text is
    /// wrapped in `\x1B[200~ … \x1B[201~` so the child can distinguish
    /// it from typed input; otherwise the bytes are sent raw.
    fn paste_into_process(&mut self, text: &str) {
        let Some((proj, app, sub)) = self.state.selected_process.clone() else {
            return;
        };
        let mut pm = self.process_manager.lock().unwrap();
        let bracketed = pm.is_bracketed_paste_active(&proj, &app, &sub);
        let payload = if bracketed {
            let mut buf = Vec::with_capacity(text.len() + 12);
            buf.extend_from_slice(b"\x1B[200~");
            buf.extend_from_slice(text.as_bytes());
            buf.extend_from_slice(b"\x1B[201~");
            buf
        } else {
            text.as_bytes().to_vec()
        };
        let _ = pm.write_stdin(&proj, &app, &sub, &payload);
    }

    fn start_all_terminals(&mut self) {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let (cols, rows) = self.current_pty_dims();
        let mut seen = std::collections::HashSet::new();
        for rt_cmd in &self.flattened {
            let key = format!("{}/{}", rt_cmd.project_name, rt_cmd.app_name);
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);
            {
                let mut pm = self.process_manager.lock().unwrap();
                let _ = pm.start(
                    &rt_cmd.project_name,
                    &rt_cmd.app_name,
                    "terminal",
                    &shell,
                    &rt_cmd.workdir,
                    cols,
                    rows,
                );
            }
        }
    }

    fn auto_select_first(&mut self) {
        let items = build_visible_tree(&self.config, &self.expanded);
        for (i, item) in items.iter().enumerate() {
            if item.kind == TreeItemKind::Terminal {
                self.state.selected_index = i;
                self.update_selected_process();
                return;
            }
        }
    }

    /// Stop all running processes.
    pub fn shutdown(&mut self) {
        let mut pm = self.process_manager.lock().unwrap();
        let infos: Vec<_> = pm.list();
        for info in infos {
            if info.status == ProcessStatus::Running || info.status == ProcessStatus::Starting {
                let _ = pm.stop(&info.project, &info.app, &info.subcommand);
            }
        }
    }

    /// Render the full layout.
    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        // Vertical split: main area + command bar at the bottom.
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        // Horizontal split: sidebar (left) + log viewer (right).
        let horiz = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(1)])
            .split(vert[0]);

        let sidebar_focused = self.state.focus == Focus::Sidebar;
        let log_focused = self.state.focus == Focus::LogViewer;

        // Sidebar.
        let pm = self.process_manager.lock().unwrap();
        frame.render_widget(
            Sidebar {
                config: &self.config,
                expanded: &self.expanded,
                selected_index: self.state.selected_index,
                process_manager: &*pm,
                focused: sidebar_focused,
            },
            horiz[0],
        );

        // Log viewer. `log_lines_len` is now the *full* buffer length
        // (scrollback history + viewport), so the cursor offset math
        // below has to account for the history rows that prefix the
        // visible viewport.
        let log_area = horiz[1];
        let (log_lines_len, history_size) =
            if let Some((ref proj, ref app, ref sub)) = self.state.selected_process {
                let total = pm
                    .get_lines_with_scrollback(proj, app, sub)
                    .map(|l| l.len())
                    .unwrap_or(0);
                let history = pm.get_history_size(proj, app, sub);
                (total, history)
            } else {
                (0, 0)
            };
        let log = if let Some((ref proj, ref app, ref sub)) = self.state.selected_process {
            let bell_active = pm
                .get_bell_age(proj, app, sub)
                .map(|age| age < std::time::Duration::from_millis(150))
                .unwrap_or(false);
            LogViewer::from_manager(
                &*pm,
                proj,
                app,
                sub,
                self.state.log_scroll,
                log_focused,
                self.state.selection,
                bell_active,
            )
        } else {
            LogViewer::empty()
        };
        frame.render_widget(log, log_area);

        // Render cursor if process is active and emulator says cursor is visible.
        if let Some((ref proj, ref app, ref sub)) = self.state.selected_process
            && pm.is_cursor_visible(proj, app, sub)
            && self.state.log_scroll == 0
            && let Some((cursor_row, cursor_col)) = pm.get_cursor_position(proj, app, sub)
        {
            let inner_x = log_area.x + 1;
            let inner_y = log_area.y + 1;
            let visible_height = log_area.height.saturating_sub(2) as usize;
            let start = if log_lines_len > visible_height {
                log_lines_len.saturating_sub(visible_height + self.state.log_scroll)
            } else {
                0
            };
            // `cursor_row` from the emulator is screen-relative
            // (0..screen_lines); buffer-relative row is `history + cursor_row`.
            let cursor_buffer_row = history_size + cursor_row;
            let relative_row = cursor_buffer_row.saturating_sub(start);
            if relative_row < visible_height {
                let cursor_x = inner_x + cursor_col as u16;
                let cursor_y = inner_y + relative_row as u16;
                frame.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
            }
        }

        // Command bar.
        frame.render_widget(
            CommandBar {
                running_count: self.state.running_count,
                focus: self.state.focus,
            },
            vert[1],
        );

        // Render overlay on top if open.
        match self.state.overlay {
            Some(Overlay::Palette) => {
                let mut palette = Palette {
                    items: crate::palette::default_items(),
                    selected: self.state.overlay_selected,
                    filter: self.state.filter_text.clone(),
                };
                palette.clamp_selection();
                frame.render_widget(palette, area);
            }
            Some(Overlay::Search) => {
                let items = build_visible_tree(&self.config, &self.expanded);
                let mut dialog = SearchDialog {
                    items,
                    selected: self.state.overlay_selected,
                    filter: self.state.filter_text.clone(),
                };
                dialog.clamp_selection();
                frame.render_widget(dialog, area);
            }
            Some(Overlay::ThemeDialog) => {
                let themes = self.theme_registry.get_ids();
                let mut dialog = ThemeDialog {
                    themes,
                    selected: self.state.overlay_selected,
                    filter: self.state.filter_text.clone(),
                    active_theme: self.active_theme.clone(),
                };
                dialog.clamp_selection();
                frame.render_widget(dialog, area);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_pane_dims_typical_terminal() {
        // 120x30 terminal → sidebar 30, command bar 3, 2-cell border on log pane:
        // cols = 120 - 30 - 2 = 88 ; rows = 30 - 3 - 2 = 25.
        let (cols, rows) = log_pane_dims(120, 30);
        assert_eq!(cols, 88);
        assert_eq!(rows, 25);
    }

    #[test]
    fn log_pane_dims_clamps_to_minimum_on_tiny_terminal() {
        // Smaller than sidebar+border → cols must clamp to MIN_PTY_COLS.
        let (cols, rows) = log_pane_dims(10, 4);
        assert_eq!(cols, MIN_PTY_COLS);
        assert_eq!(rows, MIN_PTY_ROWS);
    }

    #[test]
    fn log_pane_dims_large_terminal() {
        // Standard 1080p-ish terminal.
        let (cols, rows) = log_pane_dims(220, 60);
        assert_eq!(cols, 220 - 30 - 2);
        assert_eq!(rows, 60 - 3 - 2);
    }

    #[test]
    fn log_pane_dims_zero_dimensions_clamp() {
        let (cols, rows) = log_pane_dims(0, 0);
        assert_eq!(cols, MIN_PTY_COLS);
        assert_eq!(rows, MIN_PTY_ROWS);
    }

    #[test]
    fn window_to_pty_coords_inside_pane_is_one_based() {
        // Mouse at the very top-left of the inner log area should map to
        // PTY cell (1, 1) since SGR mouse uses 1-based indexing.
        let cell = window_to_pty_coords(LOG_PANE_INNER_X, LOG_PANE_INNER_Y, 80, 24);
        assert_eq!(cell, Some((1, 1)));

        // A cell further in.
        let cell = window_to_pty_coords(LOG_PANE_INNER_X + 4, LOG_PANE_INNER_Y + 2, 80, 24);
        assert_eq!(cell, Some((5, 3)));
    }

    #[test]
    fn window_to_pty_coords_outside_pane_is_none() {
        // Inside sidebar.
        assert_eq!(window_to_pty_coords(5, 5, 80, 24), None);
        // Right of pane.
        assert_eq!(
            window_to_pty_coords(LOG_PANE_INNER_X + 80, LOG_PANE_INNER_Y, 80, 24),
            None
        );
        // Below pane.
        assert_eq!(
            window_to_pty_coords(LOG_PANE_INNER_X, LOG_PANE_INNER_Y + 24, 80, 24),
            None
        );
    }

    #[test]
    fn encode_sgr_mouse_left_press_no_mods() {
        let bytes =
            encode_sgr_mouse(MouseEventKind::Down(MouseButton::Left), KeyModifiers::NONE, 1, 1)
                .unwrap();
        assert_eq!(bytes, b"\x1B[<0;1;1M");
    }

    #[test]
    fn encode_sgr_mouse_left_release_is_lowercase_m() {
        let bytes =
            encode_sgr_mouse(MouseEventKind::Up(MouseButton::Left), KeyModifiers::NONE, 5, 3)
                .unwrap();
        assert_eq!(bytes, b"\x1B[<0;5;3m");
    }

    #[test]
    fn encode_sgr_mouse_right_with_ctrl_shift() {
        // Right (2) + Shift (4) + Ctrl (16) = 22.
        let bytes = encode_sgr_mouse(
            MouseEventKind::Down(MouseButton::Right),
            KeyModifiers::SHIFT | KeyModifiers::CONTROL,
            10,
            7,
        )
        .unwrap();
        assert_eq!(bytes, b"\x1B[<22;10;7M");
    }

    #[test]
    fn encode_sgr_mouse_drag_adds_32() {
        // Left drag = 0 + 32 = 32.
        let bytes =
            encode_sgr_mouse(MouseEventKind::Drag(MouseButton::Left), KeyModifiers::NONE, 4, 2)
                .unwrap();
        assert_eq!(bytes, b"\x1B[<32;4;2M");
    }

    #[test]
    fn encode_sgr_mouse_wheel_up_is_64() {
        let bytes =
            encode_sgr_mouse(MouseEventKind::ScrollUp, KeyModifiers::NONE, 12, 8).unwrap();
        assert_eq!(bytes, b"\x1B[<64;12;8M");
    }

    #[test]
    fn encode_sgr_mouse_wheel_down_is_65() {
        let bytes =
            encode_sgr_mouse(MouseEventKind::ScrollDown, KeyModifiers::NONE, 12, 8).unwrap();
        assert_eq!(bytes, b"\x1B[<65;12;8M");
    }
}
