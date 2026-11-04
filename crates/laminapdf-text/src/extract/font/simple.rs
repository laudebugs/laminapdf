//! Simple fonts (Type 1, TrueType, Type 3): one-byte codes, the encoding with its `/Differences`,
//! the standard 14 and the program's own glyph names.

use std::collections::BTreeMap;

use laminapdf_cos::Result;
use laminapdf_syntax::{Dict, Object};
use skrifa::{FontRef, MetadataProvider};

use super::{Reader, extent, name_of, whole};
use crate::extract::cmap::Code;
use crate::{Encoding, Standard14, agl};

/// Type 1, `MMType1`, TrueType and Type 3 fonts: one byte per code.
#[derive(Debug)]
pub(super) struct Simple {
    /// Glyph names by code, from the encoding.
    pub(super) names: Vec<Option<Box<str>>>,
    /// Widths by code, in thousandths of text space (Type 3 widths already through
    /// `/FontMatrix`).
    pub(super) widths: Vec<f64>,
    pub(super) zapf: bool,
    /// Code → text from the embedded font's `cmap` (for codes the names don't map).
    pub(super) program: BTreeMap<u8, char>,
    /// Ascent and descent, thousandths of text space.
    pub(super) extent: (f64, f64),
}

/// The text each glyph of an sfnt program stands for, from its Unicode `cmap` (lowest code
/// point per glyph; private-use symbol codes `U+F0xx` are left out).
pub(super) fn glyph_text(program: &[u8]) -> BTreeMap<u32, char> {
    let mut out = BTreeMap::new();
    let Ok(font) = FontRef::new(program) else { return out };
    for (code_point, gid) in font.charmap().mappings() {
        if (0xF000..=0xF0FF).contains(&code_point) {
            continue;
        }
        if let Some(ch) = char::from_u32(code_point) {
            out.entry(gid.to_u32()).or_insert(ch);
        }
    }
    out
}

/// The standard 14 font a `/BaseFont` names, its subset tag dropped, with the common aliases
/// (`Arial` → Helvetica, `TimesNewRoman` → Times, `CourierNew` → Courier) readers apply.
pub(super) fn standard_font(base_font: &[u8]) -> Option<Standard14> {
    let name = std::str::from_utf8(base_font).ok()?;
    let name = match name.split_once('+') {
        Some((tag, rest)) if tag.len() == 6 => rest,
        _ => name,
    };
    if let Some(font) = Standard14::from_base_font(name) {
        return Some(font);
    }
    let (family, style) = name.split_once([',', '-']).unwrap_or((name, ""));
    let bold = style.contains("Bold");
    let italic = style.contains("Italic") || style.contains("Oblique");
    let base = match family {
        "Arial" | "ArialMT" | "Helvetica" => "Helvetica",
        "TimesNewRoman" | "TimesNewRomanPS" | "TimesNewRomanPSMT" | "Times" => "Times",
        "CourierNew" | "CourierNewPSMT" | "Courier" => "Courier",
        "Symbol" => return Some(Standard14::Symbol),
        _ => return None,
    };
    let full = match (base, bold, italic) {
        ("Times", false, false) => "Times-Roman".to_owned(),
        ("Times", true, false) => "Times-Bold".to_owned(),
        ("Times", false, true) => "Times-Italic".to_owned(),
        ("Times", true, true) => "Times-BoldItalic".to_owned(),
        (b, false, false) => b.to_owned(),
        (b, true, false) => format!("{b}-Bold"),
        (b, false, true) => format!("{b}-Oblique"),
        (b, true, true) => format!("{b}-BoldOblique"),
    };
    Standard14::from_base_font(&full)
}

pub(super) fn fixed(bytes: &[u8], len: usize) -> Option<(Code, usize)> {
    let len = len.min(bytes.len());
    Some((crate::extract::cmap::code_of(bytes.get(..len)?)?, len))
}

impl Simple {
    pub(super) fn load(reader: &mut Reader<'_>, dict: &Dict) -> Result<Self> {
        let base_font = name_of(dict.get("BaseFont")).unwrap_or_default().to_vec();
        let standard = standard_font(&base_font);
        let descriptor = reader.dict(dict, "FontDescriptor")?;
        let flags = match &descriptor {
            Some(d) => reader.number(d, "Flags")?.and_then(whole).unwrap_or(0),
            None => 0,
        };
        // Bit 3 (value 4): symbolic (§9.8.2, Table 121).
        let symbolic = flags & 4 != 0;
        let program = reader.program(descriptor.as_ref())?;
        let type3 = name_of(dict.get("Subtype")) == Some(b"Type3");

        // The base encoding: the named one, else the standard 14 font's built-in one, else
        // StandardEncoding for a nonsymbolic font (§9.6.5.1); a symbolic font's built-in
        // encoding is in its program, which is not read.
        let encoding = reader.get(dict, "Encoding")?;
        let named = |name: &[u8], reader: &mut Reader<'_>| match Encoding::from_pdf_name(name) {
            Ok(e) => Some(e),
            Err(_) if name == b"StandardEncoding" => Some(Encoding::Standard),
            Err(_) => {
                reader.issues.font = true;
                None
            }
        };
        let builtin = standard
            .map(Standard14::builtin_encoding)
            .or((!symbolic && !type3).then_some(Encoding::Standard));
        let (base, differences) = match encoding.as_deref() {
            Some(Object::Name(name)) => (named(&name.0, reader).or(builtin), None),
            Some(Object::Dict(d)) => {
                let base = match name_of(d.get("BaseEncoding")) {
                    Some(name) => named(name, reader),
                    None => None,
                }
                .or(builtin);
                (base, reader.get(d, "Differences")?)
            }
            Some(_) => {
                reader.issues.font = true;
                (builtin, None)
            }
            None => (builtin, None),
        };
        let mut glyph_names: Vec<Option<Box<str>>> = (0..=255u8)
            .map(|code| base.and_then(|e| e.glyph_name(code)).map(Into::into))
            .collect();
        if let Some(differences) = differences {
            apply_differences(reader, &differences, &mut glyph_names);
        }

        let widths = Self::widths(reader, dict, descriptor.as_ref(), &glyph_names, standard, type3)?;
        let program = program.map_or_else(BTreeMap::new, |bytes| {
            let by_glyph = glyph_text(&bytes);
            let Ok(font) = FontRef::new(&bytes) else {
                return BTreeMap::new();
            };
            let charmap = font.charmap();
            (0..=255u8)
                .filter_map(|code| {
                    let gid = charmap
                        .map(0xF000 + u32::from(code))
                        .or_else(|| charmap.map(u32::from(code)))?;
                    Some((code, *by_glyph.get(&gid.to_u32())?))
                })
                .collect()
        });
        let extent = extent(reader, dict, descriptor.as_ref(), standard)?;
        Ok(Self {
            names: glyph_names,
            widths,
            zapf: standard == Some(Standard14::ZapfDingbats),
            program,
            extent,
        })
    }

    pub(super) fn widths(
        reader: &mut Reader<'_>,
        dict: &Dict,
        descriptor: Option<&Dict>,
        names: &[Option<Box<str>>],
        standard: Option<Standard14>,
        type3: bool,
    ) -> Result<Vec<f64>> {
        let missing = match descriptor {
            Some(d) => reader.number(d, "MissingWidth")?.unwrap_or(0.0),
            None => 0.0,
        };
        // Type 3 widths are in glyph space: `/FontMatrix` maps them to text space (§9.6.4).
        let scale = if type3 {
            let matrix = match reader.get(dict, "FontMatrix")? {
                Some(m) => reader.numbers(&m)?,
                None => Vec::new(),
            };
            matrix.first().copied().unwrap_or(0.001) * 1000.0
        } else {
            1.0
        };
        let first = reader.number(dict, "FirstChar")?;
        let listed = reader.get(dict, "Widths")?;
        // §9.6.2.1: codes outside `/Widths` take `/MissingWidth`; only a font without
        // `/Widths` (a standard 14 font written by name) takes the AFM widths.
        let afm = if listed.is_none() { standard } else { None };
        let mut widths: Vec<f64> = (0..256)
            .map(|code| {
                let name = names.get(code).and_then(Option::as_deref);
                match (afm, name) {
                    (Some(font), Some(name)) => font.width(name).map_or(missing, f64::from),
                    _ => missing,
                }
            })
            .collect();
        if let (Some(first), Some(list)) = (first, listed) {
            let list = reader.numbers(&list)?;
            let first = whole(first).and_then(|n| usize::try_from(n).ok()).unwrap_or(0);
            for (i, w) in list.into_iter().enumerate() {
                if let Some(slot) = widths.get_mut(first.saturating_add(i)) {
                    *slot = w * scale;
                }
            }
        }
        Ok(widths)
    }

    pub(super) fn unicode(&self, code: Code) -> Option<String> {
        let index = usize::try_from(code.value).ok()?;
        if let Some(name) = self.names.get(index).and_then(Option::as_deref) {
            let text = if self.zapf {
                agl::zapf_dingbats_to_unicode(name)
            } else {
                agl::glyph_name_to_unicode(name)
            };
            if text.is_some() {
                return text;
            }
        }
        let code = u8::try_from(code.value).ok()?;
        self.program.get(&code).map(char::to_string)
    }
}

/// `/Differences [code /name /name … code /name …]` (§9.6.5.1).
fn apply_differences(reader: &mut Reader<'_>, differences: &Object, names: &mut [Option<Box<str>>]) {
    let Some(items) = differences.as_array() else {
        reader.issues.font = true;
        return;
    };
    let mut code: Option<usize> = None;
    for item in items {
        match item {
            Object::Integer(n) => code = usize::try_from(*n).ok(),
            Object::Name(name) => {
                if let Some(c) = code {
                    if let (Some(slot), Ok(name)) = (names.get_mut(c), std::str::from_utf8(&name.0)) {
                        *slot = Some(name.into());
                    }
                    code = Some(c + 1);
                } else {
                    reader.issues.font = true;
                }
            }
            _ => reader.issues.font = true,
        }
    }
}
