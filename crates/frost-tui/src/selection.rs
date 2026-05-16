//! Linear text selection state for the log viewer.
//!
//! The selection model is the simplest one that's still useful: an
//! anchor cell and a head cell, both in 0-based PTY grid coordinates.
//! Iterating the selection in row-major order yields the cells in
//! reading order regardless of whether the user dragged forward or
//! backward.
//!
//! Word- and line-mode selection (double / triple click) is out of
//! scope for this iteration; the building block here is enough for
//! reverse-video rendering and copy-to-clipboard.

/// A single grid cell address. `row` is the screen-relative row and
/// `col` is the column, both 0-based. The renderer uses the same axes
/// when iterating `LogViewer.lines`, so no translation is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPoint {
    pub row: usize,
    pub col: usize,
}

/// A linear text selection with an anchor (where the user clicked) and a
/// head (where the pointer currently is). The two endpoints may be in
/// either reading order; helpers normalise them when iterating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: GridPoint,
    pub head: GridPoint,
}

/// Inclusive column range on a single row of the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowRange {
    pub row: usize,
    pub start_col: usize,
    pub end_col: usize,
}

impl Selection {
    /// New char-mode selection collapsed to a single cell.
    pub fn at(point: GridPoint) -> Self {
        Self {
            anchor: point,
            head: point,
        }
    }

    /// Move the head end (e.g. during mouse drag or shift-click).
    pub fn extend_to(&mut self, head: GridPoint) {
        self.head = head;
    }

    /// Endpoints in reading order: `(start, end)` where `start <= end`
    /// when laid out as `row * cols + col`.
    pub fn ordered_endpoints(&self) -> (GridPoint, GridPoint) {
        if (self.anchor.row, self.anchor.col) <= (self.head.row, self.head.col) {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    /// Whether a given cell is within the selection.
    pub fn contains(&self, row: usize, col: usize) -> bool {
        let (start, end) = self.ordered_endpoints();
        let pos = (row, col);
        pos >= (start.row, start.col) && pos <= (end.row, end.col)
    }

    /// Walk the selection row by row, yielding the inclusive column
    /// range to highlight on each row. `cols` is the grid width and is
    /// used to extend the highlight to the right edge on intermediate
    /// rows of a multi-row selection.
    pub fn iter_row_ranges(&self, cols: usize) -> Vec<RowRange> {
        let (start, end) = self.ordered_endpoints();
        if cols == 0 {
            return Vec::new();
        }
        let last_col = cols - 1;
        let mut out = Vec::with_capacity(end.row - start.row + 1);
        for row in start.row..=end.row {
            let (sc, ec) = if start.row == end.row {
                (start.col, end.col)
            } else if row == start.row {
                (start.col, last_col)
            } else if row == end.row {
                (0, end.col)
            } else {
                (0, last_col)
            };
            out.push(RowRange {
                row,
                start_col: sc,
                end_col: ec,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(row: usize, col: usize) -> GridPoint {
        GridPoint { row, col }
    }

    #[test]
    fn single_cell_selection_yields_one_range() {
        let sel = Selection::at(p(3, 7));
        let ranges = sel.iter_row_ranges(80);
        assert_eq!(
            ranges,
            vec![RowRange {
                row: 3,
                start_col: 7,
                end_col: 7
            }]
        );
    }

    #[test]
    fn forward_drag_on_same_row() {
        let mut sel = Selection::at(p(0, 2));
        sel.extend_to(p(0, 8));
        let ranges = sel.iter_row_ranges(80);
        assert_eq!(
            ranges,
            vec![RowRange {
                row: 0,
                start_col: 2,
                end_col: 8
            }]
        );
    }

    #[test]
    fn backward_drag_reorders_endpoints() {
        let mut sel = Selection::at(p(2, 5));
        sel.extend_to(p(2, 1));
        let ranges = sel.iter_row_ranges(80);
        assert_eq!(
            ranges,
            vec![RowRange {
                row: 2,
                start_col: 1,
                end_col: 5
            }]
        );
    }

    #[test]
    fn multi_row_forward_drag_fills_intermediate_rows() {
        // Anchor at (0, 70), head at (2, 10) on an 80-col grid. First
        // row should extend to col 79; middle row covers 0..=79; last
        // row covers 0..=10.
        let mut sel = Selection::at(p(0, 70));
        sel.extend_to(p(2, 10));
        let ranges = sel.iter_row_ranges(80);
        assert_eq!(
            ranges,
            vec![
                RowRange {
                    row: 0,
                    start_col: 70,
                    end_col: 79
                },
                RowRange {
                    row: 1,
                    start_col: 0,
                    end_col: 79
                },
                RowRange {
                    row: 2,
                    start_col: 0,
                    end_col: 10
                },
            ]
        );
    }

    #[test]
    fn contains_matches_iter_row_ranges() {
        let mut sel = Selection::at(p(1, 4));
        sel.extend_to(p(3, 2));
        // Cells inside.
        assert!(sel.contains(1, 4));
        assert!(sel.contains(2, 0));
        assert!(sel.contains(3, 2));
        // Cells outside.
        assert!(!sel.contains(1, 3));
        assert!(!sel.contains(3, 3));
        assert!(!sel.contains(0, 4));
    }
}
