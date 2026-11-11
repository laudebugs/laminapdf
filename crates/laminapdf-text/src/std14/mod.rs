//! The standard 14 fonts (ISO 32000-2 §9.6.2.2; research 03 §3.1): Times, Helvetica and
//! Courier in four styles, Symbol and `ZapfDingbats`, referred to by name and never embedded.
//! Their metrics come from Adobe's Core 14 AFM files, bundled as generated tables
//! ([`generated`]), so `/Widths` are always written.

pub(crate) mod generated;

pub use generated::AfmMetrics;

use crate::encoding::Encoding;
use crate::font::GlyphId;
use generated::chars::{LATIN_CHARS, SYMBOL_CHARS, ZAPF_DINGBATS_CHARS};
use generated::metrics::FONTS;

/// One of the 14 standard Type 1 fonts. [`Font::standard`](crate::Font::standard) makes a
/// [`Font`](crate::Font) of it.
///
/// ```
/// use laminapdf_text::Standard14;
///
/// let font = Standard14::from_base_font("Helvetica-Bold").unwrap();
/// assert_eq!(font, Standard14::HelveticaBold);
/// assert_eq!(font.width("A"), Some(722));
/// assert_eq!(font.metrics().cap_height, Some(718));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Standard14 {
    /// `Courier`.
    Courier,
    /// `Courier-Bold`.
    CourierBold,
    /// `Courier-Oblique`.
    CourierOblique,
    /// `Courier-BoldOblique`.
    CourierBoldOblique,
    /// `Helvetica`.
    Helvetica,
    /// `Helvetica-Bold`.
    HelveticaBold,
    /// `Helvetica-Oblique`.
    HelveticaOblique,
    /// `Helvetica-BoldOblique`.
    HelveticaBoldOblique,
    /// `Times-Roman`.
    TimesRoman,
    /// `Times-Bold`.
    TimesBold,
    /// `Times-Italic`.
    TimesItalic,
    /// `Times-BoldItalic`.
    TimesBoldItalic,
    /// `Symbol` (Greek letters and mathematical symbols; its own built-in encoding).
    Symbol,
    /// `ZapfDingbats` (ITC Zapf Dingbats; its own built-in encoding).
    ZapfDingbats,
}

impl Standard14 {
    /// All fourteen, in declaration order.
    pub const ALL: [Self; 14] = [
        Self::Courier,
        Self::CourierBold,
        Self::CourierOblique,
        Self::CourierBoldOblique,
        Self::Helvetica,
        Self::HelveticaBold,
        Self::HelveticaOblique,
        Self::HelveticaBoldOblique,
        Self::TimesRoman,
        Self::TimesBold,
        Self::TimesItalic,
        Self::TimesBoldItalic,
        Self::Symbol,
        Self::ZapfDingbats,
    ];

    /// The font's AFM metrics.
    #[must_use]
    #[expect(clippy::similar_names, reason = "one binding per font, named after it")]
    pub fn metrics(self) -> &'static AfmMetrics {
        let [
            cour,
            cour_b,
            cour_o,
            cour_bo,
            helv,
            helv_b,
            helv_o,
            helv_bo,
            times,
            times_b,
            times_i,
            times_bi,
            sym,
            zapf,
        ] = &FONTS;
        match self {
            Self::Courier => cour,
            Self::CourierBold => cour_b,
            Self::CourierOblique => cour_o,
            Self::CourierBoldOblique => cour_bo,
            Self::Helvetica => helv,
            Self::HelveticaBold => helv_b,
            Self::HelveticaOblique => helv_o,
            Self::HelveticaBoldOblique => helv_bo,
            Self::TimesRoman => times,
            Self::TimesBold => times_b,
            Self::TimesItalic => times_i,
            Self::TimesBoldItalic => times_bi,
            Self::Symbol => sym,
            Self::ZapfDingbats => zapf,
        }
    }

    /// The PostScript name, written as `/BaseFont` (`Times-Roman`).
    #[must_use]
    pub fn base_font(self) -> &'static str {
        self.metrics().font_name
    }

    /// The font named `name` (exactly one of the fourteen `/BaseFont` names).
    #[must_use]
    pub fn from_base_font(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.base_font() == name)
    }

    /// Symbol and `ZapfDingbats`: their glyphs are outside the standard Latin character set and
    /// they keep their built-in encodings.
    #[must_use]
    pub const fn is_symbolic(self) -> bool {
        matches!(self, Self::Symbol | Self::ZapfDingbats)
    }

    /// The font's built-in encoding: [`Encoding::Standard`] for the Latin fonts, or
    /// [`Encoding::Symbol`] / [`Encoding::ZapfDingbats`].
    #[must_use]
    pub const fn builtin_encoding(self) -> Encoding {
        match self {
            Self::Symbol => Encoding::Symbol,
            Self::ZapfDingbats => Encoding::ZapfDingbats,
            _ => Encoding::Standard,
        }
    }

    /// The encoding Lamina PDF writes text in: `WinAnsiEncoding` for the Latin fonts (with
    /// `/Differences` for glyphs it lacks), the built-in encoding for the symbolic ones.
    #[must_use]
    pub const fn text_encoding(self) -> Encoding {
        if self.is_symbolic() {
            self.builtin_encoding()
        } else {
            Encoding::WinAnsi
        }
    }

    /// The font's glyph names, sorted (315 for the Latin fonts, which all have the standard
    /// Latin character set; 190 for Symbol; 202 for `ZapfDingbats`).
    #[must_use]
    pub fn glyph_names(self) -> &'static [&'static str] {
        self.metrics().names
    }

    /// The advance width of glyph `name` in thousandths of an em, as in the AFM file.
    #[must_use]
    pub fn width(self, name: &str) -> Option<u16> {
        self.glyph_width(self.glyph_id(name)?)
    }

    /// The number of glyph IDs: `.notdef` (0) and one per glyph name.
    pub(crate) fn glyph_count(self) -> u16 {
        // At most 315 names.
        u16::try_from(self.glyph_names().len() + 1).unwrap_or(u16::MAX)
    }

    /// The glyph ID of glyph `name`: its index in [`Standard14::glyph_names`] plus one, as
    /// glyph 0 is `.notdef` in every font.
    pub(crate) fn glyph_id(self, name: &str) -> Option<GlyphId> {
        let index = self.glyph_names().binary_search(&name).ok()?;
        u16::try_from(index + 1).ok().map(GlyphId::new)
    }

    /// The index of `gid` in [`Standard14::glyph_names`]; `None` for `.notdef` and IDs past
    /// the last glyph.
    pub(crate) fn name_index(self, gid: GlyphId) -> Option<usize> {
        let index = usize::from(gid.get().checked_sub(1)?);
        (index < self.glyph_names().len()).then_some(index)
    }

    /// Whether a standard 14 font lays `ch` out as nothing: a control, a line or paragraph
    /// separator (a flow's hard break), or a default-ignorable format character such as a
    /// zero-width space. Shaping gives it no width and the builder draws no byte for it.
    #[must_use]
    pub const fn draws_nothing(ch: char) -> bool {
        ch.is_control()
            || matches!(
                ch,
                '\u{2028}' | '\u{2029}' | '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{206f}' | '\u{feff}'
            )
    }

    /// The glyph that draws `ch`, through the Adobe Glyph List (`é` is `eacute`). For Symbol
    /// and `ZapfDingbats` only glyphs their built-in encoding reaches are found.
    pub(crate) fn glyph_for_char(self, ch: char) -> Option<GlyphId> {
        let table: &[(char, u16)] = match self {
            Self::Symbol => &SYMBOL_CHARS,
            Self::ZapfDingbats => &ZAPF_DINGBATS_CHARS,
            _ => &LATIN_CHARS,
        };
        let i = table.binary_search_by_key(&ch, |&(c, _)| c).ok()?;
        let &(_, index) = table.get(i)?;
        index.checked_add(1).map(GlyphId::new)
    }

    /// The advance width of `gid`; `None` for `.notdef` and IDs past the last glyph.
    pub(crate) fn glyph_width(self, gid: GlyphId) -> Option<u16> {
        self.metrics().widths.get(self.name_index(gid)?).copied()
    }

    /// The name of `gid`; `None` for `.notdef` and IDs past the last glyph.
    pub(crate) fn glyph_name(self, gid: GlyphId) -> Option<&'static str> {
        self.glyph_names().get(self.name_index(gid)?).copied()
    }
}

/// The glyph `ch` maps to in the standard Latin character set (the glyphs every Latin standard
/// 14 font has), through the Adobe Glyph List.
pub(crate) fn latin_glyph_for_char(ch: char) -> Option<&'static str> {
    let font = Standard14::Helvetica;
    font.glyph_name(font.glyph_for_char(ch)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_font_is_found_by_its_base_font() {
        assert!(
            Standard14::ALL
                .into_iter()
                .all(|f| Standard14::from_base_font(f.base_font()) == Some(f))
        );
    }

    #[test]
    fn widths_and_names_have_one_entry_per_glyph() {
        assert!(
            Standard14::ALL
                .into_iter()
                .all(|f| f.metrics().widths.len() == f.glyph_names().len())
        );
    }

    #[test]
    fn courier_is_fixed_pitch() {
        assert_eq!(Standard14::CourierOblique.width("W"), Some(600));
    }

    #[test]
    fn unknown_glyph_has_no_width() {
        assert_eq!(Standard14::Helvetica.width("alpha"), None);
    }

    #[test]
    fn symbol_finds_greek_capital_omega() {
        let omega = Standard14::Symbol
            .glyph_for_char('Ω')
            .and_then(|g| Standard14::Symbol.glyph_name(g));
        assert_eq!(omega, Some("Omega"));
    }

    #[test]
    fn glyph_zero_is_notdef_not_the_first_name() {
        let a = Standard14::Helvetica.glyph_for_char('A').unwrap();
        assert_ne!(a, GlyphId::NOTDEF);
    }

    #[test]
    fn notdef_has_no_name_or_width() {
        let font = Standard14::Helvetica;
        assert_eq!(
            (font.glyph_name(GlyphId::NOTDEF), font.glyph_width(GlyphId::NOTDEF)),
            (None, None)
        );
    }

    #[test]
    fn latin_finds_eacute() {
        assert_eq!(latin_glyph_for_char('é'), Some("eacute"));
    }
}
