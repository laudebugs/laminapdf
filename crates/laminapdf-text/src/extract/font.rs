//! Fonts as text extraction reads them (ISO 32000-2 §9.5–9.10; research 03 §3–5): how a string
//! splits into codes, how far each code moves the pen, and what text it stands for.
//!
//! Unicode is resolved in this order (§9.10.2, research 03 §8.1):
//!
//! 1. the font's ToUnicode CMap;
//! 2. simple fonts: the code's glyph name (`/Encoding`, its `/Differences`, else the built-in or
//!    standard encoding) through the AGL ([`crate::agl`]: `uniXXXX`, `uXXXX[XX]`, `a.sc`,
//!    `f_f_i`); composite fonts with a predefined `Uni…-UCS2-…`/`Uni…-UTF16-…` CMap: the code
//!    itself, which is UTF-16;
//! 3. an embedded TrueType/OpenType program's `cmap`, read backwards from the glyph;
//! 4. nothing: the caller extracts U+FFFD and raises `unmapped-glyph`.
//!
//! Character-collection CMaps (`Adobe-Japan1-UCS2` and the rest) are not bundled, and Type 1
//! programs are not parsed for their built-in encodings: such fonts need a ToUnicode CMap.

use std::collections::BTreeMap;
use std::sync::Arc;

use laminapdf_cos::{Document, Result, Warning, absent_if_malformed};
use laminapdf_syntax::{Dict, Object};
use skrifa::{FontRef, MetadataProvider};

use super::cmap::{CMap, Code};
use crate::{Encoding, Standard14, agl};

/// How many `usecmap` / `/UseCMap` links are followed from a font's CMap (TN #5014 allows
/// one; files nest a few). Further links are dropped with `cmap-invalid`.
pub const MAX_USECMAP_DEPTH: usize = 8;

/// The default vertical metrics of a CIDFont, `/DW2 [880 −1000]` (§9.7.4.3).
const DW2: (f64, f64) = (880.0, -1000.0);

/// The extent used when a font gives none (thousandths of text space): about what common
/// Latin fonts have.
const DEFAULT_EXTENT: (f64, f64) = (800.0, -200.0);

/// A font loaded for extraction.
#[derive(Debug)]
pub(crate) struct ExtractFont {
    kind: Kind,
    to_unicode: Option<CMap>,
    /// `/BaseFont` (the font's name), read lossily; shared by every glyph shown in the font.
    pub(crate) name: Arc<str>,
    /// Ascent and descent in thousandths of text space, for glyph boxes.
    pub(crate) extent: (f64, f64),
}

#[derive(Debug)]
enum Kind {
    Simple(Simple),
    Composite(Box<Composite>),
}

/// Type 1, `MMType1`, TrueType and Type 3 fonts: one byte per code.
#[derive(Debug)]
struct Simple {
    /// Glyph names by code, from the encoding.
    names: Vec<Option<Box<str>>>,
    /// Widths by code, in thousandths of text space (Type 3 widths already through
    /// `/FontMatrix`).
    widths: Vec<f64>,
    zapf: bool,
    /// Code → text from the embedded font's `cmap` (for codes the names don't map).
    program: BTreeMap<u8, char>,
    /// Ascent and descent, thousandths of text space.
    extent: (f64, f64),
}

/// Type 0 fonts: codes by CMap, CIDs, `/W`, `/W2`.
#[derive(Debug)]
struct Composite {
    encoding: CidEncoding,
    vertical: bool,
    widths: Widths<f64>,
    default_width: f64,
    vertical_widths: Widths<(f64, f64, f64)>,
    default_vertical: (f64, f64),
    /// CID → text from the embedded font's `cmap` through `/CIDToGIDMap`.
    program: BTreeMap<u32, char>,
    /// Ascent and descent, thousandths of text space.
    extent: (f64, f64),
}

#[derive(Debug)]
enum CidEncoding {
    /// `Identity-H`/`-V`: a two-byte code is the CID.
    Identity,
    /// A predefined `Uni…-UCS2-…` or `Uni…-UTF16-…` CMap: the code is UTF-16, the CID unknown.
    Utf16,
    /// An embedded CMap stream.
    Embedded(Box<CMap>),
}

/// `/W` or `/W2`: CID ranges with one value each, or a run of values from a first CID.
#[derive(Debug)]
struct Widths<T> {
    ranges: BTreeMap<u32, (u32, Run<T>)>,
}

#[derive(Debug)]
enum Run<T> {
    Same(T),
    Each(Vec<T>),
}

impl<T> Default for Widths<T> {
    fn default() -> Self {
        Self {
            ranges: BTreeMap::new(),
        }
    }
}

impl<T: Copy> Widths<T> {
    fn get(&self, cid: u32) -> Option<T> {
        let (&lo, (hi, run)) = self.ranges.range(..=cid).next_back()?;
        if cid > *hi {
            return None;
        }
        match run {
            Run::Same(value) => Some(*value),
            Run::Each(values) => values.get(usize::try_from(cid - lo).ok()?).copied(),
        }
    }
}

/// Problems met while loading, reported once each by the caller.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Issues {
    pub(crate) font: bool,
    pub(crate) cmap: bool,
}

/// Reads entries, resolving references; a malformed object reads as absent and is noted, and
/// any other error ends the load ([`absent_if_malformed`]).
struct Reader<'d> {
    doc: &'d Document,
    issues: Issues,
}

impl Reader<'_> {
    fn get(&mut self, dict: &Dict, key: &str) -> Result<Option<Arc<Object>>> {
        let Some(value) = dict.get(key) else { return Ok(None) };
        let resolved = absent_if_malformed(self.doc.deref(value))?;
        self.issues.font |= resolved.is_none();
        Ok(resolved.map(laminapdf_cos::Resolved::into_arc))
    }

    fn dict(&mut self, dict: &Dict, key: &str) -> Result<Option<Dict>> {
        Ok(self.get(dict, key)?.and_then(|o| match &*o {
            Object::Dict(d) => Some(d.clone()),
            Object::Stream(s) => Some(s.dict.clone()),
            _ => None,
        }))
    }

    fn number(&mut self, dict: &Dict, key: &str) -> Result<Option<f64>> {
        Ok(self.get(dict, key)?.and_then(|o| o.as_f64()))
    }

    /// A stream's decoded bytes; a stream that can't be decoded reads as absent.
    fn stream(&mut self, object: &Object) -> Result<Option<Vec<u8>>> {
        let Object::Stream(stream) = object else {
            return Ok(None);
        };
        let bytes = absent_if_malformed(self.doc.decode_stream(stream))?;
        self.issues.font |= bytes.is_none();
        Ok(bytes)
    }

    /// The numbers of an array (non-numbers dropped, and noted; a fatal error while
    /// resolving an element ends the load).
    fn numbers(&mut self, object: &Object) -> Result<Vec<f64>> {
        let Some(items) = object.as_array() else {
            self.issues.font = true;
            return Ok(Vec::new());
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let value = absent_if_malformed(self.doc.deref(item))?.and_then(|resolved| resolved.as_f64());
            match value {
                Some(n) if n.is_finite() => out.push(n),
                _ => self.issues.font = true,
            }
        }
        Ok(out)
    }

    /// The embedded font program of a descriptor, if TrueType or OpenType (skrifa reads
    /// those; bare CFF and Type 1 programs are not read).
    fn program(&mut self, descriptor: Option<&Dict>) -> Result<Option<Vec<u8>>> {
        let Some(descriptor) = descriptor else { return Ok(None) };
        for key in ["FontFile2", "FontFile3"] {
            if let Some(object) = self.get(descriptor, key)? {
                let open_type = key == "FontFile2"
                    || matches!(&*object, Object::Stream(s) if s.dict.get("Subtype").is_some_and(|n| matches!(n, Object::Name(n) if n.0 == b"OpenType")));
                if open_type {
                    return self.stream(&object);
                }
            }
        }
        Ok(None)
    }
}

/// A non-negative whole number that fits a `u32` (CIDs, codes, flags), else `None`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "checked: finite, whole, within 0..=u32::MAX"
)]
fn whole(n: f64) -> Option<u32> {
    (n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX)).then_some(n as u32)
}

fn name_of(object: Option<&Object>) -> Option<&[u8]> {
    match object {
        Some(Object::Name(name)) => Some(&name.0),
        _ => None,
    }
}

/// The text each glyph of an sfnt program stands for, from its Unicode `cmap` (lowest code
/// point per glyph; private-use symbol codes `U+F0xx` are left out).
fn glyph_text(program: &[u8]) -> BTreeMap<u32, char> {
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
fn standard_font(base_font: &[u8]) -> Option<Standard14> {
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

impl ExtractFont {
    /// Loads the font dictionary `dict`. Never fails on a malformed font (it decodes with
    /// defaults and returns [`Issues`]); fails only on hard limits and in strict mode.
    pub(crate) fn load(doc: &Document, dict: &Dict) -> Result<(Self, Issues)> {
        let mut reader = Reader {
            doc,
            issues: Issues::default(),
        };
        let to_unicode = match reader.get(dict, "ToUnicode")? {
            Some(object) => match reader.stream(&object)? {
                Some(bytes) => {
                    let cmap = CMap::parse(&bytes);
                    reader.issues.cmap |= cmap.skipped() > 0;
                    Some(cmap)
                }
                // A name here (`/Identity-H`, written by some producers) maps nothing.
                None => None,
            },
            None => None,
        };
        let composite = name_of(dict.get("Subtype")) == Some(b"Type0");
        let kind = if composite {
            Kind::Composite(Box::new(Composite::load(&mut reader, dict)?))
        } else {
            Kind::Simple(Simple::load(&mut reader, dict)?)
        };
        let extent = match &kind {
            Kind::Simple(s) => s.extent,
            Kind::Composite(c) => c.extent,
        };
        let name = Arc::from(String::from_utf8_lossy(
            name_of(dict.get("BaseFont")).unwrap_or_default(),
        ));
        Ok((
            Self {
                kind,
                to_unicode,
                name,
                extent,
            },
            reader.issues,
        ))
    }

    /// The position vector `(vx, vy)` of a glyph in vertical writing, thousandths of text
    /// space (§9.7.4.3): from `/W2`, else half the horizontal width and `/DW2`'s first number.
    pub(crate) fn position_vector(&self, code: Code) -> (f64, f64) {
        match &self.kind {
            Kind::Simple(_) => (0.0, 0.0),
            Kind::Composite(c) => {
                let cid = c.cid(code);
                c.vertical_widths.get(cid).map_or_else(
                    || (c.widths.get(cid).unwrap_or(c.default_width) / 2.0, c.default_vertical.0),
                    |(_, vx, vy)| (vx, vy),
                )
            }
        }
    }

    /// Whether the font writes vertically (`/WMode 1`, `Identity-V`).
    pub(crate) fn vertical(&self) -> bool {
        matches!(&self.kind, Kind::Composite(c) if c.vertical)
    }

    /// The first code of `bytes` and how many bytes it took.
    pub(crate) fn next_code(&self, bytes: &[u8]) -> Option<(Code, usize)> {
        match &self.kind {
            Kind::Simple(_) => {
                let value = u32::from(*bytes.first()?);
                Some((Code { value, len: 1 }, 1))
            }
            Kind::Composite(c) => match &c.encoding {
                CidEncoding::Identity => fixed(bytes, 2),
                CidEncoding::Utf16 => {
                    let high = bytes.first().is_some_and(|b| (0xD8..=0xDB).contains(b));
                    fixed(bytes, if high && bytes.len() >= 4 { 4 } else { 2 })
                }
                CidEncoding::Embedded(cmap) => cmap.next_code(bytes, 2),
            },
        }
    }

    /// The text `code` stands for (see the module docs for the order); `None` when nothing
    /// maps it. An empty string is a mapping too: a glyph that stands for no text.
    pub(crate) fn unicode(&self, code: Code) -> Option<String> {
        if let Some(text) = self.to_unicode.as_ref().and_then(|cmap| cmap.unicode(code)) {
            return Some(text);
        }
        match &self.kind {
            Kind::Simple(s) => s.unicode(code),
            Kind::Composite(c) => c.unicode(code),
        }
    }

    /// The glyph's displacement `(w0, w1)` in thousandths of text space (§9.4.4): `w0` for
    /// horizontal writing, `w1` for vertical.
    pub(crate) fn displacement(&self, code: Code) -> (f64, f64) {
        match &self.kind {
            Kind::Simple(s) => (
                usize::try_from(code.value)
                    .ok()
                    .and_then(|i| s.widths.get(i))
                    .copied()
                    .unwrap_or(0.0),
                0.0,
            ),
            Kind::Composite(c) => {
                let cid = c.cid(code);
                let w0 = c.widths.get(cid).unwrap_or(c.default_width);
                let w1 = c.vertical_widths.get(cid).map_or(c.default_vertical.1, |(w1, _, _)| w1);
                (w0, w1)
            }
        }
    }

    /// Whether `Tw` applies to `code`: the single-byte code 32 (§9.3.3).
    pub(crate) const fn is_word_space(code: Code) -> bool {
        code.len == 1 && code.value == 32
    }
}

fn fixed(bytes: &[u8], len: usize) -> Option<(Code, usize)> {
    let len = len.min(bytes.len());
    Some((super::cmap::code_of(bytes.get(..len)?)?, len))
}

impl Simple {
    fn load(reader: &mut Reader<'_>, dict: &Dict) -> Result<Self> {
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

    fn widths(
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

    fn unicode(&self, code: Code) -> Option<String> {
        let index = usize::try_from(code.value).ok()?;
        if let Some(name) = self.names.get(index).and_then(Option::as_deref) {
            let text = if self.zapf {
                agl::zapf_dingbats_to_unicode(name)
            } else {
                agl::to_unicode(name)
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

impl Composite {
    fn load(reader: &mut Reader<'_>, dict: &Dict) -> Result<Self> {
        let (encoding, vertical) = match reader.get(dict, "Encoding")?.as_deref() {
            Some(Object::Name(name)) => predefined(reader, &name.0),
            Some(stream @ Object::Stream(_)) => {
                let cmap = embedded_cmap(reader, stream, 0)?;
                let vertical = cmap.vertical();
                (CidEncoding::Embedded(Box::new(cmap)), vertical)
            }
            _ => {
                reader.issues.font = true;
                (CidEncoding::Identity, false)
            }
        };
        let Some(cid_font) = descendant(reader, dict)? else {
            reader.issues.font = true;
            return Ok(Self {
                encoding,
                vertical,
                widths: Widths {
                    ranges: BTreeMap::new(),
                },
                default_width: 1000.0,
                vertical_widths: Widths {
                    ranges: BTreeMap::new(),
                },
                default_vertical: DW2,
                program: BTreeMap::new(),
                extent: DEFAULT_EXTENT,
            });
        };
        let default_width = reader.number(&cid_font, "DW")?.unwrap_or(1000.0);
        let widths = match reader.get(&cid_font, "W")? {
            Some(w) => cid_widths(reader, &w, 1, |v| v.first().copied())?,
            None => Widths {
                ranges: BTreeMap::new(),
            },
        };
        let default_vertical = match reader.get(&cid_font, "DW2")? {
            Some(dw2) => match reader.numbers(&dw2)?.as_slice() {
                [vy, w1] => (*vy, *w1),
                _ => DW2,
            },
            None => DW2,
        };
        let vertical_widths = match reader.get(&cid_font, "W2")? {
            Some(w2) => cid_widths(reader, &w2, 3, |v| match v {
                [w1, vx, vy] => Some((*w1, *vx, *vy)),
                _ => None,
            })?,
            None => Widths {
                ranges: BTreeMap::new(),
            },
        };
        let descriptor = reader.dict(&cid_font, "FontDescriptor")?;
        // Only a CIDFontType2's program is read: its `/CIDToGIDMap` (default identity) gives
        // the glyph of a CID. A CIDFontType0's CFF maps CIDs through its charset, which is
        // not read, so its glyphs need a ToUnicode CMap.
        let truetype = name_of(cid_font.get("Subtype")) == Some(b"CIDFontType2");
        let program = match reader.program(descriptor.as_ref().filter(|_| truetype))? {
            Some(bytes) => {
                let by_glyph = glyph_text(&bytes);
                let map = match reader.get(&cid_font, "CIDToGIDMap")? {
                    Some(object) if matches!(&*object, Object::Stream(_)) => reader.stream(&object)?,
                    _ => None,
                };
                match map {
                    Some(map) => map
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .enumerate()
                        .filter_map(|(cid, pair)| {
                            let gid = u32::from(u16::from_be_bytes(*pair));
                            Some((u32::try_from(cid).ok()?, *by_glyph.get(&gid)?))
                        })
                        .collect(),
                    None => by_glyph,
                }
            }
            None => BTreeMap::new(),
        };
        let extent = extent(reader, &cid_font, descriptor.as_ref(), None)?;
        Ok(Self {
            encoding,
            vertical,
            widths,
            default_width,
            vertical_widths,
            default_vertical,
            program,
            extent,
        })
    }

    fn cid(&self, code: Code) -> u32 {
        match &self.encoding {
            CidEncoding::Identity => code.value,
            // The CID of a UTF-16 CMap needs the collection's tables, not bundled: CID 0
            // (default widths).
            CidEncoding::Utf16 => 0,
            CidEncoding::Embedded(cmap) => cmap.cid(code).unwrap_or(0),
        }
    }

    fn unicode(&self, code: Code) -> Option<String> {
        if matches!(self.encoding, CidEncoding::Utf16) {
            let units: Vec<u16> = if code.len == 4 {
                vec![(code.value >> 16) as u16, (code.value & 0xFFFF) as u16]
            } else {
                vec![u16::try_from(code.value).ok()?]
            };
            return Some(super::cmap::utf16(&units));
        }
        self.program.get(&self.cid(code)).map(char::to_string)
    }
}

/// A Type 0 font's descendant CIDFont dictionary (`/DescendantFonts`' first element).
fn descendant(reader: &mut Reader<'_>, dict: &Dict) -> Result<Option<Dict>> {
    let Some(list) = reader.get(dict, "DescendantFonts")? else {
        return Ok(None);
    };
    let Some(first) = list.as_array().and_then(<[Object]>::first) else {
        return Ok(None);
    };
    Ok(absent_if_malformed(reader.doc.deref(first))?.and_then(|object| object.as_dict().cloned()))
}

/// A font's ascent and descent in thousandths of text space, for glyph boxes: Type 3's
/// `/FontBBox` through `/FontMatrix` (from `dict`); else the descriptor's `/Ascent` and
/// `/Descent`, else its `/FontBBox`; else a standard 14 font's AFM values; else
/// [`DEFAULT_EXTENT`]. A positive descent (a sign error some producers make) counts as
/// negative. (pdf.js and PDFium differ here; B9c's quad tests set the tolerance.)
fn extent(
    reader: &mut Reader<'_>,
    dict: &Dict,
    descriptor: Option<&Dict>,
    standard: Option<Standard14>,
) -> Result<(f64, f64)> {
    let valid = |ascent: f64, descent: f64| {
        let descent = -descent.abs();
        (ascent > 0.0 && ascent.is_finite() && descent.is_finite()).then_some((ascent, descent))
    };
    let bbox = |numbers: &[f64]| match *numbers {
        [_, y0, _, y1] => valid(y1.max(y0), y0.min(y1).min(0.0)),
        _ => None,
    };
    if name_of(dict.get("Subtype")) == Some(b"Type3") {
        let matrix = match reader.get(dict, "FontMatrix")? {
            Some(m) => reader.numbers(&m)?,
            None => Vec::new(),
        };
        let scale = matrix.get(3).copied().unwrap_or(0.001) * 1000.0;
        let numbers = match reader.get(dict, "FontBBox")? {
            Some(b) => reader.numbers(&b)?,
            None => Vec::new(),
        };
        let scaled: Vec<f64> = numbers.iter().map(|n| n * scale).collect();
        return Ok(bbox(&scaled).unwrap_or(DEFAULT_EXTENT));
    }
    if let Some(descriptor) = descriptor {
        let ascent = reader.number(descriptor, "Ascent")?;
        let descent = reader.number(descriptor, "Descent")?;
        if let Some(found) = ascent.zip(descent).and_then(|(a, d)| valid(a, d)) {
            return Ok(found);
        }
        if let Some(b) = reader.get(descriptor, "FontBBox")?
            && let Some(found) = bbox(&reader.numbers(&b)?)
        {
            return Ok(found);
        }
    }
    if let Some(font) = standard {
        let m = font.metrics();
        let afm = m.ascender.zip(m.descender).map(|(a, d)| (f64::from(a), f64::from(d)));
        let from_bbox = bbox(&m.bbox.map(f64::from));
        if let Some(found) = afm.and_then(|(a, d)| valid(a, d)).or(from_bbox) {
            return Ok(found);
        }
    }
    Ok(DEFAULT_EXTENT)
}

/// A predefined CMap by name (§9.7.5.2, Table 116): Identity-H/V, and the UCS-2/UTF-16 ones
/// whose codes are Unicode. Others need Adobe's CMap files, not bundled: `cmap-invalid`, read
/// as Identity.
fn predefined(reader: &mut Reader<'_>, name: &[u8]) -> (CidEncoding, bool) {
    let vertical = name.ends_with(b"-V");
    match name {
        b"Identity-H" | b"Identity-V" => (CidEncoding::Identity, vertical),
        _ if name.starts_with(b"Uni") && (contains(name, b"-UCS2-") || contains(name, b"-UTF16-")) => {
            (CidEncoding::Utf16, vertical)
        }
        _ => {
            reader.issues.cmap = true;
            (CidEncoding::Identity, vertical)
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// An embedded CMap stream with its `/UseCMap` (a stream, or a `usecmap` name inside), to
/// [`MAX_USECMAP_DEPTH`].
fn embedded_cmap(reader: &mut Reader<'_>, object: &Object, depth: usize) -> Result<CMap> {
    let Some(bytes) = reader.stream(object)? else {
        return Ok(CMap::default());
    };
    let mut cmap = CMap::parse(&bytes);
    reader.issues.cmap |= cmap.skipped() > 0;
    let Object::Stream(stream) = object else {
        return Ok(cmap);
    };
    // The dictionary's `/WMode` (Table 118) where the source states none.
    if let Some(wmode) = reader.number(&stream.dict, "WMode")? {
        cmap.default_vertical(whole(wmode) == Some(1));
    }
    // `/UseCMap` in the dictionary (a stream or a predefined name), else `usecmap` in the
    // source (a name).
    let parent = match reader.get(&stream.dict, "UseCMap")? {
        Some(parent) => Some(parent),
        None => cmap
            .use_cmap()
            .map(|name| Arc::new(Object::Name(laminapdf_syntax::Name::new(name.to_vec())))),
    };
    match parent.as_deref() {
        Some(parent @ Object::Stream(_)) => {
            if depth >= MAX_USECMAP_DEPTH {
                reader.issues.cmap = true;
            } else {
                let parent = embedded_cmap(reader, parent, depth + 1)?;
                cmap.inherit(&parent);
            }
        }
        // A predefined parent: Identity adds the two-byte codespace and identity CIDs; the
        // others are not bundled.
        Some(Object::Name(name)) if name.0 == b"Identity-H" || name.0 == b"Identity-V" => {
            cmap.inherit(&CMap::parse(
                b"1 begincodespacerange <0000> <FFFF> endcodespacerange 1 begincidrange <0000> <FFFF> 0 endcidrange",
            ));
            cmap.default_vertical(name.0 == b"Identity-V");
        }
        Some(_) => reader.issues.cmap = true,
        None => {}
    }
    Ok(cmap)
}

/// `/W` (`width` = 1 value per CID) or `/W2` (3): `c [v …]` runs and `c_first c_last v…` ranges
/// (§9.7.4.3).
fn cid_widths<T: Copy>(
    reader: &mut Reader<'_>,
    object: &Object,
    width: usize,
    value: impl Fn(&[f64]) -> Option<T>,
) -> Result<Widths<T>> {
    let mut ranges = BTreeMap::new();
    let Some(items) = object.as_array() else {
        reader.issues.font = true;
        return Ok(Widths::default());
    };
    let mut i = 0;
    let int = |o: Option<&Object>| o.and_then(Object::as_f64).and_then(whole);
    while let Some(first) = int(items.get(i)) {
        if let Some(list @ (Object::Array(_) | Object::Ref(_))) = items.get(i + 1) {
            // `c [v …]`: a run of values from `c`.
            let list = match absent_if_malformed(reader.doc.deref(list))? {
                Some(resolved) => reader.numbers(&resolved)?,
                None => Vec::new(),
            };
            let values: Vec<T> = list.chunks_exact(width).filter_map(&value).collect();
            if let Some(count) = u32::try_from(values.len()).ok().filter(|n| *n > 0) {
                ranges.insert(first, (first.saturating_add(count - 1), Run::Each(values)));
            }
            i += 2;
        } else {
            // `c_first c_last v`: one value for the range.
            let Some(last) = int(items.get(i + 1)) else { break };
            let numbers: Vec<f64> = (0..width)
                .filter_map(|k| items.get(i + 2 + k).and_then(Object::as_f64))
                .collect();
            if let Some(v) = value(&numbers).filter(|_| numbers.len() == width && last >= first) {
                ranges.insert(first, (last, Run::Same(v)));
            } else {
                reader.issues.font = true;
                break;
            }
            i += 2 + width;
        }
    }
    if i < items.len() {
        reader.issues.font = true;
    }
    Ok(Widths { ranges })
}

/// Fonts already loaded in one document, by font object: each is parsed once.
#[derive(Debug, Default)]
pub(crate) struct FontCache {
    fonts: BTreeMap<laminapdf_syntax::ObjRef, Arc<ExtractFont>>,
    /// Direct font dictionaries (no object to key by), compared by value: their streams are
    /// indirect, so a comparison is cheap. At most [`DIRECT_FONTS`].
    direct: Vec<(Dict, Arc<ExtractFont>)>,
}

/// Direct font dictionaries cached per extractor; further ones are loaded on each use.
const DIRECT_FONTS: usize = 64;

impl FontCache {
    /// The font of `dict` (object `object`, when indirect), loading it on first use and
    /// reporting its issues once, on the document.
    pub(crate) fn get(
        &mut self,
        doc: &Document,
        object: Option<laminapdf_syntax::ObjRef>,
        dict: &Dict,
    ) -> Result<Arc<ExtractFont>> {
        if let Some(font) = object.and_then(|r| self.fonts.get(&r)) {
            return Ok(Arc::clone(font));
        }
        if object.is_none()
            && let Some((_, font)) = self.direct.iter().find(|(d, _)| d == dict)
        {
            return Ok(Arc::clone(font));
        }
        let (font, issues) = ExtractFont::load(doc, dict)?;
        let num = object.map(|r| r.num);
        if issues.font {
            doc.report(Warning::FontInvalid { object: num })?;
        }
        if issues.cmap {
            doc.report(Warning::CMapInvalid { object: num })?;
        }
        let font = Arc::new(font);
        match object {
            Some(r) => {
                self.fonts.insert(r, Arc::clone(&font));
            }
            None if self.direct.len() < DIRECT_FONTS => self.direct.push((dict.clone(), Arc::clone(&font))),
            None => {}
        }
        Ok(font)
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::{Document, OpenOptions};
    use laminapdf_syntax::{Dict, Name, Object, Stream, parse_object};

    use super::{Code, ExtractFont, Issues, MAX_USECMAP_DEPTH, standard_font};
    use crate::Standard14;

    fn dict(source: &str) -> Dict {
        match parse_object(source.as_bytes()) {
            Ok(Object::Dict(d)) => d,
            other => panic!("not a dictionary: {other:?}"),
        }
    }

    fn stream(dict_source: &str, data: &[u8]) -> Object {
        Object::Stream(Stream::new(dict(dict_source), data.to_vec()))
    }

    fn load(font: &Dict) -> (ExtractFont, Issues) {
        let doc = Document::empty(None, &OpenOptions::default());
        ExtractFont::load(&doc, font).expect("loads")
    }

    fn text(font: &ExtractFont, bytes: &[u8]) -> Vec<Option<String>> {
        let mut rest = bytes;
        let mut out = Vec::new();
        while let Some((code, used)) = font.next_code(rest) {
            out.push(font.unicode(code));
            rest = &rest[used..];
        }
        out
    }

    const fn byte(value: u32) -> Code {
        Code { value, len: 1 }
    }

    const fn two(value: u32) -> Code {
        Code { value, len: 2 }
    }

    fn type0(encoding: &str, descendant: &str) -> Dict {
        dict(&format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /F /Encoding {encoding} /DescendantFonts [{descendant}] >>"
        ))
    }

    #[test]
    fn differences_override_the_base_encoding_and_go_through_the_agl() {
        let (font, issues) = load(&dict(
            "<< /Subtype /Type1 /BaseFont /Helvetica /Encoding << /BaseEncoding /WinAnsiEncoding
               /Differences [128 /Lslash /uni0105 39 /quoteright] >> >>",
        ));
        assert_eq!(
            (text(&font, &[0x41, 0x80, 0x81, 0x27, 0xE9]), issues.font),
            (
                vec![
                    Some("A".into()),
                    Some("Ł".into()),
                    Some("ą".into()),
                    Some("’".into()),
                    Some("é".into())
                ],
                false
            )
        );
    }

    #[test]
    fn standard_14_fonts_without_widths_use_the_afm_metrics() {
        let (font, _) = load(&dict(
            "<< /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        ));
        assert_eq!(
            [font.displacement(byte(0x57)), font.displacement(byte(0x20))],
            [(944.0, 0.0), (278.0, 0.0)]
        );
    }

    #[test]
    fn widths_cover_their_codes_and_missing_width_the_rest() {
        let mut font = dict(
            "<< /Subtype /TrueType /BaseFont /Arial /FirstChar 65 /Widths [500 600]
               /FontDescriptor << /Flags 32 /MissingWidth 250 >> >>",
        );
        font.insert("Encoding", Object::Name(Name::from("WinAnsiEncoding")));
        let (font, _) = load(&font);
        assert_eq!(
            [byte(65), byte(66), byte(0x57)].map(|c| font.displacement(c).0),
            [500.0, 600.0, 250.0]
        );
    }

    #[test]
    fn a_nonsymbolic_font_without_encoding_reads_standard_encoding() {
        let (font, _) = load(&dict("<< /Subtype /Type1 /BaseFont /Unknown >>"));
        assert_eq!(text(&font, &[0x27, 0x41]), [Some("’".into()), Some("A".into())]);
    }

    #[test]
    fn arbitrary_glyph_names_map_to_nothing() {
        let (font, _) = load(&dict(
            "<< /Subtype /Type1 /BaseFont /X /Encoding << /Differences [1 /g37] >> >>",
        ));
        assert_eq!(text(&font, &[1]), [None]);
    }

    #[test]
    fn zapf_dingbats_names_use_their_own_list() {
        let (font, _) = load(&dict("<< /Subtype /Type1 /BaseFont /ZapfDingbats >>"));
        assert_eq!(text(&font, &[0x34]), [Some("✔".into())]);
    }

    #[test]
    fn type3_widths_go_through_the_font_matrix() {
        let (font, _) = load(&dict(
            "<< /Subtype /Type3 /FontMatrix [0.01 0 0 0.01 0 0] /FirstChar 1 /Widths [50]
               /Encoding << /Differences [1 /a] >> >>",
        ));
        assert_eq!(
            (font.displacement(byte(1)), text(&font, &[1])),
            ((500.0, 0.0), vec![Some("a".into())])
        );
    }

    #[test]
    fn to_unicode_wins_over_the_encoding() {
        let mut font = dict("<< /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
        font.insert("ToUnicode", stream("<< >>", b"1 beginbfchar <41> <0042> endbfchar"));
        let (font, _) = load(&font);
        assert_eq!(text(&font, b"AC"), [Some("B".into()), Some("C".into())]);
    }

    #[test]
    fn identity_h_widths_come_from_w_runs_ranges_and_dw() {
        let (font, issues) = load(&type0(
            "/Identity-H",
            "<< /Subtype /CIDFontType2 /DW 400 /W [1 [500 600] 10 20 700] >>",
        ));
        assert_eq!(
            (
                [1, 2, 15, 30].map(|cid| font.displacement(two(cid)).0),
                font.vertical(),
                issues.font
            ),
            ([500.0, 600.0, 700.0, 400.0], false, false)
        );
    }

    #[test]
    fn identity_v_writes_vertically_with_w2_and_dw2() {
        let (font, _) = load(&type0(
            "/Identity-V",
            "<< /Subtype /CIDFontType0 /W2 [5 [-900 500 800]] /DW2 [800 -1000] >>",
        ));
        assert_eq!(
            (
                font.vertical(),
                font.displacement(two(5)).1,
                font.displacement(two(6)).1
            ),
            (true, -900.0, -1000.0)
        );
    }

    #[test]
    fn ucs2_cmaps_read_codes_as_utf16() {
        let (font, issues) = load(&type0("/UniGB-UCS2-H", "<< /Subtype /CIDFontType0 >>"));
        assert_eq!(
            (text(&font, &[0x4F, 0x60, 0x59, 0x7D]), issues.cmap),
            (vec![Some("你".into()), Some("好".into())], false)
        );
    }

    #[test]
    fn utf16_cmaps_read_surrogate_pairs_as_one_code() {
        let (font, _) = load(&type0("/UniJIS-UTF16-H", "<< /Subtype /CIDFontType0 >>"));
        assert_eq!(text(&font, &[0xD8, 0x40, 0xDC, 0x0B]), [Some("\u{2000B}".into())]);
    }

    #[test]
    fn other_predefined_cmaps_are_an_issue_and_read_as_identity() {
        let (font, issues) = load(&type0("/90ms-RKSJ-H", "<< /Subtype /CIDFontType0 >>"));
        assert_eq!(
            (issues.cmap, font.next_code(&[1, 2, 3]).map(|(c, n)| (c.value, n))),
            (true, Some((0x0102, 2)))
        );
    }

    #[test]
    fn an_embedded_cmap_splits_codes_and_maps_cids() {
        let mut font = type0("null", "<< /Subtype /CIDFontType2 /W [500 [250]] >>");
        font.insert(
            "Encoding",
            stream(
                "<< /Type /CMap >>",
                b"/WMode 1 def 2 begincodespacerange <00> <7F> <8000> <FFFF> endcodespacerange
                  1 begincidrange <8000> <80FF> 400 endcidrange",
            ),
        );
        let (font, _) = load(&font);
        let (code, used) = font.next_code(&[0x80, 0x64]).expect("a code");
        assert_eq!(
            (code, used, font.displacement(code).0, font.vertical()),
            (two(0x8064), 2, 250.0, true)
        );
    }

    /// A Type 0 font whose CMap has a `/UseCMap` chain `depth` streams deep below it.
    fn chained(depth: usize) -> Dict {
        let mut parent = stream("<< >>", b"1 begincidchar <0001> 77 endcidchar");
        for _ in 0..depth {
            let Object::Stream(mut s) = stream("<< >>", b"1 begincodespacerange <0000> <FFFF> endcodespacerange")
            else {
                unreachable!()
            };
            s.dict.insert("UseCMap", parent);
            parent = Object::Stream(s);
        }
        let mut font = type0("null", "<< /Subtype /CIDFontType2 /W [77 [123]] >>");
        font.insert("Encoding", parent);
        font
    }

    #[test]
    fn usecmap_chains_are_followed_to_the_depth_limit() {
        let (font, issues) = load(&chained(MAX_USECMAP_DEPTH));
        assert_eq!((font.displacement(two(1)).0, issues.cmap), (123.0, false));
    }

    #[test]
    fn usecmap_chains_past_the_depth_limit_are_cut() {
        let (font, issues) = load(&chained(MAX_USECMAP_DEPTH + 1));
        assert_eq!((font.displacement(two(1)).0, issues.cmap), (1000.0, true));
    }

    #[test]
    fn a_predefined_use_cmap_in_the_dictionary_is_inherited_with_its_wmode() {
        let mut font = type0("null", "<< /Subtype /CIDFontType2 /W [9 [300]] >>");
        font.insert(
            "Encoding",
            stream(
                "<< /Type /CMap /UseCMap /Identity-V >>",
                b"1 begincidchar <0001> 9 endcidchar",
            ),
        );
        let (font, issues) = load(&font);
        assert_eq!(
            (
                font.displacement(two(1)).0,
                font.displacement(two(9)).0,
                font.vertical(),
                issues.cmap
            ),
            (300.0, 300.0, true, false)
        );
    }

    #[test]
    fn the_cmap_dictionarys_wmode_applies_when_the_source_has_none() {
        let mut font = type0("null", "<< /Subtype /CIDFontType2 >>");
        font.insert(
            "Encoding",
            stream(
                "<< /Type /CMap /WMode 1 >>",
                b"1 begincodespacerange <0000> <FFFF> endcodespacerange",
            ),
        );
        assert!(load(&font).0.vertical());
    }

    #[test]
    fn a_malformed_w_array_is_an_issue_and_keeps_what_parsed() {
        let (font, issues) = load(&type0("/Identity-H", "<< /Subtype /CIDFontType2 /W [1 [500] 2 /x] >>"));
        assert_eq!((font.displacement(two(1)).0, issues.font), (500.0, true));
    }

    #[test]
    fn a_type0_font_without_descendant_is_an_issue_with_default_widths() {
        let (font, issues) = load(&dict("<< /Subtype /Type0 /Encoding /Identity-H >>"));
        assert_eq!((font.displacement(two(9)).0, issues.font), (1000.0, true));
    }

    #[test]
    fn a_malformed_to_unicode_is_an_issue_and_keeps_its_good_entries() {
        let mut font = dict("<< /Subtype /Type1 /BaseFont /Helvetica >>");
        font.insert(
            "ToUnicode",
            stream("<< >>", b"2 beginbfchar <41> <0042> <42> 7 endbfchar"),
        );
        let (font, issues) = load(&font);
        assert_eq!((text(&font, b"A"), issues.cmap), (vec![Some("B".into())], true));
    }

    #[test]
    fn extents_come_from_the_descriptor_then_its_bbox_then_the_afm() {
        let ascent = |source: &str| load(&dict(source)).0.extent;
        assert_eq!(
            [
                ascent("<< /Subtype /TrueType /BaseFont /X /FontDescriptor << /Ascent 900 /Descent -100 >> >>"),
                ascent(
                    "<< /Subtype /TrueType /BaseFont /X /FontDescriptor << /Ascent 0 /Descent 0 /FontBBox [0 -300 1000 950] >> >>"
                ),
                ascent("<< /Subtype /Type1 /BaseFont /Helvetica >>"),
                ascent("<< /Subtype /Type1 /BaseFont /Unknown >>"),
                ascent("<< /Subtype /Type3 /FontMatrix [0.01 0 0 0.01 0 0] /FontBBox [0 -10 100 90] >>"),
            ],
            [
                (900.0, -100.0),
                (950.0, -300.0),
                (718.0, -207.0),
                (800.0, -200.0),
                (900.0, -100.0)
            ]
        );
    }

    #[test]
    fn a_positive_descent_counts_as_negative() {
        let (font, _) = load(&dict(
            "<< /Subtype /TrueType /BaseFont /X /FontDescriptor << /Ascent 900 /Descent 200 >> >>",
        ));
        assert_eq!(font.extent, (900.0, -200.0));
    }

    #[test]
    fn a_type0_extent_comes_from_its_descendants_descriptor() {
        let (font, _) = load(&type0(
            "/Identity-H",
            "<< /Subtype /CIDFontType2 /FontDescriptor << /Ascent 1069 /Descent -293 >> >>",
        ));
        assert_eq!((font.extent, &*font.name), ((1069.0, -293.0), "F"));
    }

    #[test]
    fn position_vectors_come_from_w2_else_half_the_width_and_dw2() {
        let (font, _) = load(&type0(
            "/Identity-V",
            "<< /Subtype /CIDFontType0 /W [6 [600]] /W2 [5 [-900 500 800]] /DW2 [870 -1000] >>",
        ));
        assert_eq!(
            [font.position_vector(two(5)), font.position_vector(two(6))],
            [(500.0, 800.0), (300.0, 870.0)]
        );
    }

    #[test]
    fn word_spacing_applies_to_the_single_byte_code_32_only() {
        assert_eq!(
            [
                ExtractFont::is_word_space(byte(32)),
                ExtractFont::is_word_space(two(32))
            ],
            [true, false]
        );
    }

    #[test]
    fn standard_names_aliases_and_subset_tags_resolve() {
        assert_eq!(
            [
                standard_font(b"Helvetica-Bold"),
                standard_font(b"ABCDEF+Times-Roman"),
                standard_font(b"Arial,BoldItalic"),
                standard_font(b"TimesNewRomanPSMT"),
                standard_font(b"NotoSans"),
            ],
            [
                Some(Standard14::HelveticaBold),
                Some(Standard14::TimesRoman),
                Some(Standard14::HelveticaBoldOblique),
                Some(Standard14::TimesRoman),
                None
            ]
        );
    }
}
