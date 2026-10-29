//! Simple fonts: the standard 14 written as non-embedded `/Type1` fonts with one-byte codes,
//! an encoding, `/Widths` and a ToUnicode CMap (ISO 32000-2 §9.6.2, §9.6.5, §9.10.3;
//! research 03 §3.1, §3.8, §4.1).
//!
//! A Latin font's text is encoded in `WinAnsiEncoding`. Glyphs of the standard Latin character
//! set it lacks (`Lslash`, `Scedilla`, `minus`, `fi`, …) get the codes `WinAnsiEncoding`
//! leaves undefined, in order of first use, through a `/Differences` array (§9.6.5.1): so
//! "Łódź" works in Helvetica. Symbol and `ZapfDingbats` use their built-in encodings, which
//! reach every glyph lamina maps text to.

mod tounicode;

use std::collections::BTreeMap;

use lamina_cos::{CosMut, Dict, Name, ObjRef, Object, Stream};

use crate::embed::Encoded;
use crate::encoding::Encoding;
use crate::error::{Error, Result, Warning};
use crate::font::{Font, GlyphId};
use crate::std14::Standard14;

/// The codes a font's base encoding leaves undefined, which `/Differences` may assign: the
/// high ones first, then the control codes (code 0 is never used).
fn free_codes(base: Encoding) -> impl Iterator<Item = u8> {
    (0x80..=0xFF)
        .chain(0x01..0x80)
        .filter(move |&code| base.glyph_name(code).is_none())
}

/// The codes one standard 14 font uses in a document: the `/Differences` it needs and the
/// text each code stands for (the ToUnicode CMap). Read it with
/// [`FontRegistry::usage`](crate::FontRegistry::usage).
#[derive(Debug, Clone)]
pub struct SimpleUse {
    font: Standard14,
    /// The encoding codes are looked up in first.
    base: Encoding,
    /// Code → the text it stands for, for every code drawn (the first text, empty included).
    text: BTreeMap<u8, String>,
    /// `/Differences`: code → glyph, in order of code.
    differences: BTreeMap<u8, GlyphId>,
    /// The same, glyph → code.
    by_glyph: BTreeMap<GlyphId, u8>,
}

impl SimpleUse {
    pub(crate) fn new(font: Standard14) -> Self {
        Self {
            font,
            base: font.text_encoding(),
            text: BTreeMap::new(),
            differences: BTreeMap::new(),
            by_glyph: BTreeMap::new(),
        }
    }

    /// The standard 14 font.
    #[must_use]
    pub fn font(&self) -> Standard14 {
        self.font
    }

    /// Code → the text it stands for, for every code drawn, in code order: what the ToUnicode
    /// CMap maps. An empty text maps the code to nothing (`<code> <>`), as for Type 0 fonts.
    #[must_use]
    pub fn text(&self) -> &BTreeMap<u8, String> {
        &self.text
    }

    /// The `/Differences` entries: (code, glyph name), in code order.
    pub fn differences(&self) -> impl Iterator<Item = (u8, &'static str)> + '_ {
        self.differences
            .iter()
            .filter_map(|(&code, &gid)| Some((code, self.font.glyph_name(gid)?)))
    }

    /// The code `gid` already has: in the base encoding, else in `/Differences`.
    fn code_of(&self, gid: GlyphId) -> Option<u8> {
        let index = self.font.name_index(gid)?;
        self.base
            .code_for_index(index)
            .or_else(|| self.by_glyph.get(&gid).copied())
    }

    /// The codes for `glyphs`, assigning `/Differences` codes to glyphs that have none, in
    /// order of first use. All or nothing: when the glyphs need more new codes than are free,
    /// nothing is assigned or recorded.
    fn codes_for(&mut self, glyphs: &[GlyphId]) -> Result<Vec<u8>> {
        let count = self.font.glyph_count();
        // `.notdef` (0) is never drawn: lamina has no name to map it to.
        if let Some(&gid) = glyphs.iter().find(|&&g| self.font.name_index(g).is_none()) {
            return Err(Error::GlyphOutOfRange {
                gid: gid.get(),
                count,
                font: self.font.base_font().to_owned(),
            });
        }
        let mut new: Vec<GlyphId> = Vec::new();
        for &gid in glyphs {
            if self.code_of(gid).is_none() && !new.contains(&gid) {
                new.push(gid);
            }
        }
        let limit = free_codes(self.base).count();
        let free: Vec<u8> = free_codes(self.base)
            .filter(|c| !self.differences.contains_key(c))
            .collect();
        if free.len() < new.len() {
            return Err(Error::EncodingFull {
                font: self.font.base_font().to_owned(),
                limit,
                free: free.len(),
            });
        }
        for (gid, code) in new.into_iter().zip(free) {
            self.differences.insert(code, gid);
            self.by_glyph.insert(gid, code);
        }
        // Every glyph has a code now; `filter_map` cannot drop one.
        Ok(glyphs.iter().filter_map(|&g| self.code_of(g)).collect())
    }

    /// Records `text` for `code`. As for Type 0 fonts (B4b), the first text is kept, the empty
    /// text included, and a different later text (empty or not) is returned as a warning (the
    /// builder covers that occurrence with `/ActualText`).
    fn record(&mut self, code: u8, gid: GlyphId, text: &str) -> Option<Warning> {
        let Some(kept) = self.text.get(&code) else {
            self.text.insert(code, text.to_owned());
            return None;
        };
        (kept != text).then(|| Warning::ToUnicodeConflict {
            font: self.font.base_font().to_owned(),
            gid: gid.get(),
            kept: kept.clone(),
            dropped: text.to_owned(),
        })
    }

    /// Records that `gid` is drawn standing for `text` and returns its one-byte code.
    pub(crate) fn use_glyph(&mut self, gid: GlyphId, text: &str) -> Result<(u8, Option<Warning>)> {
        let code = match self.codes_for(&[gid])?.as_slice() {
            [code] => *code,
            // `codes_for` returns one code per glyph.
            _ => unreachable!("one glyph, one code"),
        };
        Ok((code, self.record(code, gid, text)))
    }

    /// `text` as one byte per character, each standing for its character.
    pub(crate) fn encode(&mut self, font: &Font, text: &str) -> Result<(Encoded, Vec<Warning>)> {
        let glyphs = font.map_text(text)?;
        let gids: Vec<GlyphId> = glyphs.iter().map(|&(g, _, _)| g).collect();
        let bytes = self.codes_for(&gids)?;
        let mut warnings = Vec::new();
        let mut buf = [0u8; 4];
        for (&code, &(gid, ch, _)) in bytes.iter().zip(&glyphs) {
            warnings.extend(self.record(code, gid, ch.encode_utf8(&mut buf)));
        }
        let advance = glyphs.iter().map(|&(_, _, w)| f64::from(w)).sum();
        Ok((Encoded { bytes, advance }, warnings))
    }

    /// How many distinct codes were drawn.
    pub(crate) fn code_count(&self) -> usize {
        self.text.len()
    }

    /// The glyph `code` draws: `/Differences` first, then the base encoding.
    fn glyph_at(&self, code: u8) -> Option<GlyphId> {
        self.differences
            .get(&code)
            .copied()
            .or_else(|| self.font.glyph_id(self.base.glyph_name(code)?))
    }

    /// Writes the font descriptor and the ToUnicode CMap as new objects and the font
    /// dictionary into the reserved `object`.
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

        let cmap = tounicode::one_byte_cmap(&self.text);
        let to_unicode = cos.create(Object::Stream(Stream::flate(Dict::new(), &cmap)))?;

        // A font no text was drawn with still needs a valid range: the space.
        let first = self.text.keys().next().copied().unwrap_or(0x20);
        let last = self.text.keys().next_back().copied().unwrap_or(0x20);
        let widths: Vec<Object> = (first..=last)
            .map(|code| {
                let width = self.glyph_at(code).and_then(|g| self.font.glyph_width(g)).unwrap_or(0);
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
        dict.insert("ToUnicode", to_unicode);
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
        for (code, name) in self.differences() {
            if next != Some(code) {
                differences.push(Object::Integer(i64::from(code)));
            }
            differences.push(Name::from(name).into());
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

    fn glyph(font: Standard14, name: &str) -> GlyphId {
        font.glyph_id(name).unwrap()
    }

    #[test]
    fn win_ansi_has_37_free_codes() {
        assert_eq!(free_codes(Encoding::WinAnsi).count(), 37);
    }

    #[test]
    fn win_ansi_glyphs_keep_their_codes() {
        let mut usage = SimpleUse::new(Standard14::Helvetica);
        let code = usage.use_glyph(glyph(Standard14::Helvetica, "Euro"), "€");
        assert_eq!(code, Ok((0x80, None)));
    }

    #[test]
    fn first_glyph_name_is_drawable() {
        // `A` sorts first; its ID is 1, not `.notdef`'s 0.
        let mut usage = SimpleUse::new(Standard14::Helvetica);
        assert_eq!(
            usage.use_glyph(glyph(Standard14::Helvetica, "A"), "A"),
            Ok((0x41, None))
        );
    }

    #[test]
    fn notdef_is_out_of_range() {
        let mut usage = SimpleUse::new(Standard14::Helvetica);
        assert_eq!(
            usage.use_glyph(GlyphId::NOTDEF, "").unwrap_err().code(),
            "glyph-out-of-range"
        );
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
        let all: Vec<GlyphId> = (1..=315).map(GlyphId::new).collect();
        let err = usage.codes_for(&all).unwrap_err();
        assert_eq!((err.code(), usage.differences().count()), ("encoding-full", 0));
    }

    #[test]
    fn encoding_full_reports_the_codes_left() {
        let font = Standard14::Helvetica;
        let mut usage = SimpleUse::new(font);
        let extra: Vec<GlyphId> = font
            .glyph_names()
            .iter()
            .filter(|n| Encoding::WinAnsi.code(n).is_none())
            .map(|n| glyph(font, n))
            .collect();
        usage.codes_for(&extra[..30]).unwrap();
        assert_eq!(
            usage.codes_for(&extra[30..40]).unwrap_err(),
            Error::EncodingFull {
                font: "Helvetica".to_owned(),
                limit: 37,
                free: 7
            }
        );
    }

    #[test]
    fn a_glyph_drawn_for_two_texts_keeps_the_first() {
        let mut usage = SimpleUse::new(Standard14::Helvetica);
        let a = glyph(Standard14::Helvetica, "A");
        usage.use_glyph(a, "A").unwrap();
        let (_, warning) = usage.use_glyph(a, "Α").unwrap();
        assert_eq!(
            (warning.map(|w| w.code()), usage.text().get(&0x41).map(String::as_str)),
            (Some("tounicode-conflict"), Some("A"))
        );
    }

    #[test]
    fn empty_then_real_text_conflicts() {
        // B4b's rule for Type 0 fonts: the empty text is a text.
        let mut usage = SimpleUse::new(Standard14::Symbol);
        let omega = glyph(Standard14::Symbol, "Omega");
        let (code, _) = usage.use_glyph(omega, "").unwrap();
        let (_, warning) = usage.use_glyph(omega, "Ω").unwrap();
        assert_eq!(
            (warning.map(|w| w.code()), usage.text().get(&code).map(String::as_str)),
            (Some("tounicode-conflict"), Some(""))
        );
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
