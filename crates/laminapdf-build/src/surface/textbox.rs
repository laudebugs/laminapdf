//! Text boxes: a paragraph broken into lines that fit a rectangle (research 03 §7.4), with
//! `laminapdf-text`'s greedy, bidi-aware layout.

use std::ops::Range;

use laminapdf_model::{PointPt, Pt, RectPt};
use laminapdf_text::{Alignment, Direction, Feature};

use super::check_size;
use super::text::{self, FontHandle, LaidLine, Spec, lay_out};
use crate::error::{Error, Result};

/// What happens to lines that don't fit a text box's height (or a single grapheme cluster
/// wider than the box).
///
/// Added in 0.2.0.
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
///
/// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Line alignment (default: the paragraph's start side).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// The distance between baselines as a multiple of the font size (default 1.2).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_line_height(mut self, factor: f64) -> Self {
        self.line_height = factor;
        self
    }

    /// What happens to lines that don't fit (default [`Overflow::Clip`]).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// The paragraph direction (default: from the first strong character).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    /// The text's language (BCP 47), for the shaper.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// An OpenType feature for every run.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_feature(mut self, feature: Feature) -> Self {
        self.features.push(feature);
        self
    }

    /// The alignment.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn align(&self) -> Alignment {
        self.align
    }

    /// The line height factor.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn line_height(&self) -> f64 {
        self.line_height
    }

    /// The overflow behaviour.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn overflow(&self) -> Overflow {
        self.overflow
    }
}

/// One drawn line of a text box, page space.
///
/// Added in 0.2.0.
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
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextLayout {
    /// The lines drawn, top to bottom.
    pub lines: Vec<LineBox>,
    /// With [`Overflow::Clip`], the byte offset of the first text that was not drawn because
    /// it didn't fit; `None` when everything fit.
    pub overflow: Option<usize>,
    /// [`bottom`](Self::bottom), worked out while laying out, when the text is at hand.
    pub(crate) bottom: Option<Pt>,
}

impl TextLayout {
    /// Whether some text didn't fit.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn overflowed(&self) -> bool {
        self.overflow.is_some()
    }

    /// Where the next element can start below this text: the bottom of the last drawn line
    /// that draws something (its `rect.y1`), before the surface's transform, rather than the
    /// bottom of the requested box. `None` when no line does: empty text, text of only white
    /// space and control characters, or a box too short for one line. Blank lines after the last
    /// drawn one don't count. A clipped box gives its last drawn line; check
    /// [`overflow`](Self::overflow) before treating the text as placed.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn bottom(&self) -> Option<Pt> {
        self.bottom
    }
}

/// A text box laid out and not drawn yet: what [`Surface::text_box`](crate::Surface::text_box)
/// draws, and the flow layer draws with `Surface::draw_measured` after measuring it.
#[derive(Debug, Clone)]
pub(crate) struct Measured {
    pub(crate) spec: Spec,
    /// The box's left edge and the first line's baseline, page space.
    pub(crate) origin: PointPt,
    /// From one baseline to the next, points.
    pub(crate) leading: f64,
    /// The lines that fit, shaped.
    pub(crate) lines: Vec<LaidLine>,
    pub(crate) layout: TextLayout,
}

/// Lays `text` out in `rect` as [`Surface::text_box`](crate::Surface::text_box) describes.
///
/// # Errors
/// As [`Surface::text_box`](crate::Surface::text_box).
pub(crate) fn measure(
    text: &str,
    font: &FontHandle,
    size: Pt,
    rect: RectPt,
    options: &TextBoxOptions,
) -> Result<Measured> {
    check_size(size)?;
    if !rect.is_finite() || rect.width().get() <= 0.0 {
        return Err(Error::InvalidGeometry {
            what: "a text box must be finite and wider than 0",
        });
    }
    if !(options.line_height.is_finite() && options.line_height > 0.0) {
        return Err(Error::InvalidGeometry {
            what: "a line height must be a positive number",
        });
    }
    let size_pt = size.get();
    let spec = Spec {
        text: text.to_owned(),
        font: font.clone(),
        size: size_pt,
        align: options.align,
        direction: options.direction,
        language: options.language.clone(),
        features: options.features.clone(),
        width: Some(rect.width().get()),
    };
    let mut lines = lay_out(&spec)?;
    let leading = options.line_height * size_pt;
    let first = rect.y0.get() + font.ascent(size_pt);
    let descent = font.descent(size_pt);
    let limit = rect.width().get();
    let fits = |i: usize, line: &LaidLine| {
        #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
        let bottom = first + i as f64 * leading + descent;
        // A hair of tolerance for the f32 widths the layout measured with.
        bottom <= rect.y1.get() + 1e-9 && line.width <= limit + 1e-3
    };
    let fit = lines
        .iter()
        .enumerate()
        .position(|(i, l)| !fits(i, l))
        .unwrap_or(lines.len());
    let overflow = lines.get(fit).map(|l| l.range.start);
    if overflow.is_some() && options.overflow == Overflow::Error {
        return Err(Error::TextOverflow {
            fits: fit,
            needed: lines.len(),
        });
    }
    lines.truncate(fit);
    let x = rect.x0.get();
    let boxes: Vec<LineBox> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
            let baseline = first + i as f64 * leading;
            LineBox {
                range: l.range.clone(),
                content: l.content.clone(),
                rect: RectPt::new(
                    Pt::new(x + l.x),
                    Pt::new(baseline - font.ascent(size_pt)),
                    Pt::new(x + l.x + l.width),
                    Pt::new(baseline + descent),
                ),
                baseline: Pt::new(baseline),
            }
        })
        .collect();
    let bottom = boxes
        .iter()
        .rev()
        .find(|b| text::draws(&spec.text, &b.content))
        .map(|b| b.rect.y1);
    Ok(Measured {
        spec,
        origin: PointPt::new(rect.x0, Pt::new(first)),
        leading,
        lines,
        layout: TextLayout {
            lines: boxes,
            overflow,
            bottom,
        },
    })
}
