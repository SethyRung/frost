use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use frost_core::{DisplayLine, ProcessManager, RGBA};

/// Render the terminal log viewer.
pub struct LogViewer {
    pub lines: Vec<DisplayLine>,
    pub scroll: usize,
    pub scrolled: bool,
    pub focused: bool,
}

impl LogViewer {
    pub fn empty() -> Self {
        Self {
            lines: Vec::new(),
            scroll: 0,
            scrolled: false,
            focused: false,
        }
    }

    pub fn from_manager(
        manager: &ProcessManager,
        project: &str,
        app: &str,
        subcommand: &str,
        scroll: usize,
        focused: bool,
    ) -> Self {
        let lines = manager
            .get_display_lines(project, app, subcommand)
            .unwrap_or_default();
        let scrolled = scroll > 0;
        Self { lines, scroll, scrolled, focused }
    }
}

fn rgba_to_ratatui(rgba: RGBA) -> Color {
    Color::Rgb(
        (rgba.r * 255.0) as u8,
        (rgba.g * 255.0) as u8,
        (rgba.b * 255.0) as u8,
    )
}

impl Widget for LogViewer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused { Color::Green } else { Color::DarkGray };
        let title = if self.focused {
            " Log (interactive) "
        } else if self.scrolled {
            " Log (scrolled) "
        } else {
            " Log "
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.lines.is_empty() {
            let msg = Span::styled(
                "No process selected — use sidebar to start one",
                Style::default().fg(Color::DarkGray),
            );
            buf.set_line(inner.x, inner.y, &Line::from(msg), inner.width);
            return;
        }

        let visible_height = inner.height as usize;
        let start = if self.lines.len() > visible_height {
            self.lines.len().saturating_sub(visible_height + self.scroll)
        } else {
            0
        };

        for (i, line) in self.lines.iter().skip(start).take(visible_height).enumerate() {
            let row = inner.y + i as u16;
            if row >= inner.y + inner.height {
                break;
            }

            let spans: Vec<Span> = line
                .iter()
                .map(|cell| {
                    let mut style = Style::default().fg(rgba_to_ratatui(cell.fg));
                    if cell.bold {
                        style = style.add_modifier(ratatui::style::Modifier::BOLD);
                    }
                    if cell.italic {
                        style = style.add_modifier(ratatui::style::Modifier::ITALIC);
                    }
                    if cell.underline {
                        style = style.add_modifier(ratatui::style::Modifier::UNDERLINED);
                    }
                    Span::styled(cell.c.to_string(), style)
                })
                .collect();

            let ratatui_line = Line::from(spans);
            buf.set_line(inner.x, row, &ratatui_line, inner.width);
        }
    }
}
