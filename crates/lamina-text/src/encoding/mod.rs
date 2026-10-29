//! Simple-font encodings (ISO 32000-2 §9.6.5, Annex D; research 03 §4.1): one-byte codes →
//! glyph names, and glyph names → Unicode through the Adobe Glyph List ([`crate::agl`]).
//! Shared by the simple-font writer and by text extraction (B9b).

mod pdfdoc;

use crate::agl;
use crate::std14::Standard14;
use crate::std14::generated::NONE;
use crate::std14::generated::encodings::{MAC_ROMAN, STANDARD, SYMBOL, WIN_ANSI, ZAPF_DINGBATS};

/// A simple-font encoding: a table of 256 codes, each undefined or naming a glyph.
///
/// ```
/// use lamina_text::Encoding;
///
/// assert_eq!(Encoding::WinAnsi.glyph_name(0x80), Some("Euro"));
/// assert_eq!(Encoding::WinAnsi.code("space"), Some(0x20)); // also 0xA0: the lower code
/// assert_eq!(Encoding::Symbol.glyph_name(0x61), Some("alpha"));
/// assert_eq!(Encoding::MacRoman.to_unicode(0x8E).as_deref(), Some("é"));
/// assert_eq!(Encoding::WinAnsi.glyph_name(0x81), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Encoding {
    /// `StandardEncoding`: the built-in encoding of the Latin standard 14 fonts (Annex D.2).
    /// It encodes no accented letters.
    Standard,
    /// `WinAnsiEncoding` (Annex D.2; Windows code page 1252).
    WinAnsi,
    /// `MacRomanEncoding` (Annex D.2): the Mac OS Roman set restricted to the standard Latin
    /// character set (no `apple`, `Omega`, `notequal`, …; `currency` at 333 octal).
    MacRoman,
    /// `PDFDocEncoding` (Annex D.2, D.3), for text strings, not fonts: the table in
    /// `lamina_syntax::text` with each character's glyph in the standard Latin character set.
    PdfDoc,
    /// The Symbol font's built-in encoding (Annex D.5).
    Symbol,
    /// The `ZapfDingbats` font's built-in encoding (Annex D.6).
    ZapfDingbats,
}

impl Encoding {
    /// The name of a predefined encoding as `/Encoding` or `/BaseEncoding` writes it
    /// (§9.6.5.1, Table 112): only `WinAnsiEncoding` and `MacRomanEncoding` of these.
    #[must_use]
    pub const fn pdf_name(self) -> Option<&'static str> {
        match self {
            Self::WinAnsi => Some("WinAnsiEncoding"),
            Self::MacRoman => Some("MacRomanEncoding"),
            _ => None,
        }
    }

    /// The encoding named `name` in an `/Encoding` or `/BaseEncoding` entry.
    #[must_use]
    pub fn from_pdf_name(name: &[u8]) -> Option<Self> {
        match name {
            b"WinAnsiEncoding" => Some(Self::WinAnsi),
            b"MacRomanEncoding" => Some(Self::MacRoman),
            _ => None,
        }
    }

    /// The table and the glyph names it indexes into.
    fn table(self) -> Option<(&'static [u16; 256], &'static [&'static str])> {
        let latin = Standard14::Helvetica.glyph_names();
        match self {
            Self::Standard => Some((&STANDARD, latin)),
            Self::WinAnsi => Some((&WIN_ANSI, latin)),
            Self::MacRoman => Some((&MAC_ROMAN, latin)),
            Self::Symbol => Some((&SYMBOL, Standard14::Symbol.glyph_names())),
            Self::ZapfDingbats => Some((&ZAPF_DINGBATS, Standard14::ZapfDingbats.glyph_names())),
            Self::PdfDoc => None,
        }
    }

    /// The glyph `code` names; `None` when the code is undefined.
    #[must_use]
    pub fn glyph_name(self, code: u8) -> Option<&'static str> {
        let Some((table, names)) = self.table() else {
            return pdfdoc::glyph_name(code);
        };
        let index = *table.get(usize::from(code))?;
        if index == NONE {
            return None;
        }
        names.get(usize::from(index)).copied()
    }

    /// The code for glyph `name`; the lowest when there are two (`WinAnsiEncoding` has `space`
    /// at 0x20 and 0xA0, `hyphen` at 0x2D and 0xAD).
    #[must_use]
    pub fn code(self, name: &str) -> Option<u8> {
        if self == Self::PdfDoc {
            return pdfdoc::code(name);
        }
        (0..=u8::MAX).find(|&code| self.glyph_name(code) == Some(name))
    }

    /// The text `code` stands for: its glyph name through the Adobe Glyph List (the ITC Zapf
    /// Dingbats list for [`Encoding::ZapfDingbats`]).
    #[must_use]
    pub fn to_unicode(self, code: u8) -> Option<String> {
        let name = self.glyph_name(code)?;
        if self == Self::ZapfDingbats {
            agl::zapf_dingbats_to_unicode(name)
        } else {
            agl::to_unicode(name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_ansi_names_round_trip() {
        assert_eq!(Encoding::WinAnsi.code("Euro"), Some(0x80));
    }

    #[test]
    fn standard_encoding_has_no_accented_letters() {
        assert_eq!(Encoding::Standard.code("eacute"), None);
    }

    #[test]
    fn mac_roman_has_currency_at_333_octal() {
        assert_eq!(Encoding::MacRoman.glyph_name(0o333), Some("currency"));
    }

    #[test]
    fn symbol_has_euro_at_160() {
        assert_eq!(Encoding::Symbol.code("Euro"), Some(160));
    }

    #[test]
    fn zapf_dingbats_maps_to_dingbats() {
        assert_eq!(Encoding::ZapfDingbats.to_unicode(0x34).as_deref(), Some("✔"));
    }

    #[test]
    fn names_parse_back() {
        assert_eq!(Encoding::from_pdf_name(b"MacRomanEncoding"), Some(Encoding::MacRoman));
    }
}
