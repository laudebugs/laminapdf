//! Embedding: the per-document [`FontRegistry`] reserves each font's object when content first
//! refers to it and writes the subsets at [`FontRegistry::finish_fonts`] ([ADR 0005] §6:
//! reserve, then fill).
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md

pub(crate) mod subset;

pub(crate) use subset::milli;
mod tounicode;
mod type0;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};

use lamina_cos::{CosMut, Document, ObjRef};

use crate::error::{Error, Result, Warning};
use crate::font::{Font, GlyphId};
use crate::fstype::EmbedPolicy;
use crate::usage::FontUse;

/// A font registered with a [`FontRegistry`]. Only meaningful to the registry that returned
/// it: a key carries its registry's identity, and any other registry refuses it with
/// [`Error::UnknownFont`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontKey {
    registry: u32,
    index: usize,
}

/// Registry identities, unique within the process. Only compared, never written to output.
static NEXT_REGISTRY: AtomicU32 = AtomicU32::new(0);

/// Text encoded for one font: the bytes to show (`Tj`/`TJ` operand, two bytes per glyph,
/// Identity-H) and their advance.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Encoded {
    /// The string operand: each glyph's CID, big-endian.
    pub bytes: Vec<u8>,
    /// The total advance in thousandths of the font size (glyph space units of `/W`), without
    /// character or word spacing.
    pub advance: f64,
}

/// What [`FontRegistry::finish_fonts`] embedded for one font.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EmbeddedFont {
    /// The font's key.
    pub key: FontKey,
    /// The Type 0 font dictionary (the object content streams refer to).
    pub object: ObjRef,
    /// `/BaseFont`: `ABCDEF+PostScriptName` for a subset, the PostScript name alone for a
    /// font embedded whole.
    pub base_font: String,
    /// The glyphs in the embedded program's `/W` and ToUnicode, `.notdef` included.
    pub glyphs: usize,
    /// The size of the font program before compression.
    pub program_len: usize,
    /// What embedding this font had to work around ([`Warning::WholeFontRebuilt`]).
    pub warnings: Vec<Warning>,
}

#[derive(Debug)]
struct Slot {
    font: Font,
    usage: FontUse,
    /// The Type 0 font dictionary, once reserved.
    object: Option<ObjRef>,
}

/// The fonts of one document under construction: each font's glyph usage, and the Type 0
/// font object content refers to, reserved at first use and written by
/// [`FontRegistry::finish_fonts`] once the subsets are known.
///
/// ```
/// # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
/// # fn empty_pdf() -> Vec<u8> {
/// #     use lamina_cos::{Dict, Object};
/// #     let mut w = lamina_save::Writer::new(1, 7);
/// #     let pages = w.add([("Type", Object::name("Pages")), ("Kids", Object::Array(vec![])), ("Count", 0.into())].into_iter().collect::<Dict>());
/// #     let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// #     w.finish(&[("Root", root)].into_iter().collect()).unwrap()
/// # }
/// use lamina_text::{EmbedPolicy, Font, FontRegistry};
///
/// let mut doc = lamina_cos::Document::open(empty_pdf(), Default::default())?;
/// let mut fonts = FontRegistry::new();
/// let key = fonts.add(Font::from_bytes(font_bytes())?, EmbedPolicy::Respect)?;
///
/// // While writing content: the font resource's object, and the bytes to show.
/// let font_ref = fonts.reference(key, &doc)?;
/// let hello = fonts.encode(key, "Hello")?;
/// assert_eq!(hello.bytes.len(), 10);
///
/// // At the end: subset, and fill the reserved object.
/// let embedded = fonts.finish_fonts(&mut doc.edit())?;
/// assert_eq!(embedded[0].object, font_ref);
/// assert!(embedded[0].base_font.ends_with("+NotoSans-Regular"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct FontRegistry {
    id: u32,
    slots: Vec<Slot>,
    warnings: Vec<Warning>,
}

impl Default for FontRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FontRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: NEXT_REGISTRY.fetch_add(1, Ordering::Relaxed),
            slots: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn slot(&self, key: FontKey) -> Result<&Slot> {
        self.slots
            .get(key.index)
            .filter(|_| key.registry == self.id)
            .ok_or(Error::UnknownFont { key: key.index })
    }

    fn slot_mut(&mut self, key: FontKey) -> Result<&mut Slot> {
        let id = self.id;
        self.slots
            .get_mut(key.index)
            .filter(|_| key.registry == id)
            .ok_or(Error::UnknownFont { key: key.index })
    }

    /// Registers `font`. Its `fsType` is checked now, so a font that may not be embedded fails
    /// before any content uses it. Registering the same font twice gives two independent
    /// subsets.
    ///
    /// # Errors
    /// [`Error::FontNotEmbeddable`] when `fsType` forbids embedding (restricted licence,
    /// bitmap only) and `policy` is [`EmbedPolicy::Respect`].
    pub fn add(&mut self, font: Font, policy: EmbedPolicy) -> Result<FontKey> {
        let info = font.info();
        if policy == EmbedPolicy::Respect && !info.fs_type.allows_embedding() {
            return Err(Error::FontNotEmbeddable {
                font: info.postscript_name.clone(),
                fs_type: info.fs_type,
            });
        }
        let usage = FontUse::new(&font);
        self.slots.push(Slot {
            font,
            usage,
            object: None,
        });
        Ok(FontKey {
            registry: self.id,
            index: self.slots.len() - 1,
        })
    }

    /// The font registered as `key`.
    #[must_use]
    pub fn font(&self, key: FontKey) -> Option<&Font> {
        self.slot(key).ok().map(|s| &s.font)
    }

    /// The font registered as `key`, or [`Error::UnknownFont`].
    pub(crate) fn try_font(&self, key: FontKey) -> Result<&Font> {
        self.slot(key).map(|s| &s.font)
    }

    /// The glyph usage recorded for `key` so far.
    #[must_use]
    pub fn usage(&self, key: FontKey) -> Option<&FontUse> {
        self.slot(key).ok().map(|s| &s.usage)
    }

    /// The Type 0 font object for `key`, to put in a page's `/Resources /Font`. The first call
    /// reserves a fresh object number in `doc` ([`Document::reserve_number`]); the object is
    /// written by [`FontRegistry::finish_fonts`].
    ///
    /// # Errors
    /// [`Error::UnknownFont`]; as [`Document::reserve_number`] when numbers run out.
    pub fn reference(&mut self, key: FontKey, doc: &Document) -> Result<ObjRef> {
        let slot = self.slot_mut(key)?;
        if let Some(r) = slot.object {
            return Ok(r);
        }
        let r = doc.reserve_number()?;
        slot.object = Some(r);
        Ok(r)
    }

    /// Records that glyph `gid` of `key` is drawn standing for `text` (for shaped text: the
    /// cluster's characters on its first glyph, empty on the others) and returns its CID. A
    /// conflicting text is kept as a warning ([`FontRegistry::warnings`]).
    ///
    /// # Errors
    /// [`Error::UnknownFont`]; [`Error::GlyphOutOfRange`].
    pub fn use_glyph(&mut self, key: FontKey, gid: GlyphId, text: &str) -> Result<u16> {
        let slot = self.slot_mut(key)?;
        let (cid, warning) = slot.usage.use_glyph(gid, text)?;
        self.warnings.extend(warning);
        Ok(cid)
    }

    /// The CID of glyph `gid` of `key`, recorded as used without claiming any text
    /// ([`FontUse`]'s `glyph_cid`, B4b).
    pub(crate) fn glyph_cid(&mut self, key: FontKey, gid: GlyphId) -> Result<u16> {
        self.slot_mut(key)?.usage.glyph_cid(gid)
    }

    /// Encodes `text` one glyph per character through the font's `cmap` (no shaping: ligatures
    /// and complex scripts need B4b's shaper, which calls [`FontRegistry::use_glyph`]).
    ///
    /// # Errors
    /// [`Error::UnknownFont`]; [`Error::MissingGlyph`] for the first character the font has no
    /// glyph for (nothing is recorded for `text` then).
    pub fn encode(&mut self, key: FontKey, text: &str) -> Result<Encoded> {
        let id = self.id;
        let Self { slots, warnings, .. } = self;
        let slot = slots
            .get_mut(key.index)
            .filter(|_| key.registry == id)
            .ok_or(Error::UnknownFont { key: key.index })?;
        // Every glyph is looked up (and checked) before any is recorded.
        let glyphs = slot.font.map_text(text)?;
        let scale = 1000.0 / f64::from(slot.font.info().units_per_em);
        let mut bytes = Vec::with_capacity(glyphs.len() * 2);
        let mut advance = 0.0;
        let mut buf = [0u8; 4];
        for (gid, ch, width) in glyphs {
            let (cid, warning) = slot.usage.use_glyph(gid, ch.encode_utf8(&mut buf))?;
            warnings.extend(warning);
            bytes.extend_from_slice(&cid.to_be_bytes());
            advance += f64::from(width) * scale;
        }
        Ok(Encoded { bytes, advance })
    }

    /// Warnings raised so far (ToUnicode conflicts).
    #[must_use]
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    /// Subsets every font that was [referenced](FontRegistry::reference) and writes it into the
    /// document: the font program (Flate), its descriptor, the CIDFont with `/W`, the ToUnicode
    /// CMap as new objects, and the reserved Type 0 dictionary. Fonts are written in the order
    /// they were registered, so the output is deterministic. A font that was registered but
    /// never referenced writes nothing.
    ///
    /// # Errors
    /// [`Error::WrongDocument`] (before writing anything) when a reserved object is not a free
    /// number reserved in this document; [`Error::Subset`] when the subsetter fails; as
    /// [`CosMut::create`] / [`CosMut::set`].
    /// Objects written before the error stay in the overlay: take a
    /// [`Document::journal_mark`] first to roll back. The registry is consumed: read its
    /// [`FontRegistry::warnings`] before.
    pub fn finish_fonts(self, cos: &mut CosMut<'_>) -> Result<Vec<EmbeddedFont>> {
        // Every reserved object must still be a free number this document reserved, or the
        // registry was used with another document: check all before writing any.
        for object in self.slots.iter().filter_map(|s| s.object) {
            let doc = cos.document();
            if !doc.is_reserved_number(object.num) || cos.get(object)?.is_some() {
                return Err(Error::WrongDocument { object: object.num });
            }
        }
        let mut tags = BTreeSet::new();
        let mut embedded = Vec::new();
        for (index, slot) in self.slots.iter().enumerate() {
            let Some(object) = slot.object else {
                continue;
            };
            let glyphs = slot.usage.glyphs();
            let (program, warning) = subset::build(&slot.font, &slot.usage, &glyphs)?;
            let name = &slot.font.info().postscript_name;
            let base_font = if slot.usage.is_whole() {
                name.clone()
            } else {
                let tag = subset::unique_tag(&tags, name, |salt| subset::subset_tag(&slot.font, &glyphs, salt))?;
                tags.insert(tag.clone());
                format!("{tag}+{name}")
            };
            type0::write_font(
                cos,
                &type0::FontObjects {
                    font: &slot.font,
                    usage: &slot.usage,
                    program: &program,
                    base_font: &base_font,
                    type0: object,
                },
            )?;
            embedded.push(EmbeddedFont {
                key: FontKey {
                    registry: self.id,
                    index,
                },
                object,
                base_font,
                glyphs: glyphs.len(),
                program_len: program.data().len(),
                warnings: warning.into_iter().collect(),
            });
        }
        Ok(embedded)
    }
}
