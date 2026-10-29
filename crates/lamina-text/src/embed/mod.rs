//! Embedding: the per-document [`FontRegistry`] reserves each font's object when content first
//! refers to it and writes the subsets at [`FontRegistry::finish_fonts`] ([ADR 0005] §6:
//! reserve, then fill).
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md

mod subset;
mod tounicode;
mod type0;

use std::collections::BTreeSet;

use lamina_cos::{CosMut, Document, ObjRef};

use crate::error::{Error, Result, Warning};
use crate::font::{Font, GlyphId};
use crate::fstype::EmbedPolicy;
use crate::usage::FontUse;

/// A font registered with a [`FontRegistry`]. Only meaningful to the registry that returned
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontKey(usize);

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
#[derive(Debug, Default)]
pub struct FontRegistry {
    slots: Vec<Slot>,
    warnings: Vec<Warning>,
}

impl FontRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
        Ok(FontKey(self.slots.len() - 1))
    }

    /// The font registered as `key`.
    #[must_use]
    pub fn font(&self, key: FontKey) -> Option<&Font> {
        self.slots.get(key.0).map(|s| &s.font)
    }

    /// The glyph usage recorded for `key` so far.
    #[must_use]
    pub fn usage(&self, key: FontKey) -> Option<&FontUse> {
        self.slots.get(key.0).map(|s| &s.usage)
    }

    /// The Type 0 font object for `key`, to put in a page's `/Resources /Font`. The first call
    /// reserves a fresh object number in `doc` ([`Document::reserve_number`]); the object is
    /// written by [`FontRegistry::finish_fonts`].
    ///
    /// # Errors
    /// [`Error::UnknownFont`]; as [`Document::reserve_number`] when numbers run out.
    pub fn reference(&mut self, key: FontKey, doc: &Document) -> Result<ObjRef> {
        let slot = self.slots.get_mut(key.0).ok_or(Error::UnknownFont { key: key.0 })?;
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
        let slot = self.slots.get_mut(key.0).ok_or(Error::UnknownFont { key: key.0 })?;
        let (cid, warning) = slot.usage.use_glyph(&slot.font, gid, text)?;
        self.warnings.extend(warning);
        Ok(cid)
    }

    /// Encodes `text` one glyph per character through the font's `cmap` (no shaping: ligatures
    /// and complex scripts need B4b's shaper, which calls [`FontRegistry::use_glyph`]).
    ///
    /// # Errors
    /// [`Error::UnknownFont`]; [`Error::MissingGlyph`] for the first character the font has no
    /// glyph for (nothing is recorded for `text` then).
    pub fn encode(&mut self, key: FontKey, text: &str) -> Result<Encoded> {
        let slot = self.slots.get(key.0).ok_or(Error::UnknownFont { key: key.0 })?;
        let font = slot.font.clone();
        let mut glyphs = Vec::with_capacity(text.len());
        for ch in text.chars() {
            let gid = font.glyph_for(ch).ok_or_else(|| Error::MissingGlyph {
                ch,
                font: font.info().postscript_name.clone(),
            })?;
            glyphs.push((gid, ch));
        }
        let scale = 1000.0 / f64::from(font.info().units_per_em);
        let mut bytes = Vec::with_capacity(glyphs.len() * 2);
        let mut advance = 0.0;
        let mut buf = [0u8; 4];
        for (gid, ch) in glyphs {
            let cid = self.use_glyph(key, gid, ch.encode_utf8(&mut buf))?;
            bytes.extend_from_slice(&cid.to_be_bytes());
            advance += f64::from(font.advance(gid).unwrap_or(0.0)) * scale;
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
    /// [`Error::Subset`] when the subsetter fails; as [`CosMut::create`] / [`CosMut::set`].
    /// Objects written before the error stay in the overlay: take a
    /// [`Document::journal_mark`] first to roll back. The registry is consumed: read its
    /// [`FontRegistry::warnings`] before.
    pub fn finish_fonts(self, cos: &mut CosMut<'_>) -> Result<Vec<EmbeddedFont>> {
        let mut tags = BTreeSet::new();
        let mut embedded = Vec::new();
        for (index, slot) in self.slots.iter().enumerate() {
            let Some(object) = slot.object else {
                continue;
            };
            let program = subset::subset(&slot.font, &slot.usage)?;
            let name = &slot.font.info().postscript_name;
            let base_font = if slot.usage.is_whole() {
                name.clone()
            } else {
                // A collision needs two subsets hashing to the same six letters (1 in 26^6),
                // so this ends after one or two rounds; it is bounded by the font count.
                let tag = (0..=u32::try_from(self.slots.len()).unwrap_or(u32::MAX))
                    .map(|salt| subset::subset_tag(&slot.font, &slot.usage, salt))
                    .find(|tag| !tags.contains(tag))
                    .unwrap_or_default();
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
                key: FontKey(index),
                object,
                base_font,
                glyphs: slot.usage.glyphs().len(),
                program_len: program.data.len(),
            });
        }
        Ok(embedded)
    }
}
