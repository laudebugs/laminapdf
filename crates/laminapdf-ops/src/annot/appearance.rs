//! Where appearances come from (E4 plan §2.2). An annotation the ops make always has an
//! appearance stream, but this crate can't draw one: the drawing lives in `laminapdf-build`,
//! which depends on this crate. So the ops call [`Appearances`], which the builder implements and
//! [`Document::with_appearances`](crate::Document::with_appearances) sets. Without it, an op that
//! needs an appearance fails with `appearance-unavailable`, so no path writes an annotation
//! without one.

use std::fmt;

use laminapdf_cos::{CosMut, ObjRef};
use laminapdf_model::UserSpaceRect;

use super::freetext::{StandardFamily, TextAlign};
use super::geom::{AnnotColor, Opacity};
use super::spec::AnnotSpec;
use crate::blob::{BlobId, BlobStore};

/// Draws annotations' appearance streams (ISO 32000-2 §12.5.5) for the annotation ops.
///
/// `laminapdf-build` implements it, and the `laminapdf` façade sets that implementation on every
/// document it opens. An implementation must be deterministic: the same request writes the same
/// bytes on every platform, so a replayed op log writes the same file.
///
/// Added in 0.3.0.
pub trait Appearances {
    /// Writes the normal appearance (`/AP /N`) for `request` into the overlay through `cos`, and
    /// returns it with the `/Rect` the annotation must have for the appearance to fit: the
    /// geometry's bounds, grown to hold what is drawn around it.
    ///
    /// # Errors
    /// The overlay's errors.
    ///
    /// Added in 0.3.0.
    fn normal(&self, cos: &mut CosMut<'_>, request: &AppearanceRequest<'_>) -> laminapdf_cos::Result<Appearance>;

    /// Whether [`normal`](Self::normal) can draw `request`, asked before anything is written. A
    /// generator built without text layout refuses a `FreeText` it can't lay out, so the op fails
    /// cleanly instead of drawing it wrong. Every request can be drawn unless the generator says
    /// otherwise.
    ///
    /// # Errors
    /// Why it can't draw the request.
    ///
    /// Added in 0.5.0.
    fn check(&self, request: &AppearanceRequest<'_>) -> Result<(), AppearanceRefusal> {
        let _ = request;
        Ok(())
    }

    /// Writes the normal appearance of a form field's widget for `request` (ISO 32000-2
    /// §12.7.4.3) into the overlay through `cos`: a Form XObject the widget's `/AP /N` names, or
    /// a check box's or radio button's state.
    ///
    /// A generator that draws no form fields keeps this default, which refuses: its
    /// [`check_field`](Self::check_field) refuses first, so the op never calls it.
    ///
    /// # Errors
    /// The overlay's errors.
    ///
    /// Added in 0.6.0.
    fn field(
        &self,
        cos: &mut CosMut<'_>,
        request: &FieldAppearanceRequest<'_>,
    ) -> laminapdf_cos::Result<FieldAppearance> {
        let _ = (cos, request);
        Err(laminapdf_cos::Error::InvalidStructure {
            kind: "form field",
            message: "the appearance generator draws no form fields",
        })
    }

    /// Whether [`field`](Self::field) can draw `request`, asked before anything is written. The
    /// default refuses: a generator draws form fields only when it says it can.
    ///
    /// # Errors
    /// Why it can't draw the request.
    ///
    /// Added in 0.6.0.
    fn check_field(&self, request: &FieldAppearanceRequest<'_>) -> Result<(), AppearanceRefusal> {
        let _ = request;
        Err(AppearanceRefusal::Unavailable)
    }

    /// Writes the Form XObject of an object added to a page as content into the overlay through
    /// `cos`, and returns it: its `/BBox` and drawing in the object's own space, as
    /// [`ContentObjectSpec`](crate::ContentObjectSpec) gives it, with no `/Matrix`. The op names it
    /// and places it.
    ///
    /// A generator that draws no added content keeps this default, which refuses: its
    /// [`check_content`](Self::check_content) refuses first, so the op never calls it.
    ///
    /// # Errors
    /// The overlay's errors.
    ///
    /// Added in 0.7.0.
    fn content(&self, cos: &mut CosMut<'_>, request: &crate::ContentRequest<'_>) -> laminapdf_cos::Result<ObjRef> {
        let _ = (cos, request);
        Err(laminapdf_cos::Error::InvalidStructure {
            kind: "added content",
            message: "the appearance generator draws no added content",
        })
    }

    /// Whether [`content`](Self::content) can draw `request`, asked before anything is written:
    /// an image whose bytes it can't read is [`AppearanceRefusal::ImageInvalid`]. The default
    /// refuses: a generator draws added content only when it says it can.
    ///
    /// # Errors
    /// Why it can't draw the request.
    ///
    /// Added in 0.7.0.
    fn check_content(&self, request: &crate::ContentRequest<'_>) -> Result<(), AppearanceRefusal> {
        let _ = request;
        Err(AppearanceRefusal::Unavailable)
    }
}

/// What a form field widget's appearance draws (ISO 32000-2 §12.7.4.3): its box and how it is
/// turned, its background and border from `/MK`, the text style its `/DA` sets, and the value.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct FieldAppearanceRequest<'a> {
    /// The widget's `/Rect`.
    pub rect: UserSpaceRect,
    /// `/MK /R`: how far the widget's content is turned, counterclockwise: 0, 90, 180 or 270.
    pub rotation: u16,
    /// `/MK /BG`, the background.
    pub background: Option<AnnotColor>,
    /// `/MK /BC`, the border's colour.
    pub border: Option<AnnotColor>,
    /// The border's width (`/BS /W`, else `/Border`, else 1), points; drawn only with a colour.
    pub border_width: f64,
    /// The standard family of `/DA`'s font, or Helvetica when it names another.
    pub family: StandardFamily,
    /// `/DA`'s size; `None` for 0, which asks for the text to fit the box.
    pub size: Option<f64>,
    /// `/DA`'s fill colour (black when it sets none).
    pub color: AnnotColor,
    /// `/Q`.
    pub align: TextAlign,
    /// What to draw.
    pub content: FieldContent<'a>,
    /// Fonts by their bytes, tried in order for a character the standard 14 faces lack.
    pub fallbacks: &'a [BlobId],
    /// The session's bytes, which the fallbacks name.
    pub blobs: Option<&'a BlobStore>,
    /// The device space of a PDF/A file's output intent, which the appearance paints in.
    pub intent: Option<IntentSpace>,
    /// Whether every character must be drawn in an embedded font: the file claims PDF/A, which a
    /// standard 14 font, never embedded, would break (ISO 19005-2 §6.2.11.4). The first of
    /// [`fallbacks`](Self::fallbacks) then draws the value; without one, the standard 14 face
    /// does, and the save warns (`pdfa-font-not-embedded`).
    pub embed_fonts: bool,
}

/// The value a field widget shows.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum FieldContent<'a> {
    /// A text field's or combo box's text.
    Text {
        /// The text.
        text: &'a str,
        /// `Multiline`: the text wraps at the box's width.
        multiline: bool,
        /// `Comb`: the box is split into this many cells of equal width, a character in each.
        comb: Option<u32>,
        /// `Password`: each character is drawn as a bullet.
        password: bool,
    },
    /// A list box's rows from its top row (`/TI`), the chosen ones highlighted.
    List {
        /// Each option's text, in `/Opt` order.
        rows: &'a [String],
        /// The chosen rows, by index.
        selected: &'a [usize],
        /// `/TI`: the first row shown.
        top: usize,
    },
    /// A check box's or radio button's state: its `/MK /CA` character in `ZapfDingbats` when on,
    /// nothing but the background and border when off.
    Button {
        /// The `ZapfDingbats` character.
        caption: char,
        /// Whether this is the on state.
        on: bool,
    },
}

/// A form field widget's appearance, written into the overlay.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FieldAppearance {
    /// The Form XObject.
    pub normal: ObjRef,
    /// The fonts given by their bytes it embeds, by the name its resources give them, for the
    /// form's `/DR`.
    pub fonts: Vec<(String, ObjRef)>,
}

impl FieldAppearance {
    /// The appearance `normal`, embedding `fonts`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(normal: ObjRef, fonts: Vec<(String, ObjRef)>) -> Self {
        Self { normal, fonts }
    }
}

/// Why a generator can't draw an appearance ([`Appearances::check`]).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AppearanceRefusal {
    /// The text needs layout the generator was built without: a font given by its bytes, or a
    /// fallback font. In WebAssembly, the text module has it (`appearance-needs-text`).
    NeedsText,
    /// A character is in none of the box's fonts (`annotation-glyph-missing`).
    GlyphMissing,
    /// A font's bytes aren't a font the generator can read and embed (`annotation-font-invalid`).
    FontInvalid,
    /// The generator draws no form fields (`appearance-unavailable`).
    ///
    /// Added in 0.6.0.
    Unavailable,
    /// An image stamp's bytes aren't a JPEG or PNG image the generator reads
    /// (`stamp-image-invalid`).
    ///
    /// Added in 0.6.0.
    ImageInvalid,
}

/// What an appearance draws.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct AppearanceRequest<'a> {
    /// The mark and its geometry.
    pub spec: &'a AnnotSpec,
    /// `/C`; the generator's default for the mark when absent.
    pub color: Option<AnnotColor>,
    /// The opacity, which the appearance applies itself.
    pub opacity: Opacity,
    /// The device space of a PDF/A file's output intent, which the appearance paints in
    /// (ISO 19005-2 §6.2.4.3); `None` when the file claims no PDF/A part.
    pub intent: Option<IntentSpace>,
    /// The session's bytes, which a `FreeText`'s fonts and an image stamp name by their IDs.
    ///
    /// Added in 0.5.0.
    pub blobs: Option<&'a BlobStore>,
}

impl<'a> AppearanceRequest<'a> {
    /// A request for `spec` in its default colour, opaque, with no output intent.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn new(spec: &'a AnnotSpec) -> Self {
        Self {
            spec,
            color: None,
            opacity: Opacity::OPAQUE,
            intent: None,
            blobs: None,
        }
    }

    /// The same request with the session's bytes, which a `FreeText`'s fonts name.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn with_blobs(mut self, blobs: &'a BlobStore) -> Self {
        self.blobs = Some(blobs);
        self
    }
}

/// The device colour space of a PDF/A output intent.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IntentSpace {
    /// An RGB profile (`/N 3`).
    Rgb,
    /// A CMYK profile (`/N 4`).
    Cmyk,
}

/// An appearance written into the overlay.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Appearance {
    /// The Form XObject for `/AP /N`.
    pub normal: ObjRef,
    /// The `/Rect` it fits.
    pub rect: UserSpaceRect,
}

impl Appearance {
    /// The appearance `normal`, which fits `rect`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn new(normal: ObjRef, rect: UserSpaceRect) -> Self {
        Self { normal, rect }
    }
}

/// The generator a document holds.
pub(crate) struct Generator(pub(crate) Box<dyn Appearances + Send>);

impl fmt::Debug for Generator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Appearances")
    }
}

impl From<AppearanceRefusal> for crate::Error {
    fn from(refusal: AppearanceRefusal) -> Self {
        match refusal {
            AppearanceRefusal::NeedsText => Self::AppearanceNeedsText,
            AppearanceRefusal::GlyphMissing => Self::AnnotationGlyphMissing,
            AppearanceRefusal::FontInvalid => Self::AnnotationFontInvalid,
            AppearanceRefusal::Unavailable => Self::AppearanceUnavailable,
            AppearanceRefusal::ImageInvalid => Self::StampImageInvalid,
        }
    }
}
