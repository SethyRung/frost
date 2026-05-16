use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use frost_core::{FrostConfig, ProcessManager, ProcessStatus, ResolvedTheme};

use crate::state::AppState;
use crate::theme_adapter::to_color;

/// A visible node in the flattened sidebar tree.
#[derive(Debug, Clone)]
pub struct TreeItem {
    pub kind: TreeItemKind,
    pub name: String,
    pub path: String,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeItemKind {
    Project,
    App,
    Subcommand,
    Terminal,
}

/// Build the flattened list of visible tree items from config + expanded state.
pub fn build_visible_tree(config: &FrostConfig, expanded: &HashSet<String>) -> Vec<TreeItem> {
    let mut items = Vec::new();
    let mut project_names: Vec<_> = config.projects.keys().cloned().collect();
    project_names.sort();

    for project_name in project_names {
        let project = config.projects.get(&project_name).unwrap();
        items.push(TreeItem {
            kind: TreeItemKind::Project,
            name: project_name.clone(),
            path: project_name.clone(),
            depth: 0,
        });

        if !expanded.contains(&project_name) {
            continue;
        }

        let mut app_names: Vec<_> = project.apps.keys().cloned().collect();
        app_names.sort();

        for app_name in app_names {
            let app = project.apps.get(&app_name).unwrap();
            let app_path = format!("{}/{}", project_name, app_name);
            items.push(TreeItem {
                kind: TreeItemKind::App,
                name: app_name.clone(),
                path: app_path.clone(),
                depth: 1,
            });

            if !expanded.contains(&app_path) {
                continue;
            }

            let term_path = format!("{}/{}/terminal", project_name, app_name);
            items.push(TreeItem {
                kind: TreeItemKind::Terminal,
                name: "terminal".to_string(),
                path: term_path,
                depth: 2,
            });

            let mut sub_names: Vec<_> = if let Some(cmds) = &app.commands {
                cmds.keys().cloned().collect()
            } else if app.command.is_some() {
                vec!["default".to_string()]
            } else {
                Vec::new()
            };
            sub_names.sort();

            for sub_name in sub_names {
                let sub_path = format!("{}/{}/{}", project_name, app_name, sub_name);
                items.push(TreeItem {
                    kind: TreeItemKind::Subcommand,
                    name: sub_name.clone(),
                    path: sub_path,
                    depth: 2,
                });
            }
        }
    }

    items
}

/// Get the process status for a subcommand.
fn get_status(
    manager: &ProcessManager,
    project: &str,
    app: &str,
    subcommand: &str,
) -> ProcessStatus {
    manager
        .get_info(project, app, subcommand)
        .map(|info| info.status)
        .unwrap_or(ProcessStatus::Stopped)
}

/// Render the sidebar tree widget.
pub struct Sidebar<'a> {
    pub config: &'a FrostConfig,
    pub expanded: &'a HashSet<String>,
    pub selected_index: usize,
    pub process_manager: &'a ProcessManager,
    pub focused: bool,
    pub theme: Option<&'a ResolvedTheme>,
}

impl<'a> Widget for Sidebar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if let Some(t) = self.theme {
            if self.focused {
                to_color(t.border_active)
            } else {
                to_color(t.border)
            }
        } else if self.focused {
            ratatui::style::Color::Cyan
        } else {
            ratatui::style::Color::DarkGray
        };

        let block = Block::default()
            .title(" Projects ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        let items = build_visible_tree(self.config, self.expanded);
        if items.is_empty() {
            let msg_color = self
                .theme
                .map(|t| to_color(t.text_muted))
                .unwrap_or(ratatui::style::Color::DarkGray);
            let msg = Span::styled("No projects configured", Style::default().fg(msg_color));
            buf.set_line(inner.x, inner.y, &Line::from(msg), inner.width);
            return;
        }

        let selected_index = self.selected_index.min(items.len().saturating_sub(1));
        let visible_height = inner.height as usize;
        let start = if items.len() > visible_height {
            let max_start = items.len().saturating_sub(visible_height);
            selected_index.min(max_start)
        } else {
            0
        };

        let accent = self.theme.map(|t| to_color(t.accent));
        let text = self.theme.map(|t| to_color(t.text));
        let text_muted = self.theme.map(|t| to_color(t.text_muted));
        let success = self.theme.map(|t| to_color(t.success));
        let warning = self.theme.map(|t| to_color(t.warning));
        let error = self.theme.map(|t| to_color(t.error));
        let bg_panel = self.theme.map(|t| to_color(t.background_panel));

        for (i, item) in items.iter().skip(start).take(visible_height).enumerate() {
            let row = inner.y + i as u16;
            if row >= inner.y + inner.height {
                break;
            }

            let is_selected = start + i == selected_index;
            let base_style = if is_selected {
                let mut s = Style::default();
                if let Some(c) = bg_panel {
                    s = s.bg(c);
                } else {
                    s = s.bg(ratatui::style::Color::DarkGray);
                }
                s
            } else {
                Style::default()
            };

            let mut spans: Vec<Span> = Vec::new();

            if is_selected {
                spans.push(Span::styled(
                    "› ",
                    base_style
                        .fg(accent.unwrap_or(ratatui::style::Color::Cyan))
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled("  ", base_style));
            }

            spans.push(Span::styled("  ".repeat(item.depth), base_style));

            match item.kind {
                TreeItemKind::Project | TreeItemKind::App => {
                    let chevron = if self.expanded.contains(&item.path) {
                        "▾ "
                    } else {
                        "▸ "
                    };
                    spans.push(Span::styled(
                        chevron,
                        base_style.fg(text_muted.unwrap_or(ratatui::style::Color::DarkGray)),
                    ));

                    let icon = match item.kind {
                        TreeItemKind::Project => self
                            .config
                            .projects
                            .get(&item.path)
                            .and_then(|p| p.icon.clone())
                            .unwrap_or_default(),
                        TreeItemKind::App => {
                            let parts: Vec<_> = item.path.split('/').collect();
                            if parts.len() == 2 {
                                self.config
                                    .projects
                                    .get(parts[0])
                                    .and_then(|p| p.apps.get(parts[1]))
                                    .and_then(|a| a.icon.clone())
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            }
                        }
                        _ => unreachable!(),
                    };
                    if !icon.is_empty() {
                        spans.push(Span::styled(
                            format!("{} ", icon),
                            base_style.fg(text.unwrap_or(ratatui::style::Color::White)),
                        ));
                    }

                    let name_fg = if item.kind == TreeItemKind::Project {
                        accent.unwrap_or(ratatui::style::Color::Cyan)
                    } else {
                        text.unwrap_or(ratatui::style::Color::White)
                    };
                    spans.push(Span::styled(
                        item.name.clone(),
                        base_style.fg(name_fg).add_modifier(Modifier::BOLD),
                    ));
                }
                TreeItemKind::Terminal | TreeItemKind::Subcommand => {
                    let parts: Vec<_> = item.path.split('/').collect();
                    if parts.len() == 3 {
                        let status = get_status(self.process_manager, parts[0], parts[1], parts[2]);
                        let status_fg = match status {
                            ProcessStatus::Running => {
                                success.unwrap_or(ratatui::style::Color::Green)
                            }
                            ProcessStatus::Starting => {
                                accent.unwrap_or(ratatui::style::Color::Yellow)
                            }
                            ProcessStatus::Stopping => {
                                warning.unwrap_or(ratatui::style::Color::Yellow)
                            }
                            ProcessStatus::Crashed => error.unwrap_or(ratatui::style::Color::Red),
                            ProcessStatus::Stopped => {
                                text_muted.unwrap_or(ratatui::style::Color::DarkGray)
                            }
                        };
                        spans.push(Span::styled(
                            AppState::status_icon(status),
                            base_style.fg(status_fg),
                        ));
                        spans.push(Span::styled(" ", base_style));
                    }

                    let name_fg = if item.kind == TreeItemKind::Terminal {
                        text_muted.unwrap_or(ratatui::style::Color::DarkGray)
                    } else {
                        text.unwrap_or(ratatui::style::Color::White)
                    };
                    spans.push(Span::styled(item.name.clone(), base_style.fg(name_fg)));
                }
            }

            let line = Line::from(spans);
            buf.set_line(inner.x, row, &line, inner.width);
        }
    }
}
