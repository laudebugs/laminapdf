//! Paragraph layout in a box (research 03 §7, §7.4, §7.6): bidi resolution,
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

mod breaker;
mod line;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Range;

use icu_properties::CodePointMapData;
use icu_properties::props::BidiClass;

use crate::bidi::BidiParagraph;
use crate::error::{Error, Result};
use crate::font::Font;
use crate::itemize::itemize;
use crate::shape::{Direction, Feature, Shaper};

use breaker::Layouter;
pub use line::{Line, LineRun};

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

/// Lays `text` out in a box `options.width` points wide with greedy (first fit) line
/// breaking, `fonts` in order of preference ([`itemize`]). Each paragraph (ended by a line
/// feed, U+2029 or another UAX #9 paragraph separator) is resolved on its own: its
/// direction, its alignment side and its isolates.
///
/// ```
/// # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
/// use laminapdf_text::{Font, HarfRust, LayoutOptions, break_lines};
///
/// let fonts = [Font::from_bytes(font_bytes())?];
/// let text = "The quick brown fox jumps over the lazy dog.";
/// let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(120.0, 12.0))?;
/// assert!(lines.len() > 1);
/// assert!(lines.iter().all(|l| l.width <= 120.0));
/// # Ok::<(), laminapdf_text::Error>(())
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
            shaped: RefCell::new(BTreeMap::new()),
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
