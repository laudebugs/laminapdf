//! The font program to embed (`subsetter`, or the font's own bytes when it must be embedded
//! whole), its widths and its subset tag (ISO 32000-2 §9.9, §9.6.4; research 03 §6.2).
//!
//! Ported in spirit from the krilla-fit spike's experiment C (`spikes/krilla-fit/src/exp_c.rs`),
//! which follows krilla (MIT OR Apache-2.0): one `GlyphRemapper` per font, filled while content
//! is written, subset at the end.

use std::collections::BTreeSet;
use std::ops::Range;

use sha2::{Digest, Sha256};
use skrifa::instance::{LocationRef, Size};
use skrifa::raw::TableProvider as _;
use skrifa::{FontRef, MetadataProvider as _};

use crate::error::{Error, Result, Warning};
use crate::font::{Font, GlyphId};
use crate::usage::FontUse;

/// Widths in thousandths of a `/W` unit (millionths of an em), so equal widths compare exactly
/// and the output does not depend on float formatting.
pub(crate) type Milli = i64;

/// How the program is embedded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    /// TrueType outlines: `CIDFontType2`, `/FontFile2` with the whole `sfnt`.
    TrueType,
    /// A subset with CFF outlines: `CIDFontType0`, `/FontFile3 /CIDFontType0C` with the bare
    /// (CID-keyed) `CFF ` table, which is this range of the `sfnt`.
    Cff(Range<usize>),
    /// A whole OpenType font with CFF outlines: `CIDFontType0`, `/FontFile3 /OpenType`
    /// (PDF 1.6) with the font's own bytes.
    OpenType,
}

/// The program as it is embedded, with what `/W` and the descriptor need from it.
pub(crate) struct Program {
    /// The `sfnt`: the subset `subsetter` wrote, or the font's own bytes.
    pub(crate) sfnt: Vec<u8>,
    pub(crate) kind: Kind,
    /// (CID, width) for every glyph drawn, `.notdef` first, from the program's own `hmtx`.
    pub(crate) widths: Vec<(u16, Milli)>,
    /// For an instance: the union of the instanced glyphs' bounds in thousandths of an em
    /// (`head`'s bbox is not varied).
    pub(crate) bbox: Option<[f32; 4]>,
}

impl Program {
    /// The bytes of the font file stream.
    pub(crate) fn data(&self) -> &[u8] {
        match &self.kind {
            Kind::Cff(range) => self.sfnt.get(range.clone()).unwrap_or_default(),
            Kind::TrueType | Kind::OpenType => &self.sfnt,
        }
    }

    pub(crate) const fn is_cff(&self) -> bool {
        matches!(self.kind, Kind::Cff(_) | Kind::OpenType)
    }
}

/// Builds the program for `font` with the glyphs `usage` recorded (`glyphs`: its
/// [`FontUse::glyphs`]).
///
/// * Subset (the usual case): `subsetter`, at the instance's location. Dropped: `cmap` (the PDF
///   supplies the mapping), layout tables (PDF viewers never shape), `OS/2`. A CFF2 font
///   comes back with TrueType outlines.
/// * "No subsetting" (`fsType` 0x0100): the font's own bytes, unchanged (a collection member
///   is copied out as a standalone font with its tables as they are). A variable-font
///   instance or a CFF2 font can't be embedded as it is, so it goes through `subsetter` with
///   every glyph kept, and [`Warning::WholeFontRebuilt`] says so.
pub(crate) fn build(font: &Font, usage: &FontUse, glyphs: &[(u16, GlyphId)]) -> Result<(Program, Option<Warning>)> {
    let name = || font.info().postscript_name.clone();
    let source = font.font_ref()?;
    let has = |tag: &[u8; 4]| source.table_data(skrifa::Tag::new(tag)).is_some();
    let can_embed_as_is = font.coordinates().is_empty() && !has(b"CFF2");
    let (sfnt, warning) =
        if usage.is_whole() && can_embed_as_is {
            let sfnt = standalone(font, &source).ok_or_else(|| Error::Subset {
                font: name(),
                reason: "the font's table directory is malformed",
            })?;
            (sfnt, None)
        } else {
            let coords: Vec<(subsetter::Tag, f32)> = font
                .coordinates()
                .iter()
                .map(|&(tag, value)| (subsetter::Tag::new(&tag.to_bytes()), value))
                .collect();
            let sfnt = subsetter::subset_with_variations(font.bytes(), font.index(), &coords, &usage.remapper())
                .map_err(|e| Error::Subset {
                    font: name(),
                    reason: reason(e),
                })?;
            let warning = usage.is_whole().then(|| Warning::WholeFontRebuilt { font: name() });
            (sfnt, warning)
        };
    let unreadable = || Error::Subset {
        font: name(),
        reason: "the subset is not a readable font",
    };
    let parsed = FontRef::new(&sfnt).map_err(|_| unreadable())?;
    let cff = parsed
        .table_directory
        .table_records()
        .iter()
        .find(|r| r.tag() == CFF_TAG);
    if cff.is_some() && !has_cff_table(&source) {
        // `FontUse` decided from the source whether glyphs may get several CIDs; a CFF
        // program must not get CIDs its charset doesn't have.
        return Err(Error::Subset {
            font: name(),
            reason: "the subset's outline format differs from the font's",
        });
    }
    let kind = match cff {
        None => Kind::TrueType,
        Some(_) if usage.is_whole() && can_embed_as_is => Kind::OpenType,
        Some(record) => {
            let start = usize::try_from(record.offset()).map_err(|_| unreadable())?;
            let len = usize::try_from(record.length()).map_err(|_| unreadable())?;
            let end = start
                .checked_add(len)
                .filter(|&e| e <= sfnt.len())
                .ok_or_else(unreadable)?;
            Kind::Cff(start..end)
        }
    };
    let upem = parsed.head().map_err(|_| unreadable())?.units_per_em();
    if upem == 0 {
        return Err(unreadable());
    }
    // `/W` from the original font at the instance's location (the advances the shaper used),
    // not the subset's hmtx, so `TJ` corrections computed from the same function are exact.
    let original = font.font_ref()?;
    let original_metrics = original.glyph_metrics(Size::unscaled(), font.location_ref());
    let widths = glyphs
        .iter()
        .map(|&(cid, gid)| Ok((cid, advance_milli(&original_metrics, gid, upem))))
        .collect::<Result<Vec<_>>>()?;
    let bbox = if font.coordinates().is_empty() {
        None
    } else {
        // The instanced subset's own outlines: what the viewer draws.
        let metrics = parsed.glyph_metrics(Size::unscaled(), LocationRef::default());
        let count = parsed.maxp().map_err(|_| unreadable())?.num_glyphs();
        let scale = 1000.0 / f32::from(upem);
        (0..count)
            .filter_map(|gid| metrics.bounds(skrifa::GlyphId::from(gid)))
            .map(|b| [b.x_min, b.y_min, b.x_max, b.y_max])
            .reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
            .map(|b| b.map(|v| v * scale))
    };
    Ok((
        Program {
            sfnt,
            kind,
            widths,
            bbox,
        },
        warning,
    ))
}

/// The font as a standalone `sfnt`: its own bytes for a single font; for a collection member,
/// a new table directory with the member's tables copied unchanged (each 4-byte aligned).
/// `None` when a table record points outside the data.
fn standalone(font: &Font, source: &FontRef<'_>) -> Option<Vec<u8>> {
    let bytes = font.bytes();
    if Font::collection_len(bytes).ok()? == 1 {
        return Some(bytes.to_vec());
    }
    let dir = &source.table_directory;
    let records = dir.table_records();
    let mut out = Vec::new();
    out.extend_from_slice(&dir.sfnt_version().to_be_bytes());
    out.extend_from_slice(&dir.num_tables().to_be_bytes());
    out.extend_from_slice(&dir.search_range().to_be_bytes());
    out.extend_from_slice(&dir.entry_selector().to_be_bytes());
    out.extend_from_slice(&dir.range_shift().to_be_bytes());
    let mut offset = 12usize.checked_add(records.len().checked_mul(16)?)?;
    let mut data = Vec::new();
    for record in records {
        let start = usize::try_from(record.offset()).ok()?;
        let len = usize::try_from(record.length()).ok()?;
        let table = bytes.get(start..start.checked_add(len)?)?;
        out.extend_from_slice(&record.tag().to_be_bytes());
        out.extend_from_slice(&record.checksum().to_be_bytes());
        out.extend_from_slice(&u32::try_from(offset).ok()?.to_be_bytes());
        out.extend_from_slice(&record.length().to_be_bytes());
        data.extend_from_slice(table);
        let padded = len.checked_add(3)? & !3;
        data.resize(data.len() + (padded - len), 0);
        offset = offset.checked_add(padded)?;
    }
    out.extend_from_slice(&data);
    Some(out)
}

fn reason(e: subsetter::Error) -> &'static str {
    match e {
        subsetter::Error::UnknownKind => "unknown font kind",
        subsetter::Error::MalformedFont => "malformed font",
        subsetter::Error::Unimplemented => "the font uses a feature the subsetter does not support",
        subsetter::Error::SubsetError => "internal subsetter error",
        subsetter::Error::OverflowError => "overflow while subsetting",
        subsetter::Error::CFFError => "malformed CFF table",
    }
}

const CFF_TAG: skrifa::Tag = skrifa::Tag::new(b"CFF ");

/// Whether `font` has CFF outlines that the embedded program keeps (a `CFF ` table: such a
/// font is embedded as `CIDFontType0`, one CID per glyph). CFF2 is converted to TrueType
/// outlines by the subsetter. [`FontUse`] and [`build`] both decide with this.
pub(crate) fn has_cff_table(font: &FontRef<'_>) -> bool {
    font.table_data(CFF_TAG).is_some()
}

/// How many salts [`unique_tag`] tries before giving up. Two subsets collide with probability
/// 26⁻⁶, so even the first retry is rare; the bound only keeps a broken hash from looping.
const MAX_TAG_ATTEMPTS: u32 = 1024;

/// The first of `make(0)`, `make(1)`, … that is not in `taken`.
///
/// # Errors
/// [`Error::Subset`] when [`MAX_TAG_ATTEMPTS`] salts all collide.
pub(crate) fn unique_tag(taken: &BTreeSet<String>, font: &str, make: impl Fn(u32) -> String) -> Result<String> {
    (0..MAX_TAG_ATTEMPTS)
        .map(make)
        .find(|tag| !taken.contains(tag))
        .ok_or_else(|| Error::Subset {
            font: font.to_owned(),
            reason: "no free subset tag",
        })
}

/// The six-letter subset tag (§9.6.4: "six uppercase letters"; different subsets in one file
/// must have different tags): the first six bytes of a SHA-256 over the font's bytes (their
/// digest), the collection index, the instance's coordinates and the glyphs in CID order
/// (which determine the subset), each taken modulo 26. `salt` is 0 unless another subset in
/// the same document already has the tag ([`unique_tag`]).
pub(crate) fn subset_tag(font: &Font, glyphs: &[(u16, GlyphId)], salt: u32) -> String {
    let mut hash = Sha256::new();
    hash.update(b"lamina subset tag v1");
    hash.update(font.digest());
    hash.update(font.index().to_be_bytes());
    for (tag, value) in font.coordinates() {
        hash.update(tag.to_bytes());
        hash.update(value.to_bits().to_be_bytes());
    }
    for (_, gid) in glyphs {
        hash.update(gid.get().to_be_bytes());
    }
    hash.update(salt.to_be_bytes());
    hash.finalize()
        .iter()
        .take(6)
        .map(|b| char::from(b'A' + b % 26))
        .collect()
}

/// A glyph's advance as `/W` writes it and as the shaped runs' `TJ` numbers (B4b) assume it:
/// the original font's advance at the instance's location, rounded to a whole font unit, in
/// millionths of an em. One function for both, so the two can't disagree.
pub(crate) fn advance_milli(metrics: &skrifa::metrics::GlyphMetrics<'_>, gid: GlyphId, upem: u16) -> i64 {
    let advance = metrics.advance_width(skrifa::GlyphId::from(gid.get())).unwrap_or(0.0);
    milli(f64::from(advance.round()), upem)
}

/// `units` font units in millionths of an em (thousandths of a `/W` unit), rounded: the one
/// conversion `/W` and the shaped runs' `TJ` numbers (B4b) both use.
pub(crate) fn milli(units: f64, upem: u16) -> i64 {
    round_i64(units * 1_000_000.0 / f64::from(upem.max(1)))
}

fn round_i64(v: f64) -> i64 {
    if !v.is_finite() {
        return 0;
    }
    // Font values are bounded by i16 font units scaled by at most 10^6 / 16; far from i64's
    // range, so the cast is exact after rounding.
    #[expect(clippy::cast_possible_truncation, reason = "bounded, see above")]
    let r = v.round() as i64;
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_tag_skips_a_taken_tag() {
        let taken = BTreeSet::from(["AAAAAA".to_owned()]);
        let tag = unique_tag(&taken, "F", |salt| {
            if salt == 0 { "AAAAAA" } else { "BBBBBB" }.to_owned()
        });
        assert_eq!(tag.unwrap(), "BBBBBB");
    }

    #[test]
    fn unique_tag_gives_up_when_every_salt_collides() {
        let taken = BTreeSet::from(["AAAAAA".to_owned()]);
        assert_eq!(
            unique_tag(&taken, "F", |_| "AAAAAA".to_owned()).unwrap_err().code(),
            "subset-failed"
        );
    }
}
