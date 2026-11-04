//! `PDFDocEncoding` as code → glyph name. The workspace has one `PDFDocEncoding` table, in
//! `laminapdf_syntax::text`; this only names each character's glyph in the standard Latin
//! character set, through the Adobe Glyph List.

use laminapdf_syntax::text::{pdf_doc_decode, pdf_doc_encode};

use crate::agl;
use crate::std14::latin_glyph_for_char;

/// The glyph `code` stands for; `None` for undefined codes and control characters.
pub(super) fn glyph_name(code: u8) -> Option<&'static str> {
    latin_glyph_for_char(pdf_doc_decode(code)?)
}

/// The code of glyph `name`, when its character has one in `PDFDocEncoding`.
pub(super) fn code(name: &str) -> Option<u8> {
    let text = agl::glyph_name_to_unicode(name)?;
    let mut chars = text.chars();
    let (Some(ch), None) = (chars.next(), chars.next()) else {
        return None;
    };
    let code = pdf_doc_encode(ch)?;
    // Only names this table uses (`mu`, not `mu1`), so code and name round-trip.
    (glyph_name(code) == Some(name)).then_some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euro_is_at_a0() {
        assert_eq!(glyph_name(0xA0), Some("Euro"));
    }

    #[test]
    fn accents_have_glyphs() {
        assert_eq!(glyph_name(0x18), Some("breve"));
    }

    #[test]
    fn tab_has_no_glyph() {
        assert_eq!(glyph_name(0x09), None);
    }

    #[test]
    fn names_map_back_to_their_code() {
        assert_eq!(code("trademark"), Some(0x92));
    }
}
