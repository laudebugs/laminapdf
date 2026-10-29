//! Simple fonts: the standard 14 written as non-embedded `/Type1` fonts with one-byte codes,
//! an encoding and `/Widths` (ISO 32000-2 §9.6.2, §9.6.5; research 03 §3.1, §3.8, §4.1).
//!
//! A Latin font's text is encoded in `WinAnsiEncoding`. Glyphs of the standard Latin character
//! set it lacks (`Lslash`, `Scedilla`, `minus`, `fi`, …) get the codes `WinAnsiEncoding`
//! leaves undefined, in order of first use, through a `/Differences` array (§9.6.5.1): so
//! "Łódź" works in Helvetica. Symbol and `ZapfDingbats` use their built-in encodings, which
//! reach every glyph lamina maps text to.

use std::collections::{BTreeMap, BTreeSet};

use lamina_cos::{CosMut, Dict, Name, ObjRef, Object};

use crate::embed::Encoded;
use crate::encoding::Encoding;
use crate::error::{Error, Result};
use crate::font::{Font, GlyphId};
use crate::std14::Standard14;

/// The codes a font's base encoding leaves undefined, which `/Differences` may assign: the
/// high ones first, then the control codes (code 0 is never used).
fn free_codes(base: Encoding) -> impl Iterator<Item = u8> {
    (0x80..=0xFF)
        .chain(0x01..0x80)
        .filter(move |&code| base.glyph_name(code).is_none())
}

/// The codes one standard 14 font uses in a document, and the `/Differences` it needs.
#[derive(Debug, Clone)]
pub(crate) struct SimpleUse {
    font: Standard14,
    /// The encoding codes are looked up in first.
    base: Encoding,
    /// Codes drawn so far.
    used: BTreeSet<u8>,
    /// `/Differences`: code → glyph (index into the font's glyph names), in order of code.
    differences: BTreeMap<u8, u16>,
}

impl SimpleUse {
    pub(crate) fn new(font: Standard14) -> Self {
        Self {
            font,
            base: font.text_encoding(),
            used: BTreeSet::new(),
            differences: BTreeMap::new(),
        }
    }

    /// The code `glyph` already has: in the base encoding, else in `/Differences`.
    fn code_of(&self, glyph: u16) -> Option<u8> {
        let name = self.font.name_of(glyph)?;
        self.base.code(name).or_else(|| {
            self.differences
                .iter()
                .find_map(|(&code, &g)| (g == glyph).then_some(code))
        })
    }

    /// The codes for `glyphs`, assigning `/Differences` codes to glyphs that have none. Checks
    /// everything before recording anything.
    fn codes_for(&mut self, glyphs: &[u16]) -> Result<Vec<u8>> {
        let names = self.font.glyph_names();
        let count = u16::try_from(names.len()).unwrap_or(u16::MAX);
        if let Some(&gid) = glyphs.iter().find(|&&g| g >= count) {
            return Err(Error::GlyphOutOfRange {
                gid,
                count,
                font: self.font.base_font().to_owned(),
            });
        }
        let new: BTreeSet<u16> = glyphs.iter().copied().filter(|&g| self.code_of(g).is_none()).collect();
        let free: Vec<u8> = free_codes(self.base)
            .filter(|c| !self.differences.contains_key(c))
            .take(new.len())
            .collect();
        if free.len() < new.len() {
            return Err(Error::EncodingFull {
                font: self.font.base_font().to_owned(),
                limit: free_codes(self.base).count(),
            });
        }
        // New glyphs take codes in order of first use, so the same text gives the same codes.
        let mut free = free.into_iter();
        for &glyph in glyphs {
            if self.code_of(glyph).is_none()
                && let Some(code) = free.next()
            {
                self.differences.insert(code, glyph);
            }
        }
        let codes: Vec<u8> = glyphs.iter().filter_map(|&g| self.code_of(g)).collect();
        self.used.extend(codes.iter().copied());
        Ok(codes)
    }

    /// Records that `gid` is drawn and returns its one-byte code.
    pub(crate) fn use_glyph(&mut self, gid: GlyphId) -> Result<u8> {
        let codes = self.codes_for(&[gid.get()])?;
        codes.first().copied().ok_or(Error::GlyphOutOfRange {
            gid: gid.get(),
            count: u16::try_from(self.font.glyph_names().len()).unwrap_or(u16::MAX),
            font: self.font.base_font().to_owned(),
        })
    }

    /// `text` as one byte per character.
    pub(crate) fn encode(&mut self, font: &Font, text: &str) -> Result<Encoded> {
        let glyphs = font.map_text(text)?;
        let gids: Vec<u16> = glyphs.iter().map(|(g, _, _)| g.get()).collect();
        let bytes = self.codes_for(&gids)?;
        let advance = glyphs.iter().map(|&(_, _, w)| f64::from(w)).sum();
        Ok(Encoded { bytes, advance })
    }

    /// How many distinct codes were drawn.
    pub(crate) fn code_count(&self) -> usize {
        self.used.len()
    }

    /// The glyph `code` draws: `/Differences` first, then the base encoding.
    fn glyph_at(&self, code: u8) -> Option<u16> {
        self.differences
            .get(&code)
            .copied()
            .or_else(|| self.base.glyph_name(code).and_then(|n| self.font.glyph_index(n)))
    }

    /// Writes the font descriptor as a new object and the font dictionary into the reserved
    /// `object`.
    pub(crate) fn write(&self, cos: &mut CosMut<'_>, font: &Font, object: ObjRef) -> Result<()> {
        let info = font.info();
        let base_font = Name::from(self.font.base_font());
        let round = |v: f32| Object::Integer(crate::embed::round_i64(f64::from(v)));

        let mut descriptor = Dict::new();
        descriptor.insert("Type", Name::from("FontDescriptor"));
        descriptor.insert("FontName", base_font.clone());
        descriptor.insert("Flags", i64::from(info.flags));
        descriptor.insert("FontBBox", Object::Array(info.bbox.iter().map(|&v| round(v)).collect()));
        descriptor.insert("ItalicAngle", crate::embed::real(f64::from(info.italic_angle)));
        descriptor.insert("Ascent", round(info.ascent));
        descriptor.insert("Descent", round(info.descent));
        // §9.8.1 Table 120: CapHeight is required for fonts with Latin characters.
        if let Some(cap_height) = info.cap_height {
            descriptor.insert("CapHeight", round(cap_height));
        }
        if let Some(x_height) = info.x_height {
            descriptor.insert("XHeight", round(x_height));
        }
        let metrics = self.font.metrics();
        descriptor.insert("StemV", i64::from(metrics.std_vw));
        descriptor.insert("StemH", i64::from(metrics.std_hw));
        let descriptor = cos.create(Object::Dict(descriptor))?;

        // A font no text was drawn with still needs a valid range: the space.
        let first = self.used.first().copied().unwrap_or(0x20);
        let last = self.used.last().copied().unwrap_or(0x20);
        let widths: Vec<Object> = (first..=last)
            .map(|code| {
                let width = self.glyph_at(code).and_then(|g| self.font.width_of(g)).unwrap_or(0);
                Object::Integer(i64::from(width))
            })
            .collect();

        let mut dict = Dict::new();
        dict.insert("Type", Name::from("Font"));
        dict.insert("Subtype", Name::from("Type1"));
        dict.insert("BaseFont", base_font);
        if let Some(encoding) = self.encoding_object() {
            dict.insert("Encoding", encoding);
        }
        dict.insert("FirstChar", i64::from(first));
        dict.insert("LastChar", i64::from(last));
        dict.insert("Widths", Object::Array(widths));
        dict.insert("FontDescriptor", descriptor);
        cos.set(object, Object::Dict(dict))?;
        Ok(())
    }

    /// `/Encoding`: the base encoding's name, or a dictionary with `/Differences` (§9.6.5.1,
    /// Table 112); `None` for a symbolic font drawn with its built-in encoding only.
    fn encoding_object(&self) -> Option<Object> {
        let base = self.base.pdf_name();
        if self.differences.is_empty() {
            return base.map(|b| Name::from(b).into());
        }
        let mut differences = Vec::new();
        let mut next = None;
        for (&code, &glyph) in &self.differences {
            if next != Some(code) {
                differences.push(Object::Integer(i64::from(code)));
            }
            differences.push(Name::from(self.font.name_of(glyph).unwrap_or(".notdef")).into());
            next = code.checked_add(1);
        }
        let mut dict = Dict::new();
        dict.insert("Type", Name::from("Encoding"));
        if let Some(base) = base {
            dict.insert("BaseEncoding", Name::from(base));
        }
        dict.insert("Differences", Object::Array(differences));
        Some(Object::Dict(dict))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(font: Standard14, name: &str) -> u16 {
        font.glyph_index(name).unwrap()
    }

    #[test]
    fn win_ansi_has_37_free_codes() {
        assert_eq!(free_codes(Encoding::WinAnsi).count(), 37);
    }

    #[test]
    fn win_ansi_glyphs_keep_their_codes() {
        let mut usage = SimpleUse::new(Standard14::Helvetica);
        let code = usage.use_glyph(GlyphId::new(glyph(Standard14::Helvetica, "Euro")));
        assert_eq!(code, Ok(0x80));
    }

    #[test]
    fn other_latin_glyphs_take_free_codes_in_order() {
        let mut usage = SimpleUse::new(Standard14::Helvetica);
        let codes = usage.codes_for(&[
            glyph(Standard14::Helvetica, "Lslash"),
            glyph(Standard14::Helvetica, "A"),
            glyph(Standard14::Helvetica, "minus"),
            glyph(Standard14::Helvetica, "Lslash"),
        ]);
        assert_eq!(codes, Ok(vec![0x81, 0x41, 0x8D, 0x81]));
    }

    #[test]
    fn running_out_of_codes_records_nothing() {
        let mut usage = SimpleUse::new(Standard14::Helvetica);
        let all: Vec<u16> = (0..315).collect();
        let err = usage.codes_for(&all).unwrap_err();
        assert_eq!((err.code(), usage.code_count()), ("encoding-full", 0));
    }

    #[test]
    fn differences_group_consecutive_codes() {
        let mut usage = SimpleUse::new(Standard14::Helvetica);
        let font = Standard14::Helvetica;
        usage
            .codes_for(&[
                glyph(font, "Lslash"),
                glyph(font, "lslash"),
                glyph(font, "minus"),
                glyph(font, "Scedilla"),
            ])
            .unwrap();
        let text = String::from_utf8(usage.encoding_object().unwrap().to_bytes()).unwrap();
        assert_eq!(
            text,
            "<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [129 /Lslash 141 /lslash 143 /minus /Scedilla] >>"
        );
    }
}
