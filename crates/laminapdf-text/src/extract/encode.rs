//! New text in a font a page already uses (ADR 0022 §6, step 1): which codes of the font can
//! draw which text, so an edit reuses the embedded font when every new character has a glyph
//! in it, and is refused, with a reason, when the font can't be edited at all.

use std::collections::BTreeMap;
use std::sync::Arc;

use laminapdf_content::NotEditable;
use laminapdf_cos::Result;
use laminapdf_syntax::{Dict, ObjRef, Object};

use super::TextExtractor;
use super::cmap::Code;
use super::font::ExtractFont;

/// The longest text one code may stand for that [`FontEncoder::encode`] matches (a ligature
/// such as "ffi").
const MAX_CODE_TEXT: usize = 4;

/// One glyph of encoded text.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct EncodedGlyph {
    /// The character code to show.
    pub code: Code,
    /// Its horizontal displacement `w0` in thousandths of text space (§9.4.4).
    pub width: f64,
    /// Whether word spacing (`Tw`) applies to it: the single-byte code 32 (§9.3.3).
    pub word_space: bool,
}

/// The codes of one font that can draw new text, and why the font can't be edited when it
/// can't: built by [`TextExtractor::font_encoder`].
///
/// Added in 0.5.0.
#[derive(Debug)]
pub struct FontEncoder {
    font: Arc<ExtractFont>,
    /// Each text a usable code stands for, with that code.
    by_text: BTreeMap<String, Code>,
    refusal: Option<NotEditable>,
}

impl FontEncoder {
    /// Why text in this font can't be edited, or `None` when it can: a Type 3 font
    /// ([`NotEditable::Type3Font`]), vertical writing ([`NotEditable::VerticalText`]), or no
    /// usable code mapped to text ([`NotEditable::NoUnicodeMap`]).
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn refusal(&self) -> Option<NotEditable> {
        self.refusal
    }

    /// The code `code` of this font with its width, as [`FontEncoder::encode`] gives it.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn glyph(&self, code: Code) -> EncodedGlyph {
        EncodedGlyph {
            code,
            width: self.font.displacement(code).0,
            word_space: ExtractFont::is_word_space(code),
        }
    }

    /// `text` as codes of this font, longest match first, with their widths.
    ///
    /// # Errors
    /// The first character no usable code stands for.
    ///
    /// Added in 0.5.0.
    pub fn encode(&self, text: &str) -> std::result::Result<Vec<EncodedGlyph>, char> {
        let chars: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        let mut at = 0;
        while let Some(&first) = chars.get(at) {
            let longest = (1..=MAX_CODE_TEXT.min(chars.len() - at)).rev().find_map(|n| {
                let piece: String = chars.get(at..at + n)?.iter().collect();
                self.by_text.get(&piece).map(|&code| (code, n))
            });
            let Some((code, n)) = longest else {
                return Err(first);
            };
            out.push(self.glyph(code));
            at += n;
        }
        Ok(out)
    }
}

/// Whether `name` is a subset's (`ABCDEF+Name`, §9.6.4): six capital letters and a plus.
fn is_subset(name: &[u8]) -> bool {
    name.len() > 7 && name.get(6) == Some(&b'+') && name.iter().take(6).all(u8::is_ascii_uppercase)
}

/// The name a dictionary entry holds, as bytes.
fn name<'d>(dict: &'d Dict, key: &str) -> Option<&'d [u8]> {
    match dict.get(key) {
        Some(Object::Name(name)) => Some(name.as_bytes()),
        _ => None,
    }
}

impl TextExtractor {
    /// Which codes of the font `font` can draw new text (ADR 0022 §6, step 1). A code is usable
    /// when it maps to text (ToUnicode, else the encoding and glyph names) and the font program
    /// surely holds its glyph: for a subset (`ABCDEF+Name`) or a composite font, only the codes
    /// in `used`, which the document already draws in this font; for a simple font embedded
    /// whole or not embedded, every code of its encoding as well. When two codes stand for the
    /// same text, the first used one wins.
    ///
    /// # Errors
    /// Errors reading the font, as [`TextExtractor::page`] meets them.
    ///
    /// Added in 0.5.0.
    pub fn font_encoder(
        &mut self,
        doc: &laminapdf_model::Document,
        font: ObjRef,
        used: &[Code],
    ) -> Result<FontEncoder> {
        let object = doc.resolve(font)?;
        let dict = object.as_dict().cloned().unwrap_or_default();
        let loaded = self.fonts.get(doc, Some(font), &dict)?;
        let subtype = name(&dict, "Subtype");
        let whole = subtype != Some(b"Type0") && !name(&dict, "BaseFont").is_some_and(is_subset);
        let mut by_text: BTreeMap<String, Code> = BTreeMap::new();
        let mut add = |code: Code| {
            if let Some(text) = loaded.unicode(code)
                && !text.is_empty()
                && text.chars().count() <= MAX_CODE_TEXT
            {
                by_text.entry(text).or_insert(code);
            }
        };
        used.iter().copied().for_each(&mut add);
        if whole {
            (0..=255).map(|value| Code { value, len: 1 }).for_each(&mut add);
        }
        let refusal = if subtype == Some(b"Type3") {
            Some(NotEditable::Type3Font)
        } else if loaded.vertical() {
            Some(NotEditable::VerticalText)
        } else if by_text.is_empty() {
            Some(NotEditable::NoUnicodeMap)
        } else {
            None
        };
        Ok(FontEncoder {
            font: loaded,
            by_text,
            refusal,
        })
    }
}
