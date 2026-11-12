//! Styled spans: the font list, size, rise, language and features of a stretch of a
//! paragraph's text, and the groups of equally styled spans that are shaped together.

use std::ops::Range;

use super::LayoutOptions;
use crate::error::{Error, Result};
use crate::linebreak::grapheme_boundaries;
use crate::shape::Feature;

/// A stretch of text in one style, for [`break_spans`](super::break_spans).
///
/// Every field but the range defaults to the paragraph's [`LayoutOptions`]: a span made with
/// [`StyledSpan::new`] alone is laid out exactly as [`break_lines`](super::break_lines) lays out
/// its text.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct StyledSpan {
    /// The span's bytes in the text.
    pub range: Range<usize>,
    /// The fonts the span's text is set in, in order of preference: indices into the font list
    /// given to [`break_spans`](super::break_spans) (a bold word: the bold font, then the
    /// fallbacks). `None`: the whole list, in its order.
    pub fonts: Option<Vec<usize>>,
    /// The font size in points; `None`: [`LayoutOptions::size`].
    pub size: Option<f32>,
    /// How far the span's baseline sits above the line's, in points (a superscript; negative
    /// for a subscript). Drawn as the text rise `Ts` (ISO 32000-2 §9.3.7) by
    /// [`pdf_line_run`](crate::pdf_line_run).
    pub rise: f32,
    /// The span's language (BCP 47), passed to the shaper; `None`: [`LayoutOptions::language`].
    pub language: Option<String>,
    /// OpenType features applied after [`LayoutOptions::features`] (so a span can turn one of
    /// them off again).
    pub features: Vec<Feature>,
}

impl StyledSpan {
    /// The bytes `range` of the text, in the paragraph's style.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(range: Range<usize>) -> Self {
        Self {
            range,
            ..Self::default()
        }
    }

    /// With the fonts set: indices into the font list, in order of preference.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_fonts(mut self, fonts: impl Into<Vec<usize>>) -> Self {
        self.fonts = Some(fonts.into());
        self
    }

    /// With the font size set, in points.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// With the baseline rise set, in points (negative: lowered).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_rise(mut self, rise: f32) -> Self {
        self.rise = rise;
        self
    }

    /// With the language set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// With a feature added.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_feature(mut self, feature: Feature) -> Self {
        self.features.push(feature);
        self
    }
}

/// A span's style with the paragraph's defaults filled in.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Style {
    pub(super) fonts: Vec<usize>,
    pub(super) size: f32,
    pub(super) rise: f32,
    pub(super) language: Option<String>,
    pub(super) features: Vec<Feature>,
}

impl Style {
    pub(super) fn of(span: &StyledSpan, options: &LayoutOptions, font_count: usize) -> Self {
        let mut features = options.features.clone();
        features.extend(span.features.iter().copied());
        Self {
            fonts: span.fonts.clone().unwrap_or_else(|| (0..font_count).collect()),
            size: span.size.unwrap_or(options.size),
            rise: span.rise,
            language: span.language.clone().or_else(|| options.language.clone()),
            features,
        }
    }
}

/// Consecutive spans of one style within a paragraph: shaped as one, so kerning, ligatures
/// and joining carry across their boundaries (a link inside a sentence changes nothing).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Group {
    /// The group's bytes in the paragraph's text.
    pub(super) range: Range<usize>,
    /// The spans it holds: indices into the caller's list.
    pub(super) spans: Range<usize>,
    pub(super) style: Style,
}

/// Checks that `spans` cover `text` once, in order, starting and ending between grapheme
/// clusters (a style change can't split a letter from its accent), and that each names at
/// least one font and only fonts of the list (`font_count` of them).
pub(super) fn check(text: &str, spans: &[StyledSpan], font_count: usize) -> Result<()> {
    let bounds = grapheme_boundaries(text);
    let between_clusters = |at: usize| bounds.binary_search(&at).is_ok();
    let mut at = 0;
    for span in spans {
        let Range { start, end } = span.range;
        if start != at || text.get(start..end).is_none() || !between_clusters(end) {
            return Err(Error::InvalidSpans { at });
        }
        if span
            .fonts
            .as_ref()
            .is_some_and(|fonts| fonts.is_empty() || fonts.iter().any(|&f| f >= font_count))
        {
            return Err(Error::InvalidSpans { at: start });
        }
        at = end;
    }
    if at == text.len() {
        Ok(())
    } else {
        Err(Error::InvalidSpans { at })
    }
}

/// The spans of `group` (indices into `spans`) that hold text of `range` (bytes of the whole
/// text): those a run of a line draws from, which can be fewer than its group's when the
/// group wraps onto another line.
pub(super) fn spans_in(spans: &[StyledSpan], group: &Range<usize>, range: &Range<usize>) -> Range<usize> {
    let held = |i: &usize| {
        spans
            .get(*i)
            .is_some_and(|s| s.range.start < range.end && range.start < s.range.end)
    };
    let first = group.clone().find(held).unwrap_or(group.start);
    let last = group.clone().rev().find(held).map_or(first, |i| i + 1);
    first..last.max(first)
}

/// The groups of the paragraph `paragraph` (bytes of the whole text), with ranges relative to
/// it: the checked `spans` clipped to it, empty ones left out, consecutive equal styles merged
/// (an empty span between two equal ones keeps them apart).
pub(super) fn groups(
    spans: &[StyledSpan],
    paragraph: &Range<usize>,
    options: &LayoutOptions,
    font_count: usize,
) -> Vec<Group> {
    let first = spans.partition_point(|s| s.range.end <= paragraph.start);
    let mut out: Vec<Group> = Vec::new();
    for (i, span) in spans.iter().enumerate().skip(first) {
        if span.range.start >= paragraph.end {
            break;
        }
        let start = span.range.start.max(paragraph.start) - paragraph.start;
        let end = span.range.end.min(paragraph.end).saturating_sub(paragraph.start);
        if end <= start {
            continue;
        }
        let style = Style::of(span, options, font_count);
        match out.last_mut() {
            Some(last) if last.style == style && last.spans.end == i => {
                last.range.end = end;
                last.spans.end = i + 1;
            }
            _ => out.push(Group {
                range: start..end,
                spans: i..i + 1,
                style,
            }),
        }
    }
    out
}
