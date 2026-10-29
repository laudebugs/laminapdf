//! Paragraph layout in a box (todo B4b; research 03 §7, §7.4, §7.6): bidi resolution,
//! itemisation, shaping, greedy line breaking at UAX #14 opportunities, per-line visual
//! reordering (UAX #9 rule L2) and alignment.
//!
//! Candidate lines are estimated from each paragraph shaped once, then the chosen line is
//! shaped exactly as it is drawn (its visual runs, the text around it as context), so
//! ligatures, kerning and Arabic joining at a break are accounted for and no line is wider
//! than the box, except a line holding a single grapheme cluster that is wider than the box
//! on its own. A word wider than the box is broken between grapheme clusters
//! (CSS `overflow-wrap: anywhere`). No hyphenation and no justification yet (pdf-library §2
//! non-goals; [`LayoutOptions::justify`] is reserved).

use std::ops::Range;

use icu_properties::CodePointMapData;
use icu_properties::props::BidiClass;

use crate::bidi::BidiParagraph;
use crate::error::{Error, Result};
use crate::font::Font;
use crate::itemize::{Item, itemize};
use crate::linebreak::{grapheme_boundaries, is_hard_break, is_trailing_space, line_breaks};
use crate::shape::{Direction, Feature, ShapeOptions, ShapedRun, Shaper};

/// Where lines sit in the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Alignment {
    /// The paragraph's start side: left for a left-to-right paragraph, right for a
    /// right-to-left one.
    #[default]
    Start,
    /// The paragraph's end side.
    End,
    /// Flush left.
    Left,
    /// Flush right.
    Right,
    /// Centred.
    Center,
}

/// How to lay out a paragraph.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct LayoutOptions {
    /// The box width in points.
    pub width: f32,
    /// The font size in points.
    pub size: f32,
    /// Line alignment.
    pub align: Alignment,
    /// The paragraph direction; `None`: from the first strong character (UAX #9 P2–P3).
    pub direction: Option<Direction>,
    /// The text's language (BCP 47), passed to the shaper.
    pub language: Option<String>,
    /// OpenType features for every run.
    pub features: Vec<Feature>,
    /// Reserved for justification (a later plan): `true` is
    /// [`Error::UnsupportedLayoutOption`].
    pub justify: bool,
}

impl LayoutOptions {
    /// A box `width` points wide, text at `size` points, everything else default.
    #[must_use]
    pub fn new(width: f32, size: f32) -> Self {
        Self {
            width,
            size,
            ..Self::default()
        }
    }

    /// With the alignment set.
    #[must_use]
    pub fn with_align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// With the paragraph direction set.
    #[must_use]
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    /// With the language set.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// With a feature added.
    #[must_use]
    pub fn with_feature(mut self, feature: Feature) -> Self {
        self.features.push(feature);
        self
    }
}

/// One shaped run of a line.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LineRun {
    /// The glyphs (clusters are byte offsets into the paragraph's text).
    pub run: ShapedRun,
    /// The font: an index into the list given to [`break_lines`].
    pub font: usize,
    /// The bidi level the run was shaped at (odd: right to left).
    pub level: u8,
    /// The run's left edge, in points from the line's left edge.
    pub x: f64,
}

/// One line of a paragraph.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Line {
    /// The line's bytes in the text, trailing spaces and the hard break included.
    pub range: Range<usize>,
    /// The bytes drawn: [`Line::range`] without trailing spaces and hard breaks, which hang
    /// past the line's end (UAX #14) and are neither drawn nor measured.
    pub content: Range<usize>,
    /// The runs in visual order, left to right.
    pub runs: Vec<LineRun>,
    /// The width of the drawn content in points.
    pub width: f64,
    /// Where the line starts, in points from the box's left edge (alignment).
    pub x: f64,
    /// The line ended at a hard break (a line feed, U+2028, …) rather than for lack of room
    /// or at the end of the text.
    pub hard_break: bool,
}

/// Lays `text` out in a box `options.width` points wide with greedy (first fit) line
/// breaking, `fonts` in order of preference ([`itemize`]). Each paragraph (ended by a line
/// feed, U+2029 or another UAX #9 paragraph separator) is resolved on its own: its
/// direction, its alignment side and its isolates.
///
/// ```
/// # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
/// use lamina_text::{Font, HarfRust, LayoutOptions, break_lines};
///
/// let fonts = [Font::from_bytes(font_bytes())?];
/// let text = "The quick brown fox jumps over the lazy dog.";
/// let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(120.0, 12.0))?;
/// assert!(lines.len() > 1);
/// assert!(lines.iter().all(|l| l.width <= 120.0));
/// # Ok::<(), lamina_text::Error>(())
/// ```
///
/// # Errors
/// As [`itemize`] (a character no font covers) and [`Shaper::shape_range`];
/// [`Error::UnsupportedLayoutOption`] when [`LayoutOptions::justify`] is set.
pub fn break_lines(shaper: &impl Shaper, fonts: &[Font], text: &str, options: &LayoutOptions) -> Result<Vec<Line>> {
    if options.justify {
        return Err(Error::UnsupportedLayoutOption { option: "justify" });
    }
    let mut lines = Vec::new();
    for paragraph in paragraphs(text) {
        let ptext = text.get(paragraph.clone()).unwrap_or_default();
        let para = BidiParagraph::new(ptext, options.direction);
        let items = itemize(&para, fonts)?;
        let layout = Layouter {
            shaper,
            fonts,
            text: ptext,
            options,
            para: &para,
            items: &items,
        };
        for mut line in layout.lines()? {
            line.shift(paragraph.start);
            lines.push(line);
        }
    }
    Ok(lines)
}

/// The paragraphs of `text` (UAX #9 P1): each ends after a paragraph separator (class `B`:
/// a line feed, carriage return, CR LF, U+0085, U+2029, U+001C–001E) or at the end.
fn paragraphs(text: &str) -> Vec<Range<usize>> {
    let classes = CodePointMapData::<BidiClass>::new();
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        if classes.get(ch) != BidiClass::ParagraphSeparator {
            continue;
        }
        let mut end = at + ch.len_utf8();
        if ch == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
            chars.next();
            end += 1;
        }
        out.push(start..end);
        start = end;
    }
    if start < text.len() || out.is_empty() {
        out.push(start..text.len());
    }
    out
}

impl Line {
    /// Moves the line's byte offsets by `by` (from its paragraph's text to the whole text).
    fn shift(&mut self, by: usize) {
        self.range = self.range.start + by..self.range.end + by;
        self.content = self.content.start + by..self.content.end + by;
        for run in &mut self.runs {
            run.run.text_range = run.run.text_range.start + by..run.run.text_range.end + by;
            for glyph in &mut run.run.glyphs {
                glyph.cluster += by;
            }
        }
    }
}

struct Layouter<'a, S> {
    shaper: &'a S,
    fonts: &'a [Font],
    /// The paragraph's text: every range here is relative to it.
    text: &'a str,
    options: &'a LayoutOptions,
    para: &'a BidiParagraph<'a>,
    items: &'a [Item],
}

impl<S: Shaper> Layouter<'_, S> {
    /// Greedy (first-fit) lines. Candidates are estimated from the paragraph shaped once
    /// (the advances of each cluster), then the chosen line is shaped as it is drawn and
    /// moved back or forward one opportunity at a time until it is the longest that fits.
    fn lines(&self) -> Result<Vec<Line>> {
        let limit = f64::from(self.options.width);
        let estimate = self.estimates()?;
        let estimated = |range: Range<usize>| {
            let content = self.content(range);
            estimate.get(content.end).copied().unwrap_or(0.0) - estimate.get(content.start).copied().unwrap_or(0.0)
        };
        let breaks = line_breaks(self.text);
        let mut lines = Vec::new();
        let mut start = 0;
        let mut i = 0;
        while let Some(first) = breaks.get(i) {
            // The furthest opportunity the estimate fits, never past a mandatory break.
            let mut j = i;
            if !first.mandatory {
                while let Some(next) = breaks.get(j + 1) {
                    if estimated(start..next.offset) > limit {
                        break;
                    }
                    j += 1;
                    if next.mandatory {
                        break;
                    }
                }
            }
            let end_at = |k: usize| breaks.get(k).map_or(self.text.len(), |b| b.offset);
            let mut line = self.line(start..end_at(j))?;
            while line.width > limit && j > i {
                j -= 1;
                line = self.line(start..end_at(j))?;
            }
            if line.width > limit {
                // Nothing fits: break the overlong stretch between grapheme clusters.
                let (full, rest) = self.split_overlong(start..end_at(i), limit)?;
                for range in full {
                    lines.push(self.line(range)?);
                }
                start = rest;
                let remainder = self.line(start..end_at(i))?;
                if remainder.width > limit || first.mandatory {
                    // A single cluster wider than the box, or the paragraph's end.
                    lines.push(remainder);
                    start = end_at(i);
                    i += 1;
                }
                continue;
            }
            while breaks.get(j).is_some_and(|b| !b.mandatory) && j + 1 < breaks.len() {
                let longer = self.line(start..end_at(j + 1))?;
                if longer.width > limit {
                    break;
                }
                line = longer;
                j += 1;
            }
            start = end_at(j);
            lines.push(line);
            i = j + 1;
        }
        Ok(lines)
    }

    /// Prefix sums of the paragraph's advances in points by byte offset: each item shaped
    /// once at its own level, every glyph's advance counted at its cluster.
    fn estimates(&self) -> Result<Vec<f64>> {
        let mut at = vec![0.0; self.text.len() + 1];
        for item in self.items {
            let Some(run) = self.shape(item, item.range.clone(), item.level)? else {
                continue;
            };
            for glyph in &run.glyphs {
                if let Some(slot) = at.get_mut(glyph.cluster) {
                    *slot += crate::shape::to_points(i64::from(glyph.x_advance), run.units_per_em, self.options.size);
                }
            }
        }
        let mut sum = 0.0;
        let mut prefix = Vec::with_capacity(at.len());
        prefix.push(0.0);
        for v in at.iter().take(self.text.len()) {
            sum += v;
            prefix.push(sum);
        }
        Ok(prefix)
    }

    /// `range` without trailing spaces and hard breaks.
    fn content(&self, range: Range<usize>) -> Range<usize> {
        let line = self.text.get(range.clone()).unwrap_or_default();
        let kept = line.trim_end_matches(|ch| is_trailing_space(ch) || is_hard_break(ch));
        range.start..range.start + kept.len()
    }

    /// The items intersecting `range`, clipped to it, in logical order (items are sorted and
    /// contiguous, so a binary search finds the first).
    fn parts(&self, range: Range<usize>) -> impl Iterator<Item = (&Item, Range<usize>)> + '_ {
        let first = self.items.partition_point(|item| item.range.end <= range.start);
        self.items
            .get(first..)
            .unwrap_or_default()
            .iter()
            .take_while(move |item| item.range.start < range.end)
            .map(move |item| (item, item.range.start.max(range.start)..item.range.end.min(range.end)))
    }

    /// `range` of `item` shaped at `level`; `None` when the item's font is not in the list,
    /// which [`itemize`] only allows for text with nothing to draw.
    fn shape(&self, item: &Item, range: Range<usize>, level: u8) -> Result<Option<ShapedRun>> {
        let Some(font) = self.fonts.get(item.font) else {
            return Ok(None);
        };
        let options = ShapeOptions {
            script: Some(item.script),
            language: self.options.language.clone(),
            direction: Some(Direction::from_level(level)),
            features: self.options.features.clone(),
        };
        self.shaper.shape_range(font, self.text, range, &options).map(Some)
    }

    /// The line `range`: shaped as drawn (visual runs at their post-L1 levels), reordered
    /// and aligned.
    fn line(&self, range: Range<usize>) -> Result<Line> {
        let content = self.content(range.clone());
        let mut runs = Vec::new();
        let mut x = 0.0;
        for visual in self.para.visual_runs(content.clone())? {
            let mut parts: Vec<(&Item, Range<usize>)> = self.parts(visual.range.clone()).collect();
            if visual.direction().is_rtl() {
                parts.reverse();
            }
            for (item, part) in parts {
                let Some(run) = self.shape(item, part, visual.level)? else {
                    continue;
                };
                let width = run.width(self.options.size);
                runs.push(LineRun {
                    run,
                    font: item.font,
                    level: visual.level,
                    x,
                });
                x += width;
            }
        }
        let room = f64::from(self.options.width) - x;
        let rtl = self.para.base_direction().is_rtl();
        let offset = match (self.options.align, rtl) {
            (Alignment::Left, _) | (Alignment::Start, false) | (Alignment::End, true) => 0.0,
            (Alignment::Right, _) | (Alignment::Start, true) | (Alignment::End, false) => room,
            (Alignment::Center, _) => room / 2.0,
        };
        let hard_break = self
            .text
            .get(range.clone())
            .and_then(|t| t.chars().next_back())
            .is_some_and(is_hard_break);
        Ok(Line {
            range,
            content,
            runs,
            width: x,
            x: offset,
            hard_break,
        })
    }

    /// Splits `range`, too wide for one line, into lines that fit between grapheme clusters
    /// (at least one cluster per line; spaces after a line's last cluster hang on it, so no
    /// line holds only spaces). Returns the full lines and where the remainder starts: the
    /// remainder fits, or is a single cluster wider than the box.
    fn split_overlong(&self, range: Range<usize>, limit: f64) -> Result<(Vec<Range<usize>>, usize)> {
        let segment = self.text.get(range.clone()).unwrap_or_default();
        let bounds: Vec<usize> = grapheme_boundaries(segment)
            .into_iter()
            .map(|b| range.start + b)
            .filter(|&b| b > range.start)
            .collect();
        // A line ending at `end` takes the spaces right after it.
        let hang = |end: usize| {
            let rest = self.text.get(end..range.end).unwrap_or_default();
            end + (rest.len() - rest.trim_start_matches(is_trailing_space).len())
        };
        let mut lines = Vec::new();
        let mut start = range.start;
        let mut last_fit: Option<usize> = None;
        let mut i = 0;
        while let Some(&b) = bounds.get(i) {
            if b <= start {
                i += 1;
                continue;
            }
            if self.line(start..b)?.width <= limit {
                last_fit = Some(b);
                i += 1;
                continue;
            }
            let end = match last_fit.take() {
                Some(end) => end,
                None if bounds.get(i + 1).is_none() => break,
                // One cluster wider than the box: it gets a line of its own.
                None => b,
            };
            let end = hang(end);
            if end >= range.end {
                break;
            }
            lines.push(start..end);
            start = end;
        }
        Ok((lines, start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::HarfRust;
    use crate::test_fonts::font;

    fn texts<'t>(text: &'t str, lines: &[Line]) -> Vec<&'t str> {
        lines.iter().map(|l| &text[l.content.clone()]).collect()
    }

    #[test]
    fn short_text_is_one_line() {
        let text = "Hello";
        let lines = break_lines(
            &HarfRust,
            &[font("NotoSans-Regular.ttf")],
            text,
            &LayoutOptions::new(500.0, 12.0),
        )
        .unwrap();
        assert_eq!(texts(text, &lines), ["Hello"]);
    }

    #[test]
    fn breaks_at_spaces_when_the_box_is_narrow() {
        let text = "aaa bbb ccc";
        let fonts = [font("NotoSans-Regular.ttf")];
        let one = HarfRust
            .shape(&fonts[0], "aaa bbb", &ShapeOptions::new())
            .unwrap()
            .width(10.0);
        let one = format!("{one}").parse::<f32>().unwrap() + 0.01;
        let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(one, 10.0)).unwrap();
        assert_eq!(texts(text, &lines), ["aaa bbb", "ccc"]);
    }

    #[test]
    fn hard_breaks_end_lines() {
        let text = "a\nb";
        let lines = break_lines(
            &HarfRust,
            &[font("NotoSans-Regular.ttf")],
            text,
            &LayoutOptions::new(500.0, 12.0),
        )
        .unwrap();
        assert_eq!(texts(text, &lines), ["a", "b"]);
        assert!(lines[0].hard_break);
        assert!(!lines[1].hard_break);
    }

    #[test]
    fn overlong_words_break_between_graphemes() {
        let text = "abcdefghij";
        let lines = break_lines(
            &HarfRust,
            &[font("NotoSans-Regular.ttf")],
            text,
            &LayoutOptions::new(20.0, 12.0),
        )
        .unwrap();
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| l.width <= 20.0));
        assert_eq!(texts(text, &lines).concat(), text);
    }

    #[test]
    fn right_and_centre_alignment_offset_lines() {
        let fonts = [font("NotoSans-Regular.ttf")];
        let right = break_lines(
            &HarfRust,
            &fonts,
            "ab",
            &LayoutOptions::new(100.0, 10.0).with_align(Alignment::Right),
        )
        .unwrap();
        let centre = break_lines(
            &HarfRust,
            &fonts,
            "ab",
            &LayoutOptions::new(100.0, 10.0).with_align(Alignment::Center),
        )
        .unwrap();
        assert!((right[0].x + right[0].width - 100.0).abs() < 1e-9);
        assert!((centre[0].x * 2.0 + centre[0].width - 100.0).abs() < 1e-9);
    }

    #[test]
    fn rtl_paragraphs_start_on_the_right_and_reverse_runs() {
        let fonts = [font("NotoSansHebrew-Regular.ttf"), font("NotoSans-Regular.ttf")];
        let text = "שלום abc";
        let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(300.0, 10.0)).unwrap();
        let line = &lines[0];
        assert!((line.x + line.width - 300.0).abs() < 1e-9);
        let order: Vec<&str> = line.runs.iter().map(|r| &text[r.run.text_range.clone()]).collect();
        assert_eq!(order, ["abc", "שלום "]);
    }
}
