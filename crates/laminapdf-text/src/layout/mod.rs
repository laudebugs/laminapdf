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
mod span;
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
use crate::itemize::{Item, charmaps, itemize_range};
use crate::shape::Script;
use crate::shape::{Direction, Feature, Shaper};

use breaker::Layouter;
pub use line::{Line, LineRun};
pub use span::StyledSpan;

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
/// breaking, `fonts` in order of preference ([`itemize`](crate::itemize)). Each paragraph (ended by a line
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
/// As [`break_spans`].
pub fn break_lines(shaper: &impl Shaper, fonts: &[Font], text: &str, options: &LayoutOptions) -> Result<Vec<Line>> {
    break_spans(shaper, fonts, text, &[StyledSpan::new(0..text.len())], options)
}

/// [`break_lines`] over styled spans: words in other fonts, sizes, rises and languages
/// inside one paragraph (a bold word, a superscript, an Arabic name in an English line).
/// `spans` cover `text` once, in order; each sets its own [`StyledSpan::fonts`] (indices into
/// `fonts`), size, rise, language and features, defaulting to `options`. Lines break where
/// the text allows (UAX #14 on the whole text), never because a span ends, so a style change
/// inside a word is not a break opportunity. Consecutive spans of one style are shaped as one
/// run, and a line's ascent and descent come from its tallest run.
///
/// ```
/// # fn regular() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
/// # fn bold() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-RegularBold.ttc")).to_vec() }
/// use laminapdf_text::{Font, HarfRust, LayoutOptions, StyledSpan, break_spans};
///
/// let fonts = [Font::from_bytes(regular())?, Font::from_collection(bold(), 1)?];
/// let text = "A bold word and E = mc2.";
/// let spans = [
///     StyledSpan::new(0..2),
///     StyledSpan::new(2..6).with_fonts([1, 0]),
///     StyledSpan::new(6..22),
///     StyledSpan::new(22..23).with_size(7.0).with_rise(4.0),
///     StyledSpan::new(23..24),
/// ];
/// let lines = break_spans(&HarfRust, &fonts, text, &spans, &LayoutOptions::new(300.0, 12.0))?;
/// let runs = &lines[0].runs;
/// assert_eq!(runs[1].font, 1); // "bold" in the bold font
/// assert_eq!((runs[3].size, runs[3].rise), (7.0, 4.0)); // the raised "2"
/// # Ok::<(), laminapdf_text::Error>(())
/// ```
///
/// # Errors
/// [`Error::InvalidSpans`] when `spans` don't cover `text` once, in order, between grapheme
/// clusters, or a span lists no fonts or one past `fonts`;
/// [`Error::MissingGlyph`] for a character none of its span's fonts covers, and the other
/// errors of [`itemize`](crate::itemize) and [`Shaper::shape_range`];
/// [`Error::UnsupportedLayoutOption`] when [`LayoutOptions::justify`] is set.
pub fn break_spans(
    shaper: &impl Shaper,
    fonts: &[Font],
    text: &str,
    spans: &[StyledSpan],
    options: &LayoutOptions,
) -> Result<Vec<Line>> {
    if options.justify {
        return Err(Error::UnsupportedLayoutOption { option: "justify" });
    }
    span::check(text, spans, fonts.len())?;
    let charmaps = charmaps(fonts)?;
    let mut lines = Vec::new();
    for paragraph in paragraphs(text) {
        let ptext = text.get(paragraph.clone()).unwrap_or_default();
        let para = BidiParagraph::new(ptext, options.direction);
        let groups = span::groups(spans, &paragraph, options, fonts.len());
        let mut items = Vec::new();
        let mut item_groups = Vec::new();
        for (g, group) in groups.iter().enumerate() {
            let group_items = itemize_range(&para, fonts, &charmaps, &group.style.fonts, group.range.clone())?;
            item_groups.extend(std::iter::repeat_n(g, group_items.len()));
            items.extend(group_items);
        }
        inherit_scripts(&mut items, &item_groups);
        let layout = Layouter {
            shaper,
            fonts,
            text: ptext,
            options,
            para: &para,
            items: &items,
            groups: &groups,
            item_groups: &item_groups,
            default_style: span::Style::of(&StyledSpan::new(0..0), options, fonts.len()),
            shaped: RefCell::new(BTreeMap::new()),
        };
        for mut line in layout.lines()? {
            line.shift(paragraph.start);
            for run in &mut line.runs {
                run.spans = span::spans_in(spans, &run.spans, &run.run.text_range);
            }
            lines.push(line);
        }
    }
    Ok(lines)
}

/// Gives a group's first item, when it holds no character of a real script (a span of digits
/// or punctuation), the script of the item before it at the same level, else of the one
/// after: the script [`itemize`](crate::itemize) gives such text inside one span, so a bold
/// number in an Arabic line is shaped as Arabic.
fn inherit_scripts(items: &mut [Item], item_groups: &[usize]) {
    for i in 0..items.len() {
        let group = item_groups.get(i);
        let starts_group = i == 0 || item_groups.get(i - 1) != group;
        let Some(item) = items.get(i) else { continue };
        if !starts_group || item.script != Script::COMMON {
            continue;
        }
        // Only another group's items: inside one span, `itemize` has already decided.
        let level = item.level;
        let real = |j: usize| {
            items
                .get(j)
                .filter(|o| item_groups.get(j) != group && o.level == level && o.script != Script::COMMON)
                .map(|o| o.script)
        };
        let before = i.checked_sub(1).and_then(real);
        if let Some(script) = before.or_else(|| (i + 1..items.len()).find_map(real))
            && let Some(item) = items.get_mut(i)
        {
            item.script = script;
        }
    }
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
