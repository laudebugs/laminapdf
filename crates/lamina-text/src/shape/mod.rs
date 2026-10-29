//! Shaping (todo B4b; research 03 §7.2): characters → positioned glyphs through the font's
//! OpenType layout tables (GSUB substitutions such as ligatures and contextual forms, GPOS
//! kerning and mark attachment) and the script-specific shapers (Arabic joining, Indic
//! reordering). [`Shaper`] is the seam; [`HarfRust`] (HarfBuzz's algorithm in Rust,
//! [ADR 0005] §6) is the implementation.
//!
//! The output keeps HarfBuzz's model: per glyph an ID, a *cluster* (the byte offset of the
//! first character it came from) and advances and offsets in font units. Clusters are the
//! source of truth for ToUnicode and `ActualText` ([`crate::pdf_run`]).
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md

mod engine;

use std::fmt;
use std::ops::Range;
use std::str::FromStr;

use icu_properties::props::Script as IcuScript;
use icu_properties::{CodePointMapData, PropertyNamesShort};

pub use self::engine::HarfRust;
use crate::error::{Error, Result};
use crate::font::{Font, GlyphId, Tag};

/// A horizontal writing direction (vertical writing is out of scope, research 03 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    /// Left to right (Latin, Devanagari, CJK set horizontally).
    #[default]
    LeftToRight,
    /// Right to left (Arabic, Hebrew).
    RightToLeft,
}

impl Direction {
    /// The direction of a UAX #9 embedding level: even levels are left to right.
    #[must_use]
    pub const fn from_level(level: u8) -> Self {
        if level.is_multiple_of(2) {
            Self::LeftToRight
        } else {
            Self::RightToLeft
        }
    }

    /// Whether this is [`Direction::RightToLeft`].
    #[must_use]
    pub const fn is_rtl(self) -> bool {
        matches!(self, Self::RightToLeft)
    }
}

/// A Unicode script as its ISO 15924 code (`Latn`, `Arab`, `Hebr`, `Deva`, `Hani`), the
/// `Script` property of UAX #24 from ICU4X's data.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Script([u8; 4]);

impl Script {
    /// `Zyyy`: characters shared by many scripts (spaces, digits, most punctuation).
    pub const COMMON: Self = Self(*b"Zyyy");
    /// `Zinh`: combining marks that take the script of their base.
    pub const INHERITED: Self = Self(*b"Zinh");
    /// `Zzzz`: unassigned code points.
    pub const UNKNOWN: Self = Self(*b"Zzzz");
    /// `Latn`.
    pub const LATIN: Self = Self(*b"Latn");
    /// `Arab`.
    pub const ARABIC: Self = Self(*b"Arab");
    /// `Hebr`.
    pub const HEBREW: Self = Self(*b"Hebr");
    /// `Deva`.
    pub const DEVANAGARI: Self = Self(*b"Deva");
    /// `Hani` (Han ideographs).
    pub const HAN: Self = Self(*b"Hani");

    /// A script from its four-letter ISO 15924 code (title case: `Latn`).
    #[must_use]
    pub const fn new(code: &[u8; 4]) -> Self {
        Self(*code)
    }

    /// The `Script` property of `ch` (UAX #24).
    #[must_use]
    pub fn of(ch: char) -> Self {
        let script: IcuScript = CodePointMapData::<IcuScript>::new().get(ch);
        PropertyNamesShort::<IcuScript>::new()
            .get_locale_script(script)
            .and_then(|s| <[u8; 4]>::try_from(s.as_str().as_bytes()).ok())
            .map_or(Self::UNKNOWN, Self)
    }

    /// The first script in `text` that is not [`Script::COMMON`], [`Script::INHERITED`] or
    /// [`Script::UNKNOWN`]; `None` when there is none (digits and punctuation only).
    #[must_use]
    pub fn dominant(text: &str) -> Option<Self> {
        text.chars().map(Self::of).find(|s| s.is_real())
    }

    /// The ISO 15924 code's four bytes.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 4] {
        self.0
    }

    /// Whether this is a script of its own, not `Zyyy`, `Zinh` or `Zzzz`: characters of these
    /// take the script of their neighbours when text is itemised.
    #[must_use]
    pub fn is_real(self) -> bool {
        self != Self::COMMON && self != Self::INHERITED && self != Self::UNKNOWN
    }

    /// The direction the script is written in when nothing else says (UAX #9 decides inside a
    /// paragraph; this is for a run shaped on its own).
    #[must_use]
    pub fn direction(self) -> Direction {
        // HarfBuzz's table of right-to-left scripts, reached through an empty buffer (HarfRust
        // keeps the table itself private).
        let mut probe = ::harfrust::UnicodeBuffer::new();
        if let Some(script) = ::harfrust::Script::from_iso15924_tag(::harfrust::Tag::new(&self.0)) {
            probe.set_script(script);
        }
        probe.guess_segment_properties();
        match probe.direction() {
            ::harfrust::Direction::RightToLeft => Direction::RightToLeft,
            _ => Direction::LeftToRight,
        }
    }
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
    }
}

impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Script({self})")
    }
}

/// An OpenType feature setting for a run, such as `liga=0` (no standard ligatures), `smcp`
/// (small capitals) or `tnum` (tabular figures).
///
/// Parsed from HarfBuzz's syntax without ranges (features apply to the whole run): `tag` or
/// `+tag` (on, value 1), `-tag` (off, value 0), `tag=N`, `tag=on`, `tag=off`. Tags are one to
/// four printable ASCII characters, padded with spaces.
///
/// ```
/// use lamina_text::Feature;
///
/// let liga_off: Feature = "liga=0".parse()?;
/// assert_eq!(liga_off, "-liga".parse()?);
/// assert_eq!(liga_off.value(), 0);
/// assert_eq!("smcp".parse::<Feature>()?.value(), 1);
/// assert_eq!("liga=x".parse::<Feature>().unwrap_err().code(), "invalid-feature");
/// # Ok::<(), lamina_text::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Feature {
    tag: Tag,
    value: u32,
}

impl Feature {
    /// A feature from its tag and value (0: off, 1: on, higher: an alternate's index).
    #[must_use]
    pub const fn new(tag: Tag, value: u32) -> Self {
        Self { tag, value }
    }

    /// The feature tag.
    #[must_use]
    pub const fn tag(self) -> Tag {
        self.tag
    }

    /// The value: 0 turns the feature off.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.value
    }
}

impl fmt::Debug for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Feature({}={})", self.tag, self.value)
    }
}

impl FromStr for Feature {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let invalid = || Error::InvalidFeature { feature: s.to_owned() };
        let (sign, rest) = match s.trim().split_at_checked(1) {
            Some(("+", rest)) => (Some(true), rest),
            Some(("-", rest)) => (Some(false), rest),
            _ => (None, s.trim()),
        };
        let (name, value) = match rest.split_once('=') {
            Some((name, value)) => (name.trim(), Some(value.trim())),
            None => (rest, None),
        };
        let value = match (sign, value) {
            (Some(_), Some(_)) => return Err(invalid()),
            (Some(on), None) => u32::from(on),
            (None, None | Some("on")) => 1,
            (None, Some("off")) => 0,
            (None, Some(v)) => v.parse().map_err(|_| invalid())?,
        };
        if name.is_empty() || name.len() > 4 || !name.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(invalid());
        }
        let mut tag = *b"    ";
        for (slot, b) in tag.iter_mut().zip(name.bytes()) {
            *slot = b;
        }
        Ok(Self::new(Tag::new(&tag), value))
    }
}

/// What to shape a run with. Every field left empty is worked out from the text: the script
/// from its first character of a real script ([`Script::dominant`]), the direction from the
/// script.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ShapeOptions {
    /// The run's script; `None`: detected.
    pub script: Option<Script>,
    /// The run's language as a BCP 47 tag (`en`, `ar`, `hi`, `zh-Hans`): it picks
    /// language-specific forms (`locl`). `None`: the font's default language system.
    pub language: Option<String>,
    /// The run's direction; `None`: the script's ([`Script::direction`]).
    pub direction: Option<Direction>,
    /// OpenType features on top of the shaper's defaults (`liga=0`, `smcp`, `tnum`).
    pub features: Vec<Feature>,
}

impl ShapeOptions {
    /// Options with everything detected and no extra features.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// With the script set.
    #[must_use]
    pub fn with_script(mut self, script: Script) -> Self {
        self.script = Some(script);
        self
    }

    /// With the language set (a BCP 47 tag).
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// With the direction set.
    #[must_use]
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    /// With a feature added.
    #[must_use]
    pub fn with_feature(mut self, feature: Feature) -> Self {
        self.features.push(feature);
        self
    }
}

/// One glyph of a [`ShapedRun`]. Lengths are in font units (y grows upwards, as in HarfBuzz).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ShapedGlyph {
    /// The glyph, in the font's own numbering.
    pub gid: GlyphId,
    /// The byte offset in the shaped text of the first character of the cluster this glyph
    /// belongs to. Glyphs of one cluster share it; it never splits a grapheme cluster.
    pub cluster: usize,
    /// How far the pen moves after the glyph, horizontally.
    pub x_advance: i32,
    /// How far the pen moves after the glyph, vertically (0 in horizontal text).
    pub y_advance: i32,
    /// Where the glyph is drawn relative to the pen, horizontally (mark positioning).
    pub x_offset: i32,
    /// Where the glyph is drawn relative to the pen, vertically.
    pub y_offset: i32,
}

/// A run of text shaped with one font, script, direction and set of features: glyphs in
/// visual order (left to right on the page, so a right-to-left run's clusters decrease).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShapedRun {
    /// The glyphs, left to right.
    pub glyphs: Vec<ShapedGlyph>,
    /// The bytes of the text this run shaped.
    pub text_range: Range<usize>,
    /// The direction it was shaped in.
    pub direction: Direction,
    /// The script it was shaped as.
    pub script: Script,
    /// The font's units per em, to scale the glyph lengths: `points = units × size / upem`.
    pub units_per_em: u16,
    /// The PostScript name of the font (instance) it was shaped with:
    /// [`crate::pdf_run`] refuses a key for another font.
    pub font: String,
}

impl ShapedRun {
    /// The sum of the glyphs' horizontal advances, in font units.
    #[must_use]
    pub fn advance(&self) -> i64 {
        self.glyphs.iter().map(|g| i64::from(g.x_advance)).sum()
    }

    /// The run's width in points at font size `size` (points).
    #[must_use]
    pub fn width(&self, size: f32) -> f64 {
        to_points(self.advance(), self.units_per_em, size)
    }

    /// The glyphs that came from the bytes `range` of the text (by cluster).
    pub fn glyphs_in(&self, range: Range<usize>) -> impl Iterator<Item = &ShapedGlyph> {
        self.glyphs.iter().filter(move |g| range.contains(&g.cluster))
    }

    /// Whether a glyph is `.notdef`: the font had no glyph for a character of its cluster.
    #[must_use]
    pub fn has_notdef(&self) -> bool {
        self.glyphs.iter().any(|g| g.gid == GlyphId::NOTDEF)
    }
}

/// `units` font units at font size `size` points, in points.
pub(crate) fn to_points(units: i64, units_per_em: u16, size: f32) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "advances are far below 2^52 font units")]
    let units = units as f64;
    units * f64::from(size) / f64::from(units_per_em.max(1))
}

/// Turns text into glyphs with one font.
pub trait Shaper {
    /// Shapes the bytes `range` of `text` with `font`. The text around the range is context
    /// (Arabic joining across the range's ends, for example) but is not shaped. Clusters
    /// are byte offsets into `text` (not into the range).
    ///
    /// The shaper never fails over characters the font lacks: they come out as `.notdef`
    /// ([`ShapedRun::has_notdef`]); [`crate::pdf_run`] refuses them.
    ///
    /// # Errors
    /// [`Error::InvalidTextRange`] when `range` is out of bounds or not on character
    /// boundaries; [`Error::InvalidFont`] when the font can't be read.
    fn shape_range(&self, font: &Font, text: &str, range: Range<usize>, options: &ShapeOptions) -> Result<ShapedRun>;

    /// Shapes the whole of `text` with `font`.
    ///
    /// # Errors
    /// As [`Shaper::shape_range`].
    fn shape(&self, font: &Font, text: &str, options: &ShapeOptions) -> Result<ShapedRun> {
        self.shape_range(font, text, 0..text.len(), options)
    }
}

/// `text[range]`, or [`Error::InvalidTextRange`].
pub(crate) fn sub(text: &str, range: Range<usize>) -> Result<&str> {
    text.get(range.clone()).ok_or(Error::InvalidTextRange {
        start: range.start,
        end: range.end,
        len: text.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_parses_harfbuzz_syntax() {
        let liga = Tag::new(b"liga");
        assert_eq!("liga".parse::<Feature>().unwrap(), Feature::new(liga, 1));
        assert_eq!("+liga".parse::<Feature>().unwrap(), Feature::new(liga, 1));
        assert_eq!("-liga".parse::<Feature>().unwrap(), Feature::new(liga, 0));
        assert_eq!("liga=0".parse::<Feature>().unwrap(), Feature::new(liga, 0));
        assert_eq!("liga=off".parse::<Feature>().unwrap(), Feature::new(liga, 0));
        assert_eq!("aalt=3".parse::<Feature>().unwrap(), Feature::new(Tag::new(b"aalt"), 3));
        assert_eq!("cv1".parse::<Feature>().unwrap(), Feature::new(Tag::new(b"cv1 "), 1));
    }

    #[test]
    fn feature_rejects_malformed_settings() {
        for bad in ["", "-", "toolong", "liga=", "liga=-1", "+liga=1", "li ga", "liga=x"] {
            assert_eq!(
                bad.parse::<Feature>().map_err(|e| e.code()),
                Err("invalid-feature"),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn script_of_reads_ucd_scripts() {
        assert_eq!(Script::of('a'), Script::LATIN);
        assert_eq!(Script::of('ب'), Script::ARABIC);
        assert_eq!(Script::of('א'), Script::HEBREW);
        assert_eq!(Script::of('क'), Script::DEVANAGARI);
        assert_eq!(Script::of('中'), Script::HAN);
        assert_eq!(Script::of(' '), Script::COMMON);
        assert_eq!(Script::of('\u{0301}'), Script::INHERITED);
    }

    #[test]
    fn script_direction_follows_the_script() {
        assert_eq!(Script::ARABIC.direction(), Direction::RightToLeft);
        assert_eq!(Script::HEBREW.direction(), Direction::RightToLeft);
        assert_eq!(Script::LATIN.direction(), Direction::LeftToRight);
        assert_eq!(Script::COMMON.direction(), Direction::LeftToRight);
    }

    #[test]
    fn dominant_skips_common_characters() {
        assert_eq!(Script::dominant("12, שלום"), Some(Script::HEBREW));
        assert_eq!(Script::dominant("12, 34"), None);
    }
}
