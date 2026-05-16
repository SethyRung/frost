use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use frost_core::{DisplayLine, ProcessManager, RGBA, ResolvedTheme, TerminalCell};

use crate::selection::Selection;
use crate::theme_adapter::to_color;

/// Render the terminal log viewer.
pub struct LogViewer<'a> {
    pub lines: Vec<DisplayLine>,
    pub scroll: usize,
    pub scrolled: bool,
    pub focused: bool,
    /// Optional selection in grid coordinates. Cells inside the
    /// selection are rendered with fg/bg swapped (reverse video).
    pub selection: Option<Selection>,
    /// True for ~150 ms after the child rings the bell (`\x07`).
    /// Repaints the border in yellow so the user sees the alert.
    pub bell_active: bool,
    pub theme: Option<&'a ResolvedTheme>,
}

impl<'a> LogViewer<'a> {
    pub fn empty() -> Self {
        Self {
            lines: Vec::new(),
            scroll: 0,
            scrolled: false,
            focused: false,
            selection: None,
            bell_active: false,
            theme: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_manager(
        manager: &ProcessManager,
        project: &str,
        app: &str,
        subcommand: &str,
        scroll: usize,
        focused: bool,
        selection: Option<Selection>,
        bell_active: bool,
        theme: Option<&'a ResolvedTheme>,
    ) -> Self {
        // Pull history + viewport so the scrollback nav keys can show
        // older output by moving `scroll` further from the bottom.
        let lines = manager
            .get_lines_with_scrollback(project, app, subcommand)
            .unwrap_or_default();
        let scrolled = scroll > 0;
        Self {
            lines,
            scroll,
            scrolled,
            focused,
            selection,
            bell_active,
            theme,
        }
    }
}

fn rgba_to_ratatui(rgba: RGBA) -> Color {
    Color::Rgb(
        (rgba.r * 255.0).clamp(0.0, 255.0) as u8,
        (rgba.g * 255.0).clamp(0.0, 255.0) as u8,
        (rgba.b * 255.0).clamp(0.0, 255.0) as u8,
    )
}

/// Compute the rendered style for one cell. `selected` swaps fg/bg the
/// same way the `reverse` flag does; combining both flags is a no-op
/// (selected + reverse cancel out) which matches what most terminal
/// emulators show.
fn cell_style(cell: &TerminalCell, selected: bool) -> Style {
    let swap = cell.reverse ^ selected;
    let (fg_rgba, bg_rgba) = if swap {
        (cell.bg, cell.fg)
    } else {
        (cell.fg, cell.bg)
    };

    let mut style = Style::default()
        .fg(rgba_to_ratatui(fg_rgba))
        .bg(rgba_to_ratatui(bg_rgba));

    if cell.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if cell.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.dim {
        style = style.add_modifier(Modifier::DIM);
    }
    if cell.strikethrough {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }
    if cell.hidden {
        style = style.add_modifier(Modifier::HIDDEN);
    }
    style
}

/// Character a cell contributes to the rendered span. `hidden` cells render
/// as a space so the background colour still paints but the glyph is gone.
fn cell_glyph(cell: &TerminalCell) -> String {
    if cell.hidden {
        " ".to_string()
    } else {
        cell.c.to_string()
    }
}

impl<'a> Widget for LogViewer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.bell_active {
            Color::Yellow
        } else if let Some(t) = self.theme {
            if self.focused {
                to_color(t.border_active)
            } else {
                to_color(t.border)
            }
        } else if self.focused {
            Color::Green
        } else {
            Color::DarkGray
        };
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
            let msg_color = self.theme.map(|t| to_color(t.text_muted)).unwrap_or(Color::DarkGray);
            let msg = Span::styled(
                "No process selected — use sidebar to start one",
                Style::default().fg(msg_color),
            );
            buf.set_line(inner.x, inner.y, &Line::from(msg), inner.width);
            return;
        }

        let visible_height = inner.height as usize;
        let start = if self.lines.len() > visible_height {
            self.lines
                .len()
                .saturating_sub(visible_height + self.scroll)
        } else {
            0
        };

        for (i, line) in self
            .lines
            .iter()
            .skip(start)
            .take(visible_height)
            .enumerate()
        {
            let row = inner.y + i as u16;
            if row >= inner.y + inner.height {
                break;
            }

            // Selection is stored in grid (row, col) where row indexes
            // self.lines directly — translate the display row back to a
            // grid row to test membership.
            let grid_row = start + i;
            let spans: Vec<Span> = line
                .iter()
                .enumerate()
                .map(|(col, cell)| {
                    let selected = self
                        .selection
                        .map(|sel| sel.contains(grid_row, col))
                        .unwrap_or(false);
                    Span::styled(cell_glyph(cell), cell_style(cell, selected))
                })
                .collect();

            let ratatui_line = Line::from(spans);
            buf.set_line(inner.x, row, &ratatui_line, inner.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::{GridPoint, Selection};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn plain_cell(c: char, fg: RGBA, bg: RGBA) -> TerminalCell {
        TerminalCell {
            c,
            fg,
            bg,
            bold: false,
            italic: false,
            underline: false,
            reverse: false,
            dim: false,
            hidden: false,
            strikethrough: false,
            wide: false,
        }
    }

    #[test]
    fn renders_background_color_per_cell() {
        let red = RGBA::new(1.0, 0.0, 0.0, 1.0);
        let white = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let cell = plain_cell('A', white, red);
        let style = cell_style(&cell, false);
        assert_eq!(style.fg, Some(Color::Rgb(255, 255, 255)));
        assert_eq!(style.bg, Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn reverse_swaps_fg_and_bg() {
        let red = RGBA::new(1.0, 0.0, 0.0, 1.0);
        let white = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let mut cell = plain_cell('A', white, red);
        cell.reverse = true;
        let style = cell_style(&cell, false);
        assert_eq!(style.fg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(style.bg, Some(Color::Rgb(255, 255, 255)));
    }

    #[test]
    fn selection_swaps_fg_and_bg_for_normal_cell() {
        let red = RGBA::new(1.0, 0.0, 0.0, 1.0);
        let white = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let cell = plain_cell('A', white, red);
        let style = cell_style(&cell, true);
        assert_eq!(style.fg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(style.bg, Some(Color::Rgb(255, 255, 255)));
    }

    #[test]
    fn selection_xor_reverse_is_identity() {
        let red = RGBA::new(1.0, 0.0, 0.0, 1.0);
        let white = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let mut cell = plain_cell('A', white, red);
        cell.reverse = true;
        let style = cell_style(&cell, true);
        assert_eq!(style.fg, Some(Color::Rgb(255, 255, 255)));
        assert_eq!(style.bg, Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn modifiers_applied_for_bold_italic_underline() {
        let fg = RGBA::new(0.5, 0.5, 0.5, 1.0);
        let bg = RGBA::new(0.0, 0.0, 0.0, 1.0);
        let mut cell = plain_cell('A', fg, bg);
        cell.bold = true;
        cell.italic = true;
        cell.underline = true;
        let style = cell_style(&cell, false);
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
        assert!(style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn widget_paints_bg_into_buffer() {
        let red = RGBA::new(1.0, 0.0, 0.0, 1.0);
        let white = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let viewer = LogViewer {
            lines: vec![vec![
                plain_cell('A', white, red),
                plain_cell('B', white, red),
            ]],
            scroll: 0,
            scrolled: false,
            focused: false,
            selection: None,
            bell_active: false,
            theme: None,
        };

        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        viewer.render(area, &mut buf);

        // Inner area starts at (1, 1) because of border.
        let cell_a = buf.cell((1, 1)).expect("buffer cell at (1,1)");
        assert_eq!(cell_a.symbol(), "A");
        assert_eq!(cell_a.fg, Color::Rgb(255, 255, 255));
        assert_eq!(cell_a.bg, Color::Rgb(255, 0, 0));

        let cell_b = buf.cell((2, 1)).expect("buffer cell at (2,1)");
        assert_eq!(cell_b.symbol(), "B");
        assert_eq!(cell_b.bg, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn dim_strike_hidden_map_to_modifier_bits() {
        let fg = RGBA::new(0.5, 0.5, 0.5, 1.0);
        let bg = RGBA::new(0.0, 0.0, 0.0, 1.0);
        let mut cell = plain_cell('A', fg, bg);
        cell.dim = true;
        cell.strikethrough = true;
        cell.hidden = true;
        let style = cell_style(&cell, false);
        assert!(style.add_modifier.contains(Modifier::DIM));
        assert!(style.add_modifier.contains(Modifier::CROSSED_OUT));
        assert!(style.add_modifier.contains(Modifier::HIDDEN));
    }

    #[test]
    fn hidden_cell_renders_as_space_but_keeps_bg() {
        let red = RGBA::new(1.0, 0.0, 0.0, 1.0);
        let white = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let mut cell = plain_cell('A', white, red);
        cell.hidden = true;

        let viewer = LogViewer {
            lines: vec![vec![cell]],
            scroll: 0,
            scrolled: false,
            focused: false,
            selection: None,
            bell_active: false,
            theme: None,
        };

        let area = Rect::new(0, 0, 5, 3);
        let mut buf = Buffer::empty(area);
        viewer.render(area, &mut buf);

        let painted = buf.cell((1, 1)).expect("buffer cell at (1,1)");
        assert_eq!(
            painted.symbol(),
            " ",
            "hidden cell glyph must be replaced with a space"
        );
        assert_eq!(painted.bg, Color::Rgb(255, 0, 0), "bg must still paint");
    }

    #[test]
    fn widget_paints_reversed_cell() {
        let red = RGBA::new(1.0, 0.0, 0.0, 1.0);
        let white = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let mut cell = plain_cell('A', white, red);
        cell.reverse = true;

        let viewer = LogViewer {
            lines: vec![vec![cell]],
            scroll: 0,
            scrolled: false,
            focused: false,
            selection: None,
            bell_active: false,
            theme: None,
        };

        let area = Rect::new(0, 0, 5, 3);
        let mut buf = Buffer::empty(area);
        viewer.render(area, &mut buf);

        let painted = buf.cell((1, 1)).expect("buffer cell at (1,1)");
        assert_eq!(painted.fg, Color::Rgb(255, 0, 0));
        assert_eq!(painted.bg, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn widget_paints_selected_cells_in_reverse() {
        let red = RGBA::new(1.0, 0.0, 0.0, 1.0);
        let white = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let viewer = LogViewer {
            lines: vec![vec![
                plain_cell('A', white, red),
                plain_cell('B', white, red),
                plain_cell('C', white, red),
            ]],
            scroll: 0,
            scrolled: false,
            focused: false,
            // Select B only (row 0, col 1).
            selection: Some(Selection {
                anchor: GridPoint { row: 0, col: 1 },
                head: GridPoint { row: 0, col: 1 },
            }),
            bell_active: false,
            theme: None,
        };

        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        viewer.render(area, &mut buf);

        // A and C are NOT selected — original colours.
        let a = buf.cell((1, 1)).unwrap();
        assert_eq!(a.fg, Color::Rgb(255, 255, 255));
        assert_eq!(a.bg, Color::Rgb(255, 0, 0));

        let c = buf.cell((3, 1)).unwrap();
        assert_eq!(c.fg, Color::Rgb(255, 255, 255));
        assert_eq!(c.bg, Color::Rgb(255, 0, 0));

        // B is selected — fg/bg swapped.
        let b = buf.cell((2, 1)).unwrap();
        assert_eq!(b.fg, Color::Rgb(255, 0, 0));
        assert_eq!(b.bg, Color::Rgb(255, 255, 255));
    }
}
