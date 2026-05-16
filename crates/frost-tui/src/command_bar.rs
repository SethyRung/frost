use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use frost_core::ResolvedTheme;
use crate::state::Focus;
use crate::theme_adapter::to_color;

/// Render the command bar with shortcuts and running count.
pub struct CommandBar<'a> {
    pub running_count: usize,
    pub focus: Focus,
    pub theme: Option<&'a ResolvedTheme>,
}

impl<'a> Widget for CommandBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = self.theme.map(|t| to_color(t.border)).unwrap_or(ratatui::style::Color::Yellow);
        let block = Block::default()
            .title(" Command Bar ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        let key_color = self.theme.map(|t| to_color(t.accent)).unwrap_or(ratatui::style::Color::Yellow);
        let desc_color = self.theme.map(|t| to_color(t.text_muted)).unwrap_or(ratatui::style::Color::Gray);
        let success_color = self.theme.map(|t| to_color(t.success)).unwrap_or(ratatui::style::Color::Green);

        let key_style = Style::default().fg(key_color).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(desc_color);

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
            Span::styled("Running: ", Style::default().fg(desc_color)),
            Span::styled(
                self.running_count.to_string(),
                Style::default().fg(success_color).add_modifier(Modifier::BOLD),
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
