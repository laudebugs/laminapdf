//! Fonts loaded from caller-supplied bytes: single fonts and collection members (`.ttc`,
//! `.otc`), TrueType or CFF outlines, static or variable (research 03 §3.6, §6.1).

use std::fmt;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, OnceLock};

use sha2::{Digest, Sha256};
use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::raw::{FileRef, TableProvider};
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider};

use crate::error::{Error, Result};
use crate::fstype::FsType;
use crate::std14::Standard14;

/// A four-byte OpenType tag, such as the variation axis `wght`.
///
/// Added in 0.2.0.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpenTypeTag([u8; 4]);

impl OpenTypeTag {
    /// The weight axis, `wght`.
    ///
    /// Added in 0.2.0.
    pub const WEIGHT: Self = Self(*b"wght");
    /// The width axis, `wdth`.
    ///
    /// Added in 0.2.0.
    pub const WIDTH: Self = Self(*b"wdth");

    /// A tag from its four bytes.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(bytes: &[u8; 4]) -> Self {
        Self(*bytes)
    }

    /// The tag's four bytes.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 4] {
        self.0
    }

    pub(crate) fn to_skrifa(self) -> skrifa::Tag {
        skrifa::Tag::new(&self.0)
    }
}

impl fmt::Display for OpenTypeTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
    }
}

impl fmt::Debug for OpenTypeTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OpenTypeTag({self})")
    }
}

/// A glyph ID in a font's own numbering (an index into `glyf`/`loca` or the CFF
/// `CharStrings` INDEX).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GlyphId(u16);

impl GlyphId {
    /// `.notdef`, glyph 0.
    ///
    /// Added in 0.2.0.
    pub const NOTDEF: Self = Self(0);

    /// A glyph ID.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// The numeric ID.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A variation axis of a variable font (`fvar`), in user units.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Axis {
    /// The axis tag, such as `wght`.
    pub tag: OpenTypeTag,
    /// The smallest value.
    pub min: f32,
    /// The default value.
    pub default: f32,
    /// The largest value.
    pub max: f32,
}

/// What a font says about itself, at its instance's location for a variable font. Lengths are
/// in font units (see [`FontInfo::units_per_em`]); y grows upwards.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FontInfo {
    /// The family name (typographic family, `name` ID 16, else ID 1).
    pub family: String,
    /// The PostScript name (`name` ID 6), restricted to the characters a PDF name may hold
    /// unescaped. For a variable-font instance the axis values are appended
    /// (`NotoSans-Regular_wght700`), so instances of one family have distinct names
    /// (research 03 §7.9). At most [`MAX_NAME_LEN`] characters, so that with a subset tag
    /// (`ABCDEF+`) `/BaseFont` stays within PostScript's 63; a longer name is cut and ends
    /// with `_` and eight hex digits of its SHA-256, so cut names stay distinct.
    pub postscript_name: String,
    /// Font units per em (`head.unitsPerEm`).
    pub units_per_em: u16,
    /// The number of glyphs (`maxp.numGlyphs`).
    pub glyph_count: u16,
    /// Ascender, written as `/Ascent`: `OS/2.sTypoAscender` when the font sets
    /// `USE_TYPO_METRICS`, else `hhea.ascender` (`FreeType`'s choice, which `skrifa` follows).
    pub ascent: f32,
    /// Descender (negative below the baseline), written as `/Descent`: `OS/2.sTypoDescender`
    /// or `hhea.descender`, as for [`FontInfo::ascent`].
    pub descent: f32,
    /// Capital height, when the font records it (`OS/2` version 2 and later).
    pub cap_height: Option<f32>,
    /// x-height, when the font records it.
    pub x_height: Option<f32>,
    /// Italic angle in degrees, counter-clockwise from vertical (`post.italicAngle`).
    pub italic_angle: f32,
    /// Weight class, 1–1000 (`OS/2.usWeightClass`, or the `wght` coordinate of an instance).
    pub weight: u16,
    /// The font bounding box `[x_min, y_min, x_max, y_max]` (`head`, which variations don't
    /// change: an instance's embedded `/FontBBox` is computed from its instanced outlines).
    pub bbox: [f32; 4],
    /// The font descriptor flags (ISO 32000-2 §9.8.2): `FixedPitch` (bit
    /// 1), `Symbolic` (bit 3: always set for a loaded font, as a CIDFont's glyphs are addressed
    /// by CID, not by the standard Latin encoding) and `Italic` (bit 7). A standard 14 font
    /// sets `Serif` (bit 2) for Times, `Symbolic` for Symbol and `ZapfDingbats` and `Nonsymbolic`
    /// (bit 6) for the others.
    pub flags: u32,
    /// CFF (or CFF2) outlines, as opposed to TrueType `glyf` outlines.
    pub is_cff: bool,
    /// The font has variation axes (`fvar`).
    pub is_variable: bool,
    /// The variation axes, in `fvar` order (empty for a static font).
    pub axes: Vec<Axis>,
    /// The embedding permissions (`OS/2.fsType`).
    pub fs_type: FsType,
}

/// The longest [`FontInfo::postscript_name`]: 63 (the PostScript name limit) minus the
/// seven characters of a subset tag.
///
/// Added in 0.2.0.
pub const MAX_NAME_LEN: usize = 56;

/// The bytes a font was loaded from, shared by every instance of it.
struct Source {
    bytes: Arc<[u8]>,
    index: u32,
    /// SHA-256 of `bytes`, computed when a subset tag first needs it.
    digest: OnceLock<[u8; 32]>,
    /// HarfRust's tables for the face, built when it is first shaped.
    shaping: OnceLock<harfrust::ShaperData>,
    /// HarfRust's shape plans (the lookups and feature masks for one direction, script,
    /// language, feature list and set of feature variations), built at first use and reused,
    /// because building one costs more than shaping a short run with it. Least recently used
    /// first.
    plans: Mutex<Vec<Arc<harfrust::ShapePlan>>>,
}

/// The most shape plans one face keeps; past it, the least recently used one is dropped.
pub(crate) const MAX_PLANS: usize = 64;

/// A font loaded from caller-supplied bytes (for a variable font, one instance of it), or one
/// of the [standard 14](Standard14) referred to by name. Cheap to clone (the bytes are
/// shared). `Send + Sync`.
///
/// ```
/// # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
/// use laminapdf_text::Font;
///
/// let font = Font::from_bytes(font_bytes())?;
/// assert_eq!(font.info().postscript_name, "NotoSans-Regular");
/// assert!(font.glyph_for('A').is_some());
/// # Ok::<(), laminapdf_text::Error>(())
/// ```
///
/// Added in 0.2.0.
#[derive(Clone)]
pub struct Font {
    source: Arc<Source>,
    /// The instance's user coordinates, sorted by tag (empty: the default instance).
    coords: Arc<[(OpenTypeTag, f32)]>,
    /// The same, normalised, in `fvar` axis order (empty: the default instance).
    location: Arc<[NormalizedCoord]>,
    info: Arc<FontInfo>,
    /// A standard 14 font: no program (its `source` bytes are empty), metrics from its AFM.
    standard: Option<Standard14>,
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font")
            .field("postscript_name", &self.info.postscript_name)
            .field("index", &self.source.index)
            .field("coords", &self.coords)
            .finish_non_exhaustive()
    }
}

impl Font {
    /// Loads a font (`.ttf`, `.otf`); for a collection, its first member.
    ///
    /// # Errors
    /// [`Error::InvalidFont`] when the bytes are not a readable font.
    ///
    /// Added in 0.2.0.
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Result<Self> {
        Self::from_collection(bytes, 0)
    }

    /// Loads member `index` of a font collection (`.ttc`, `.otc`); index 0 of a single font
    /// is the font itself.
    ///
    /// # Errors
    /// [`Error::InvalidFont`] when the bytes are not a readable font or collection;
    /// [`Error::CollectionIndex`] when `index` is past its last member.
    ///
    /// Added in 0.2.0.
    pub fn from_collection(bytes: impl Into<Arc<[u8]>>, index: u32) -> Result<Self> {
        let bytes: Arc<[u8]> = bytes.into();
        let count = Self::collection_len(&bytes).map_err(|_| Error::InvalidFont { index })?;
        if index >= count {
            return Err(Error::CollectionIndex { index, count });
        }
        let source = Arc::new(Source {
            bytes,
            index,
            digest: OnceLock::new(),
            shaping: OnceLock::new(),
            plans: Mutex::new(Vec::new()),
        });
        let font = FontRef::from_index(&source.bytes, index).map_err(|_| Error::InvalidFont { index })?;
        let info = font_info(&font, &[], LocationRef::default()).ok_or(Error::InvalidFont { index })?;
        Ok(Self {
            source: Arc::clone(&source),
            coords: Arc::from([]),
            location: Arc::from([]),
            info: Arc::new(info),
            standard: None,
        })
    }

    /// One of the standard 14 fonts, which every PDF reader provides (ISO 32000-2 §9.6.2.2):
    /// no font bytes needed, metrics from Adobe's AFM files, never embedded. Text is written
    /// as one byte per character: `WinAnsiEncoding` for the Latin fonts (glyphs of the
    /// standard Latin character set outside it through `/Differences`), the built-in encoding
    /// for Symbol and `ZapfDingbats`. PDF/A and PDF/UA require embedded fonts, so their profiles
    /// reject these ([`Font::requires_embedding`]).
    ///
    /// ```
    /// use laminapdf_text::{Font, Standard14};
    ///
    /// let font = Font::standard(Standard14::Helvetica);
    /// assert_eq!(font.info().postscript_name, "Helvetica");
    /// assert!(font.standard14().is_some() && !font.requires_embedding());
    /// assert!(font.glyph_for('é').is_some());
    /// assert!(font.glyph_for('中').is_none());
    /// ```
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn standard(font: Standard14) -> Self {
        let m = font.metrics();
        let bbox = m.bbox.map(f32::from);
        let [_, y_min, _, y_max] = bbox;
        let italic = m.italic_angle != 0.0;
        // ISO 32000-2 §9.8.2: bit 1 FixedPitch, 2 Serif, 3 Symbolic, 6 Nonsymbolic, 7 Italic.
        let flags = u32::from(m.is_fixed_pitch)
            | if m.family_name == "Times" { 2 } else { 0 }
            | if font.is_symbolic() { 4 } else { 32 }
            | if italic { 64 } else { 0 };
        let info = FontInfo {
            family: m.family_name.to_owned(),
            postscript_name: m.font_name.to_owned(),
            units_per_em: 1000,
            // `.notdef` and one glyph per name.
            glyph_count: font.glyph_count(),
            ascent: m.ascender.map_or(y_max, f32::from),
            descent: m.descender.map_or(y_min, f32::from),
            cap_height: m.cap_height.map(f32::from),
            x_height: m.x_height.map(f32::from),
            italic_angle: m.italic_angle,
            weight: if m.weight == "Bold" { 700 } else { 400 },
            bbox,
            flags,
            is_cff: false,
            is_variable: false,
            axes: Vec::new(),
            fs_type: FsType::new(0),
        };
        Self {
            source: Arc::new(Source {
                bytes: Arc::from([]),
                index: 0,
                digest: OnceLock::new(),
                shaping: OnceLock::new(),
                plans: Mutex::new(Vec::new()),
            }),
            coords: Arc::from([]),
            location: Arc::from([]),
            info: Arc::new(info),
            standard: Some(font),
        }
    }

    /// Which standard 14 font this is, if it is one ([`Font::standard`]); `None` for a font
    /// loaded from bytes.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn standard14(&self) -> Option<Standard14> {
        self.standard
    }

    /// Whether a document using this font has to embed it to be self-contained: `false` only
    /// for the standard 14, which readers provide. PDF/A and PDF/UA require every font
    /// embedded, so the builder's conformance profiles reject a font for which this is `false`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn requires_embedding(&self) -> bool {
        self.standard.is_none()
    }

    /// The number of fonts in `bytes`: the member count of a collection, 1 for a single font.
    ///
    /// # Errors
    /// [`Error::InvalidFont`] when the bytes are neither.
    ///
    /// Added in 0.2.0.
    pub fn collection_len(bytes: &[u8]) -> Result<u32> {
        match FileRef::new(bytes) {
            Ok(FileRef::Font(_)) => Ok(1),
            Ok(FileRef::Collection(c)) => Ok(c.len()),
            Err(_) => Err(Error::InvalidFont { index: 0 }),
        }
    }

    /// The instance of this variable font at `settings` (user units, such as `wght` 700),
    /// starting from this font's own coordinates. Values outside an axis's range are clamped
    /// to it; when a tag is given twice the last value wins; a value equal to the axis's
    /// default is dropped, so an instance at every default *is* the default instance (same
    /// name, same subset). The instance is subset at that location when embedded
    /// (`subsetter`'s instancing), so its metrics, `/W` widths and outlines agree.
    ///
    /// # Errors
    /// [`Error::UnknownAxis`] for a tag the font has no axis for (a static font has none);
    /// [`Error::InvalidAxisValue`] for a NaN or infinite value; [`Error::InvalidFont`] when the
    /// font's `fvar` record for the axis is not `min ≤ default ≤ max`.
    ///
    /// Added in 0.2.0.
    pub fn instance(&self, settings: &[(OpenTypeTag, f32)]) -> Result<Self> {
        if self.standard.is_some() {
            return match settings.first() {
                Some(&(tag, _)) => Err(Error::UnknownAxis {
                    font: self.info.postscript_name.clone(),
                    tag,
                }),
                None => Ok(self.clone()),
            };
        }
        let font = self.font_ref()?;
        let axes = font.axes();
        let index = self.source.index;
        let mut coords: Vec<(OpenTypeTag, f32)> = self.coords.to_vec();
        for &(tag, value) in settings {
            let found = axes.get_by_tag(tag.to_skrifa()).ok_or_else(|| Error::UnknownAxis {
                font: self.info.postscript_name.clone(),
                tag,
            })?;
            if !value.is_finite() {
                return Err(Error::InvalidAxisValue { tag });
            }
            let (min, default, max) = (found.min_value(), found.default_value(), found.max_value());
            // `f32::clamp` panics when min > max; a malformed fvar must not reach it.
            if !(min <= default && default <= max) {
                return Err(Error::InvalidFont { index });
            }
            let value = value.clamp(min, max);
            coords.retain(|(t, _)| *t != tag);
            if value.to_bits() != default.to_bits() {
                coords.push((tag, value));
            }
        }
        coords.sort_by_key(|&(tag, _)| tag);
        let location = axes.location(coords.iter().map(|&(tag, value)| (tag.to_skrifa(), value)));
        let normalized: Arc<[NormalizedCoord]> = if coords.is_empty() {
            Arc::from([])
        } else {
            Arc::from(location.coords())
        };
        let info = font_info(&font, &coords, LocationRef::new(&normalized)).ok_or(Error::InvalidFont { index })?;
        Ok(Self {
            source: Arc::clone(&self.source),
            coords: Arc::from(coords),
            location: normalized,
            info: Arc::new(info),
            standard: None,
        })
    }

    /// What the font says about itself.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn info(&self) -> &FontInfo {
        &self.info
    }

    /// The instance's variation coordinates (user units, sorted by tag); empty for a static
    /// font or the default instance.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn coordinates(&self) -> &[(OpenTypeTag, f32)] {
        &self.coords
    }

    /// The collection member this font was loaded from (0 for a single font).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn index(&self) -> u32 {
        self.source.index
    }

    /// The glyph the font's `cmap` maps `ch` to; `None` when it has none (the font would draw
    /// `.notdef`). This is the unshaped, one-glyph-per-character mapping; shaping picks
    /// glyphs from the same font. For a standard 14 font the glyph is found by name through
    /// the Adobe Glyph List; its ID is its index in [`Standard14::glyph_names`] plus one, as
    /// glyph 0 is `.notdef`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn glyph_for(&self, ch: char) -> Option<GlyphId> {
        if let Some(standard) = self.standard {
            return standard.glyph_for_char(ch);
        }
        let gid = self.font_ref().ok()?.charmap().map(ch)?;
        u16::try_from(gid.to_u32()).ok().filter(|&g| g != 0).map(GlyphId)
    }

    /// The horizontal advance of `gid` in font units at this instance's location, rounded to
    /// a whole unit as `hmtx` stores it (so it matches the embedded instance's widths); `None`
    /// for a glyph past the font's glyph count.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn advance(&self, gid: GlyphId) -> Option<f32> {
        if gid.get() >= self.info.glyph_count {
            return None;
        }
        if let Some(standard) = self.standard {
            return standard.glyph_width(gid).map(f32::from);
        }
        let font = self.font_ref().ok()?;
        font.glyph_metrics(Size::unscaled(), self.location_ref())
            .advance_width(skrifa::GlyphId::from(gid.get()))
            .map(f32::round)
    }

    /// The glyph name of `gid` in a standard 14 font (`None` for other fonts).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn glyph_name(&self, gid: GlyphId) -> Option<&'static str> {
        self.standard?.glyph_name(gid)
    }

    /// The bytes the font was loaded from (the whole collection for a member); empty for a
    /// standard 14 font.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.source.bytes
    }

    pub(crate) fn digest(&self) -> &[u8; 32] {
        self.source
            .digest
            .get_or_init(|| Sha256::digest(&self.source.bytes).into())
    }

    /// `text` one glyph per character through the `cmap`, with each glyph's advance in font
    /// units (rounded, as [`Font::advance`]): the font is parsed once for the whole text.
    ///
    /// # Errors
    /// [`Error::MissingGlyph`] for the first character without a glyph;
    /// [`Error::GlyphOutOfRange`] when the `cmap` points past the glyph count (a malformed font).
    pub(crate) fn map_text(&self, text: &str) -> Result<Vec<(GlyphId, char, f32)>> {
        if let Some(standard) = self.standard {
            return text
                .chars()
                .map(|ch| {
                    let gid = standard.glyph_for_char(ch).ok_or_else(|| Error::MissingGlyph {
                        ch,
                        font: self.info.postscript_name.clone(),
                    })?;
                    Ok((gid, ch, standard.glyph_width(gid).map_or(0.0, f32::from)))
                })
                .collect();
        }
        let font = self.font_ref()?;
        let charmap = font.charmap();
        let metrics = font.glyph_metrics(Size::unscaled(), self.location_ref());
        let count = self.info.glyph_count;
        let name = || self.info.postscript_name.clone();
        text.chars()
            .map(|ch| {
                let gid = charmap
                    .map(ch)
                    .and_then(|g| u16::try_from(g.to_u32()).ok())
                    .filter(|&g| g != 0)
                    .ok_or_else(|| Error::MissingGlyph { ch, font: name() })?;
                if gid >= count {
                    return Err(Error::GlyphOutOfRange {
                        gid,
                        count,
                        font: name(),
                    });
                }
                let advance = metrics
                    .advance_width(skrifa::GlyphId::from(gid))
                    .map_or(0.0, f32::round);
                Ok((GlyphId(gid), ch, advance))
            })
            .collect()
    }

    pub(crate) fn location_ref(&self) -> LocationRef<'_> {
        LocationRef::new(&self.location)
    }

    /// HarfRust's shaping data for the face, built once and shared by every instance and clone.
    pub(crate) fn shaper_data(&self) -> Result<&harfrust::ShaperData> {
        if let Some(data) = self.source.shaping.get() {
            return Ok(data);
        }
        let font = self.font_ref()?;
        Ok(self.source.shaping.get_or_init(|| harfrust::ShaperData::new(&font)))
    }

    /// The cached shape plan `matches` accepts (HarfRust's `ShapePlanKey::matches`), else the
    /// one `build` makes, cached for every instance and clone of the face. A plan depends only
    /// on the face and what its key compares, so a cached one shapes exactly as a fresh one.
    pub(crate) fn shape_plan(
        &self,
        matches: impl Fn(&harfrust::ShapePlan) -> bool,
        build: impl FnOnce() -> harfrust::ShapePlan,
    ) -> Arc<harfrust::ShapePlan> {
        // A poisoned lock (a panic while another thread held it) only costs the cache.
        if let Ok(mut plans) = self.source.plans.lock()
            && let Some(at) = plans.iter().position(|plan| matches(plan))
        {
            // The most recently used goes last.
            let plan = plans.remove(at);
            plans.push(Arc::clone(&plan));
            return plan;
        }
        let plan = Arc::new(build());
        if let Ok(mut plans) = self.source.plans.lock()
            && !plans.iter().any(|cached| matches(cached))
        {
            if plans.len() >= MAX_PLANS {
                plans.remove(0);
            }
            plans.push(Arc::clone(&plan));
        }
        plan
    }

    /// How many shape plans the font's face has cached.
    #[cfg(test)]
    pub(crate) fn cached_plans(&self) -> usize {
        self.source.plans.lock().map_or(0, |plans| plans.len())
    }

    /// The font program for shaping and subsetting.
    ///
    /// # Errors
    /// [`Error::FontNotShapable`] for a standard 14 font, which has no program (so shaping,
    /// itemizing, line breaking and [`crate::pdf_run`] fail clearly for one);
    /// [`Error::InvalidFont`].
    pub(crate) fn font_ref(&self) -> Result<FontRef<'_>> {
        if self.standard.is_some() {
            return Err(Error::FontNotShapable {
                font: self.info.postscript_name.clone(),
            });
        }
        let index = self.source.index;
        FontRef::from_index(&self.source.bytes, index).map_err(|_| Error::InvalidFont { index })
    }
}

/// Reads [`FontInfo`] at `location`; `None` when a required table is missing or malformed.
fn font_info(font: &FontRef<'_>, coords: &[(OpenTypeTag, f32)], location: LocationRef<'_>) -> Option<FontInfo> {
    let head = font.head().ok()?;
    font.hhea().ok()?;
    font.hmtx().ok()?;
    let maxp = font.maxp().ok()?;
    let units_per_em = head.units_per_em();
    if units_per_em == 0 {
        return None;
    }
    let metrics = font.metrics(Size::unscaled(), location);
    let os2 = font.os2().ok();
    let name = |id: StringId| {
        font.localized_strings(id)
            .english_or_first()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    };
    let family = name(StringId::TYPOGRAPHIC_FAMILY_NAME)
        .or_else(|| name(StringId::FAMILY_NAME))
        .unwrap_or_default();
    let base = name(StringId::POSTSCRIPT_NAME)
        .map(|s| pdf_name_chars(&s))
        .filter(|s| !s.is_empty())
        .or_else(|| Some(pdf_name_chars(&family)).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "Font".to_owned());
    let postscript_name = cap_name(instance_name(&base, coords));
    let italic = metrics.italic_angle != 0.0
        || os2.as_ref().is_some_and(|o| {
            o.fs_selection()
                .contains(skrifa::raw::tables::os2::SelectionFlags::ITALIC)
        });
    // ISO 32000-2 §9.8.2: bit 1 FixedPitch, bit 3 Symbolic, bit 7 Italic.
    let flags = u32::from(metrics.is_monospace) | 4 | if italic { 64 } else { 0 };
    let weight = coords
        .iter()
        .find(|(tag, _)| *tag == OpenTypeTag::WEIGHT)
        .map(|&(_, w)| clamp_weight(w))
        .or_else(|| os2.as_ref().map(skrifa::raw::tables::os2::Os2::us_weight_class))
        .unwrap_or(400);
    let axes = font
        .axes()
        .iter()
        .map(|a| Axis {
            tag: OpenTypeTag(a.tag().to_be_bytes()),
            min: a.min_value(),
            default: a.default_value(),
            max: a.max_value(),
        })
        .collect::<Vec<_>>();
    let bbox = metrics
        .bounds
        .map_or([0.0; 4], |b| [b.x_min, b.y_min, b.x_max, b.y_max]);
    let has = |tag: &[u8; 4]| font.table_data(skrifa::Tag::new(tag)).is_some();
    Some(FontInfo {
        family,
        postscript_name,
        units_per_em,
        glyph_count: maxp.num_glyphs(),
        ascent: metrics.ascent,
        descent: metrics.descent,
        cap_height: metrics.cap_height,
        x_height: metrics.x_height,
        italic_angle: metrics.italic_angle,
        weight,
        bbox,
        flags,
        is_cff: !has(b"glyf") && (has(b"CFF ") || has(b"CFF2")),
        is_variable: !axes.is_empty(),
        axes,
        fs_type: FsType::new(os2.as_ref().map_or(0, skrifa::raw::tables::os2::Os2::fs_type)),
    })
}

/// `w` as a weight class (1–1000, rounded).
fn clamp_weight(w: f32) -> u16 {
    // The clamp keeps the value in u16 range, so the cast is exact.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 1..=1000"
    )]
    let weight = w.clamp(1.0, 1000.0).round() as u16;
    weight
}

/// `base` with the instance's coordinates appended (`_wght700_wdth87.5`), so instances of one
/// family get distinct `/BaseFont` names.
fn instance_name(base: &str, coords: &[(OpenTypeTag, f32)]) -> String {
    let mut name = base.to_owned();
    for (tag, value) in coords {
        let tag = pdf_name_chars(&tag.to_string());
        let value = laminapdf_cos::Object::Real(f64::from(*value));
        let mut formatted = Vec::new();
        value.write_to(&mut formatted);
        name.push('_');
        name.push_str(&tag);
        name.push_str(&String::from_utf8_lossy(&formatted));
    }
    name
}

/// `name` cut to [`MAX_NAME_LEN`] characters, ending with `_` and eight hex digits of the
/// full name's SHA-256 when it was longer (names are ASCII, see [`pdf_name_chars`]).
fn cap_name(name: String) -> String {
    if name.len() <= MAX_NAME_LEN {
        return name;
    }
    let digest = Sha256::digest(name.as_bytes());
    let suffix = digest.iter().take(4).fold(String::new(), |mut hex, b| {
        let _ = write!(hex, "{b:02X}");
        hex
    });
    let keep = MAX_NAME_LEN - suffix.len() - 1;
    format!("{}_{suffix}", name.get(..keep).unwrap_or_default())
}

/// Keeps the characters a PostScript font name may use (printable ASCII except the PDF
/// delimiters, `%` and `#`), so `/BaseFont` needs no escapes.
fn pdf_name_chars(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() && !"()<>[]{}/%#".contains(*c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_cache_drops_the_least_recently_used_past_its_limit() {
        use harfrust::{Direction, Feature, ShapePlan, ShapePlanKey, Tag as HbTag};
        use std::cell::Cell;

        let font = crate::test_fonts::font("NotoSans-Regular.ttf");
        let font_ref = font.font_ref().unwrap();
        let shaper = font.shaper_data().unwrap().shaper(&font_ref).build();
        let builds = Cell::new(0);
        // Plans told apart by a `kern` value; returns whether it had to be built.
        let plan = |n: u32| {
            let features = [Feature::new(HbTag::new(b"kern"), n, ..)];
            let key = ShapePlanKey::new(None, Direction::LeftToRight).features(&features);
            let before = builds.get();
            font.shape_plan(
                |p| key.matches(p),
                || {
                    builds.set(builds.get() + 1);
                    ShapePlan::new(&shaper, Direction::LeftToRight, None, None, &features)
                },
            );
            builds.get() > before
        };
        let limit = u32::try_from(MAX_PLANS).unwrap();
        assert!((0..limit).all(plan), "each new key is built");
        assert!(!plan(0), "a cached key is not rebuilt");
        assert!(plan(limit), "a new key past the limit is built");
        assert_eq!(font.cached_plans(), MAX_PLANS);
        assert!(!plan(0), "0 was used recently, so it was kept");
        assert!(plan(1), "1 was the least recently used, so it was dropped");
    }

    #[test]
    fn pdf_name_chars_drops_spaces_and_delimiters() {
        assert_eq!(pdf_name_chars("Noto Sans (Bold)/x#1"), "NotoSansBoldx1");
    }

    #[test]
    fn instance_name_appends_sorted_coordinates() {
        let coords = [(OpenTypeTag::WEIGHT, 700.0), (OpenTypeTag::WIDTH, 87.5)];
        assert_eq!(instance_name("Noto", &coords), "Noto_wght700_wdth87.5");
    }

    #[test]
    fn long_names_are_capped_with_a_hash() {
        let long = format!("{}_wght700", "A".repeat(60));
        let capped = cap_name(long.clone());
        assert_eq!((capped.len(), capped == cap_name(long)), (MAX_NAME_LEN, true));
    }

    #[test]
    fn capped_names_stay_distinct() {
        let a = cap_name(format!("{}_wght700", "A".repeat(60)));
        let b = cap_name(format!("{}_wght800", "A".repeat(60)));
        assert_ne!(a, b);
    }

    #[test]
    fn short_names_are_kept() {
        assert_eq!(cap_name("NotoSans-Regular".to_owned()), "NotoSans-Regular");
    }

    #[test]
    fn invalid_bytes_are_not_a_font() {
        assert_eq!(
            Font::from_bytes(b"not a font".to_vec()).unwrap_err(),
            Error::InvalidFont { index: 0 }
        );
    }

    #[test]
    fn font_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Font>();
    }

    #[test]
    fn tag_displays_its_ascii() {
        assert_eq!(OpenTypeTag::WEIGHT.to_string(), "wght");
    }
}
