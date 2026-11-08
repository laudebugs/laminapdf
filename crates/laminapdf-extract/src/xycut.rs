//! Reading order from the layout: XY-Cut++ ([ADR 0030] §2; Nagy and Seth's recursive cuts with
//! [XY-Cut++]'s masking of cross-column blocks).
//!
//! Blocks wider than [`WIDE`] of the page's text that span two columns of narrower blocks and
//! share no height with a narrower block (titles, full-width figures and paragraphs over
//! columns) are masked first: they split the page into
//! bands, each band is cut on its own, and the masked blocks go back between the bands. A band
//! is cut recursively at the widest gap in the projection of its boxes: a column gap first, so
//! paragraphs whose breaks line up across columns still read column by column, unless every
//! block in the group is one line (a grid such as a table), which reads row by row. A group
//! with no gap reads top to bottom.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
//! [XY-Cut++]: https://arxiv.org/abs/2504.10258

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;

use laminapdf_model::RectPt;
use laminapdf_syntax::CancelToken;

/// A block at least this fraction of the page's text width is a masking candidate.
const WIDE: f64 = 0.6;

/// Gaps narrower than this, in points, are not cuts (kerning, overlapping descenders).
const MIN_GAP: f64 = 1.0;

/// A group with at least this many blocks, three quarters or more of them one line, may be a
/// grid with a wrapped cell (`near-grid`).
const NEAR_GRID_BLOCKS: usize = 4;

/// At most this many groups per page are cut a second time to test `near-grid`.
const MAX_ALTERNATIVES: u32 = 8;
/// Two blocks read as one column when they share this much of the wider one's width.
const SAME_COLUMN: f64 = 0.6;

/// Why the rules had to guess the order, and the blocks (indices) the guess concerns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Doubt {
    /// `columns-revisited` or `near-grid` (`narrow-gutter` is found on the lines, in `page.rs`).
    pub(crate) reason: &'static str,
    pub(crate) blocks: Vec<usize>,
}

/// The reading order, and the first doubt about it, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ordered {
    pub(crate) order: Vec<usize>,
    pub(crate) doubt: Option<Doubt>,
}

/// The cut was stopped by its cancel token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cancelled;

/// Where the cut runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// A column gap: the cut is a vertical line.
    Columns,
    /// A row gap: the cut is a horizontal line.
    Rows,
}

struct Cutter<'a> {
    boxes: &'a [RectPt],
    /// How many lines each block has.
    lines: &'a [usize],
    rtl: bool,
    max_depth: u32,
    cancel: &'a CancelToken,
    /// How many groups were cut a second time (`near-grid`).
    alternatives: Cell<u32>,
    /// The first group whose order by rows differs from the order by columns.
    near_grid: RefCell<Option<Vec<usize>>>,
}

fn rect(boxes: &[RectPt], i: usize) -> RectPt {
    boxes.get(i).copied().unwrap_or_default()
}

/// The reading order of `boxes` (indices), whose blocks have `lines` lines each, columns right
/// to left when `rtl`, with the first doubt about it: a band whose order leaves a column and
/// comes back to it (`columns-revisited`), else a group the grid rule decided (`near-grid`).
pub(crate) fn order(
    boxes: &[RectPt],
    lines: &[usize],
    rtl: bool,
    max_depth: u32,
    cancel: &CancelToken,
) -> Result<Ordered, Cancelled> {
    let cutter = Cutter {
        boxes,
        lines,
        rtl,
        max_depth,
        cancel,
        alternatives: Cell::new(0),
        near_grid: RefCell::new(None),
    };
    let all: Vec<usize> = (0..boxes.len()).collect();
    let (masked, rest) = cutter.mask(&all);
    let mut out = Vec::with_capacity(boxes.len());
    let mut bands: Vec<Vec<usize>> = vec![Vec::new(); masked.len() + 1];
    for i in rest {
        let centre = centre_y(&rect(boxes, i));
        let band = masked.iter().filter(|&&m| centre_y(&rect(boxes, m)) < centre).count();
        if let Some(b) = bands.get_mut(band) {
            b.push(i);
        }
    }
    let mut revisited = None;
    for (k, band) in bands.into_iter().enumerate() {
        let start = out.len();
        cutter.cut(band, 0, &mut out)?;
        if revisited.is_none() {
            let band = out.get(start..).unwrap_or_default();
            revisited = cutter.revisits(band);
        }
        if let Some(&m) = masked.get(k) {
            out.push(m);
        }
    }
    let doubt = revisited
        .map(|blocks| Doubt {
            reason: "columns-revisited",
            blocks,
        })
        .or_else(|| {
            cutter.near_grid.take().map(|blocks| Doubt {
                reason: "near-grid",
                blocks,
            })
        });
    Ok(Ordered { order: out, doubt })
}

/// Whether `a` and `b` read as one column: they share at least [`SAME_COLUMN`] of the wider's
/// width.
fn same_column(a: &RectPt, b: &RectPt) -> bool {
    let shared = a.x1.get().min(b.x1.get()) - a.x0.get().max(b.x0.get());
    shared >= SAME_COLUMN * a.width().get().max(b.width().get())
}

fn centre_y(r: &RectPt) -> f64 {
    f64::midpoint(r.y0.get(), r.y1.get())
}

impl Cutter<'_> {
    fn one_line(&self, i: usize) -> bool {
        self.lines.get(i).copied().unwrap_or(1) <= 1
    }

    /// Where `band`, in reading order, leaves a block for one beside it (apart across, sharing height) and later comes back
    /// below the pair, each in its own column: the blocks from the first to the last return, or
    /// `None`. A column-first reading never comes back to a column within a band, but row cuts
    /// forced by a block across the gutter do. A row of blocks over the columns (authors over
    /// an abstract) isn't a return, since neither keeps its column's width below, nor is a pair
    /// of one-line blocks, since a grid reads row by row on purpose.
    fn revisits(&self, band: &[usize]) -> Option<Vec<usize>> {
        band.windows(2).enumerate().find_map(|(i, pair)| {
            let [a, b] = pair else { return None };
            let (ra, rb) = (rect(self.boxes, *a), rect(self.boxes, *b));
            let beside = (ra.x1 <= rb.x0 || rb.x1 <= ra.x0) && ra.y0 < rb.y1 && rb.y0 < ra.y1;
            if !beside || (self.one_line(*a) && self.one_line(*b)) {
                return None;
            }
            let after = band.get(i + 2..).unwrap_or_default();
            // Below the pair, so a row's own lines (a name, then its address) don't count.
            let floor = ra.y1.get().max(rb.y1.get());
            let back = |top: &RectPt, other: &RectPt| {
                after.iter().position(|&c| {
                    let rc = rect(self.boxes, c);
                    rc.y0.get() >= floor && same_column(top, &rc) && (rc.x1 <= other.x0 || rc.x0 >= other.x1)
                })
            };
            let last = back(&ra, &rb)?.max(back(&rb, &ra)?);
            band.get(i..=i + 2 + last).map(<[usize]>::to_vec)
        })
    }

    /// The masked blocks (top to bottom) and the rest.
    fn mask(&self, all: &[usize]) -> (Vec<usize>, Vec<usize>) {
        let (x0, x1) = all.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &i| {
            let r = rect(self.boxes, i);
            (a.min(r.x0.get()), b.max(r.x1.get()))
        });
        let width = x1 - x0;
        let wide = |i: usize| rect(self.boxes, i).width().get() >= WIDE * width;
        let narrow: Vec<RectPt> = all
            .iter()
            .filter(|&&i| !wide(i))
            .map(|&i| rect(self.boxes, i))
            .collect();
        // Two narrow blocks side by side under `r` (one ends left of where another starts): it
        // crosses a column gap.
        let crosses = |r: &RectPt| {
            let (first_end, last_start) = narrow
                .iter()
                .filter(|o| o.x0 < r.x1 && o.x1 > r.x0)
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(end, start), o| {
                    (end.min(o.x1.get()), start.max(o.x0.get()))
                });
            first_end <= last_start
        };
        let mut masked: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&i| {
                let r = rect(self.boxes, i);
                wide(i) && crosses(&r) && narrow.iter().all(|o| o.y1 <= r.y0 || o.y0 >= r.y1)
            })
            .collect();
        masked.sort_by(|&a, &b| self.reading(a, b));
        let rest = all.iter().copied().filter(|i| !masked.contains(i)).collect();
        (masked, rest)
    }

    /// Top to bottom, then along the line.
    fn reading(&self, a: usize, b: usize) -> Ordering {
        let (ra, rb) = (rect(self.boxes, a), rect(self.boxes, b));
        let along = if self.rtl {
            rb.x1.get().total_cmp(&ra.x1.get())
        } else {
            ra.x0.get().total_cmp(&rb.x0.get())
        };
        ra.y0.get().total_cmp(&rb.y0.get()).then(along)
    }

    /// The widest gap along `axis` between the boxes of `group`: its width and the position
    /// of the cut.
    fn gap(&self, group: &[usize], axis: Axis) -> Option<(f64, f64)> {
        let mut spans: Vec<(f64, f64)> = group
            .iter()
            .map(|&i| {
                let r = rect(self.boxes, i);
                match axis {
                    Axis::Columns => (r.x0.get(), r.x1.get()),
                    Axis::Rows => (r.y0.get(), r.y1.get()),
                }
            })
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut best: Option<(f64, f64)> = None;
        let mut reach = f64::NEG_INFINITY;
        for (start, end) in spans {
            let width = start - reach;
            if reach.is_finite() && width > MIN_GAP && best.is_none_or(|(w, _)| width > w) {
                best = Some((width, reach + width / 2.0));
            }
            reach = reach.max(end);
        }
        best
    }

    /// `group` in reading order, appended to `out`.
    fn cut(&self, mut group: Vec<usize>, depth: u32, out: &mut Vec<usize>) -> Result<(), Cancelled> {
        if self.cancel.is_cancelled() {
            return Err(Cancelled);
        }
        if group.len() <= 1 || depth >= self.max_depth {
            group.sort_by(|&a, &b| self.reading(a, b));
            out.extend(group);
            return Ok(());
        }
        let columns = self.gap(&group, Axis::Columns);
        let rows = self.gap(&group, Axis::Rows);
        let ones = group.iter().filter(|&&i| self.one_line(i)).count();
        let grid = ones == group.len();
        let axis = match (columns, rows) {
            (Some(_), Some(_)) if grid => Some((Axis::Rows, rows)),
            (Some(_), _) => Some((Axis::Columns, columns)),
            (None, Some(_)) => Some((Axis::Rows, rows)),
            (None, None) => None,
        };
        let Some((axis, Some((_, at)))) = axis else {
            group.sort_by(|&a, &b| self.reading(a, b));
            out.extend(group);
            return Ok(());
        };
        // Mostly one-line blocks with both gaps, but not all: the grid rule's all-or-nothing
        // test chose columns. Cut by rows too, and doubt the order when the two differ.
        let near_grid = !grid && group.len() >= NEAR_GRID_BLOCKS && ones * 4 >= group.len() * 3;
        let second = match rows {
            Some((_, rows_at)) if near_grid && axis == Axis::Columns && self.near_grid.borrow().is_none() => {
                let tried = self.alternatives.get();
                (tried < MAX_ALTERNATIVES).then(|| {
                    self.alternatives.set(tried + 1);
                    (group.clone(), rows_at)
                })
            }
            _ => None,
        };
        let start = out.len();
        self.split(group, axis, at, depth, out)?;
        if let Some((group, rows_at)) = second {
            // The second cut's own groups never make the order, so nothing they find, nor the
            // second cuts they spend, counts.
            let (found, tried) = (self.near_grid.take(), self.alternatives.get());
            let mut by_rows = Vec::with_capacity(group.len());
            self.split(group.clone(), Axis::Rows, rows_at, depth, &mut by_rows)?;
            self.alternatives.set(tried);
            let differs = out.get(start..) != Some(by_rows.as_slice());
            *self.near_grid.borrow_mut() = found.or_else(|| differs.then_some(group));
        }
        Ok(())
    }

    /// `group` cut along `axis` at `at`, each side in reading order, appended to `out`.
    fn split(&self, group: Vec<usize>, axis: Axis, at: f64, depth: u32, out: &mut Vec<usize>) -> Result<(), Cancelled> {
        let (before, after): (Vec<usize>, Vec<usize>) = group.into_iter().partition(|&i| {
            let r = rect(self.boxes, i);
            match axis {
                Axis::Columns => r.x1.get() <= at,
                Axis::Rows => r.y1.get() <= at,
            }
        });
        let (first, second) = if axis == Axis::Columns && self.rtl {
            (after, before)
        } else {
            (before, after)
        };
        self.cut(first, depth + 1, out)?;
        self.cut(second, depth + 1, out)
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_model::RectPt;
    use laminapdf_syntax::CancelToken;

    use super::{Cancelled, MAX_ALTERNATIVES, order};

    fn r(x: f64, y: f64, w: f64, h: f64) -> RectPt {
        RectPt::from_points(x, y, w, h)
    }

    /// Paragraphs: three lines each.
    fn run(boxes: &[RectPt], rtl: bool) -> Vec<usize> {
        order(boxes, &vec![3; boxes.len()], rtl, 64, &CancelToken::new()).map_or_else(|_| Vec::new(), |o| o.order)
    }

    #[test]
    fn two_columns_read_column_by_column() {
        // Left column blocks 0 and 2, right column 1 and 3, given row by row.
        let boxes = [
            r(50.0, 100.0, 220.0, 80.0),
            r(320.0, 100.0, 220.0, 120.0),
            r(50.0, 200.0, 220.0, 80.0),
            r(320.0, 240.0, 220.0, 40.0),
        ];
        assert_eq!(run(&boxes, false), [0, 2, 1, 3]);
    }

    #[test]
    fn columns_come_first_even_when_paragraph_breaks_line_up() {
        // The breaks between 0/2 and 1/3 leave a wider row gap than the gutter.
        let boxes = [
            r(50.0, 100.0, 220.0, 100.0),
            r(300.0, 100.0, 220.0, 100.0),
            r(50.0, 330.0, 220.0, 100.0),
            r(300.0, 330.0, 220.0, 100.0),
        ];
        assert_eq!(run(&boxes, false), [0, 2, 1, 3]);
    }

    #[test]
    fn a_grid_of_one_line_cells_reads_row_by_row() {
        let boxes = [
            r(56.0, 90.0, 120.0, 12.0),
            r(216.0, 90.0, 120.0, 12.0),
            r(56.0, 130.0, 120.0, 12.0),
            r(216.0, 130.0, 120.0, 12.0),
        ];
        let got = order(&boxes, &[1; 4], false, 64, &CancelToken::new());
        assert_eq!(got.map(|o| o.order), Ok(vec![0, 1, 2, 3]));
    }

    #[test]
    fn right_to_left_pages_read_the_right_column_first() {
        let boxes = [r(50.0, 100.0, 220.0, 80.0), r(320.0, 100.0, 220.0, 80.0)];
        assert_eq!(run(&boxes, true), [1, 0]);
    }

    #[test]
    fn a_full_width_title_and_footer_frame_the_columns() {
        let boxes = [
            r(50.0, 700.0, 490.0, 20.0), // footer
            r(320.0, 100.0, 220.0, 500.0),
            r(50.0, 100.0, 220.0, 500.0),
            r(50.0, 40.0, 490.0, 30.0), // title
        ];
        assert_eq!(run(&boxes, false), [3, 2, 1, 0]);
    }

    #[test]
    fn a_full_width_figure_splits_the_columns_into_sections() {
        let boxes = [
            r(50.0, 100.0, 220.0, 100.0),
            r(320.0, 100.0, 220.0, 100.0),
            r(50.0, 220.0, 490.0, 100.0), // figure
            r(50.0, 340.0, 220.0, 100.0),
            r(320.0, 340.0, 220.0, 100.0),
        ];
        assert_eq!(run(&boxes, false), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_single_column_reads_top_to_bottom() {
        let boxes = [
            r(50.0, 300.0, 400.0, 50.0),
            r(50.0, 100.0, 400.0, 50.0),
            r(50.0, 200.0, 400.0, 50.0),
        ];
        assert_eq!(run(&boxes, false), [1, 2, 0]);
    }

    #[test]
    fn a_depth_limit_falls_back_to_top_to_bottom() {
        let boxes = [r(320.0, 100.0, 200.0, 50.0), r(50.0, 120.0, 200.0, 50.0)];
        let got = order(&boxes, &[3, 3], false, 0, &CancelToken::new());
        assert_eq!(got.map(|o| o.order), Ok(vec![0, 1]));
    }

    #[test]
    fn a_cancelled_token_stops_the_cut() {
        let cancel = CancelToken::new();
        cancel.cancel();
        let boxes = [r(50.0, 100.0, 220.0, 80.0), r(320.0, 100.0, 220.0, 80.0)];
        assert_eq!(
            order(&boxes, &[3, 3], false, 64, &cancel).map(|o| o.order),
            Err(Cancelled)
        );
    }

    fn reason(boxes: &[RectPt], lines: &[usize]) -> Option<&'static str> {
        order(boxes, lines, false, 64, &CancelToken::new())
            .ok()
            .and_then(|o| o.doubt)
            .map(|d| d.reason)
    }

    #[test]
    fn a_pull_quote_across_the_gutter_cuts_the_columns_into_rows_and_is_doubted() {
        // Two columns, with a boxed quote over the gutter halfway down that shares height with
        // the paragraphs beside it, which are narrowed round it.
        let boxes = [
            r(50.0, 100.0, 220.0, 150.0),
            r(320.0, 100.0, 220.0, 150.0),
            r(50.0, 270.0, 160.0, 80.0),
            r(200.0, 265.0, 190.0, 90.0), // the quote
            r(380.0, 270.0, 160.0, 80.0),
            r(50.0, 370.0, 220.0, 150.0),
            r(320.0, 370.0, 220.0, 150.0),
        ];
        assert_eq!(reason(&boxes, &[5; 7]), Some("columns-revisited"));
    }

    #[test]
    fn clean_layouts_raise_no_doubt() {
        let two_columns = [
            r(50.0, 100.0, 220.0, 80.0),
            r(320.0, 100.0, 220.0, 120.0),
            r(50.0, 200.0, 220.0, 80.0),
            r(320.0, 240.0, 220.0, 40.0),
        ];
        assert_eq!(reason(&two_columns, &[3; 4]), None);
        let framed = [
            r(50.0, 40.0, 490.0, 30.0),
            r(50.0, 100.0, 220.0, 100.0),
            r(320.0, 100.0, 220.0, 100.0),
            r(50.0, 220.0, 490.0, 100.0),
            r(50.0, 340.0, 220.0, 100.0),
            r(320.0, 340.0, 220.0, 100.0),
        ];
        assert_eq!(reason(&framed, &[3; 6]), None);
        let sidebar = [r(50.0, 100.0, 330.0, 500.0), r(410.0, 100.0, 130.0, 200.0)];
        assert_eq!(reason(&sidebar, &[20, 8]), None);
        let grid: Vec<RectPt> = (0..9)
            .map(|i| {
                r(
                    56.0 + 160.0 * f64::from(i % 3),
                    90.0 + 40.0 * f64::from(i / 3),
                    120.0,
                    12.0,
                )
            })
            .collect();
        assert_eq!(reason(&grid, &[1; 9]), None);
    }

    #[test]
    fn a_row_of_authors_over_two_columns_or_an_equation_and_its_number_raise_no_doubt() {
        // Three authors over the gutter, then an abstract and a paragraph on the left and one
        // tall column on the right, as on the first page of a paper.
        let authors = [
            r(81.0, 145.0, 126.0, 78.0),
            r(232.0, 145.0, 138.0, 78.0),
            r(383.0, 145.0, 145.0, 78.0),
            r(88.0, 277.0, 187.0, 190.0),
            r(71.0, 480.0, 221.0, 260.0),
            r(306.0, 265.0, 221.0, 467.0),
        ];
        assert_eq!(reason(&authors, &[6, 6, 6, 14, 20, 40]), None);
        // An equation and its number on one row of a column, then the paragraph under both.
        let equation = [
            r(50.0, 100.0, 220.0, 80.0),
            r(90.0, 200.0, 120.0, 20.0),
            r(250.0, 202.0, 20.0, 12.0),
            r(50.0, 240.0, 220.0, 80.0),
        ];
        assert_eq!(reason(&equation, &[5, 2, 1, 5]), None);
    }

    #[test]
    fn a_grid_with_one_wrapped_cell_is_read_by_columns_and_doubted() {
        let mut grid: Vec<RectPt> = (0..9)
            .map(|i| {
                r(
                    56.0 + 160.0 * f64::from(i % 3),
                    90.0 + 40.0 * f64::from(i / 3),
                    120.0,
                    12.0,
                )
            })
            .collect();
        grid[4] = r(216.0, 130.0, 120.0, 26.0);
        let mut lines = [1; 9];
        lines[4] = 2;
        let got = order(&grid, &lines, false, 64, &CancelToken::new()).ok();
        assert_eq!(
            got.as_ref().map(|o| o.order.clone()),
            Some(vec![0, 3, 6, 1, 4, 7, 2, 5, 8])
        );
        assert_eq!(got.and_then(|o| o.doubt).map(|d| d.reason), Some("near-grid"));
    }

    #[test]
    fn second_cuts_stop_at_their_limit() {
        // Many separate near-grids side by side down the page: no panic, and the order is whole.
        let count = MAX_ALTERNATIVES as usize + 4;
        let mut boxes = Vec::new();
        let mut lines = Vec::new();
        for g in 0..count {
            let top = 100.0 + 200.0 * f64::from(u32::try_from(g).unwrap_or(0));
            for i in 0..4u32 {
                boxes.push(r(
                    56.0 + 160.0 * f64::from(i % 2),
                    top + 40.0 * f64::from(i / 2),
                    120.0,
                    12.0,
                ));
                lines.push(if i == 3 { 2 } else { 1 });
            }
        }
        let got = order(&boxes, &lines, false, 64, &CancelToken::new());
        assert_eq!(got.map(|o| o.order.len()), Ok(boxes.len()));
    }
}
