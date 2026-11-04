//! Reading order from the layout: XY-Cut++ ([ADR 0030] §2; Nagy and Seth's recursive cuts with
//! [XY-Cut++]'s masking of cross-column blocks).
//!
//! Blocks wider than [`WIDE`] of the page's text and sharing no height with a narrower block
//! (titles, full-width figures and paragraphs) are masked first: they split the page into
//! bands, each band is cut on its own, and the masked blocks go back between the bands. A band
//! is cut recursively at the widest gap in the projection of its boxes, a column gap before a
//! row gap on a tie; a group with no gap reads top to bottom.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
//! [XY-Cut++]: https://arxiv.org/abs/2504.10258

use std::cmp::Ordering;

use laminapdf_model::RectPt;
use laminapdf_syntax::CancelToken;

/// A block at least this fraction of the page's text width is a masking candidate.
const WIDE: f64 = 0.6;

/// Gaps narrower than this, in points, are not cuts (kerning, overlapping descenders).
const MIN_GAP: f64 = 1.0;

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
    rtl: bool,
    max_depth: u32,
    cancel: &'a CancelToken,
}

fn rect(boxes: &[RectPt], i: usize) -> RectPt {
    boxes.get(i).copied().unwrap_or_default()
}

/// The reading order of `boxes` (indices), columns right to left when `rtl`.
pub(crate) fn order(
    boxes: &[RectPt],
    rtl: bool,
    max_depth: u32,
    cancel: &CancelToken,
) -> Result<Vec<usize>, Cancelled> {
    let cutter = Cutter {
        boxes,
        rtl,
        max_depth,
        cancel,
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
    for (k, band) in bands.into_iter().enumerate() {
        cutter.cut(band, 0, &mut out)?;
        if let Some(&m) = masked.get(k) {
            out.push(m);
        }
    }
    Ok(out)
}

fn centre_y(r: &RectPt) -> f64 {
    f64::midpoint(r.y0.get(), r.y1.get())
}

impl Cutter<'_> {
    /// The masked blocks (top to bottom) and the rest.
    fn mask(&self, all: &[usize]) -> (Vec<usize>, Vec<usize>) {
        let (x0, x1) = all.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &i| {
            let r = rect(self.boxes, i);
            (a.min(r.x0.get()), b.max(r.x1.get()))
        });
        let width = x1 - x0;
        let wide = |i: usize| rect(self.boxes, i).width().get() >= WIDE * width;
        let mut masked: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&i| {
                let r = rect(self.boxes, i);
                wide(i)
                    && all.iter().all(|&j| {
                        let o = rect(self.boxes, j);
                        wide(j) || o.y1 <= r.y0 || o.y0 >= r.y1
                    })
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
        let axis = match (columns, rows) {
            (Some((c, _)), Some((r, _))) if c >= r => Some((Axis::Columns, columns)),
            (Some(_), None) => Some((Axis::Columns, columns)),
            (_, Some(_)) => Some((Axis::Rows, rows)),
            (None, None) => None,
        };
        let Some((axis, Some((_, at)))) = axis else {
            group.sort_by(|&a, &b| self.reading(a, b));
            out.extend(group);
            return Ok(());
        };
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

    use super::{Cancelled, order};

    fn r(x: f64, y: f64, w: f64, h: f64) -> RectPt {
        RectPt::from_points(x, y, w, h)
    }

    fn run(boxes: &[RectPt], rtl: bool) -> Vec<usize> {
        order(boxes, rtl, 64, &CancelToken::new()).unwrap_or_default()
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
        assert_eq!(order(&boxes, false, 0, &CancelToken::new()), Ok(vec![0, 1]));
    }

    #[test]
    fn a_cancelled_token_stops_the_cut() {
        let cancel = CancelToken::new();
        cancel.cancel();
        let boxes = [r(50.0, 100.0, 220.0, 80.0), r(320.0, 100.0, 220.0, 80.0)];
        assert_eq!(order(&boxes, false, 64, &cancel), Err(Cancelled));
    }
}
