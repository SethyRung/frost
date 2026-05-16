use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use frost_core::{
    FrostConfig, ProcessManager, ProcessStatus, RuntimeCommand, ThemeRegistry, flatten_config,
};

use crate::{
    actions::Action,
    command_bar::CommandBar,
    log_viewer::LogViewer,
    palette::{Palette, PaletteAction},
    search::SearchDialog,
    sidebar::{Sidebar, TreeItemKind, build_visible_tree},
    state::{AppState, Focus, Overlay},
    theme_dialog::ThemeDialog,
};

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

        let mut app = Self {
            state: AppState::default(),
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
            Action::ToggleFocus => self.toggle_focus(),
            Action::WriteInput(bytes) => self.write_to_process(&bytes),
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
        if let Some(item) = items.get(self.state.selected_index) {
            if item.kind == TreeItemKind::Terminal || item.kind == TreeItemKind::Subcommand {
                let parts: Vec<_> = item.path.split('/').collect();
                if parts.len() == 3 {
                    self.state.selected_process =
                        Some((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()));
                }
            }
        }
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
                    let workdir = self.flattened.iter()
                        .find(|c| c.project_name == project && c.app_name == app)
                        .map(|c| c.workdir.clone())
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    {
                        let mut pm = self.process_manager.lock().unwrap();
                        let _ = pm.start(project, app, subcommand, &shell, &workdir, 80, 24);
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

                let status = {
                    let pm = self.process_manager.lock().unwrap();
                    pm.get_info(project, app, subcommand)
                        .map(|info| info.status)
                        .unwrap_or(ProcessStatus::Stopped)
                };

                match status {
                    ProcessStatus::Stopped | ProcessStatus::Crashed => {
                        self.start_process(project, app, subcommand);
                    }
                    ProcessStatus::Running | ProcessStatus::Starting => {
                        self.stop_process(project, app, subcommand);
                    }
                    ProcessStatus::Stopping => {
                        // Already stopping — ignore.
                    }
                }
            }
        }
    }

    fn start_process(&mut self, project: &str, app: &str, subcommand: &str) {
        let rt_cmd = self
            .flattened
            .iter()
            .find(|c| {
                c.project_name == project
                    && c.app_name == app
                    && c.subcommand_name == subcommand
            })
            .cloned();

        let Some(rt_cmd) = rt_cmd else {
            return;
        };

        // Stop any other running subcommand in the same app.
        {
            let pm = self.process_manager.lock().unwrap();
            for info in pm.list() {
                if info.project == project
                    && info.app == app
                    && info.subcommand != subcommand
                    && (info.status == ProcessStatus::Running
                        || info.status == ProcessStatus::Starting)
                {
                    drop(pm);
                    let mut pm = self.process_manager.lock().unwrap();
                    let _ = pm.stop(project, app, &info.subcommand);
                    break;
                }
            }
        }

        {
            let mut pm = self.process_manager.lock().unwrap();
            let _ = pm.start(
                project,
                app,
                subcommand,
                &rt_cmd.command,
                &rt_cmd.workdir,
                80,
                24,
            );
        }

        self.update_selected_process();
    }

    fn stop_process(&mut self, project: &str, app: &str, subcommand: &str) {
        let mut pm = self.process_manager.lock().unwrap();
        let _ = pm.stop(project, app, subcommand);
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

    fn resize_all_processes(&mut self, width: u16, height: u16) {
        let cols = width.saturating_sub(30).max(20);
        let rows = height.saturating_sub(3).max(5);
        let mut pm = self.process_manager.lock().unwrap();
        for info in pm.list() {
            if info.status == ProcessStatus::Running || info.status == ProcessStatus::Starting {
                let _ = pm.resize(&info.project, &info.app, &info.subcommand, cols, rows);
            }
        }
    }

    fn scroll_up(&mut self) {
        self.state.log_scroll += 10;
    }

    fn scroll_down(&mut self) {
        if self.state.log_scroll > 10 {
            self.state.log_scroll -= 10;
        } else {
            self.state.log_scroll = 0;
        }
    }

    fn scroll_bottom(&mut self) {
        self.state.log_scroll = 0;
    }

    fn toggle_focus(&mut self) {
        self.state.focus = match self.state.focus {
            Focus::Sidebar => Focus::LogViewer,
            Focus::LogViewer => Focus::Sidebar,
        };
        if self.state.focus == Focus::LogViewer {
            self.update_selected_process();
        }
    }

    fn write_to_process(&mut self, data: &[u8]) {
        if let Some((ref proj, ref app, ref sub)) = self.state.selected_process {
            let mut pm = self.process_manager.lock().unwrap();
            let _ = pm.write_stdin(proj, app, sub, data);
        }
    }

    fn start_all_terminals(&mut self) {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut seen = std::collections::HashSet::new();
        for rt_cmd in &self.flattened {
            let key = format!("{}/{}", rt_cmd.project_name, rt_cmd.app_name);
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);
            let mut pm = self.process_manager.lock().unwrap();
            let _ = pm.start(
                &rt_cmd.project_name,
                &rt_cmd.app_name,
                "terminal",
                &shell,
                &rt_cmd.workdir,
                80,
                24,
            );
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

        // Log viewer.
        let log_area = horiz[1];
        let log_lines_len = if let Some((ref proj, ref app, ref sub)) = self.state.selected_process {
            pm.get_display_lines(proj, app, sub).map(|l| l.len()).unwrap_or(0)
        } else {
            0
        };
        let log = if let Some((ref proj, ref app, ref sub)) = self.state.selected_process {
            LogViewer::from_manager(&*pm, proj, app, sub, self.state.log_scroll, log_focused)
        } else {
            LogViewer::empty()
        };
        frame.render_widget(log, log_area);

        // Render cursor if process is active.
        if let Some((ref proj, ref app, ref sub)) = self.state.selected_process {
            if let Some((cursor_row, cursor_col)) = pm.get_cursor_position(proj, app, sub) {
                let inner_x = log_area.x + 1;
                let inner_y = log_area.y + 1;
                let visible_height = log_area.height.saturating_sub(2) as usize;
                let start = if log_lines_len > visible_height {
                    log_lines_len.saturating_sub(visible_height + self.state.log_scroll)
                } else {
                    0
                };
                let relative_row = cursor_row.saturating_sub(start);
                if relative_row < visible_height {
                    let cursor_x = inner_x + cursor_col as u16;
                    let cursor_y = inner_y + relative_row as u16;
                    frame.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
                }
            }
        }

        // Command bar.
        frame.render_widget(
            CommandBar {
                running_count: self.state.running_count,
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
