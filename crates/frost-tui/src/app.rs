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
    state::{AppState, Overlay},
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

        // Expand all projects by default.
        let mut expanded = HashSet::new();
        for project_name in config.projects.keys() {
            expanded.insert(project_name.clone());
        }

        Self {
            state: AppState::default(),
            process_manager: Arc::new(Mutex::new(process_manager)),
            config,
            expanded,
            flattened,
            config_path,
            theme_registry: ThemeRegistry::with_builtin_themes(),
            active_theme: "opencode".to_string(),
        }
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
            }
            Action::Up => self.nav_up(),
            Action::Down => self.nav_down(),
            Action::Toggle => self.toggle_selected(),
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
            if item.kind == TreeItemKind::Subcommand {
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

        // Sidebar.
        let pm = self.process_manager.lock().unwrap();
        frame.render_widget(
            Sidebar {
                config: &self.config,
                expanded: &self.expanded,
                selected_index: self.state.selected_index,
                process_manager: &*pm,
            },
            horiz[0],
        );

        // Log viewer.
        let log = if let Some((ref proj, ref app, ref sub)) = self.state.selected_process {
            LogViewer::from_manager(&*pm, proj, app, sub)
        } else {
            LogViewer::empty()
        };
        frame.render_widget(log, horiz[1]);

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
