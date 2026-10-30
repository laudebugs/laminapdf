//! Glyph usage: which glyphs of a font a document draws, the CID each is drawn with, and the
//! text each CID stands for (the ToUnicode map).

use std::collections::BTreeMap;

use subsetter::GlyphRemapper;

use crate::error::{Error, Result, Warning};
use crate::font::{Font, GlyphId};

/// The CIDs a glyph that stands for no text is drawn with (B4d), while they last: code points
/// of general category Cf (format characters) in the order tried. A ToUnicode CMap maps such
/// a CID to the empty string (`<cid> <>`), which pypdf and pdftotext read as nothing; pdf.js
/// reads an empty destination as the code itself, and skips a code that is a format
/// character. Ordinary CIDs never take these values (in a subset; in a font embedded whole,
/// only those at or past its glyph count are set aside).
const TEXTLESS_CIDS: [std::ops::RangeInclusive<u16>; 6] = [
    0x200B..=0x200F,
    0x202A..=0x202E,
    0x2060..=0x2064,
    0x2066..=0x206F,
    0xFEFF..=0xFEFF,
    0xFFF9..=0xFFFB,
];

/// How many values [`TEXTLESS_CIDS`] holds.
const TEXTLESS_LEN: u32 = textless_len();

const fn textless_len() -> u32 {
    let mut n = 0;
    let mut rest: &[std::ops::RangeInclusive<u16>] = &TEXTLESS_CIDS;
    while let [r, tail @ ..] = rest {
        n += (*r.end() - *r.start()) as u32 + 1;
        rest = tail;
    }
    n
}

fn is_textless_slot(cid: u16) -> bool {
    TEXTLESS_CIDS.iter().any(|r| r.contains(&cid))
}

/// The glyphs one font uses in a document, collected while content is written and turned into
/// a subset at the end.
///
/// Each CID (two bytes in the content stream, Identity-H) stands for one glyph and one text
/// (B4d): a glyph drawn with two texts (a pre-base matra standing for the consonant before
/// it, the same glyph elsewhere for itself) gets two CIDs, so the ToUnicode CMap (one code,
/// one text: ISO 32000-2 §9.10.3) maps both exactly. In a subset, glyphs are renumbered
/// densely in order of first use, `.notdef` staying 0, and CIDs are numbered densely in order
/// of first (glyph, text) use; when no glyph has two texts the two coincide and the CIDFont
/// maps CIDs to glyphs with `/CIDToGIDMap /Identity` (§9.7.4.2), otherwise with a
/// `/CIDToGIDMap` stream. A font whose `fsType` forbids subsetting keeps its own glyph IDs
/// (a glyph's first CID is its original GID; further CIDs follow the glyph count) and is
/// embedded whole.
///
/// A font with CFF outlines (`CIDFontType0`) addresses glyphs through its CFF charset, not
/// `/CIDToGIDMap`, so there each glyph keeps one CID and its first text; a later different
/// text is a [`Warning::ToUnicodeConflict`] (the builder marks that occurrence with
/// `/ActualText`, as [`crate::pdf_run`] does).
#[derive(Debug, Clone)]
pub struct FontUse {
    mapping: Mapping,
    /// CID → the original glyph it draws, `.notdef` (CID 0) included.
    cids: BTreeMap<u16, GlyphId>,
    /// Original glyph → its CIDs, in order of allocation.
    by_glyph: BTreeMap<GlyphId, Vec<u16>>,
    /// CID → the text the glyph stands for (sorted: the ToUnicode CMap is written in order).
    text: BTreeMap<u16, String>,
    /// The next ordinary CID to try: in a subset, the dense numbering; in a font embedded
    /// whole, the CIDs past its glyph count.
    next_cid: u32,
    /// Whether a glyph may get several CIDs (`false` for CFF outlines).
    per_text: bool,
    /// Whether the textless slots are set aside ([`TEXTLESS_CIDS`]): with several CIDs per
    /// glyph, unless a subset's glyphs could need every ordinary CID (over 65 507 glyphs).
    pool: bool,
    glyph_count: u16,
    /// The font's PostScript name, for errors and warnings.
    font: String,
}

#[derive(Debug, Clone)]
enum Mapping {
    /// Old GID → new GID, in order of first use.
    Subset(GlyphRemapper),
    /// The whole font is embedded (new GID = old GID).
    Whole,
}

impl FontUse {
    /// Usage of `font`; subset unless its `fsType` forbids subsetting.
    #[must_use]
    pub fn new(font: &Font) -> Self {
        let info = font.info();
        let mapping = if info.fs_type.forbids_subsetting() {
            Mapping::Whole
        } else {
            let mut remapper = GlyphRemapper::new();
            // .notdef stays glyph 0 (every font program needs it, §9.9).
            remapper.remap(0);
            Mapping::Subset(remapper)
        };
        // The program keeps CFF outlines (`CIDFontType0`) when the font has a `CFF ` table
        // (the same test `subset::build` makes on the program); CFF2 is converted to TrueType
        // outlines by the subsetter.
        let cff = font.font_ref().is_ok_and(|f| crate::embed::subset::has_cff_table(&f));
        let (next_cid, pool) = match mapping {
            Mapping::Subset(_) => (1, u32::from(info.glyph_count) + TEXTLESS_LEN <= 0x1_0000),
            Mapping::Whole => (u32::from(info.glyph_count), true),
        };
        Self {
            mapping,
            cids: BTreeMap::from([(0, GlyphId::NOTDEF)]),
            by_glyph: BTreeMap::from([(GlyphId::NOTDEF, vec![0])]),
            text: BTreeMap::new(),
            next_cid,
            per_text: !cff,
            pool: pool && !cff,
            glyph_count: info.glyph_count,
            font: info.postscript_name.clone(),
        }
    }

    /// Records that `gid` is drawn standing for `text` (one character, a ligature's several,
    /// or empty for a glyph that stands for no text of its own, such as a dot drawn apart from
    /// its letter) and returns the CID to draw it with.
    ///
    /// The same (glyph, text) always gets the same CID. A glyph drawn with another text gets
    /// another CID mapped to the same glyph; an empty text takes a CID that extractors skip
    /// (a format character's code point, while those last). Only where a glyph can't get
    /// another CID (CFF outlines, or the 65 536 CIDs used up) does it keep its first text, and
    /// the different one is returned as a [`Warning::ToUnicodeConflict`].
    ///
    /// # Errors
    /// [`Error::GlyphOutOfRange`] for a glyph past the font's glyph count.
    pub fn use_glyph(&mut self, gid: GlyphId, text: &str) -> Result<(u16, Option<Warning>)> {
        self.use_glyph_exact(gid, text).map(|(cid, warning, _)| (cid, warning))
    }

    /// [`FontUse::use_glyph`], also saying whether the CID extracts as `text` exactly: `false`
    /// on a conflict, and for an empty text drawn with an ordinary CID (no textless slot left,
    /// or none in this font), which pdf.js reads as the code. [`crate::pdf_run`] marks such
    /// clusters with `/ActualText`.
    pub(crate) fn use_glyph_exact(&mut self, gid: GlyphId, text: &str) -> Result<(u16, Option<Warning>, bool)> {
        self.check(gid)?;
        if gid == GlyphId::NOTDEF {
            return Ok((0, None, true));
        }
        let (cid, warning) = self.claim(gid, text);
        let exact = warning.is_none() && (!text.is_empty() || self.reserved(cid));
        Ok((cid, warning, exact))
    }

    fn claim(&mut self, gid: GlyphId, text: &str) -> (u16, Option<Warning>) {
        let existing = self.by_glyph.get(&gid).map_or(&[][..], Vec::as_slice);
        let same = existing
            .iter()
            .copied()
            .find(|c| self.text.get(c).is_some_and(|t| t == text));
        // A CID drawn so far without a text claims this one.
        let untexted = existing.iter().copied().find(|c| !self.text.contains_key(c));
        let first = existing.first().copied();
        if let Some(cid) = same {
            return (cid, None);
        }
        if let Some(cid) = untexted {
            self.text.insert(cid, text.to_owned());
            return (cid, None);
        }
        let Some(first) = first else {
            let cid = self.first_cid(gid, text.is_empty());
            self.text.insert(cid, text.to_owned());
            return (cid, None);
        };
        match self.extra_cid(text.is_empty()) {
            Some(cid) => {
                self.bind(cid, gid);
                self.text.insert(cid, text.to_owned());
                (cid, None)
            }
            None => (
                first,
                Some(Warning::ToUnicodeConflict {
                    font: self.font.clone(),
                    gid: gid.get(),
                    kept: self.text.get(&first).cloned().unwrap_or_default(),
                    dropped: text.to_owned(),
                }),
            ),
        }
    }

    /// Records that `gid` is drawn without claiming any text for it (B4b: a hidden
    /// default-ignorable drawn as the space glyph) and returns its CID: the glyph's first
    /// one. The CID's ToUnicode entry is whatever other uses give it.
    ///
    /// # Errors
    /// [`Error::GlyphOutOfRange`] for a glyph past the font's glyph count.
    pub(crate) fn glyph_cid(&mut self, gid: GlyphId) -> Result<u16> {
        self.check(gid)?;
        if let Some(&cid) = self.by_glyph.get(&gid).and_then(|c| c.first()) {
            return Ok(cid);
        }
        Ok(self.first_cid(gid, false))
    }

    /// Whether a glyph drawn with a second text gets a CID of its own ([`FontUse::use_glyph`]):
    /// `false` for CFF outlines.
    #[must_use]
    pub const fn splits_texts(&self) -> bool {
        self.per_text
    }

    fn check(&self, gid: GlyphId) -> Result<()> {
        if gid.get() >= self.glyph_count {
            return Err(Error::GlyphOutOfRange {
                gid: gid.get(),
                count: self.glyph_count,
                font: self.font.clone(),
            });
        }
        Ok(())
    }

    /// The CID of a glyph not drawn before: a textless slot for `textless` while one is free;
    /// otherwise, in a subset, the next dense CID, and in a font embedded whole, its own GID.
    fn first_cid(&mut self, gid: GlyphId, textless: bool) -> u16 {
        let new_gid = match &mut self.mapping {
            Mapping::Subset(remapper) => Some(remapper.remap(gid.get())),
            Mapping::Whole => None,
        };
        let slot = textless.then(|| self.free_textless_slot()).flatten();
        let cid = match (new_gid, slot) {
            (_, Some(slot)) => slot,
            (None, None) => gid.get(),
            // CFF: CID = the subset's GID (its charset is the identity).
            (Some(new_gid), None) if !self.per_text => new_gid,
            (Some(_), None) => self.next_ordinary(),
        };
        self.bind(cid, gid);
        cid
    }

    /// A further CID for a glyph that already has one: a free textless slot for `textless`,
    /// else the next ordinary CID; `None` when the glyph can't have another (CFF outlines, or
    /// too few CIDs left to be sure every glyph not drawn yet still gets one).
    fn extra_cid(&mut self, textless: bool) -> Option<u16> {
        if !self.per_text {
            return None;
        }
        if let Some(slot) = textless.then(|| self.free_textless_slot()).flatten() {
            return Some(slot);
        }
        // Ordinary CIDs still needed at worst: this one, one per glyph not drawn yet (in a
        // subset; a font embedded whole draws those with their own GIDs) and the textless
        // slots the numbering may skip.
        let unseen = match self.mapping {
            Mapping::Subset(_) => {
                u32::from(self.glyph_count).saturating_sub(u32::try_from(self.by_glyph.len()).unwrap_or(u32::MAX))
            }
            Mapping::Whole => 0,
        };
        let skipped = if self.pool { TEXTLESS_LEN } else { 0 };
        if self.next_cid + 1 + unseen + skipped > 0x1_0000 {
            return None;
        }
        Some(self.next_ordinary())
    }

    /// The next ordinary CID, skipping the textless slots. The room check in
    /// [`FontUse::extra_cid`] (and, without the slots, a glyph count below 0x10000) keeps it
    /// below 0x10000.
    fn next_ordinary(&mut self) -> u16 {
        loop {
            let candidate = self.next_cid;
            self.next_cid += 1;
            let Ok(cid) = u16::try_from(candidate) else {
                // Unreachable (see above); the last CID rather than a wrap.
                return u16::MAX;
            };
            if !self.reserved(cid) {
                return cid;
            }
        }
    }

    /// The first textless slot no glyph has taken yet.
    fn free_textless_slot(&self) -> Option<u16> {
        TEXTLESS_CIDS
            .iter()
            .flat_map(Clone::clone)
            .find(|&cid| self.reserved(cid) && !self.cids.contains_key(&cid))
    }

    /// Whether `cid` is a textless slot ordinary CIDs skip.
    fn reserved(&self, cid: u16) -> bool {
        self.pool
            && is_textless_slot(cid)
            && match self.mapping {
                Mapping::Subset(_) => true,
                Mapping::Whole => cid >= self.glyph_count,
            }
    }

    fn bind(&mut self, cid: u16, gid: GlyphId) {
        self.cids.insert(cid, gid);
        self.by_glyph.entry(gid).or_default().push(cid);
    }

    /// CID → text, in CID order: what the ToUnicode CMap maps.
    #[must_use]
    pub fn text(&self) -> &BTreeMap<u16, String> {
        &self.text
    }

    /// The CIDs used, as (CID, original glyph ID) in CID order, `.notdef` first. A glyph
    /// drawn with several texts appears once per CID. For a font embedded whole, only the
    /// glyphs drawn (and `.notdef`) are listed.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(u16, GlyphId)> {
        self.cids.iter().map(|(&cid, &gid)| (cid, gid)).collect()
    }

    /// How many distinct glyphs are drawn, `.notdef` included.
    pub(crate) fn glyph_total(&self) -> usize {
        self.by_glyph.len()
    }

    /// Whether the font is embedded whole (its `fsType` forbids subsetting).
    #[must_use]
    pub fn is_whole(&self) -> bool {
        matches!(self.mapping, Mapping::Whole)
    }

    /// The glyph in the embedded program each CID draws, when some CID differs from it: the
    /// `/CIDToGIDMap` stream's content (two bytes per CID from 0 to the highest, big-endian,
    /// 0 for CIDs not used). `None` when every CID is its glyph's ID (`/Identity`).
    pub(crate) fn cid_to_gid_map(&self) -> Option<Vec<u8>> {
        let program_gid = |gid: GlyphId| match &self.mapping {
            Mapping::Subset(remapper) => remapper.get(gid.get()).unwrap_or(0),
            Mapping::Whole => gid.get(),
        };
        if self.cids.iter().all(|(&cid, &gid)| cid == program_gid(gid)) {
            return None;
        }
        let last = self.cids.keys().next_back().copied().unwrap_or(0);
        let mut map = vec![0u8; (usize::from(last) + 1) * 2];
        for (&cid, &gid) in &self.cids {
            let at = usize::from(cid) * 2;
            if let Some(slot) = map.get_mut(at..at + 2) {
                slot.copy_from_slice(&program_gid(gid).to_be_bytes());
            }
        }
        Some(map)
    }

    /// The remapper to subset with: the glyphs used in order of first use, or every glyph in
    /// order for a font embedded whole (so that new GID = old GID).
    pub(crate) fn remapper(&self) -> GlyphRemapper {
        match &self.mapping {
            Mapping::Subset(remapper) => remapper.clone(),
            Mapping::Whole => {
                let all: Vec<u16> = (0..self.glyph_count).collect();
                GlyphRemapper::new_from_glyphs_sorted(&all)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fonts::font;

    fn latin() -> (FontUse, GlyphId, GlyphId) {
        let f = font("NotoSans-Regular.ttf");
        let (a, b) = (f.glyph_for('a').unwrap(), f.glyph_for('b').unwrap());
        (FontUse::new(&f), a, b)
    }

    fn cid(usage: &mut FontUse, gid: GlyphId, text: &str) -> u16 {
        let (cid, warning) = usage.use_glyph(gid, text).unwrap();
        assert_eq!(warning, None);
        cid
    }

    #[test]
    fn one_text_per_glyph_keeps_cid_equal_to_the_subset_gid() {
        let (mut usage, a, b) = latin();
        let cids = [
            cid(&mut usage, a, "a"),
            cid(&mut usage, b, "b"),
            cid(&mut usage, a, "a"),
        ];
        assert_eq!((cids, usage.cid_to_gid_map()), ([1, 2, 1], None));
    }

    #[test]
    fn a_second_text_gets_the_next_cid_for_the_same_glyph() {
        let (mut usage, a, b) = latin();
        let cids = [
            cid(&mut usage, a, "a"),
            cid(&mut usage, a, "α"),
            cid(&mut usage, b, "b"),
        ];
        assert_eq!(
            (cids, usage.glyphs()),
            ([1, 2, 3], vec![(0, GlyphId::NOTDEF), (1, a), (2, a), (3, b)])
        );
    }

    #[test]
    fn the_map_sends_every_cid_to_its_subset_glyph() {
        let (mut usage, a, b) = latin();
        cid(&mut usage, a, "a");
        cid(&mut usage, a, "α");
        cid(&mut usage, b, "b");
        // CID 0 → 0, 1 → 1 (a), 2 → 1 (a again), 3 → 2 (b).
        assert_eq!(usage.cid_to_gid_map(), Some(vec![0, 0, 0, 1, 0, 1, 0, 2]));
    }

    #[test]
    fn a_textless_glyph_takes_a_format_character_cid() {
        let (mut usage, a, b) = latin();
        let cids = [cid(&mut usage, a, ""), cid(&mut usage, b, "b"), cid(&mut usage, a, "a")];
        assert_eq!(cids, [0x200B, 1, 2]);
    }

    #[test]
    fn textless_cids_are_used_once_each_in_order() {
        let (mut usage, a, b) = latin();
        cid(&mut usage, a, "a");
        cid(&mut usage, b, "b");
        let cids = [cid(&mut usage, a, ""), cid(&mut usage, b, ""), cid(&mut usage, a, "")];
        assert_eq!(cids, [0x200B, 0x200C, 0x200B]);
    }

    #[test]
    fn ordinary_cids_skip_the_textless_slots() {
        let (mut usage, _, _) = latin();
        usage.next_cid = 0x200A;
        assert_eq!([usage.next_ordinary(), usage.next_ordinary()], [0x200A, 0x2010]);
    }

    #[test]
    fn a_glyph_drawn_without_text_then_with_one_keeps_its_cid() {
        let (mut usage, a, _) = latin();
        let first = usage.glyph_cid(a).unwrap();
        assert_eq!((first, cid(&mut usage, a, "a"), usage.glyph_cid(a).unwrap()), (1, 1, 1));
    }

    #[test]
    fn a_textless_glyph_past_the_slots_is_not_exact() {
        // 29 textless slots; the 30th textless glyph gets an ordinary CID, which pdf.js
        // would read as its code: reported as not exact, for `/ActualText`.
        let f = font("NotoSans-Regular.ttf");
        let mut usage = FontUse::new(&f);
        let exact: Vec<bool> = (1..=30)
            .map(|g| usage.use_glyph_exact(GlyphId::new(g), "").unwrap().2)
            .collect();
        assert_eq!(
            (exact.iter().filter(|&&e| e).count(), exact.last().copied()),
            (29, Some(false))
        );
    }

    #[test]
    fn a_conflict_is_not_exact() {
        let (mut usage, a, _) = latin();
        usage.per_text = false;
        usage.use_glyph(a, "a").unwrap();
        let (_, warning, exact) = usage.use_glyph_exact(a, "α").unwrap();
        assert_eq!((warning.is_some(), exact), (true, false));
    }

    #[test]
    fn a_font_with_too_many_glyphs_for_the_slots_still_splits() {
        // As `new` sets it up for a 65 535-glyph font: no textless slots, CIDs still split
        // while there is room for every glyph not drawn yet.
        let (mut usage, a, _) = latin();
        usage.glyph_count = u16::MAX;
        usage.pool = false;
        cid(&mut usage, a, "a");
        let (second, warning, _) = usage.use_glyph_exact(a, "α").unwrap();
        assert_eq!((second, warning), (2, None));
    }

    #[test]
    fn a_textless_glyph_in_a_whole_font_takes_a_slot() {
        let (mut usage, a, _) = latin();
        usage.mapping = Mapping::Whole;
        usage.next_cid = u32::from(usage.glyph_count);
        assert_eq!(usage.use_glyph_exact(a, "").unwrap(), (0x200B, None, true));
    }

    #[test]
    fn textless_cids_are_format_characters() {
        // pdf.js skips a code whose fallback character has general category Cf.
        use icu_properties::props::{EnumeratedProperty as _, GeneralCategory};
        let cf = TEXTLESS_CIDS.iter().flat_map(Clone::clone).all(|c| {
            char::from_u32(u32::from(c)).is_some_and(|ch| GeneralCategory::for_char(ch) == GeneralCategory::Format)
        });
        assert!(cf);
    }
}
