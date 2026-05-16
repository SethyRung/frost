use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

/// A palette command entry.
#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub label: String,
    pub action: PaletteAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteAction {
    SwitchTheme,
    ReloadConfig,
    Quit,
}

pub fn default_items() -> Vec<PaletteItem> {
    vec![
        PaletteItem {
            label: "Switch Theme".to_string(),
            action: PaletteAction::SwitchTheme,
        },
        PaletteItem {
            label: "Reload Config".to_string(),
            action: PaletteAction::ReloadConfig,
        },
        PaletteItem {
            label: "Quit".to_string(),
            action: PaletteAction::Quit,
        },
    ]
}

/// Render the command palette overlay.
pub struct Palette {
    pub items: Vec<PaletteItem>,
    pub selected: usize,
    pub filter: String,
}

impl Palette {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            items: default_items(),
            selected: 0,
            filter: String::new(),
        }
    }

    pub fn filtered(&self) -> Vec<&PaletteItem> {
        if self.filter.is_empty() {
            self.items.iter().collect()
        } else {
            let lower = self.filter.to_lowercase();
            self.items
                .iter()
                .filter(|item| item.label.to_lowercase().contains(&lower))
                .collect()
        }
    }

    pub fn selected_action(&self) -> Option<PaletteAction> {
        let filtered = self.filtered();
        filtered.get(self.selected).map(|item| item.action.clone())
    }

    pub fn clamp_selection(&mut self) {
        let count = self.filtered().len();
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
    }
}

impl Widget for Palette {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Centered overlay.
        let overlay_area = center_rect(area, 50, 12);
        Clear.render(overlay_area, buf);

        let block = Block::default()
            .title(" Command Palette ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        // Filter input at top.
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
        let filter_para = Paragraph::new(filter_text).style(filter_style);
        filter_para.render(
            Rect::new(inner.x, inner.y, inner.width, 1),
            buf,
        );

        // Divider.
        let divider = "─".repeat(inner.width as usize);
        buf.set_line(
            inner.x,
            inner.y + 1,
            &Line::from(Span::styled(divider, Style::default().fg(Color::DarkGray))),
            inner.width,
        );

        // Items list.
        let filtered = self.filtered();
        let list_area = Rect::new(
            inner.x,
            inner.y + 2,
            inner.width,
            inner.height.saturating_sub(2),
        );

        for (i, item) in filtered.iter().enumerate().take(list_area.height as usize) {
            let row = list_area.y + i as u16;
            let is_selected = i == self.selected;
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            let line = Line::from(Span::styled(&item.label, style));
            buf.set_line(list_area.x, row, &line, list_area.width);
        }

        if filtered.is_empty() {
            let msg = Span::styled("No matches", Style::default().fg(Color::DarkGray));
            buf.set_line(list_area.x, list_area.y, &Line::from(msg), list_area.width);
        }
    }
}

/// Calculate a centered rectangle.
fn center_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
