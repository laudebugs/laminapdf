//! What turns a replacement image's bytes into an image XObject (E7 plan §2.3): `laminapdf-build`
//! reads JPEG and PNG, and this crate can't depend on it, so the `laminapdf` façade sets its
//! writer on every document it opens, as it does the annotation appearances.

use std::fmt;

use laminapdf_cos::{CosMut, ObjRef};
use laminapdf_model::Matrix;

/// Writes image XObjects for `ContentEdit::ReplaceImage` (feature `content-edit`).
/// `laminapdf-build` implements it, and the `laminapdf` façade sets that implementation on
/// every document it opens. An implementation must be deterministic: the same bytes write the
/// same objects on every platform, so a replayed op log writes the same file.
///
/// Added in 0.5.0.
pub trait ImageXObjects {
    /// Writes `bytes` (a JPEG or PNG file) as an image XObject into the overlay through `cos`.
    /// `None` when the bytes aren't an image the implementation reads.
    ///
    /// # Errors
    /// The overlay's errors.
    ///
    /// Added in 0.5.0.
    fn write(&self, cos: &mut CosMut<'_>, bytes: &[u8]) -> laminapdf_cos::Result<Option<WrittenImage>>;
}

/// An image XObject [`ImageXObjects::write`] wrote.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct WrittenImage {
    /// The image XObject.
    pub object: ObjRef,
    /// The matrix, in the image's unit square, that shows the image upright: the identity, or
    /// the turn or mirror its file's EXIF orientation asks for.
    pub unit: Matrix,
}

impl WrittenImage {
    /// The image `object`, shown through `unit`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(object: ObjRef, unit: Matrix) -> Self {
        Self { object, unit }
    }
}

/// The document's image writer, boxed so a document can hold one.
pub(crate) struct ImageWriter(pub(crate) Box<dyn ImageXObjects + Send>);

impl fmt::Debug for ImageWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ImageXObjects")
    }
}
