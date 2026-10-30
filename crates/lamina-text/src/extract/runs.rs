//! Glyphs to runs and runs to lines (todo B9d; research 03 §5.3, §8.1).

use std::ops::Range;
use std::sync::Arc;

use lamina_model::{Pt, RectPt};
use lamina_syntax::ObjRef;

use icu_properties::CodePointMapData;
use icu_properties::props::CanonicalCombiningClass;

use super::Glyph;
use crate::{BidiParagraph, grapheme_boundaries};

/// Consecutive glyphs drawn alike: one font, size, writing direction, MCID, `ActualText` span
/// and visibility, one after the other on one baseline.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextRun {
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
    /// The union of the glyphs' boxes, in page space.
    pub bounds: RectPt,
}

/// A baseline moving by more than this fraction of the glyph size starts a new run and line.
const LINE_SHIFT: f64 = 0.5;

/// The pen jumping forward by more than this fraction of the size inserts a space.
const SPACE_GAP: f64 = 0.25;

/// The pen jumping back by more than this fraction of the size starts a new run (the next
/// run of a right-to-left line drawn in logical order, or an overprint).
const BACK_JUMP: f64 = 0.5;

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
}

/// Groups `glyphs` into runs.
pub(super) fn runs(glyphs: &[Glyph], actual_texts: &[String]) -> Vec<TextRun> {
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
            Some(TextRun {
                text,
                glyphs: range,
                font: first.font,
                font_name: Arc::clone(&first.font_name),
                font_size: first.size,
                vertical: first.vertical,
                mcid: first.mcid,
                actual_text: first.actual_text.is_some(),
                invisible: first.invisible,
                bounds: bounds(run),
            })
        })
        .collect()
}

/// The glyphs' text in drawing order, with a space where the pen jumps forward.
fn joined(run: &[Glyph]) -> String {
    let mut text = String::new();
    let mut previous: Option<&Glyph> = None;
    for glyph in run {
        if let Some(p) = previous
            && let Some(direction) = unit(p.advance)
        {
            let (_, along) = offset(end_of(p), direction, glyph);
            if needs_space(&text, &glyph.text, along, p.size.max(glyph.size)) {
                text.push(' ');
            }
        }
        text.push_str(&glyph.text);
        previous = Some(glyph);
    }
    text
}

/// Whether a glyph rides on a letter rather than standing in the line: it doesn't move the
/// pen (marks such as Hebrew points and Arabic harakat, and textless format glyphs).
fn is_mark(glyph: &Glyph) -> bool {
    unit(glyph.advance).is_none()
}

/// A right-to-left run's text in visual order, from the glyphs' positions rather than their
/// drawing order: each mark goes with the letter whose advance it sits in (else the nearest
/// letter), and letters are taken left to right along the baseline, with a space where the gap
/// between two is over a quarter of the size. Producers draw a mark before or after its letter;
/// only the position says which letter it belongs to.
fn by_position(run: &[Glyph]) -> String {
    let direction = run.iter().find_map(|g| unit(g.advance)).unwrap_or((1.0, 0.0));
    let along = |p: Vector| p.0 * direction.0 + p.1 * direction.1;
    let start = |g: &Glyph| along((g.origin.x, g.origin.y));
    let letters: Vec<&Glyph> = run.iter().filter(|g| !is_mark(g)).collect();
    let mut marks: Vec<Vec<&Glyph>> = vec![Vec::new(); letters.len()];
    for mark in run.iter().filter(|g| is_mark(g)) {
        let at = start(mark);
        let inside = letters.iter().position(|l| {
            let (s, e) = (start(l), along(end_of(l)));
            s.min(e) - 0.01 <= at && at <= s.max(e) + 0.01
        });
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
        if let Some(slot) = inside.or_else(nearest).and_then(|i| marks.get_mut(i)) {
            slot.push(mark);
        }
    }
    let mut order: Vec<usize> = (0..letters.len()).collect();
    order.sort_by(|&a, &b| {
        let key = |i: usize| letters.get(i).map_or(0.0, |g| start(g));
        key(a).total_cmp(&key(b))
    });
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
        let classes = CodePointMapData::<CanonicalCombiningClass>::new();
        on_letter.sort_by_key(|m| m.text.chars().next().map_or(0, |c| classes.get(c).0));
        for mark in on_letter {
            text.push_str(&mark.text);
        }
        previous = Some(letter);
    }
    text
}

fn needs_space(before: &str, after: &str, gap: f64, size: f64) -> bool {
    gap > SPACE_GAP * size
        && !before.is_empty()
        && !before.ends_with(char::is_whitespace)
        && !after.starts_with(char::is_whitespace)
}

/// Whether `text` has a character of a right-to-left script (Hebrew, Arabic, Syriac, Thaana,
/// N’Ko and their presentation forms).
fn has_right_to_left(text: &str) -> bool {
    text.chars()
        .any(|c| matches!(c, '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}'))
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

/// The runs' text as lines (see [`PageText::lines`](super::PageText::lines)).
pub(super) fn lines(glyphs: &[Glyph], runs: &[TextRun]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    // The previous run's end, writing direction and size.
    let mut previous: Option<(Vector, Vector, f64)> = None;
    for run in runs {
        let (Some(first), Some(last)) = (
            glyphs.get(run.glyphs.start),
            run.glyphs.end.checked_sub(1).and_then(|i| glyphs.get(i)),
        ) else {
            continue;
        };
        let direction = unit(first.advance)
            .or(previous.map(|(_, d, _)| d))
            .unwrap_or((1.0, 0.0));
        if let Some((end, prev_direction, size)) = previous {
            let size = size.max(run.font_size).max(f64::EPSILON);
            let (across, along) = offset(end, prev_direction, first);
            let turned = direction.0 * prev_direction.0 + direction.1 * prev_direction.1 < 0.9;
            if across > LINE_SHIFT * size || turned {
                lines.push(std::mem::take(&mut line));
            } else if needs_space(&line, &run.text, along, size) {
                line.push(' ');
            }
        }
        line.push_str(&run.text);
        let last_direction = unit(last.advance).unwrap_or(direction);
        previous = Some((end_of(last), last_direction, run.font_size));
    }
    lines.push(line);
    lines
        .into_iter()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
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
    fn left_to_right_text_is_unchanged() {
        assert_eq!(logical("Hello, world"), "Hello, world");
    }

    #[test]
    fn digits_inside_right_to_left_text_keep_their_order() {
        assert_eq!(logical("123 םולש"), "שלום 123");
    }
}
