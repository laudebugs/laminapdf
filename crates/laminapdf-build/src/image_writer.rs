//! The image writer `laminapdf-ops` asks for when an edit replaces an image on a page (E7 plan
//! §2.3): a JPEG or PNG written as an image XObject, the same way the builder writes one.

use laminapdf_cos::CosMut;
use laminapdf_model::Matrix;
use laminapdf_ops::{ImageXObjects, WrittenImage};

use crate::resources::write_image;
use crate::surface::image::Image;

/// Writes the JPEG and PNG files that `laminapdf-ops`' content edits put in place of an image,
/// as the builder writes images: a JPEG as it is, a PNG's alpha as a soft mask. The `laminapdf`
/// façade sets it on every document it opens.
///
/// ```
/// let doc = laminapdf_build::Builder::new(laminapdf_build::BuildOptions::default()).finish()?;
/// let doc = doc.with_images(laminapdf_build::ImageXObjectWriter);
/// # let _ = doc;
/// # Ok::<(), laminapdf_build::Error>(())
/// ```
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, Default)]
pub struct ImageXObjectWriter;

impl ImageXObjects for ImageXObjectWriter {
    fn write(&self, cos: &mut CosMut<'_>, bytes: &[u8]) -> laminapdf_cos::Result<Option<WrittenImage>> {
        let image = if bytes.starts_with(&[0xFF, 0xD8]) {
            Image::jpeg(bytes.to_vec())
        } else if bytes.starts_with(b"\x89PNG") {
            Image::png(bytes)
        } else {
            return Ok(None);
        };
        let Ok(image) = image else {
            return Ok(None);
        };
        let object = match write_image(cos, &image) {
            Ok(object) => object,
            Err(crate::Error::Cos(e)) => return Err(e),
            Err(_) => return Ok(None),
        };
        // The file's orientation as a matrix of the unit square, whose y runs up: flip to the
        // stored image's y-down rows, turn as the file says, flip back.
        let flip = Matrix::new([1.0, 0.0, 0.0, -1.0, 0.0, 1.0]);
        let turn = Matrix::new(image.orientation().matrix(1.0, 1.0));
        Ok(Some(WrittenImage::new(object, flip.then(&turn).then(&flip))))
    }
}
