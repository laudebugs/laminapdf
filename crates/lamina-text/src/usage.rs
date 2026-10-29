//! Glyph usage: which glyphs of a font a document draws, the CID each is drawn with, and the
//! text each stands for (the ToUnicode map).

use std::collections::{BTreeMap, BTreeSet};

use subsetter::GlyphRemapper;

use crate::error::{Error, Result, Warning};
use crate::font::{Font, GlyphId};

/// The glyphs one font uses in a document, collected while content is written and turned into
/// a subset at the end.
///
/// The CID a glyph is drawn with (two bytes in the content stream, Identity-H) is its glyph ID
/// *in the subset*: glyphs are renumbered densely in order of first use, `.notdef` staying 0,
/// so the same sequence of uses always gives the same CIDs and the same subset. The embedded
/// CIDFont then maps CIDs to glyphs with `/CIDToGIDMap /Identity` (ISO 32000-2 §9.7.4.2). A
/// font whose `fsType` forbids subsetting keeps its own glyph IDs (CID = original GID) and is
/// embedded whole.
#[derive(Debug, Clone)]
pub struct FontUse {
    mapping: Mapping,
    /// CID → the text the glyph stands for (sorted: the ToUnicode CMap is written in order).
    text: BTreeMap<u16, String>,
    glyph_count: u16,
    /// The font's PostScript name, for errors and warnings.
    font: String,
}

#[derive(Debug, Clone)]
enum Mapping {
    /// Old GID → CID (= new GID), in order of first use.
    Subset(GlyphRemapper),
    /// CID = GID; the whole font is embedded. Holds the glyphs drawn.
    Whole(BTreeSet<u16>),
}

impl FontUse {
    /// Usage of `font`; subset unless its `fsType` forbids subsetting.
    #[must_use]
    pub fn new(font: &Font) -> Self {
        let info = font.info();
        let mapping = if info.fs_type.forbids_subsetting() {
            Mapping::Whole(BTreeSet::from([0]))
        } else {
            let mut remapper = GlyphRemapper::new();
            // .notdef stays glyph 0 (every font program needs it, §9.9).
            remapper.remap(0);
            Mapping::Subset(remapper)
        };
        Self {
            mapping,
            text: BTreeMap::new(),
            glyph_count: info.glyph_count,
            font: info.postscript_name.clone(),
        }
    }

    /// Records that `gid` is drawn standing for `text` (one character, a ligature's several,
    /// or empty for a glyph that stands for no text of its own, such as the second glyph of a
    /// decomposed cluster) and returns the CID to draw it with.
    ///
    /// A glyph keeps the first non-empty text it is used with; a different text later is
    /// returned as a [`Warning::ToUnicodeConflict`] (the builder marks that occurrence with
    /// `/ActualText`).
    ///
    /// # Errors
    /// [`Error::GlyphOutOfRange`] for a glyph past the font's glyph count.
    pub fn use_glyph(&mut self, gid: GlyphId, text: &str) -> Result<(u16, Option<Warning>)> {
        if gid.get() >= self.glyph_count {
            return Err(Error::GlyphOutOfRange {
                gid: gid.get(),
                count: self.glyph_count,
                font: self.font.clone(),
            });
        }
        let cid = match &mut self.mapping {
            Mapping::Subset(remapper) => remapper.remap(gid.get()),
            Mapping::Whole(used) => {
                used.insert(gid.get());
                gid.get()
            }
        };
        if text.is_empty() || cid == 0 {
            return Ok((cid, None));
        }
        let warning = match self.text.get(&cid) {
            None => {
                self.text.insert(cid, text.to_owned());
                None
            }
            Some(kept) if kept == text => None,
            Some(kept) => Some(Warning::ToUnicodeConflict {
                font: self.font.clone(),
                gid: gid.get(),
                kept: kept.clone(),
                dropped: text.to_owned(),
            }),
        };
        Ok((cid, warning))
    }

    /// CID → text, in CID order: what the ToUnicode CMap maps.
    #[must_use]
    pub fn text(&self) -> &BTreeMap<u16, String> {
        &self.text
    }

    /// The glyphs used, as (CID, original glyph ID) in CID order, `.notdef` first. For a font
    /// embedded whole, only the glyphs drawn (and `.notdef`) are listed.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(u16, GlyphId)> {
        match &self.mapping {
            Mapping::Subset(remapper) => remapper
                .remapped_gids()
                .zip(0u16..)
                .map(|(old, cid)| (cid, GlyphId::new(old)))
                .collect(),
            Mapping::Whole(used) => used.iter().map(|&cid| (cid, GlyphId::new(cid))).collect(),
        }
    }

    /// Whether the font is embedded whole (its `fsType` forbids subsetting).
    #[must_use]
    pub fn is_whole(&self) -> bool {
        matches!(self.mapping, Mapping::Whole(_))
    }

    /// The remapper to subset with: the glyphs used in CID order, or every glyph in order for a
    /// font embedded whole (so that CID = GID).
    pub(crate) fn remapper(&self) -> GlyphRemapper {
        match &self.mapping {
            Mapping::Subset(remapper) => remapper.clone(),
            Mapping::Whole(_) => {
                let all: Vec<u16> = (0..self.glyph_count).collect();
                GlyphRemapper::new_from_glyphs_sorted(&all)
            }
        }
    }
}
