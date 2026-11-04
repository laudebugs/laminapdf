//! Composite (Type 0) fonts: the descendant CIDFont, the CMap (predefined or embedded) and the `/W`
//! and `/W2` widths.

use std::collections::BTreeMap;
use std::sync::Arc;

use laminapdf_cos::{Result, absent_if_malformed};
use laminapdf_syntax::{Dict, Object};

use super::{DEFAULT_EXTENT, DW2, MAX_USECMAP_DEPTH, Reader, Run, Widths, extent, glyph_text, name_of, whole};
use crate::extract::cmap::{CMap, Code};

/// Type 0 fonts: codes by CMap, CIDs, `/W`, `/W2`.
#[derive(Debug)]
pub(super) struct Composite {
    pub(super) encoding: CidEncoding,
    pub(super) vertical: bool,
    pub(super) widths: Widths<f64>,
    pub(super) default_width: f64,
    pub(super) vertical_widths: Widths<(f64, f64, f64)>,
    pub(super) default_vertical: (f64, f64),
    /// CID → text from the embedded font's `cmap` through `/CIDToGIDMap`.
    pub(super) program: BTreeMap<u32, char>,
    /// Ascent and descent, thousandths of text space.
    pub(super) extent: (f64, f64),
}

#[derive(Debug)]
pub(super) enum CidEncoding {
    /// `Identity-H`/`-V`: a two-byte code is the CID.
    Identity,
    /// A predefined `Uni…-UCS2-…` or `Uni…-UTF16-…` CMap: the code is UTF-16, the CID unknown.
    Utf16,
    /// An embedded CMap stream.
    Embedded(Box<CMap>),
}

impl Composite {
    pub(super) fn load(reader: &mut Reader<'_>, dict: &Dict) -> Result<Self> {
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

    pub(super) fn cid(&self, code: Code) -> u32 {
        match &self.encoding {
            CidEncoding::Identity => code.value,
            // The CID of a UTF-16 CMap needs the collection's tables, not bundled: CID 0
            // (default widths).
            CidEncoding::Utf16 => 0,
            CidEncoding::Embedded(cmap) => cmap.cid(code).unwrap_or(0),
        }
    }

    pub(super) fn unicode(&self, code: Code) -> Option<String> {
        if matches!(self.encoding, CidEncoding::Utf16) {
            let units: Vec<u16> = if code.len == 4 {
                vec![(code.value >> 16) as u16, (code.value & 0xFFFF) as u16]
            } else {
                vec![u16::try_from(code.value).ok()?]
            };
            return Some(crate::extract::cmap::utf16(&units));
        }
        self.program.get(&self.cid(code)).map(char::to_string)
    }
}

/// A Type 0 font's descendant CIDFont dictionary (`/DescendantFonts`' first element).
pub(super) fn descendant(reader: &mut Reader<'_>, dict: &Dict) -> Result<Option<Dict>> {
    let Some(list) = reader.get(dict, "DescendantFonts")? else {
        return Ok(None);
    };
    let Some(first) = list.as_array().and_then(<[Object]>::first) else {
        return Ok(None);
    };
    Ok(absent_if_malformed(reader.doc.deref(first))?.and_then(|object| object.as_dict().cloned()))
}

/// A predefined CMap by name (§9.7.5.2, Table 116): Identity-H/V, and the UCS-2/UTF-16 ones
/// whose codes are Unicode. Others need Adobe's CMap files, not bundled: `cmap-invalid`, read
/// as Identity.
pub(super) fn predefined(reader: &mut Reader<'_>, name: &[u8]) -> (CidEncoding, bool) {
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

pub(super) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// An embedded CMap stream with its `/UseCMap` (a stream, or a `usecmap` name inside), to
/// [`MAX_USECMAP_DEPTH`].
pub(super) fn embedded_cmap(reader: &mut Reader<'_>, object: &Object, depth: usize) -> Result<CMap> {
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
pub(super) fn cid_widths<T: Copy>(
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
