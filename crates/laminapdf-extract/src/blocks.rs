//! Lines to blocks ([ADR 0030] §3): a line joins the block above it when it follows at a line's
//! pitch, at a similar size, under it. This is Docstrum's grouping by gap and alignment without
//! its nearest-neighbour search.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use laminapdf_model::RectPt;

use crate::lines::Line;

/// A line follows the one above in the same block when its baseline is at most this many
/// sizes lower (a line pitch is about 1.2 sizes; paragraph spacing pushes past this).
const PITCH: f64 = 1.6;

/// Lines whose sizes differ by more than this fraction are in different blocks.
const SIZE_CHANGE: f64 = 0.2;

/// Lines that overlap horizontally by at least this fraction of the narrower one are in one
/// column (first-line indents and ragged right edges still overlap that much).
const OVERLAP: f64 = 0.3;

/// A block before it is ordered: its lines, top to bottom.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Draft {
    /// Indices into the page's lines.
    pub(crate) lines: Vec<usize>,
    pub(crate) bbox: RectPt,
}

fn overlap(a: &RectPt, b: &RectPt) -> f64 {
    let shared = a.x1.get().min(b.x1.get()) - a.x0.get().max(b.x0.get());
    let narrower = a.width().get().min(b.width().get());
    if narrower <= 0.0 { 0.0 } else { shared / narrower }
}

/// Whether `next` continues the block whose last line is `last`.
fn continues(last: &Line, next: &Line) -> bool {
    let size = last.size.get().max(next.size.get());
    let pitch = next.baseline.get() - last.baseline.get();
    let similar = (last.size.get() - next.size.get()).abs() <= SIZE_CHANGE * size;
    pitch > 0.0 && pitch <= PITCH * size && similar && overlap(&last.bbox, &next.bbox) >= OVERLAP
}

/// The blocks of `lines` (top to bottom, as `lines::lines` returns them). A vertical line is a
/// block of its own.
pub(crate) fn blocks(lines: &[Line], vertical: impl Fn(&Line) -> bool) -> Vec<Draft> {
    let mut drafts: Vec<Draft> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let target = if vertical(line) {
            None
        } else {
            drafts
                .iter_mut()
                .filter_map(|draft| {
                    let last = draft.lines.last().and_then(|&i| lines.get(i))?;
                    continues(last, line).then(|| (line.baseline.get() - last.baseline.get(), draft))
                })
                .min_by(|(a, _), (b, _)| a.total_cmp(b))
                .map(|(_, draft)| draft)
        };
        match target {
            Some(draft) => {
                draft.lines.push(index);
                draft.bbox = draft.bbox.union(&line.bbox);
            }
            None => drafts.push(Draft {
                lines: vec![index],
                bbox: line.bbox,
            }),
        }
    }
    drafts
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::blocks;
    use crate::lines::Line;

    fn line(x: f64, baseline: f64, width: f64, size: f64) -> Line {
        Line {
            pieces: Vec::new(),
            bbox: RectPt::from_points(x, baseline - size * 0.8, width, size),
            baseline: Pt::new(baseline),
            size: Pt::new(size),
            text: String::new(),
            rtl: false,
        }
    }

    fn groups(lines: &[Line]) -> Vec<Vec<usize>> {
        blocks(lines, |_| false).into_iter().map(|d| d.lines).collect()
    }

    #[test]
    fn lines_at_a_line_pitch_form_one_block() {
        let lines = [
            line(50.0, 100.0, 400.0, 10.0),
            line(50.0, 112.0, 380.0, 10.0),
            line(50.0, 124.0, 200.0, 10.0),
        ];
        assert_eq!(groups(&lines), [vec![0, 1, 2]]);
    }

    #[test]
    fn a_wide_gap_starts_a_new_block() {
        let lines = [line(50.0, 100.0, 400.0, 10.0), line(50.0, 140.0, 400.0, 10.0)];
        assert_eq!(groups(&lines), [vec![0], vec![1]]);
    }

    #[test]
    fn a_size_change_starts_a_new_block() {
        let lines = [line(50.0, 100.0, 400.0, 18.0), line(50.0, 118.0, 400.0, 10.0)];
        assert_eq!(groups(&lines), [vec![0], vec![1]]);
    }

    #[test]
    fn two_columns_make_two_blocks_whatever_the_line_order() {
        // Left and right lines interleaved, as one baseline sweep finds them.
        let lines = [
            line(50.0, 100.0, 200.0, 10.0),
            line(300.0, 100.0, 200.0, 10.0),
            line(50.0, 112.0, 200.0, 10.0),
            line(300.0, 112.0, 200.0, 10.0),
        ];
        assert_eq!(groups(&lines), [vec![0, 2], vec![1, 3]]);
    }

    #[test]
    fn a_vertical_line_is_a_block_of_its_own() {
        let lines = [line(50.0, 100.0, 400.0, 10.0), line(50.0, 112.0, 400.0, 10.0)];
        let got: Vec<_> = blocks(&lines, |l| l.baseline.get() > 105.0)
            .into_iter()
            .map(|d| d.lines)
            .collect();
        assert_eq!(got, [vec![0], vec![1]]);
    }
}
