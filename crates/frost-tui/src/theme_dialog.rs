use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

/// Render the theme switcher dialog.
pub struct ThemeDialog {
    pub themes: Vec<String>,
    pub selected: usize,
    pub filter: String,
    pub active_theme: String,
}

impl ThemeDialog {
    #[allow(dead_code)]
    pub fn new(themes: Vec<String>, active_theme: String) -> Self {
        Self {
            themes,
            selected: 0,
            filter: String::new(),
            active_theme,
        }
    }

    pub fn filtered(&self) -> Vec<&String> {
        if self.filter.is_empty() {
            self.themes.iter().collect()
        } else {
            let lower = self.filter.to_lowercase();
            self.themes
                .iter()
                .filter(|t| t.to_lowercase().contains(&lower))
                .collect()
        }
    }

    pub fn clamp_selection(&mut self) {
        let count = self.filtered().len();
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
    }

    pub fn selected_theme(&self) -> Option<String> {
        let filtered = self.filtered();
        filtered.get(self.selected).map(|t| (*t).clone())
    }
}

impl Widget for ThemeDialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let overlay_area = center_rect(area, 40, 16);
        Clear.render(overlay_area, buf);

        let block = Block::default()
            .title(" Switch Theme ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));
        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        // Filter input.
        let filter_text = if self.filter.is_empty() {
            "Type to filter..."
        } else {
            &self.filter
        };
        let filter_style = if self.filter.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        let line = Line::from(Span::styled(filter_text, filter_style));
        buf.set_line(inner.x, inner.y, &line, inner.width);

        // Divider.
        let divider = "─".repeat(inner.width as usize);
        buf.set_line(
            inner.x,
            inner.y + 1,
            &Line::from(Span::styled(divider, Style::default().fg(Color::DarkGray))),
            inner.width,
        );

        // Theme list.
        let list_area = Rect::new(
            inner.x,
            inner.y + 2,
            inner.width,
            inner.height.saturating_sub(2),
        );

        let filtered = self.filtered();
        for (i, theme) in filtered.iter().enumerate().take(list_area.height as usize) {
            let row = list_area.y + i as u16;
            let is_selected = i == self.selected;
            let is_active = **theme == self.active_theme;

            let mut style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            if is_active {
                style = style.fg(Color::Green);
            } else {
                style = style.fg(Color::Gray);
            }

            let marker = if is_active { "● " } else { "  " };
            let text = format!("{}{}", marker, theme);
            let line = Line::from(Span::styled(text, style));
            buf.set_line(list_area.x, row, &line, list_area.width);
        }

        if filtered.is_empty() {
            let msg = Span::styled("No matches", Style::default().fg(Color::DarkGray));
            buf.set_line(list_area.x, list_area.y, &Line::from(msg), list_area.width);
        }
    }
}

fn center_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
