//! Where appearances come from (E4 plan §2.2). An annotation the ops make always has an
//! appearance stream, but this crate can't draw one: the drawing lives in `laminapdf-build`,
//! which depends on this crate. So the ops call [`Appearances`], which the builder implements and
//! [`Document::with_appearances`](crate::Document::with_appearances) sets. Without it, an op that
//! needs an appearance fails with `appearance-unavailable`, so no path writes an annotation
//! without one.

use std::fmt;

use laminapdf_cos::{CosMut, ObjRef};
use laminapdf_model::UserSpaceRect;

use super::geom::{AnnotColor, Opacity};
use super::spec::AnnotSpec;
use crate::blob::BlobStore;

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
            AppearanceRefusal::ImageInvalid => Self::StampImageInvalid,
        }
    }
}
