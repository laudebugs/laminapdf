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
        }
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
