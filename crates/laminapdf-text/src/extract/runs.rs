//! Glyphs to runs and runs to lines (research 03 §5.3, §8.1).

use std::ops::Range;
use std::sync::Arc;

use laminapdf_model::{Pt, RectPt};
use laminapdf_syntax::ObjRef;

use icu_properties::CodePointMapData;
use icu_properties::props::{CanonicalCombiningClass, GeneralCategory};

use super::Glyph;
use crate::{BidiParagraph, grapheme_boundaries};

/// Consecutive glyphs drawn alike: one font, size, writing direction, MCID, `ActualText` span,
/// visibility, artifact marking, clip box and colour, one after the other on one baseline. A
/// word in another colour, or text crossing a clip's edge, starts a run of its own, so each run
/// is either all hidden or all visible.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about one run, each read on its own"
)]
pub struct PageTextRun {
    /// The run's text in logical order: the span's `ActualText` (the span's first run; its
    /// later runs have none), else the glyphs' text with gap spaces, put in logical order
    /// when it has right-to-left characters.
    pub text: String,
    /// The run's glyphs: indices into [`PageText::glyphs`](super::PageText::glyphs).
    pub glyphs: Range<usize>,
    /// The font's object, when the font dictionary is indirect.
    pub font: Option<ObjRef>,
    /// The font's `/BaseFont`.
    pub font_name: Arc<str>,
    /// The glyph size in user space.
    pub font_size: f64,
    /// Whether the font writes vertically.
    pub vertical: bool,
    /// The innermost marked-content identifier (§14.7.5).
    pub mcid: Option<i64>,
    /// Whether `text` comes from an `ActualText` span.
    pub actual_text: bool,
    /// Drawn in text rendering mode 3 or 7 (OCR layers).
    pub invisible: bool,
    /// Inside marked content tagged `Artifact` (§14.8.2.2).
    pub artifact: bool,
    /// The clip's bounding box in page space, as [`Glyph::clip`](super::Glyph::clip).
    pub clip: Option<RectPt>,
    /// The colour it is painted in, as [`Glyph::color`](super::Glyph::color).
    pub color: Option<[f64; 3]>,
    /// The union of the glyphs' boxes, in page space.
    pub bounds: RectPt,
}

/// A baseline moving by more than this fraction of the glyph size starts a new run and line.
const LINE_SHIFT: f64 = 0.5;

/// A gap of more than this fraction of the size between two pieces of text is a space (the pen
/// jumping forward, or, between runs, a run lying wholly behind the previous one).
const SPACE_GAP: f64 = 0.25;

/// The pen jumping back by more than this fraction of the size starts a new run (the next
/// run of a right-to-left line drawn in logical order, or an overprint).
const BACK_JUMP: f64 = 0.5;

/// A nonspacing mark whose origin lies before the next letter's origin by at most this fraction
/// of the size belongs to that letter (see [`by_position`]).
const MARK_LEAD: f64 = 0.1;

/// A vector `(x, y)` in user space.
type Vector = (f64, f64);

fn unit((x, y): Vector) -> Option<Vector> {
    let length = x.hypot(y);
    (length > f64::EPSILON).then(|| (x / length, y / length))
}

/// Where the pen stands after `glyph`.
fn end_of(glyph: &Glyph) -> Vector {
    (glyph.origin.x + glyph.advance.0, glyph.origin.y + glyph.advance.1)
}

/// How `next` sits relative to the pen at `end` writing in `direction`: the move across the
/// baseline (absolute) and along it.
fn offset(end: Vector, direction: Vector, next: &Glyph) -> (f64, f64) {
    let delta = (next.origin.x - end.0, next.origin.y - end.1);
    let across = (delta.1 * direction.0 - delta.0 * direction.1).abs();
    let along = delta.0 * direction.0 + delta.1 * direction.1;
    (across, along)
}

fn alike(a: &Glyph, b: &Glyph) -> bool {
    a.font == b.font
        && a.font_name == b.font_name
        && (a.size - b.size).abs() <= 0.01 * a.size.max(b.size)
        && a.vertical == b.vertical
        && a.mcid == b.mcid
        && a.actual_text == b.actual_text
        && a.invisible == b.invisible
        && a.artifact == b.artifact
        && a.clip == b.clip
        && a.color == b.color
}

/// Groups `glyphs` into runs.
pub(super) fn runs(glyphs: &[Glyph], actual_texts: &[String]) -> Vec<PageTextRun> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut direction: Vector = (1.0, 0.0);
    // The pen after the last glyph that moved it: marks placed by moving back and forth
    // (`TJ` numbers) between letters do not break the run.
    let mut pen: Option<Vector> = None;
    for (i, glyph) in glyphs.iter().enumerate() {
        let joins = i
            .checked_sub(1)
            .and_then(|p| glyphs.get(p))
            .zip(pen)
            .is_some_and(|(previous, pen)| {
                let size = previous.size.max(glyph.size).max(f64::EPSILON);
                let (across, along) = offset(pen, direction, glyph);
                alike(previous, glyph) && across <= LINE_SHIFT * size && along >= -BACK_JUMP * size
            });
        match ranges.last_mut() {
            Some(range) if joins => range.end = i + 1,
            _ => ranges.push(i..i + 1),
        }
        if let Some(d) = unit(glyph.advance) {
            direction = d;
            pen = Some(end_of(glyph));
        } else if !joins || pen.is_none() {
            pen = Some(end_of(glyph));
        }
    }
    let mut spans_started = vec![false; actual_texts.len()];
    // Whether the previous run, on the same baseline, was right to left (a run of only
    // neutral characters between right-to-left ones is drawn right to left too).
    let mut after_right_to_left = false;
    let mut previous_end: Option<(Vector, Vector, f64)> = None;
    ranges
        .into_iter()
        .filter_map(|range| {
            let run = glyphs.get(range.clone())?;
            let first = run.first()?;
            let last = run.last()?;
            let same_line = previous_end.is_some_and(|(end, direction, size)| {
                offset(end, direction, first).0 <= LINE_SHIFT * size.max(first.size).max(f64::EPSILON)
            });
            let right_to_left = run.iter().any(|g| has_right_to_left(&g.text));
            let neutral = !run.iter().any(|g| g.text.chars().any(char::is_alphanumeric));
            let reversed = right_to_left || (neutral && same_line && after_right_to_left);
            after_right_to_left = reversed && (right_to_left || after_right_to_left);
            previous_end = Some((
                end_of(last),
                unit(last.advance).or_else(|| unit(first.advance)).unwrap_or((1.0, 0.0)),
                last.size,
            ));
            let text = match first.actual_text {
                Some(span) => {
                    let started = spans_started.get_mut(span)?;
                    let text = if *started {
                        String::new()
                    } else {
                        actual_texts.get(span).cloned().unwrap_or_default()
                    };
                    *started = true;
                    text
                }
                None if right_to_left => logical(&by_position(run)),
                None if reversed => reverse_clusters(&by_position(run)),
                None => joined(run),
            };
            Some(PageTextRun {
                text,
                glyphs: range,
                font: first.font,
                font_name: Arc::clone(&first.font_name),
                font_size: first.size,
                vertical: first.vertical,
                mcid: first.mcid,
                actual_text: first.actual_text.is_some(),
                invisible: first.invisible,
                artifact: first.artifact,
                clip: first.clip,
                color: first.color,
                bounds: bounds(run),
            })
        })
        .collect()
}

/// The glyphs' text in drawing order, with a space where the pen jumps forward.
fn joined(run: &[Glyph]) -> String {
    let mut text = String::new();
    let mut previous: Option<&Glyph> = None;
    // The last writing direction seen: a zero-width glyph (a combining mark) has none.
    let mut direction: Vector = run.iter().find_map(|g| unit(g.advance)).unwrap_or((1.0, 0.0));
    for glyph in run {
        if let Some(p) = previous {
            let (_, along) = offset(end_of(p), direction, glyph);
            if needs_space(&text, &glyph.text, along, p.size.max(glyph.size)) {
                text.push(' ');
            }
        }
        text.push_str(&glyph.text);
        if let Some(d) = unit(glyph.advance) {
            direction = d;
        }
        previous = Some(glyph);
    }
    text
}

/// Whether a glyph rides on a letter rather than standing in the line: it doesn't move the
/// pen (marks such as Hebrew points and Arabic harakat, and textless format glyphs).
fn is_mark(glyph: &Glyph) -> bool {
    unit(glyph.advance).is_none()
}

/// Whether all of a glyph's text is nonspacing or enclosing marks (general category Mn or Me).
fn is_nonspacing(glyph: &Glyph) -> bool {
    let categories = CodePointMapData::<GeneralCategory>::new();
    !glyph.text.is_empty()
        && glyph.text.chars().all(|c| {
            matches!(
                categories.get(c),
                GeneralCategory::NonspacingMark | GeneralCategory::EnclosingMark
            )
        })
}

/// A right-to-left run's text in visual order, from the glyphs' positions rather than their
/// drawing order: each mark goes with the letter whose advance it sits in (else the nearest
/// letter), and letters are taken left to right along the baseline, with a space where the gap
/// between two is over a quarter of the size. Producers draw a mark before or after its letter;
/// the position says which letter it belongs to, except at the edge between two letters.
/// Shaping engines (HarfBuzz) draw a right-to-left cluster in visual order with its marks
/// first, each at its base's origin moved by the anchor offset, which can put the mark's origin
/// just inside the letter drawn before it (a damma on a narrow letter in Noto Sans Arabic). So a
/// nonspacing mark whose origin lies before the next letter's origin in content order, by at
/// most [`MARK_LEAD`] of the size, goes with that letter. A mark at that origin exactly, drawn at
/// the pen after its letter, stays with the letter whose advance holds it.
fn by_position(run: &[Glyph]) -> String {
    let direction = run.iter().find_map(|g| unit(g.advance)).unwrap_or((1.0, 0.0));
    let along = |p: Vector| p.0 * direction.0 + p.1 * direction.1;
    let start = |g: &Glyph| along((g.origin.x, g.origin.y));
    // A run where no glyph moves the pen (a font without widths, words placed by `Td`) has
    // only letters. Check for an advancing glyph once per run to avoid a quadratic scan.
    let has_letter = run.iter().any(|l| !is_mark(l));
    let marked = |g: &&Glyph| has_letter && is_mark(g);
    let mut letters: Vec<&Glyph> = Vec::new();
    // Each mark with the index the next letter in content order has (or will have) in `letters`.
    let mut pending: Vec<(&Glyph, usize)> = Vec::new();
    for glyph in run {
        if marked(&glyph) {
            pending.push((glyph, letters.len()));
        } else {
            letters.push(glyph);
        }
    }
    let mut marks: Vec<Vec<&Glyph>> = vec![Vec::new(); letters.len()];
    for (mark, next) in pending {
        let at = start(mark);
        let leads = || {
            letters
                .get(next)
                .filter(|l| {
                    let lead = start(l) - at;
                    lead > 0.01 && lead <= MARK_LEAD * mark.size.max(l.size) && is_nonspacing(mark)
                })
                .map(|_| next)
        };
        let inside = || {
            letters.iter().position(|l| {
                let (s, e) = (start(l), along(end_of(l)));
                s.min(e) - 0.01 <= at && at <= s.max(e) + 0.01
            })
        };
        let nearest = || {
            letters
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let centre = |g: &Glyph| f64::midpoint(start(g), along(end_of(g)));
                    (centre(a) - at).abs().total_cmp(&(centre(b) - at).abs())
                })
                .map(|(i, _)| i)
        };
        if let Some(slot) = leads().or_else(inside).or_else(nearest).and_then(|i| marks.get_mut(i)) {
            slot.push(mark);
        }
    }
    let mut order: Vec<usize> = (0..letters.len()).collect();
    order.sort_by(|&a, &b| {
        let key = |i: usize| letters.get(i).map_or(0.0, |g| start(g));
        key(a).total_cmp(&key(b))
    });
    let classes = CodePointMapData::<CanonicalCombiningClass>::new();
    let mut text = String::new();
    let mut previous: Option<&Glyph> = None;
    for i in order {
        let Some(letter) = letters.get(i) else { continue };
        if let Some(p) = previous {
            let gap = start(letter) - along(end_of(p));
            if needs_space(&text, &letter.text, gap, p.size.max(letter.size)) {
                text.push(' ');
            }
        }
        text.push_str(&letter.text);
        // A letter's marks in canonical order (by combining class, stable), as the text
        // they were shaped from has them: producers draw them in any order.
        let mut on_letter: Vec<&Glyph> = marks.get(i).cloned().unwrap_or_default();
        on_letter.sort_by_key(|m| m.text.chars().next().map_or(0, |c| classes.get(c).0));
        for mark in on_letter {
            text.push_str(&mark.text);
        }
        previous = Some(letter);
    }
    text
}

/// Whether a gap of `gap` between `before` and `after` is a space: over a quarter of the size,
/// both sides with text (an `ActualText` span's later runs have none), neither with whitespace
/// at the join.
fn needs_space(before: &str, after: &str, gap: f64, size: f64) -> bool {
    gap > SPACE_GAP * size
        && !before.is_empty()
        && !after.is_empty()
        && !before.ends_with(char::is_whitespace)
        && !after.starts_with(char::is_whitespace)
}

/// Whether `text` has a character of a right-to-left script (Hebrew, Arabic, Syriac, Thaana,
/// N’Ko and their presentation forms).
fn has_right_to_left(text: &str) -> bool {
    text.chars().any(is_right_to_left)
}

/// Whether `c` belongs to a script written right to left: Hebrew, Arabic, Syriac, Thaana, N'Ko
/// and the scripts up to U+08FF, and the Hebrew and Arabic presentation forms (not the BOM,
/// U+FEFF, at the end of the last block).
#[must_use]
pub const fn is_right_to_left(c: char) -> bool {
    matches!(c, '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFC}')
}

/// Text drawn in visual order, in logical order: UAX #9 run levels of the text as it stands,
/// each right-to-left run's grapheme clusters reversed, the runs in the order UAX #9's L2
/// gives (an involution for the text a single run holds). Left-to-right text is unchanged.
fn logical(visual: &str) -> String {
    if !has_right_to_left(visual) {
        return visual.to_owned();
    }
    let paragraph = BidiParagraph::new(visual, None);
    let Ok(runs) = paragraph.visual_runs(0..visual.len()) else {
        return visual.to_owned();
    };
    let mut out = String::with_capacity(visual.len());
    for run in runs {
        let Some(part) = visual.get(run.range.clone()) else {
            continue;
        };
        if run.level % 2 == 1 {
            out.push_str(&reverse_clusters(part));
        } else {
            out.push_str(part);
        }
    }
    out
}

/// `text`'s grapheme clusters in reverse order.
fn reverse_clusters(text: &str) -> String {
    let bounds = grapheme_boundaries(text);
    bounds
        .windows(2)
        .rev()
        .filter_map(|pair| match *pair {
            [start, end] => text.get(start..end),
            _ => None,
        })
        .collect()
}

/// The smallest page-space rectangle around the glyphs' boxes.
fn bounds(run: &[Glyph]) -> RectPt {
    let points = run.iter().flat_map(|g| g.quad.iter());
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        x0 = x0.min(p.x.get());
        y0 = y0.min(p.y.get());
        x1 = x1.max(p.x.get());
        y1 = y1.max(p.y.get());
    }
    if x0 > x1 {
        return RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(0.0), Pt::new(0.0));
    }
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

/// One run on a line, for ordering the line.
struct Unit<'r> {
    text: &'r str,
    /// Where the run starts and ends along the line's direction.
    start: f64,
    end: f64,
    size: f64,
    /// 'R' (right-to-left letters), 'L' (other letters or digits) or 'N' (neither).
    class: char,
}

impl Unit<'_> {
    fn new<'r>(run: &'r PageTextRun, first: &Glyph, last: &Glyph, direction: Vector) -> Unit<'r> {
        let along = |p: Vector| p.0 * direction.0 + p.1 * direction.1;
        let class = if has_right_to_left(&run.text) {
            'R'
        } else if run.text.chars().any(char::is_alphanumeric) {
            'L'
        } else {
            'N'
        };
        Unit {
            text: &run.text,
            start: along((first.origin.x, first.origin.y)),
            end: along(end_of(last)),
            size: run.font_size,
            class,
        }
    }
}

/// The runs' text as lines (see [`PageText::lines`](super::PageText::lines)).
pub(super) fn lines(glyphs: &[Glyph], runs: &[PageTextRun]) -> Vec<String> {
    // Group runs into lines, in content order.
    let mut grouped: Vec<Vec<Unit<'_>>> = vec![Vec::new()];
    let mut previous: Option<(Vector, Vector, f64)> = None;
    let mut line_direction: Vector = (1.0, 0.0);
    for run in runs {
        let last_index = run.glyphs.end.checked_sub(1);
        let (Some(first), Some(last)) = (glyphs.get(run.glyphs.start), last_index.and_then(|i| glyphs.get(i))) else {
            continue;
        };
        let direction = unit(first.advance)
            .or(previous.map(|(_, d, _)| d))
            .unwrap_or((1.0, 0.0));
        if let Some((end, prev_direction, size)) = previous {
            let size = size.max(run.font_size).max(f64::EPSILON);
            let (across, _) = offset(end, prev_direction, first);
            let turned = direction.0 * prev_direction.0 + direction.1 * prev_direction.1 < 0.9;
            if across > LINE_SHIFT * size || turned {
                grouped.push(Vec::new());
                line_direction = direction;
            }
        } else {
            line_direction = direction;
        }
        if let Some(line) = grouped.last_mut() {
            line.push(Unit::new(run, first, last, line_direction));
        }
        let last_direction = unit(last.advance).unwrap_or(direction);
        previous = Some((end_of(last), last_direction, run.font_size));
    }
    grouped
        .into_iter()
        .map(|units| {
            if units.iter().any(|u| u.class == 'R') {
                bidi_line(units)
            } else {
                join(&units)
            }
        })
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Units joined in the order given, with a space between two that don't overlap with a gap of
/// over a quarter of the size: the next one starts that far after the previous one ends, or
/// lies wholly that far behind its start (a table cell or column drawn out of order). Units
/// that overlap (an accent drawn over its letter, an overprint, fake bold) or abut (glyphs
/// drawn backwards one by one) get none.
fn join(units: &[Unit<'_>]) -> String {
    let mut line = String::new();
    let mut previous: Option<&Unit<'_>> = None;
    for u in units {
        if let Some(p) = previous
            && needs_space(
                &line,
                u.text,
                (u.start - p.end).max(p.start - u.end),
                p.size.max(u.size).max(f64::EPSILON),
            )
        {
            line.push(' ');
        }
        line.push_str(u.text);
        previous = Some(u);
    }
    line
}

/// A line with right-to-left runs, its runs in logical order whatever order they were drawn
/// in: UAX #9 at the level of runs. The runs are taken in visual order (by position); the
/// paragraph is right to left when it has more right-to-left letters than others; a neutral
/// run takes the direction of the strong runs on both sides when they agree, else the
/// paragraph's (rules N1, N2); levels are 1 for right to left, 0 or 2 for left to right
/// (I1, I2), and L2 reverses each sequence at or above each level from the highest down. Each
/// run's own text is already in logical order (see [`runs`]). A space goes between two runs
/// that are neighbours on the page with a gap of over a quarter of the size.
fn bidi_line(mut units: Vec<Unit<'_>>) -> String {
    units.sort_by(|a, b| a.start.total_cmp(&b.start));
    let count = |class: char| {
        units
            .iter()
            .filter(|u| u.class == class)
            .map(|u| u.text.chars().filter(|c| c.is_alphabetic()).count())
            .sum::<usize>()
    };
    let paragraph = if count('R') > count('L') { 'R' } else { 'L' };
    let strong: Vec<Option<char>> = units.iter().map(|u| (u.class != 'N').then_some(u.class)).collect();
    let resolved: Vec<char> = (0..units.len())
        .map(|i| {
            if let Some(class) = strong.get(i).copied().flatten() {
                return class;
            }
            let before = strong.get(..i).and_then(|s| s.iter().rev().find_map(|c| *c));
            let after = strong.get(i + 1..).and_then(|s| s.iter().find_map(|c| *c));
            match (before, after) {
                (Some(b), Some(a)) if a == b => a,
                _ => paragraph,
            }
        })
        .collect();
    let levels: Vec<u8> = resolved
        .iter()
        .map(|&class| match (class, paragraph) {
            ('R', _) => 1,
            (_, 'R') => 2,
            _ => 0,
        })
        .collect();
    // Visual neighbours with a gap between them, by unit index.
    let gaps: Vec<bool> = units
        .windows(2)
        .map(|pair| match pair {
            [a, b] => b.start - a.end > SPACE_GAP * a.size.max(b.size).max(f64::EPSILON),
            _ => false,
        })
        .collect();
    let mut order: Vec<usize> = (0..units.len()).collect();
    for level in (1..=levels.iter().copied().max().unwrap_or(0)).rev() {
        let mut i = 0;
        while i < order.len() {
            let at = |k: usize| order.get(k).and_then(|&u| levels.get(u)).copied().unwrap_or(0);
            if at(i) >= level {
                let mut j = i;
                while j < order.len() && at(j) >= level {
                    j += 1;
                }
                if let Some(slice) = order.get_mut(i..j) {
                    slice.reverse();
                }
                i = j;
            } else {
                i += 1;
            }
        }
    }
    let mut line = String::new();
    let mut previous: Option<usize> = None;
    for &u in &order {
        let Some(unit) = units.get(u) else { continue };
        if let Some(p) = previous {
            let neighbours_with_gap = gaps.get(p.min(u)).copied().unwrap_or(false) && p.abs_diff(u) == 1;
            if neighbours_with_gap
                && !line.is_empty()
                && !line.ends_with(char::is_whitespace)
                && !unit.text.starts_with(char::is_whitespace)
            {
                line.push(' ');
            }
        }
        line.push_str(unit.text);
        previous = Some(u);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::logical;

    #[test]
    fn visual_hebrew_reverses_by_cluster_keeping_points_on_their_letters() {
        // "שָׁלוֹם" drawn left to right: ם, וֹ, ל, שָׁ (base letter, then its points).
        assert_eq!(logical("םוֹלשָׁ"), "שָׁלוֹם");
    }

    #[test]
    fn a_visual_arabic_line_comes_out_in_logical_order() {
        assert_eq!(logical("مكيلع مالسلا"), "السلام عليكم");
    }

    #[test]
    fn a_feff_does_not_make_a_run_right_to_left() {
        assert!(!super::has_right_to_left("Hello\u{FEFF}"));
    }

    #[test]
    fn left_to_right_text_is_unchanged() {
        assert_eq!(logical("Hello, world"), "Hello, world");
    }

    #[test]
    fn digits_inside_right_to_left_text_keep_their_order() {
        assert_eq!(logical("123 םולש"), "שלום 123");
    }
}
