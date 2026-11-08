//! Runs to lines ([ADR 0030] §3): runs on one baseline, split where a gap is wide enough to be
//! a column gutter, joined in visual order.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use std::cmp::Ordering;

use laminapdf_model::{Pt, RectPt};
use laminapdf_text::is_right_to_left;

use crate::lists::is_marker;

/// Baselines closer than this fraction of the size are one line (superscripts stay on it).
const SAME_BASELINE: f64 = 0.5;

/// A gap wider than this fraction of the size between two runs on one baseline is a column
/// gutter, so the runs are on two lines. Word spaces stay well below it, even in justified text.
pub(crate) const GUTTER: f64 = 1.0;

/// A gap wider than this fraction of the size, but not a gutter, is kept on one line yet may be
/// a gutter narrower than one font size: where it is is recorded (`near_gutters`).
pub(crate) const NEAR_GUTTER: f64 = 0.6;

/// A lone list marker joins the text up to this many sizes to its right on one line.
const MARKER_GAP: f64 = 4.0;

/// A gap wider than this fraction of the size between two runs is a space (as `laminapdf-text`
/// uses within a run).
pub(crate) const SPACE: f64 = 0.25;

/// One visible run, in page space.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Piece {
    pub(crate) text: String,
    pub(crate) bbox: RectPt,
    pub(crate) baseline: Pt,
    pub(crate) size: Pt,
    pub(crate) font: String,
    pub(crate) vertical: bool,
    /// The marked-content identifier its run is in, for the structure tree.
    pub(crate) mcid: Option<i64>,
    /// Inside `Artifact` marked content (kept when furniture isn't dropped).
    pub(crate) artifact: bool,
    /// Where the run's pen jumped across a gap kept in it that may be a narrow gutter: the x
    /// at which the text after the jump starts.
    pub(crate) near_gutters: Vec<f64>,
}

/// One line: pieces left to right on one baseline (right to left when the line is).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    /// Indices into the page's pieces, in reading order.
    pub(crate) pieces: Vec<usize>,
    pub(crate) bbox: RectPt,
    pub(crate) baseline: Pt,
    pub(crate) size: Pt,
    pub(crate) text: String,
    pub(crate) rtl: bool,
    /// Where two runs kept on this line sit across a gap that may be a narrow gutter (and the
    /// same from its pieces): the x at which the right-hand one starts, or on a right-to-left
    /// line the x at which it ends.
    pub(crate) near_gutters: Vec<f64>,
}

fn by_value(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// Whether more of `text`'s letters are right to left than left to right.
pub(crate) fn mostly_rtl(text: &str) -> bool {
    let (rtl, ltr) = text
        .chars()
        .filter(|c| c.is_alphabetic())
        .fold((0usize, 0usize), |(r, l), c| {
            if is_right_to_left(c) { (r + 1, l) } else { (r, l + 1) }
        });
    rtl > ltr
}

/// A piece with its index.
type Entry<'p> = (usize, &'p Piece);

/// The lines of `pieces`, top to bottom. A vertical run is a line of its own.
pub(crate) fn lines(pieces: &[Piece]) -> Vec<Line> {
    let mut order: Vec<Entry<'_>> = pieces.iter().enumerate().collect();
    order.sort_by(|(_, a), (_, b)| {
        by_value(a.baseline.get(), b.baseline.get()).then(by_value(a.bbox.x0.get(), b.bbox.x0.get()))
    });
    let mut out = Vec::new();
    let mut group: Vec<Entry<'_>> = Vec::new();
    for entry in order {
        let piece = entry.1;
        if piece.vertical {
            out.push(line(vec![entry], Vec::new()));
            continue;
        }
        let joins = group.first().is_some_and(|(_, anchor)| {
            let size = anchor.size.get().max(piece.size.get());
            (piece.baseline.get() - anchor.baseline.get()).abs() <= SAME_BASELINE * size
        });
        if !joins && !group.is_empty() {
            split(std::mem::take(&mut group), &mut out);
        }
        group.push(entry);
    }
    if !group.is_empty() {
        split(group, &mut out);
    }
    out
}

/// `group` (one baseline) split at gutters, each part a line.
fn split(mut group: Vec<Entry<'_>>, out: &mut Vec<Line>) {
    group.sort_by(|(_, a), (_, b)| by_value(a.bbox.x0.get(), b.bbox.x0.get()));
    // A right-to-left list's marker is its rightmost piece.
    let rtl = mostly_rtl(&group.iter().map(|(_, p)| p.text.as_str()).collect::<String>());
    let count = group.len();
    let mut part: Vec<Entry<'_>> = Vec::new();
    let mut near: Vec<f64> = Vec::new();
    for (index, entry) in group.into_iter().enumerate() {
        let next = entry.1;
        let mut gutter = false;
        if let Some((_, prev)) = part.last() {
            let size = prev.size.get().max(next.size.get());
            let gap = next.bbox.x0.get() - prev.bbox.x1.get();
            // A list's bullet or number is often set a tab away from its text.
            let lone = if rtl {
                index + 1 == count && is_marker(next.text.trim())
            } else {
                part.len() == 1 && is_marker(prev.text.trim())
            };
            gutter = gap > GUTTER * size && !(lone && gap <= MARKER_GAP * size);
            if !gutter && !lone && gap > NEAR_GUTTER * size {
                near.push(if rtl { prev.bbox.x1.get() } else { next.bbox.x0.get() });
            }
        }
        if gutter {
            out.push(line(std::mem::take(&mut part), std::mem::take(&mut near)));
        }
        part.push(entry);
    }
    if !part.is_empty() {
        out.push(line(part, near));
    }
}

/// The line of `members` (left to right), with the narrow-gutter points found between them.
fn line(mut members: Vec<Entry<'_>>, mut near_gutters: Vec<f64>) -> Line {
    let all: String = members.iter().map(|(_, p)| p.text.as_str()).collect();
    let rtl = mostly_rtl(&all);
    if rtl {
        members.reverse();
    }
    let mut text = String::new();
    let mut previous: Option<&Piece> = None;
    let mut bbox: Option<RectPt> = None;
    let mut size = Pt::ZERO;
    for &(_, piece) in &members {
        if let Some(prev) = previous {
            let gap = if rtl {
                prev.bbox.x0.get() - piece.bbox.x1.get()
            } else {
                piece.bbox.x0.get() - prev.bbox.x1.get()
            };
            let spaced = text.ends_with(char::is_whitespace) || piece.text.starts_with(char::is_whitespace);
            if !spaced && gap > SPACE * prev.size.get().max(piece.size.get()) {
                text.push(' ');
            }
        }
        text.push_str(&piece.text);
        bbox = Some(bbox.map_or(piece.bbox, |b| b.union(&piece.bbox)));
        if piece.size > size {
            size = piece.size;
        }
        previous = Some(piece);
        near_gutters.extend_from_slice(&piece.near_gutters);
    }
    Line {
        bbox: bbox.unwrap_or_default(),
        baseline: members.first().map_or(Pt::ZERO, |(_, p)| p.baseline),
        size,
        text,
        rtl,
        pieces: members.iter().map(|(i, _)| *i).collect(),
        near_gutters,
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{Piece, lines, mostly_rtl};

    fn piece(text: &str, x: f64, baseline: f64, width: f64) -> Piece {
        Piece {
            text: text.to_owned(),
            bbox: RectPt::from_points(x, baseline - 9.0, width, 12.0),
            baseline: Pt::new(baseline),
            size: Pt::new(12.0),
            font: "F".to_owned(),
            vertical: false,
            mcid: None,
            artifact: false,
            near_gutters: Vec::new(),
        }
    }

    fn texts(got: &[super::Line]) -> Vec<&str> {
        got.iter().map(|l| l.text.as_str()).collect()
    }

    #[test]
    fn runs_on_one_baseline_are_one_line_with_spaces_at_gaps() {
        let got = lines(&[piece("world", 60.0, 100.0, 30.0), piece("Hello", 20.0, 100.5, 30.0)]);
        assert_eq!(texts(&got), ["Hello world"]);
    }

    #[test]
    fn a_gutter_splits_one_baseline_into_two_lines() {
        let got = lines(&[piece("left", 20.0, 100.0, 200.0), piece("right", 300.0, 100.0, 200.0)]);
        assert_eq!(texts(&got), ["left", "right"]);
    }

    #[test]
    fn a_gap_between_a_word_space_and_a_gutter_is_recorded_as_a_narrow_gutter() {
        // 12 pt text: 9 pt is between 0.6 and 1 size; 2.5 pt is a word space, 15 pt a gutter.
        let got = lines(&[piece("left", 20.0, 100.0, 50.0), piece("right", 79.0, 100.0, 50.0)]);
        assert_eq!(
            got.iter().map(|l| l.near_gutters.clone()).collect::<Vec<_>>(),
            [vec![79.0]]
        );
        let spaced = lines(&[piece("a", 20.0, 100.0, 50.0), piece("b", 72.5, 100.0, 50.0)]);
        assert!(spaced.iter().all(|l| l.near_gutters.is_empty()));
        let apart = lines(&[piece("a", 20.0, 100.0, 50.0), piece("b", 85.0, 100.0, 50.0)]);
        assert_eq!(texts(&apart), ["a", "b"]);
        assert!(apart.iter().all(|l| l.near_gutters.is_empty()));
    }

    #[test]
    fn lines_come_top_to_bottom() {
        let got = lines(&[piece("second", 20.0, 130.0, 50.0), piece("first", 20.0, 100.0, 50.0)]);
        assert_eq!(texts(&got), ["first", "second"]);
    }

    #[test]
    fn a_lone_marker_a_tab_from_its_text_joins_its_line_either_way() {
        let got = lines(&[piece("•", 20.0, 100.0, 6.0), piece("Apples", 44.0, 100.0, 40.0)]);
        assert_eq!(texts(&got), ["• Apples"]);
        let got = lines(&[piece("שלום עולם", 20.0, 100.0, 60.0), piece("•", 100.0, 100.0, 6.0)]);
        assert_eq!(texts(&got), ["• שלום עולם"]);
    }

    #[test]
    fn a_right_to_left_line_is_read_from_its_right_end() {
        let got = lines(&[piece("שלום", 20.0, 100.0, 30.0), piece("עולם", 60.0, 100.0, 30.0)]);
        assert_eq!(texts(&got), ["עולם שלום"]);
        assert!(got.iter().all(|l| l.rtl));
        assert!(!mostly_rtl("Hello שלום world"));
    }
}
