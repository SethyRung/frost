use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

/// Render the command bar with shortcuts and running count.
pub struct CommandBar {
    pub running_count: usize,
}

impl Widget for CommandBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Command Bar ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(area);
        block.render(area, buf);

        let shortcuts = vec![
            Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled(":quit  ", Style::default().fg(Color::Gray)),
            Span::styled("↑↓", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled(":nav  ", Style::default().fg(Color::Gray)),
            Span::styled("Enter/Space", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled(":toggle  ", Style::default().fg(Color::Gray)),
        ];

        let count_spans = vec![
            Span::styled("Running: ", Style::default().fg(Color::Gray)),
            Span::styled(
                self.running_count.to_string(),
                Style::default().fg(Color::Green).add_modifier(ratatui::style::Modifier::BOLD),
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
