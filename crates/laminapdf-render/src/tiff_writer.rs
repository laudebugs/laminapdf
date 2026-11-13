//! TIFF files written a page at a time: Deflate-compressed RGBA, each page with its resolution.
//! [`export_pages`](crate::export_pages) writes its TIFFs through it, and a caller that draws
//! its own pages (the web reader's worker) feeds it their pixels.

use std::cell::RefCell;
use std::io::{self, Cursor, Seek, SeekFrom, Write};
use std::rc::Rc;

use tiff::encoder::{Compression, DeflateLevel, Rational, TiffEncoder, colortype::RGBA8};
use tiff::tags::ResolutionUnit;

use crate::RenderError;

/// The file being written, shared with the encoder, which takes its writer by value and never
/// gives it back.
#[derive(Clone, Default)]
struct Shared(Rc<RefCell<Cursor<Vec<u8>>>>);

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for Shared {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.borrow_mut().seek(pos)
    }
}

/// A TIFF file in memory, one page per [`TiffWriter::add_page`]: straight 8-bit RGBA,
/// Deflate-compressed, each page with its resolution in dots per inch.
///
/// ```
/// use laminapdf_render::TiffWriter;
///
/// let mut tiff = TiffWriter::new()?;
/// tiff.add_page(2, 1, &[255, 0, 0, 255, 0, 0, 255, 255], 300.0)?;
/// let file = tiff.finish();
/// assert!(file.starts_with(b"II*\0"));
/// # Ok::<(), laminapdf_render::RenderError>(())
/// ```
///
/// Added in 0.2.0.
pub struct TiffWriter {
    encoder: TiffEncoder<Shared>,
    file: Shared,
}

impl TiffWriter {
    /// An empty file.
    ///
    /// # Errors
    /// [`RenderError::ExportFailed`] if the encoder can't write the file's header.
    ///
    /// Added in 0.2.0.
    pub fn new() -> Result<Self, RenderError> {
        let file = Shared::default();
        let encoder = TiffEncoder::new(file.clone())
            .map_err(|_| failed())?
            .with_compression(Compression::Deflate(DeflateLevel::Balanced));
        Ok(Self { encoder, file })
    }

    /// Adds a page of `width` × `height` straight RGBA pixels, row by row, drawn at `dpi`.
    ///
    /// # Errors
    /// [`RenderError::InvalidRequest`] when `rgba` isn't `width × height × 4` bytes or `dpi`
    /// isn't a positive finite number; [`RenderError::ExportFailed`] if the encoder fails.
    ///
    /// Added in 0.2.0.
    pub fn add_page(&mut self, width: u32, height: u32, rgba: &[u8], dpi: f64) -> Result<(), RenderError> {
        let expected = u64::from(width) * u64::from(height) * 4;
        if width == 0 || height == 0 || rgba.len() as u64 != expected {
            return Err(RenderError::InvalidRequest {
                what: "a TIFF page's pixels must be width × height × 4 bytes of RGBA",
            });
        }
        if !dpi.is_finite() || dpi <= 0.0 {
            return Err(RenderError::InvalidRequest {
                what: "a TIFF page's resolution must be a positive number of pixels per inch",
            });
        }
        let mut image = self.encoder.new_image::<RGBA8>(width, height).map_err(|_| failed())?;
        // TIFF resolutions are rationals: hundredths of a dot per inch.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to u32's range first"
        )]
        let hundredths = (dpi * 100.0).round().clamp(1.0, f64::from(u32::MAX)) as u32;
        image.resolution(ResolutionUnit::Inch, Rational { n: hundredths, d: 100 });
        image.write_data(rgba).map_err(|_| failed())
    }

    /// The file's bytes.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        let Self { encoder, file } = self;
        // The encoder's copy of the file goes first, which leaves this one the only owner.
        drop(encoder);
        match Rc::try_unwrap(file.0) {
            Ok(cell) => cell.into_inner().into_inner(),
            Err(shared) => shared.borrow().get_ref().clone(),
        }
    }
}

fn failed() -> RenderError {
    RenderError::ExportFailed { format: "tiff" }
}
