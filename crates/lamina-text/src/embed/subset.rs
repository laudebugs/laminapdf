//! The font program to embed (`subsetter`) and its subset tag (ISO 32000-2 §9.9, §9.6.4;
//! research 03 §6.2).
//!
//! Ported in spirit from the krilla-fit spike's experiment C (`spikes/krilla-fit/src/exp_c.rs`),
//! which follows krilla (MIT OR Apache-2.0): one `GlyphRemapper` per font, filled while content
//! is written, subset at the end.

use sha2::{Digest, Sha256};
use skrifa::FontRef;

use crate::error::{Error, Result};
use crate::font::Font;
use crate::usage::FontUse;

/// The subset as it is embedded.
pub(crate) struct Program {
    /// The subset `sfnt` `subsetter` wrote (its `hmtx` gives `/W`).
    pub(crate) sfnt: Vec<u8>,
    /// What goes into the font file stream: the `sfnt` itself for TrueType outlines, the bare
    /// `CFF ` table for CFF outlines.
    pub(crate) data: Vec<u8>,
    /// CFF outlines (`CIDFontType0`, `/FontFile3 /CIDFontType0C`); else TrueType
    /// (`CIDFontType2`, `/FontFile2`).
    pub(crate) is_cff: bool,
}

/// Subsets `font` to the glyphs `usage` recorded (every glyph when it is embedded whole), at the
/// font's instance location. Dropped: `cmap` (the PDF supplies the mapping), layout tables
/// (PDF viewers never shape), `OS/2`. A CFF2 font comes back with TrueType outlines.
pub(crate) fn subset(font: &Font, usage: &FontUse) -> Result<Program> {
    let name = || font.info().postscript_name.clone();
    let coords: Vec<(subsetter::Tag, f32)> = font
        .coordinates()
        .iter()
        .map(|&(tag, value)| (subsetter::Tag::new(&tag.to_bytes()), value))
        .collect();
    let sfnt =
        subsetter::subset_with_variations(font.bytes(), font.index(), &coords, &usage.remapper()).map_err(|e| {
            Error::Subset {
                font: name(),
                reason: reason(e),
            }
        })?;
    let parsed = FontRef::new(&sfnt).map_err(|_| Error::Subset {
        font: name(),
        reason: "the subset is not a readable font",
    })?;
    let cff = parsed.table_data(skrifa::Tag::new(b"CFF "));
    let (data, is_cff) = match cff {
        Some(table) => (table.as_bytes().to_vec(), true),
        None => (sfnt.clone(), false),
    };
    Ok(Program { sfnt, data, is_cff })
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

/// The six-letter subset tag (§9.6.4: "six uppercase letters"; different subsets in one file
/// must have different tags): the first six bytes of a SHA-256 over the font's bytes (their
/// digest), the collection index, the instance's coordinates and the glyphs in CID order
/// (which determine the subset), each taken modulo 26. `salt` is 0 unless another subset in
/// the same document already has the tag.
pub(crate) fn subset_tag(font: &Font, usage: &FontUse, salt: u32) -> String {
    let mut hash = Sha256::new();
    hash.update(b"lamina subset tag v1");
    hash.update(font.digest());
    hash.update(font.index().to_be_bytes());
    for (tag, value) in font.coordinates() {
        hash.update(tag.to_bytes());
        hash.update(value.to_bits().to_be_bytes());
    }
    for (_, gid) in usage.glyphs() {
        hash.update(gid.get().to_be_bytes());
    }
    hash.update(salt.to_be_bytes());
    hash.finalize()
        .iter()
        .take(6)
        .map(|b| char::from(b'A' + b % 26))
        .collect()
}
