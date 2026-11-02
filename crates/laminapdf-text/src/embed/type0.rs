//! The Type 0 font, its descendant CIDFont, the font descriptor and the `/W` array (ISO
//! 32000-2 §9.7.1–9.7.4, §9.8; research 03 §3.5–3.7, §10.2).

use std::collections::BTreeMap;

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString, Stream};

use super::subset::{Kind, Milli, Program, round_i64};
use crate::descriptor::descriptor;
use crate::error::Result;
use crate::font::Font;
use crate::tounicode::{Codespace, to_unicode_cmap};
use crate::usage::FontUse;

/// A run of at least this many consecutive CIDs with equal widths is written as
/// `c_first c_last w` rather than inside a `c [w …]` array.
const MIN_RANGE: usize = 3;

/// What [`write_font`] writes for one font.
pub(crate) struct FontObjects<'a> {
    pub(crate) font: &'a Font,
    pub(crate) usage: &'a FontUse,
    pub(crate) program: &'a Program,
    /// The CIDFont's `/BaseFont`: the subset tag and PostScript name, `ABCDEF+Name`.
    pub(crate) base_font: &'a str,
    /// The reserved Type 0 font dictionary that content streams refer to.
    pub(crate) type0: ObjRef,
}

/// Writes the font program, descriptor, CIDFont and ToUnicode as new objects and fills the
/// reserved Type 0 dictionary.
pub(crate) fn write_font(cos: &mut CosMut<'_>, objects: &FontObjects<'_>) -> Result<()> {
    let FontObjects {
        font,
        usage,
        program,
        base_font,
        type0,
    } = *objects;
    let info = font.info();
    let base_font = Name::from(base_font);

    let mut file_dict = Dict::new();
    let file_key = match program.kind {
        Kind::TrueType => {
            file_dict.insert("Length1", i64::try_from(program.data().len()).unwrap_or(i64::MAX));
            "FontFile2"
        }
        // A bare CID-keyed CFF (`subsetter` converts name-keyed fonts), ISO 32000-2 §9.9.
        Kind::Cff(_) => {
            file_dict.insert("Subtype", Name::from("CIDFontType0C"));
            "FontFile3"
        }
        // A whole OpenType font with CFF outlines (§9.9, PDF 1.6).
        Kind::OpenType => {
            file_dict.insert("Subtype", Name::from("OpenType"));
            "FontFile3"
        }
    };
    let file = cos.create(Object::Stream(Stream::flate(file_dict, program.data())))?;

    let mut descriptor = descriptor(info, base_font.clone(), program.bbox);
    descriptor.insert("StemV", stem_v(info.weight));
    descriptor.insert(file_key, file);
    let descriptor = cos.create(Object::Dict(descriptor))?;

    let widths = &program.widths;
    let dw = default_width(widths);
    let mut cid_font = Dict::new();
    cid_font.insert("Type", Name::from("Font"));
    cid_font.insert(
        "Subtype",
        Name::from(if program.is_cff() {
            "CIDFontType0"
        } else {
            "CIDFontType2"
        }),
    );
    cid_font.insert("BaseFont", base_font.clone());
    let mut system_info = Dict::new();
    system_info.insert("Registry", PdfString::literal("Adobe"));
    system_info.insert("Ordering", PdfString::literal("Identity"));
    system_info.insert("Supplement", 0);
    cid_font.insert("CIDSystemInfo", system_info);
    cid_font.insert("FontDescriptor", descriptor);
    cid_font.insert("DW", width_object(dw));
    let w = compress_widths(widths, dw);
    if !w.is_empty() {
        cid_font.insert("W", Object::Array(w));
    }
    if !program.is_cff() {
        // ISO 32000-2 §9.7.4.2: `/Identity` when every CID is its glyph's ID in the embedded
        // program; a stream (two bytes per CID, big-endian GID) once a glyph has several CIDs
        // (one per text it is drawn with, B4d).
        match usage.cid_to_gid_map() {
            None => {
                cid_font.insert("CIDToGIDMap", Name::from("Identity"));
            }
            Some(map) => {
                let stream = cos.create(Object::Stream(Stream::flate(Dict::new(), &map)))?;
                cid_font.insert("CIDToGIDMap", stream);
            }
        }
    }
    let cid_font = cos.create(Object::Dict(cid_font))?;

    let cmap = to_unicode_cmap(Codespace::TwoByte, usage.text().iter().map(|(&c, t)| (c, t.as_str())));
    let to_unicode = cos.create(Object::Stream(Stream::flate(Dict::new(), &cmap)))?;

    let mut dict = Dict::new();
    dict.insert("Type", Name::from("Font"));
    dict.insert("Subtype", Name::from("Type0"));
    // §9.7.6.1: for a CIDFontType0 descendant, the CIDFont's name, a hyphen and the CMap
    // name; for CIDFontType2, the CIDFont's name.
    let type0_name = if program.is_cff() {
        Name::from(format!("{}-Identity-H", String::from_utf8_lossy(base_font.as_bytes())).as_str())
    } else {
        base_font
    };
    dict.insert("BaseFont", type0_name);
    dict.insert("Encoding", Name::from("Identity-H"));
    dict.insert("DescendantFonts", Object::Array(vec![cid_font.into()]));
    dict.insert("ToUnicode", to_unicode);
    cos.set(type0, Object::Dict(dict))?;
    Ok(())
}

/// The most common width (the smallest of equally common ones), written as `/DW` so that
/// `/W` lists only the others.
fn default_width(widths: &[(u16, Milli)]) -> Milli {
    let mut counts: BTreeMap<Milli, usize> = BTreeMap::new();
    for &(_, w) in widths {
        *counts.entry(w).or_default() += 1;
    }
    let mut best = (0, 0);
    for (w, n) in counts {
        if n > best.1 {
            best = (w, n);
        }
    }
    best.0
}

/// The `/W` array (§9.7.4.3) for `widths` (sorted by CID), leaving out widths equal to `dw`:
/// runs of [`MIN_RANGE`] or more consecutive CIDs with one width as `c_first c_last w`, other
/// consecutive CIDs as `c [w1 w2 …]`.
fn compress_widths(widths: &[(u16, Milli)], dw: Milli) -> Vec<Object> {
    let listed: Vec<(u16, Milli)> = widths.iter().copied().filter(|&(_, w)| w != dw).collect();
    let mut out = Vec::new();
    for group in listed.chunk_by(|a, b| a.0.checked_add(1) == Some(b.0)) {
        let mut array: Option<(u16, Vec<Object>)> = None;
        for same in group.chunk_by(|a, b| a.1 == b.1) {
            let (Some(&(first, w)), Some(&(last, _))) = (same.first(), same.last()) else {
                continue;
            };
            if same.len() >= MIN_RANGE {
                flush(&mut array, &mut out);
                out.extend([i64::from(first).into(), i64::from(last).into(), width_object(w)]);
            } else {
                let (_, ws) = array.get_or_insert_with(|| (first, Vec::new()));
                ws.extend(same.iter().map(|&(_, w)| width_object(w)));
            }
        }
        flush(&mut array, &mut out);
    }
    out
}

fn flush(array: &mut Option<(u16, Vec<Object>)>, out: &mut Vec<Object>) {
    if let Some((first, ws)) = array.take() {
        out.push(i64::from(first).into());
        out.push(Object::Array(ws));
    }
}

/// A width in thousandths as a PDF number: an integer when whole, else a real with three
/// decimals.
fn width_object(w: Milli) -> Object {
    #[expect(clippy::cast_precision_loss, reason = "widths are far below 2^52")]
    Object::number(w as f64 / 1000.0)
}

/// `StemV` estimated from the weight class (fonts don't record stem widths in a table PDF
/// readers use; the value only matters for substitution, which never happens for an embedded
/// font): `10 + 220 (weight − 50) / 900`, about 96 at 400 and 218 at 900.
fn stem_v(weight: u16) -> Object {
    let w = f64::from(weight);
    Object::Integer(round_i64(10.0 + 220.0 * (w - 50.0) / 900.0).max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(widths: &[(u16, i64)], dw: i64) -> String {
        let out = Object::Array(compress_widths(
            &widths.iter().map(|&(c, w)| (c, w * 1000)).collect::<Vec<_>>(),
            dw * 1000,
        ));
        String::from_utf8(out.to_bytes()).unwrap()
    }

    #[test]
    fn consecutive_cids_share_one_array() {
        assert_eq!(w(&[(1, 500), (2, 600), (3, 700)], 0), "[1 [500 600 700]]");
    }

    #[test]
    fn gaps_start_a_new_entry() {
        assert_eq!(w(&[(1, 500), (5, 600)], 0), "[1 [500] 5 [600]]");
    }

    #[test]
    fn three_equal_widths_become_a_range() {
        assert_eq!(w(&[(1, 500), (2, 500), (3, 500)], 0), "[1 3 500]");
    }

    #[test]
    fn range_inside_a_group_splits_the_array() {
        assert_eq!(
            w(&[(1, 400), (2, 500), (3, 500), (4, 500), (5, 600)], 0),
            "[1 [400] 2 4 500 5 [600]]"
        );
    }

    #[test]
    fn two_equal_widths_stay_in_the_array() {
        assert_eq!(w(&[(1, 500), (2, 500), (3, 600)], 0), "[1 [500 500 600]]");
    }

    #[test]
    fn default_width_entries_are_left_out() {
        assert_eq!(w(&[(1, 500), (2, 1000), (3, 600)], 1000), "[1 [500] 3 [600]]");
    }

    #[test]
    fn fractional_widths_keep_three_decimals() {
        assert_eq!(String::from_utf8(width_object(600_098).to_bytes()).unwrap(), "600.098");
    }

    #[test]
    fn default_width_is_the_most_common() {
        assert_eq!(default_width(&[(0, 600), (1, 500), (2, 500), (3, 700)]), 500);
    }

    #[test]
    fn default_width_tie_takes_the_smallest() {
        assert_eq!(default_width(&[(0, 600), (1, 500)]), 500);
    }

    #[test]
    fn stem_v_follows_the_weight() {
        assert_eq!(stem_v(400), Object::Integer(96));
    }
}
