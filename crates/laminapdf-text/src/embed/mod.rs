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

use laminapdf_cos::{CosMut, Document, ObjRef};

pub(crate) use subset::round_i64;
pub(crate) use type0::real;

use crate::error::{Error, Result, Warning};
use crate::font::{Font, GlyphId};
use crate::fstype::EmbedPolicy;
use crate::simple::SimpleUse;
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

/// Text encoded for one font: the bytes to show (`Tj`/`TJ` operand: two bytes per glyph,
/// Identity-H, for an embedded font; one byte per glyph for a standard 14 font) and their
/// advance.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Encoded {
    /// The string operand: each glyph's CID, big-endian (embedded fonts), or each glyph's
    /// one-byte code (standard 14 fonts).
    pub bytes: Vec<u8>,
    /// The total advance in thousandths of the font size (glyph space units of `/W`), without
    /// character or word spacing.
    pub advance: f64,
}

/// What [`FontRegistry::finish_fonts`] wrote for one font: an embedded Type 0 font, or a
/// standard 14 font written as a simple font without a program (`embedded` is `false`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EmbeddedFont {
    /// The font's key.
    pub key: FontKey,
    /// The font dictionary content streams refer to: Type 0, or `/Type1` for a standard 14
    /// font.
    pub object: ObjRef,
    /// Whether a font program was embedded (`false` for a standard 14 font).
    pub embedded: bool,
    /// `/BaseFont`: `ABCDEF+PostScriptName` for a subset, the PostScript name alone for a
    /// font embedded whole.
    pub base_font: String,
    /// The distinct glyphs drawn, `.notdef` included (a glyph drawn with several texts has
    /// several CIDs but counts once); for a standard 14 font, the distinct codes drawn.
    pub glyphs: usize,
    /// The size of the font program before compression (0 when not `embedded`).
    pub program_len: usize,
    /// What embedding this font had to work around ([`Warning::WholeFontRebuilt`]).
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone)]
struct Slot {
    font: Font,
    usage: Usage,
    /// The font dictionary (Type 0, or `/Type1` for a standard 14 font), once reserved.
    object: Option<ObjRef>,
}

/// The code [`FontRegistry::use_glyph`] returns: what to put in the `Tj`/`TJ` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GlyphCode {
    /// A CID of an embedded (Type 0, Identity-H) font: two bytes, big-endian.
    Cid(u16),
    /// A code of a standard 14 (simple) font: one byte.
    Byte(u8),
}

impl GlyphCode {
    /// Appends the code's bytes to a string operand.
    pub fn push_to(self, out: &mut Vec<u8>) {
        match self {
            Self::Cid(cid) => out.extend_from_slice(&cid.to_be_bytes()),
            Self::Byte(code) => out.push(code),
        }
    }

    /// The CID, for an embedded font.
    #[must_use]
    pub const fn cid(self) -> Option<u16> {
        match self {
            Self::Cid(cid) => Some(cid),
            Self::Byte(_) => None,
        }
    }
}

/// What [`FontRegistry::usage`] reports for one font.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum FontUsage<'a> {
    /// An embedded font: its glyphs and their CIDs.
    Embedded(&'a FontUse),
    /// A standard 14 font: its codes, `/Differences` and texts.
    Simple(&'a SimpleUse),
}

impl<'a> FontUsage<'a> {
    /// The usage of an embedded font.
    #[must_use]
    pub const fn embedded(self) -> Option<&'a FontUse> {
        match self {
            Self::Embedded(usage) => Some(usage),
            Self::Simple(_) => None,
        }
    }

    /// The usage of a standard 14 font.
    #[must_use]
    pub const fn simple(self) -> Option<&'a SimpleUse> {
        match self {
            Self::Simple(usage) => Some(usage),
            Self::Embedded(_) => None,
        }
    }
}

/// How one font is written.
#[derive(Debug, Clone)]
enum Usage {
    /// An embedded font: a Type 0 font over a subset.
    Embedded(FontUse),
    /// A standard 14 font: a simple, non-embedded Type 1 font.
    Simple(SimpleUse),
}

/// The fonts of one document under construction: each font's glyph usage, and the Type 0
/// font object content refers to, reserved at first use and written by
/// [`FontRegistry::finish_fonts`] once the subsets are known.
///
/// ```
/// # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
/// # fn empty_pdf() -> Vec<u8> {
/// #     use laminapdf_cos::{Dict, Object};
/// #     let mut w = laminapdf_save::Writer::new(1, 7);
/// #     let pages = w.add([("Type", Object::name("Pages")), ("Kids", Object::Array(vec![])), ("Count", 0.into())].into_iter().collect::<Dict>());
/// #     let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// #     w.finish(&[("Root", root)].into_iter().collect()).unwrap()
/// # }
/// use laminapdf_text::{EmbedPolicy, Font, FontRegistry};
///
/// let mut doc = laminapdf_cos::Document::open(empty_pdf(), Default::default())?;
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
///
/// Cloning snapshots the registry (same identity, so its keys stay valid): the builder keeps
/// one to restore when a page fails half-way, so nothing that page recorded is written.
#[derive(Debug, Clone)]
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
        let usage = match font.standard14() {
            Some(standard) => Usage::Simple(SimpleUse::new(standard)),
            None => Usage::Embedded(FontUse::new(&font)),
        };
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

    /// The glyph or code usage recorded for `key` so far.
    ///
    /// # Errors
    /// [`Error::UnknownFont`] for a key this registry did not hand out.
    pub fn usage(&self, key: FontKey) -> Result<FontUsage<'_>> {
        Ok(match &self.slot(key)?.usage {
            Usage::Embedded(usage) => FontUsage::Embedded(usage),
            Usage::Simple(usage) => FontUsage::Simple(usage),
        })
    }

    /// The font object for `key` (Type 0, or `/Type1` for a standard 14 font), to put in a
    /// page's `/Resources /Font`. The first call
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
    /// cluster's characters on its first glyph, empty on the others) and returns the code to
    /// show it with: a two-byte CID for an embedded font, a one-byte code for a standard 14
    /// font ([`GlyphCode::push_to`] writes either). The text goes into the font's ToUnicode
    /// CMap; a conflicting text is kept as a warning ([`FontRegistry::warnings`]).
    ///
    /// # Errors
    /// [`Error::UnknownFont`]; [`Error::GlyphOutOfRange`] (for a standard 14 font, also
    /// `.notdef`); [`Error::EncodingFull`] for a standard 14 font with no code left for the
    /// glyph.
    pub fn use_glyph(&mut self, key: FontKey, gid: GlyphId, text: &str) -> Result<GlyphCode> {
        let id = self.id;
        let Self { slots, warnings, .. } = self;
        let slot = slots
            .get_mut(key.index)
            .filter(|_| key.registry == id)
            .ok_or(Error::UnknownFont { key: key.index })?;
        let (code, warning) = match &mut slot.usage {
            Usage::Embedded(usage) => {
                let (cid, warning) = usage.use_glyph(gid, text)?;
                (GlyphCode::Cid(cid), warning)
            }
            Usage::Simple(usage) => {
                let (code, warning) = usage.use_glyph(gid, text)?;
                (GlyphCode::Byte(code), warning)
            }
        };
        warnings.extend(warning);
        Ok(code)
    }

    /// [`FontRegistry::use_glyph`] for an embedded font, also saying whether the CID extracts
    /// as `text` exactly (`FontUse`'s `use_glyph_exact`): `pdf_run` marks the cluster with
    /// `/ActualText` otherwise.
    ///
    /// # Errors
    /// As [`FontRegistry::use_glyph`]; [`Error::FontNotShapable`] for a standard 14 font.
    pub(crate) fn use_glyph_exact(&mut self, key: FontKey, gid: GlyphId, text: &str) -> Result<(u16, bool)> {
        let id = self.id;
        let Self { slots, warnings, .. } = self;
        let slot = slots
            .get_mut(key.index)
            .filter(|_| key.registry == id)
            .ok_or(Error::UnknownFont { key: key.index })?;
        match &mut slot.usage {
            Usage::Embedded(usage) => {
                let (cid, warning, exact) = usage.use_glyph_exact(gid, text)?;
                if let Some(warning) = warning {
                    warnings.push(warning);
                }
                Ok((cid, exact))
            }
            Usage::Simple(_) => Err(Error::FontNotShapable {
                font: slot.font.info().postscript_name.clone(),
            }),
        }
    }

    /// The CID of glyph `gid` of `key`, recorded as used without claiming any text
    /// ([`FontUse`]'s `glyph_cid`, B4b).
    ///
    /// # Errors
    /// [`Error::UnknownFont`]; [`Error::GlyphOutOfRange`]; [`Error::FontNotShapable`] for a
    /// standard 14 font, which has no CIDs (shaped runs never come from one).
    pub(crate) fn glyph_cid(&mut self, key: FontKey, gid: GlyphId) -> Result<u16> {
        let slot = self.slot_mut(key)?;
        match &mut slot.usage {
            Usage::Embedded(usage) => usage.glyph_cid(gid),
            Usage::Simple(_) => Err(Error::FontNotShapable {
                font: slot.font.info().postscript_name.clone(),
            }),
        }
    }

    /// Encodes `text` one glyph per character through the font's `cmap` (no shaping: ligatures
    /// and complex scripts need B4b's shaper, which calls [`FontRegistry::use_glyph`]). A
    /// standard 14 font maps characters to glyphs by name (Adobe Glyph List) and writes one
    /// byte per character.
    ///
    /// # Errors
    /// [`Error::UnknownFont`]; [`Error::MissingGlyph`] for the first character the font has no
    /// glyph for (nothing is recorded for `text` then); [`Error::EncodingFull`] when a
    /// standard 14 font has no code left for a glyph outside its encoding (nothing recorded).
    pub fn encode(&mut self, key: FontKey, text: &str) -> Result<Encoded> {
        let id = self.id;
        let Self { slots, warnings, .. } = self;
        let slot = slots
            .get_mut(key.index)
            .filter(|_| key.registry == id)
            .ok_or(Error::UnknownFont { key: key.index })?;
        let usage = match &mut slot.usage {
            Usage::Embedded(usage) => usage,
            Usage::Simple(usage) => {
                let (encoded, warning) = usage.encode(&slot.font, text)?;
                warnings.extend(warning);
                return Ok(encoded);
            }
        };
        // Every glyph is looked up (and checked) before any is recorded.
        let glyphs = slot.font.map_text(text)?;
        let scale = 1000.0 / f64::from(slot.font.info().units_per_em);
        let mut bytes = Vec::with_capacity(glyphs.len() * 2);
        let mut advance = 0.0;
        let mut buf = [0u8; 4];
        for (gid, ch, width) in glyphs {
            let (cid, warning) = usage.use_glyph(gid, ch.encode_utf8(&mut buf))?;
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
    /// CMap as new objects, and the reserved Type 0 dictionary. A standard 14 font is written
    /// as a `/Type1` font dictionary with `/Encoding`, `/FirstChar`, `/LastChar`, `/Widths` and
    /// a font descriptor without a font program. Fonts are written in the order
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
            let key = FontKey {
                registry: self.id,
                index,
            };
            let usage = match &slot.usage {
                Usage::Embedded(usage) => usage,
                Usage::Simple(usage) => {
                    usage.write(cos, &slot.font, object)?;
                    embedded.push(EmbeddedFont {
                        key,
                        object,
                        embedded: false,
                        base_font: slot.font.info().postscript_name.clone(),
                        glyphs: usage.code_count(),
                        program_len: 0,
                        warnings: Vec::new(),
                    });
                    continue;
                }
            };
            let glyphs = usage.glyphs();
            let (program, warning) = subset::build(&slot.font, usage, &glyphs)?;
            let name = &slot.font.info().postscript_name;
            let base_font = if usage.is_whole() {
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
                    usage,
                    program: &program,
                    base_font: &base_font,
                    type0: object,
                },
            )?;
            embedded.push(EmbeddedFont {
                key,
                object,
                embedded: true,
                base_font,
                glyphs: usage.glyph_total(),
                program_len: program.data().len(),
                warnings: warning.into_iter().collect(),
            });
        }
        Ok(embedded)
    }
}
