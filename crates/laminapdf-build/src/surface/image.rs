//! Images (ISO 32000-2 §8.9): JPEG passed through as `DCTDecode`, PNG decoded into samples
//! plus an `/SMask` for alpha (§11.6.5.3), and raw RGBA.

use std::sync::Arc;

use sha2::{Digest, Sha256};

use super::exif::{Orientation, Shown};
use crate::error::{Error, ImageFormat, Result};

/// The most decoded bytes a PNG may need (256 MiB): larger images fail with
/// [`Error::ImageTooLarge`] before the samples are allocated.
///
/// Added in 0.2.0.
pub const MAX_DECODED_IMAGE_BYTES: usize = 256 << 20;

/// An image's colour space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ImageColor {
    Gray,
    Rgb,
    Cmyk,
}

impl ImageColor {
    pub(crate) const fn pdf_name(self) -> &'static str {
        match self {
            Self::Gray => "DeviceGray",
            Self::Rgb => "DeviceRGB",
            Self::Cmyk => "DeviceCMYK",
        }
    }

    const fn channels(self) -> usize {
        match self {
            Self::Gray => 1,
            Self::Rgb => 3,
            Self::Cmyk => 4,
        }
    }
}

/// How the samples are stored in the PDF.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Samples {
    /// Decoded samples, Flate-compressed when written.
    Raw(Vec<u8>),
    /// JPEG bytes, written unchanged with `/Filter /DCTDecode`.
    Jpeg(Vec<u8>),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ImageData {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) color: ImageColor,
    pub(crate) bits: u8,
    pub(crate) samples: Samples,
    /// Alpha samples (same bit depth), written as an `/SMask`; `None` when fully opaque.
    pub(crate) alpha: Option<Vec<u8>>,
    /// An Adobe CMYK JPEG stores inverted samples: written with `/Decode [1 0 1 0 1 0 1 0]`.
    pub(crate) inverted: bool,
    /// SHA-256 over everything above: equal images are written once per document.
    pub(crate) hash: [u8; 32],
    /// How the file says to show it (not part of `hash`: it changes how the image is drawn,
    /// not the image XObject).
    pub(crate) shown: Shown,
}

/// Options for [`Image::png_with`].
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PngOptions {
    /// Convert 16-bit samples to 8 bits (smaller; PDF 1.5+ readers take 16-bit images, so it
    /// is off by default).
    pub downconvert_16: bool,
}

impl PngOptions {
    /// The defaults, as [`PngOptions::default`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// With [`PngOptions::downconvert_16`] set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_downconvert_16(mut self, on: bool) -> Self {
        self.downconvert_16 = on;
        self
    }
}

/// An image to draw with [`Surface::image`](crate::Surface::image). Cheap to clone; the
/// builder writes each distinct image once per document (by content hash).
///
/// ```
/// use laminapdf_build::Image;
///
/// let img = Image::rgba(2, 1, &[255, 0, 0, 255, 0, 0, 255, 128])?;
/// assert_eq!((img.width(), img.height(), img.has_alpha()), (2, 1, true));
/// # Ok::<(), laminapdf_build::Error>(())
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub(crate) data: Arc<ImageData>,
}

impl Image {
    fn new(
        width: u32,
        height: u32,
        color: ImageColor,
        bits: u8,
        samples: Samples,
        alpha: Option<Vec<u8>>,
        inverted: bool,
    ) -> Self {
        let mut h = Sha256::new();
        h.update(width.to_be_bytes());
        h.update(height.to_be_bytes());
        h.update([color as u8, bits, u8::from(inverted)]);
        match &samples {
            Samples::Raw(s) => {
                h.update([0]);
                h.update(u64::try_from(s.len()).unwrap_or(u64::MAX).to_be_bytes());
                h.update(s);
            }
            Samples::Jpeg(s) => {
                h.update([1]);
                h.update(u64::try_from(s.len()).unwrap_or(u64::MAX).to_be_bytes());
                h.update(s);
            }
        }
        if let Some(a) = &alpha {
            h.update([2]);
            h.update(a);
        }
        Self {
            data: Arc::new(ImageData {
                width,
                height,
                color,
                bits,
                samples,
                alpha,
                inverted,
                hash: h.finalize().into(),
                shown: Shown::default(),
            }),
        }
    }

    /// `self` with what its file says about showing it (a new image's data isn't shared yet).
    fn shown(mut self, shown: Shown) -> Self {
        if let Some(data) = Arc::get_mut(&mut self.data) {
            data.shown = shown;
        }
        self
    }

    /// Width in pixels.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.data.width
    }

    /// Height in pixels.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.data.height
    }

    /// How the file says the image is turned to be shown upright: a JPEG's or PNG's EXIF
    /// `Orientation`, [`Orientation::Upright`] without one. [`Surface::image`](super::Surface::image)
    /// draws the stored image as it is; [`images_to_pdf`](crate::images_to_pdf) applies this.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn orientation(&self) -> Orientation {
        self.data.shown.orientation
    }

    /// Whether the image has an alpha channel that isn't fully opaque (written as `/SMask`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn has_alpha(&self) -> bool {
        self.data.alpha.is_some()
    }

    /// A JPEG, embedded byte for byte with `/Filter /DCTDecode` (§7.4.8). Its size and
    /// components come from the frame header (SOF): 1 component is `DeviceGray`, 3
    /// `DeviceRGB`, 4 `DeviceCMYK`. A CMYK JPEG with an Adobe `APP14` marker stores inverted
    /// samples (as Photoshop writes them), so it gets `/Decode [1 0 1 0 1 0 1 0]`.
    ///
    /// # Errors
    /// [`Error::InvalidImage`] when there is no SOI or no frame header before the scan, or a
    /// segment runs past the end; [`Error::UnsupportedImage`] for arithmetic coding, lossless
    /// or hierarchical JPEG, a precision other than 8 bits, or 2 or more than 4 components.
    ///
    /// Added in 0.2.0.
    pub fn jpeg(bytes: Vec<u8>) -> Result<Self> {
        let header = jpeg_header(&bytes)?;
        let color = match header.components {
            1 => ImageColor::Gray,
            3 => ImageColor::Rgb,
            4 => ImageColor::Cmyk,
            _ => {
                return Err(unsupported(
                    ImageFormat::Jpeg,
                    "a number of components other than 1, 3 or 4",
                ));
            }
        };
        let inverted = color == ImageColor::Cmyk && header.adobe;
        Ok(Self::new(
            header.width,
            header.height,
            color,
            8,
            Samples::Jpeg(bytes),
            None,
            inverted,
        )
        .shown(header.shown))
    }

    /// A PNG, decoded with [`PngOptions::default`].
    ///
    /// # Errors
    /// As [`Image::png_with`].
    ///
    /// Added in 0.2.0.
    pub fn png(bytes: &[u8]) -> Result<Self> {
        Self::png_with(bytes, PngOptions::default())
    }

    /// A PNG, decoded: palettes are expanded to RGB, `tRNS` and alpha channels become an
    /// `/SMask` (left out when every pixel is opaque), bit depths below 8 are expanded to 8,
    /// and 16-bit samples stay 16-bit unless [`PngOptions::downconvert_16`]. Interlaced images
    /// are de-interlaced. Gamma and colour-profile chunks are ignored (sRGB is assumed).
    ///
    /// # Errors
    /// [`Error::InvalidImage`] for bytes the decoder rejects; [`Error::ImageTooLarge`] when
    /// the decoded image would exceed [`MAX_DECODED_IMAGE_BYTES`].
    ///
    /// Added in 0.2.0.
    pub fn png_with(bytes: &[u8], options: PngOptions) -> Result<Self> {
        let png = super::png::decode(bytes, options.downconvert_16)?;
        let color = if png.gray { ImageColor::Gray } else { ImageColor::Rgb };
        let (samples, alpha) = if png.alpha {
            split_alpha(&png.pixels, color.channels(), usize::from(png.bits / 8))
        } else {
            (png.pixels, None)
        };
        Ok(Self::new(
            png.width,
            png.height,
            color,
            png.bits,
            Samples::Raw(samples),
            alpha,
            false,
        )
        .shown(png.shown))
    }

    /// Raw 8-bit RGBA samples, row by row from the top, not premultiplied. The alpha channel
    /// becomes an `/SMask` unless every pixel is opaque.
    ///
    /// # Errors
    /// [`Error::ImageTooLarge`] when `width × height × 4` doesn't fit in memory's address
    /// space; [`Error::ImageDataLength`] when `rgba` is not that many bytes;
    /// [`Error::InvalidGeometry`] for a zero width or height.
    ///
    /// Added in 0.2.0.
    pub fn rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidGeometry {
                what: "an image needs a width and height of at least one pixel",
            });
        }
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| usize::try_from(n).ok())
            .ok_or(Error::ImageTooLarge {
                limit: MAX_DECODED_IMAGE_BYTES,
            })?;
        if rgba.len() != expected {
            return Err(Error::ImageDataLength {
                expected,
                actual: rgba.len(),
            });
        }
        let (samples, alpha) = split_alpha(rgba, 3, 1);
        Ok(Self::new(
            width,
            height,
            ImageColor::Rgb,
            8,
            Samples::Raw(samples),
            alpha,
            false,
        ))
    }
}

fn unsupported(format: ImageFormat, feature: &'static str) -> Error {
    Error::UnsupportedImage { format, feature }
}

/// Splits interleaved samples with a trailing alpha channel into colour and alpha planes
/// (`sample` bytes per sample); the alpha plane is `None` when every alpha value is opaque.
fn split_alpha(pixels: &[u8], channels: usize, sample: usize) -> (Vec<u8>, Option<Vec<u8>>) {
    let stride = (channels + 1) * sample;
    let pixel_count = pixels.len() / stride;
    let mut color = Vec::with_capacity(pixel_count * channels * sample);
    let mut alpha = Vec::with_capacity(pixel_count * sample);
    for px in pixels.chunks_exact(stride) {
        let (c, a) = px.split_at(channels * sample);
        color.extend_from_slice(c);
        alpha.extend_from_slice(a);
    }
    let opaque = alpha.iter().all(|&b| b == 0xFF);
    (color, (!opaque).then_some(alpha))
}

/// What the builder needs from a JPEG's headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JpegHeader {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) components: u8,
    /// An Adobe `APP14` segment was seen.
    pub(crate) adobe: bool,
    /// What the `JFIF` and `Exif` segments say about showing it.
    pub(crate) shown: Shown,
}

/// Walks the JPEG's marker segments (ITU-T T.81 §B.1) up to the frame header. Fill bytes
/// (`0xFF` runs) before a marker are skipped, and markers without a length (`TEM`, `RSTn`,
/// `SOI`) are stepped over: the two cases the krilla-fit spike's first parser got wrong.
pub(crate) fn jpeg_header(bytes: &[u8]) -> Result<JpegHeader> {
    let invalid = |reason| Error::InvalidImage {
        format: ImageFormat::Jpeg,
        reason,
    };
    if bytes.get(..2) != Some(&[0xFF, 0xD8]) {
        return Err(invalid("no start-of-image marker"));
    }
    let mut at = 2;
    let mut adobe = false;
    let (mut jfif, mut exif) = (None, None);
    loop {
        if bytes.get(at) != Some(&0xFF) {
            return Err(invalid("a marker was expected"));
        }
        while bytes.get(at) == Some(&0xFF) {
            at += 1;
        }
        let marker = *bytes
            .get(at)
            .ok_or_else(|| invalid("the data ends before the frame header"))?;
        at += 1;
        // Markers without a length field (T.81 §B.1.1.2, Table B.1).
        if marker == 0x01 || (0xD0..=0xD8).contains(&marker) {
            continue;
        }
        let length = bytes
            .get(at..at + 2)
            .and_then(|l| <[u8; 2]>::try_from(l).ok())
            .map(|l| usize::from(u16::from_be_bytes(l)))
            .filter(|l| *l >= 2)
            .ok_or_else(|| invalid("a segment length is missing or too short"))?;
        let segment = bytes
            .get(at + 2..at + length)
            .ok_or_else(|| invalid("a segment runs past the end of the data"))?;
        match marker {
            0xD9 | 0xDA => return Err(invalid("the scan starts before a frame header")),
            0xEE if segment.starts_with(b"Adobe") => adobe = true,
            0xE0 if jfif.is_none() && segment.starts_with(b"JFIF\0") => jfif = Some(segment),
            0xE1 if exif.is_none() && segment.starts_with(b"Exif\0\0") => exif = Some(segment),
            0xC0..=0xC2 => {
                let [precision, h0, h1, w0, w1, components, ..] = *segment else {
                    return Err(invalid("the frame header is too short"));
                };
                if precision != 8 {
                    return Err(unsupported(ImageFormat::Jpeg, "a sample precision other than 8 bits"));
                }
                let (width, height) = (u16::from_be_bytes([w0, w1]), u16::from_be_bytes([h0, h1]));
                if width == 0 || height == 0 {
                    return Err(invalid("a zero width or height (DNL is not supported)"));
                }
                return Ok(JpegHeader {
                    width: u32::from(width),
                    height: u32::from(height),
                    components,
                    adobe,
                    shown: super::exif::from_jpeg_segments(jfif, exif),
                });
            }
            0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                return Err(unsupported(
                    ImageFormat::Jpeg,
                    "lossless, hierarchical or arithmetic-coded JPEG",
                ));
            }
            _ => {}
        }
        at += length;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOI, an Adobe APP14 when `adobe`, a baseline SOF0 (`components`, 3×2 pixels), SOS.
    fn jpeg(components: u8, adobe: bool, fill: bool) -> Vec<u8> {
        let mut b = vec![0xFF, 0xD8];
        if adobe {
            b.extend_from_slice(&[0xFF, 0xEE, 0, 14]);
            b.extend_from_slice(b"Adobe\0\x64\0\0\0\0\0");
        }
        if fill {
            b.extend_from_slice(&[0xFF, 0xFF, 0xFF]);
        }
        b.extend_from_slice(&[0xFF, 0xD0]); // a standalone marker, no length
        let len = 8 + 3 * u16::from(components);
        b.extend_from_slice(&[0xFF, 0xC0]);
        b.extend_from_slice(&len.to_be_bytes());
        b.extend_from_slice(&[8, 0, 2, 0, 3, components]);
        for c in 0..components {
            b.extend_from_slice(&[c + 1, 0x11, 0]);
        }
        b.extend_from_slice(&[0xFF, 0xDA, 0, 2, 0xFF, 0xD9]);
        b
    }

    mod jpeg_header {
        use super::*;

        #[test]
        fn reads_size_and_components_from_sof() {
            assert_eq!(
                jpeg_header(&jpeg(3, false, false)).unwrap(),
                JpegHeader {
                    width: 3,
                    height: 2,
                    components: 3,
                    adobe: false,
                    shown: Shown::default(),
                }
            );
        }

        #[test]
        fn skips_fill_bytes_and_standalone_markers() {
            assert_eq!(jpeg_header(&jpeg(1, false, true)).unwrap().components, 1);
        }

        #[test]
        fn notes_the_adobe_marker() {
            assert!(jpeg_header(&jpeg(4, true, false)).unwrap().adobe);
        }

        #[test]
        fn rejects_data_without_soi() {
            assert_eq!(jpeg_header(b"\x89PNG").unwrap_err().code(), "invalid-image");
        }

        #[test]
        fn rejects_a_truncated_segment() {
            let b = jpeg(3, false, false);
            assert_eq!(jpeg_header(&b[..b.len() - 18]).unwrap_err().code(), "invalid-image");
        }

        #[test]
        fn rejects_arithmetic_coding() {
            let mut b = jpeg(3, false, false);
            let sof = b.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
            b[sof + 1] = 0xC9;
            assert_eq!(jpeg_header(&b).unwrap_err().code(), "unsupported-image");
        }

        #[test]
        fn rejects_twelve_bit_precision() {
            let mut b = jpeg(3, false, false);
            let sof = b.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
            b[sof + 4] = 12;
            assert_eq!(jpeg_header(&b).unwrap_err().code(), "unsupported-image");
        }
    }

    #[test]
    fn an_adobe_cmyk_jpeg_is_inverted() {
        assert!(Image::jpeg(jpeg(4, true, false)).unwrap().data.inverted);
    }

    #[test]
    fn an_rgb_jpeg_with_an_adobe_marker_is_not_inverted() {
        assert!(!Image::jpeg(jpeg(3, true, false)).unwrap().data.inverted);
    }

    #[test]
    fn a_two_component_jpeg_is_unsupported() {
        assert_eq!(
            Image::jpeg(jpeg(2, false, false)).unwrap_err().code(),
            "unsupported-image"
        );
    }

    #[test]
    fn rgba_checks_the_length() {
        assert_eq!(
            Image::rgba(2, 2, &[0; 15]).unwrap_err(),
            Error::ImageDataLength {
                expected: 16,
                actual: 15
            }
        );
    }

    #[test]
    fn rgba_dimensions_whose_size_overflows_are_refused() {
        assert_eq!(
            Image::rgba(u32::MAX, u32::MAX, &[]).unwrap_err().code(),
            "image-too-large"
        );
    }

    #[test]
    fn opaque_rgba_has_no_alpha_plane() {
        assert!(!Image::rgba(1, 1, &[1, 2, 3, 255]).unwrap().has_alpha());
    }

    #[test]
    fn split_alpha_keeps_16_bit_samples_together() {
        let (c, a) = split_alpha(&[1, 2, 3, 4], 1, 2);
        assert_eq!((c, a), (vec![1, 2], Some(vec![3, 4])));
    }

    #[test]
    fn equal_images_hash_equal_and_different_ones_do_not() {
        let a = Image::rgba(1, 1, &[1, 2, 3, 4]).unwrap();
        let b = Image::rgba(1, 1, &[1, 2, 3, 4]).unwrap();
        let c = Image::rgba(1, 1, &[1, 2, 3, 5]).unwrap();
        assert_eq!((a.data.hash == b.data.hash, a.data.hash == c.data.hash), (true, false));
    }

    #[test]
    fn garbage_is_not_a_png() {
        assert_eq!(Image::png(b"not a png").unwrap_err().code(), "invalid-image");
    }
}
