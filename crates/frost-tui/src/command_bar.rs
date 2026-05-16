use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::state::Focus;

/// Render the command bar with shortcuts and running count.
pub struct CommandBar {
    pub running_count: usize,
    pub focus: Focus,
}

impl Widget for CommandBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Command Bar ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(area);
        block.render(area, buf);

        let key_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(ratatui::style::Modifier::BOLD);
        let label_style = Style::default().fg(Color::Gray);

        // The shortcut hints change with focus: when the log viewer is
        // active, Ctrl+C is forwarded to the child as SIGINT, so the
        // quit binding shifts to Ctrl+Q and a "→PTY" hint is shown so
        // the user knows where their keystrokes are going.
        let shortcuts = match self.focus {
            Focus::Sidebar => vec![
                Span::styled("q", key_style),
                Span::styled(":quit  ", label_style),
                Span::styled("↑↓", key_style),
                Span::styled(":nav  ", label_style),
                Span::styled("Enter", key_style),
                Span::styled(":toggle  ", label_style),
                Span::styled("Tab", key_style),
                Span::styled(":focus log  ", label_style),
                Span::styled("Ctrl+P", key_style),
                Span::styled(":palette  ", label_style),
            ],
            Focus::LogViewer => vec![
                Span::styled("Ctrl+Q", key_style),
                Span::styled(":quit  ", label_style),
                Span::styled("Tab", key_style),
                Span::styled(":focus sidebar  ", label_style),
                Span::styled("Ctrl+P", key_style),
                Span::styled(":palette  ", label_style),
                Span::styled("keys", key_style),
                Span::styled("→PTY  ", label_style),
            ],
        };

        let count_spans = vec![
            Span::styled("Running: ", Style::default().fg(Color::Gray)),
            Span::styled(
                self.running_count.to_string(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ];

        let left = Line::from(shortcuts);
        let right = Line::from(count_spans);

        // Render shortcuts on the left.
        buf.set_line(inner.x, inner.y, &left, inner.width);

        // Render running count on the right.
        let right_text = right.to_string();
        let right_x = inner.x + inner.width - right_text.len() as u16;
        if right_x >= inner.x {
            buf.set_line(right_x, inner.y, &right, inner.width - (right_x - inner.x));
        }
    }
}
