//! Text boxes: a paragraph broken into lines that fit a rectangle (research 03 §7.4), with
//! `laminapdf-text`'s greedy, bidi-aware layout.

use std::ops::Range;

use laminapdf_model::{Pt, RectPt};
use laminapdf_text::{Alignment, Direction, Feature};

/// What happens to lines that don't fit a text box's height (or a single grapheme cluster
/// wider than the box).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Overflow {
    /// Draw the lines that fit and report the rest in [`TextLayout::overflow`] (the default).
    #[default]
    Clip,
    /// Draw nothing and fail with `text-overflow`.
    Error,
}

/// How [`Surface::text_box`](crate::Surface::text_box) lays its text out.
///
/// ```
/// use laminapdf_build::{Alignment, Overflow, TextBoxOptions};
///
/// let options = TextBoxOptions::default()
///     .with_align(Alignment::Center)
///     .with_line_height(1.4)
///     .with_overflow(Overflow::Error);
/// assert_eq!(options.line_height(), 1.4);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextBoxOptions {
    pub(crate) align: Alignment,
    pub(crate) line_height: f64,
    pub(crate) overflow: Overflow,
    pub(crate) direction: Option<Direction>,
    pub(crate) language: Option<String>,
    pub(crate) features: Vec<Feature>,
}

impl Default for TextBoxOptions {
    fn default() -> Self {
        Self {
            align: Alignment::Start,
            line_height: 1.2,
            overflow: Overflow::Clip,
            direction: None,
            language: None,
            features: Vec::new(),
        }
    }
}

impl TextBoxOptions {
    /// The defaults, as [`TextBoxOptions::default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Line alignment (default: the paragraph's start side).
    #[must_use]
    pub const fn with_align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// The distance between baselines as a multiple of the font size (default 1.2).
    #[must_use]
    pub const fn with_line_height(mut self, factor: f64) -> Self {
        self.line_height = factor;
        self
    }

    /// What happens to lines that don't fit (default [`Overflow::Clip`]).
    #[must_use]
    pub const fn with_overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// The paragraph direction (default: from the first strong character).
    #[must_use]
    pub const fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    /// The text's language (BCP 47), for the shaper.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// An OpenType feature for every run.
    #[must_use]
    pub fn with_feature(mut self, feature: Feature) -> Self {
        self.features.push(feature);
        self
    }

    /// The alignment.
    #[must_use]
    pub const fn align(&self) -> Alignment {
        self.align
    }

    /// The line height factor.
    #[must_use]
    pub const fn line_height(&self) -> f64 {
        self.line_height
    }

    /// The overflow behaviour.
    #[must_use]
    pub const fn overflow(&self) -> Overflow {
        self.overflow
    }
}

/// One drawn line of a text box, page space.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LineBox {
    /// The bytes of the text on this line, trailing spaces and the line break included.
    pub range: Range<usize>,
    /// The bytes drawn (without the trailing spaces and the line break).
    pub content: Range<usize>,
    /// The line's box: its drawn width, from the top of the font's ascent to the bottom of
    /// its descent.
    pub rect: RectPt,
    /// The line's baseline (y, page space).
    pub baseline: Pt,
}

/// How [`Surface::text_box`](crate::Surface::text_box) laid its text out.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextLayout {
    /// The lines drawn, top to bottom.
    pub lines: Vec<LineBox>,
    /// With [`Overflow::Clip`], the byte offset of the first text that was not drawn because
    /// it didn't fit; `None` when everything fit.
    pub overflow: Option<usize>,
}

impl TextLayout {
    /// Whether some text didn't fit.
    #[must_use]
    pub const fn overflowed(&self) -> bool {
        self.overflow.is_some()
    }

    /// Where the next element can start below this text: the bottom of the last drawn line
    /// that holds something (its `rect.y1`), rather than the bottom of the requested box. `None`
    /// when no line does: empty text, text of only spaces and line breaks, or a box too short
    /// for one line. Blank lines after the last drawn one don't count. A clipped box gives its
    /// last drawn line; check [`overflow`](Self::overflow) before treating the text as placed.
    #[must_use]
    pub fn bottom(&self) -> Option<Pt> {
        self.lines
            .iter()
            .rev()
            .find(|l| !l.content.is_empty())
            .map(|l| l.rect.y1)
    }
}
