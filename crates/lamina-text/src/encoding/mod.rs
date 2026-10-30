//! Simple-font encodings (ISO 32000-2 §9.6.5, Annex D; research 03 §4.1): one-byte codes →
//! glyph names, and glyph names → Unicode through the Adobe Glyph List ([`crate::agl`]).
//! Shared by the simple-font writer and by text extraction (B9b).

mod pdfdoc;

use crate::agl;
use crate::std14::Standard14;
use crate::std14::generated::NONE;
use crate::std14::generated::encodings::{
    MAC_ROMAN, MAC_ROMAN_CODES, STANDARD, STANDARD_CODES, SYMBOL, SYMBOL_CODES, WIN_ANSI, WIN_ANSI_CODES,
    ZAPF_DINGBATS, ZAPF_DINGBATS_CODES,
};

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
    ///
    /// # Errors
    /// [`UnsupportedEncoding::MacExpert`] for `MacExpertEncoding`, a predefined encoding
    /// (§9.6.5.1, Table 112) lamina has no table for, so a reader can warn rather than guess;
    /// [`UnsupportedEncoding::Unknown`] for any other name.
    pub fn from_pdf_name(name: &[u8]) -> Result<Self, UnsupportedEncoding> {
        match name {
            b"WinAnsiEncoding" => Ok(Self::WinAnsi),
            b"MacRomanEncoding" => Ok(Self::MacRoman),
            b"MacExpertEncoding" => Err(UnsupportedEncoding::MacExpert),
            _ => Err(UnsupportedEncoding::Unknown),
        }
    }

    /// The code → glyph table, glyph → lowest code table, and the glyph names they index.
    fn tables(self) -> Option<(&'static [u16; 256], &'static [u8], &'static [&'static str])> {
        let latin = Standard14::Helvetica.glyph_names();
        match self {
            Self::Standard => Some((&STANDARD, &STANDARD_CODES, latin)),
            Self::WinAnsi => Some((&WIN_ANSI, &WIN_ANSI_CODES, latin)),
            Self::MacRoman => Some((&MAC_ROMAN, &MAC_ROMAN_CODES, latin)),
            Self::Symbol => Some((&SYMBOL, &SYMBOL_CODES, Standard14::Symbol.glyph_names())),
            Self::ZapfDingbats => Some((
                &ZAPF_DINGBATS,
                &ZAPF_DINGBATS_CODES,
                Standard14::ZapfDingbats.glyph_names(),
            )),
            Self::PdfDoc => None,
        }
    }

    /// The glyph `code` names; `None` when the code is undefined.
    #[must_use]
    pub fn glyph_name(self, code: u8) -> Option<&'static str> {
        let Some((table, _, names)) = self.tables() else {
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
        let Some((_, codes, names)) = self.tables() else {
            return pdfdoc::code(name);
        };
        let index = names.binary_search(&name).ok()?;
        Self::code_at(codes, index)
    }

    /// The lowest code of the glyph at `index` in the font's sorted glyph names
    /// ([`Standard14::glyph_names`]): a table lookup, for the writer.
    pub(crate) fn code_for_index(self, index: usize) -> Option<u8> {
        match self.tables() {
            Some((_, codes, _)) => Self::code_at(codes, index),
            None => pdfdoc::code(Standard14::Helvetica.glyph_names().get(index)?),
        }
    }

    fn code_at(codes: &[u8], index: usize) -> Option<u8> {
        // Code 0 is undefined in every table, so it marks "no code".
        codes.get(index).copied().filter(|&c| c != 0)
    }

    /// The text `code` stands for: its glyph name through the Adobe Glyph List (the ITC Zapf
    /// Dingbats list for [`Encoding::ZapfDingbats`]). Symbol's Greek letters map to the Greek
    /// block: `Omega` → U+03A9, `Delta` → U+0394, `mu` → U+03BC (the AGL's `Omegagreek`, …),
    /// not the AGL's U+2126 OHM SIGN, U+2206 INCREMENT and U+00B5 MICRO SIGN, which are for the
    /// Latin fonts' glyphs of those names; so Greek text written in Symbol reads back as Greek.
    #[must_use]
    pub fn to_unicode(self, code: u8) -> Option<String> {
        self.glyph_to_unicode(self.glyph_name(code)?)
    }

    /// [`Encoding::to_unicode`] for a glyph name of this encoding's font.
    pub(crate) fn glyph_to_unicode(self, name: &str) -> Option<String> {
        match self {
            Self::ZapfDingbats => agl::zapf_dingbats_to_unicode(name),
            // Letters only: the AGL's `questiongreek` is U+037E, a punctuation mark.
            Self::Symbol => agl::to_unicode(&format!("{name}greek"))
                .filter(|t| t.chars().all(char::is_alphabetic))
                .or_else(|| agl::to_unicode(name)),
            _ => agl::to_unicode(name),
        }
    }
}

/// A predefined encoding name lamina has no table for ([`Encoding::from_pdf_name`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UnsupportedEncoding {
    /// `MacExpertEncoding` (Annex D.4: expert glyphs such as small capitals and old-style
    /// figures), predefined but not bundled.
    #[error("MacExpertEncoding is not supported")]
    MacExpert,
    /// A name that is not a predefined encoding.
    #[error("not a predefined encoding name")]
    Unknown,
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
        assert_eq!(Encoding::from_pdf_name(b"MacRomanEncoding"), Ok(Encoding::MacRoman));
    }

    #[test]
    fn mac_expert_is_reported_as_unsupported() {
        assert_eq!(
            Encoding::from_pdf_name(b"MacExpertEncoding"),
            Err(UnsupportedEncoding::MacExpert)
        );
    }

    #[test]
    fn symbol_greek_letters_read_back_as_greek() {
        let omega = Encoding::Symbol.code("Omega").unwrap();
        assert_eq!(Encoding::Symbol.to_unicode(omega).as_deref(), Some("Ω"));
    }

    #[test]
    fn symbol_question_mark_stays_ascii() {
        assert_eq!(Encoding::Symbol.to_unicode(b'?').as_deref(), Some("?"));
    }

    #[test]
    fn latin_mu_stays_the_micro_sign() {
        assert_eq!(Encoding::WinAnsi.to_unicode(0xB5).as_deref(), Some("\u{B5}"));
    }

    #[test]
    fn code_for_index_matches_code() {
        let names = Standard14::Helvetica.glyph_names();
        let i = names.binary_search(&"Euro").unwrap();
        assert_eq!(Encoding::WinAnsi.code_for_index(i), Some(0x80));
    }
}
