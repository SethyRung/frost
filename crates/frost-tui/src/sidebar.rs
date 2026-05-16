use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use frost_core::{FrostConfig, ProcessManager, ProcessStatus};

use crate::state::AppState;

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
}

impl<'a> Widget for Sidebar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let block = Block::default()
            .title(" Projects ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        let items = build_visible_tree(self.config, self.expanded);
        if items.is_empty() {
            let msg = Span::styled(
                "No projects configured",
                Style::default().fg(Color::DarkGray),
            );
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

        for (i, item) in items.iter().skip(start).take(visible_height).enumerate() {
            let row = inner.y + i as u16;
            if row >= inner.y + inner.height {
                break;
            }

            let is_selected = start + i == selected_index;
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let indent = "  ".repeat(item.depth);
            let prefix = match item.kind {
                TreeItemKind::Project | TreeItemKind::App => {
                    if self.expanded.contains(&item.path) {
                        "▼ "
                    } else {
                        "▶ "
                    }
                }
                TreeItemKind::Terminal | TreeItemKind::Subcommand => "  ",
            };

            let (status, title) = match item.kind {
                TreeItemKind::Terminal | TreeItemKind::Subcommand => {
                    let parts: Vec<_> = item.path.split('/').collect();
                    if parts.len() == 3 {
                        let status = get_status(self.process_manager, parts[0], parts[1], parts[2]);
                        let title = self
                            .process_manager
                            .get_title(parts[0], parts[1], parts[2]);
                        (format!("{} ", AppState::status_icon(status)), title)
                    } else {
                        (String::new(), None)
                    }
                }
                _ => (String::new(), None),
            };

            // Append `— <title>` when the child has set one via OSC 0/2,
            // truncated so it can't push the indicator off-screen.
            let title_suffix = title
                .as_deref()
                .filter(|t| !t.is_empty())
                .map(|t| {
                    let max = 24usize;
                    let trimmed: String = if t.chars().count() > max {
                        t.chars().take(max).collect::<String>() + "…"
                    } else {
                        t.to_string()
                    };
                    format!(" — {}", trimmed)
                })
                .unwrap_or_default();

            let text = format!(
                "{}{}{}{}{}",
                indent, prefix, status, item.name, title_suffix
            );
            let span = Span::styled(text, style);
            let line = Line::from(span);
            buf.set_line(inner.x, row, &line, inner.width);
        }
    }
}
