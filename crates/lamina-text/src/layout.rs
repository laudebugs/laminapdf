//! Paragraph layout in a box (todo B4b; research 03 §7, §7.4, §7.6): bidi resolution,
//! itemisation, shaping, greedy line breaking at UAX #14 opportunities, per-line visual
//! reordering (UAX #9 rule L2) and alignment.
//!
//! Lines are measured by shaping each candidate line as it would be drawn (the text around it
//! as context), so ligatures, kerning and Arabic joining at a break are accounted for and no
//! line is wider than the box, except a line holding a single grapheme cluster that is wider
//! than the box on its own. A word wider than the box is broken between grapheme clusters
//! (CSS `overflow-wrap: anywhere`). No hyphenation and no justification yet (pdf-library §2
//! non-goals; [`LayoutOptions::justify`] is reserved).

use std::ops::Range;

use crate::bidi::BidiParagraph;
use crate::error::Result;
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
    /// Reserved for justification (a later plan): must be `false`; ignored for now.
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

/// Lays `text` out as one paragraph in a box `options.width` points wide with greedy (first
/// fit) line breaking, `fonts` in order of preference ([`itemize`]).
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
/// As [`itemize`] (a character no font covers) and [`Shaper::shape_range`].
pub fn break_lines(shaper: &impl Shaper, fonts: &[Font], text: &str, options: &LayoutOptions) -> Result<Vec<Line>> {
    let para = BidiParagraph::new(text, options.direction);
    let items = itemize(&para, fonts)?;
    let layout = Layouter {
        shaper,
        fonts,
        text,
        options,
        para: &para,
        items: &items,
    };
    let limit = f64::from(options.width);
    let mut lines = Vec::new();
    let mut start = 0;
    let mut fit: Option<usize> = None; // the last opportunity that fits on the current line
    let breaks = line_breaks(text);
    let mut i = 0;
    while let Some(b) = breaks.get(i) {
        let fits = layout.width(start..b.offset)? <= limit;
        if fits {
            fit = Some(b.offset);
            if b.mandatory {
                lines.push(layout.line(start..b.offset)?);
                start = b.offset;
                fit = None;
            }
            i += 1;
            continue;
        }
        if let Some(end) = fit.take() {
            lines.push(layout.line(start..end)?);
            start = end;
            continue; // the same opportunity, measured from the new line start
        }
        // Nothing fits: break the overlong stretch between grapheme clusters.
        // The remainder fits, or is one grapheme cluster wider than the box: either way it
        // stays on the current line.
        let (full, rest) = layout.split_overlong(start..b.offset, limit)?;
        for range in full {
            lines.push(layout.line(range)?);
        }
        start = rest;
        fit = Some(b.offset);
        if b.mandatory {
            lines.push(layout.line(start..b.offset)?);
            start = b.offset;
            fit = None;
        }
        i += 1;
    }
    Ok(lines)
}

struct Layouter<'a, S> {
    shaper: &'a S,
    fonts: &'a [Font],
    text: &'a str,
    options: &'a LayoutOptions,
    para: &'a BidiParagraph<'a>,
    items: &'a [Item],
}

impl<S: Shaper> Layouter<'_, S> {
    /// `range` without trailing spaces and hard breaks.
    fn content(&self, range: Range<usize>) -> Range<usize> {
        let line = self.text.get(range.clone()).unwrap_or_default();
        let kept = line.trim_end_matches(|ch| is_trailing_space(ch) || is_hard_break(ch));
        range.start..range.start + kept.len()
    }

    /// The width in points of `range` drawn as one line.
    fn width(&self, range: Range<usize>) -> Result<f64> {
        let content = self.content(range);
        let mut width = 0.0;
        for (item, part) in self.parts(content) {
            width += self.shape(item, part, item.level)?.width(self.options.size);
        }
        Ok(width)
    }

    /// The items intersecting `range`, clipped to it, in logical order.
    fn parts(&self, range: Range<usize>) -> impl Iterator<Item = (&Item, Range<usize>)> + '_ {
        self.items.iter().filter_map(move |item| {
            let start = item.range.start.max(range.start);
            let end = item.range.end.min(range.end);
            (start < end).then_some((item, start..end))
        })
    }

    fn shape(&self, item: &Item, range: Range<usize>, level: u8) -> Result<ShapedRun> {
        let font = self
            .fonts
            .get(item.font)
            .ok_or(crate::Error::UnknownFont { key: item.font })?;
        let options = ShapeOptions {
            script: Some(item.script),
            language: self.options.language.clone(),
            direction: Some(Direction::from_level(level)),
            features: self.options.features.clone(),
        };
        self.shaper.shape_range(font, self.text, range, &options)
    }

    /// The line `range`: shaped, reordered and aligned.
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
                let run = self.shape(item, part, visual.level)?;
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
    /// (at least one cluster per line). Returns the full lines and where the remainder starts:
    /// the remainder fits, or is a single cluster wider than the box.
    fn split_overlong(&self, range: Range<usize>, limit: f64) -> Result<(Vec<Range<usize>>, usize)> {
        let segment = self.text.get(range.clone()).unwrap_or_default();
        let bounds: Vec<usize> = grapheme_boundaries(segment)
            .into_iter()
            .map(|b| range.start + b)
            .filter(|&b| b > range.start)
            .collect();
        let mut lines = Vec::new();
        let mut start = range.start;
        let mut last_fit: Option<usize> = None;
        let mut i = 0;
        while let Some(&b) = bounds.get(i) {
            if self.width(start..b)? <= limit {
                last_fit = Some(b);
                i += 1;
                continue;
            }
            match last_fit.take() {
                Some(end) => {
                    lines.push(start..end);
                    start = end;
                }
                None if i + 1 == bounds.len() => break,
                None => {
                    // One cluster wider than the box: it gets a line of its own.
                    lines.push(start..b);
                    start = b;
                    i += 1;
                }
            }
        }
        Ok((lines, start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::HarfRust;

    fn font(name: &str) -> Font {
        let path = format!("{}/../../fixtures/fonts/{name}", env!("CARGO_MANIFEST_DIR"));
        Font::from_bytes(std::fs::read(path).unwrap()).unwrap()
    }

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
