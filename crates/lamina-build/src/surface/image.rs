//! Images (ISO 32000-2 §8.9): JPEG passed through as `DCTDecode`, PNG decoded into samples
//! plus an `/SMask` for alpha (§11.6.5.3), and raw RGBA.

use std::io::Cursor;
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::error::{Error, ImageFormat, Result};

/// The most decoded bytes a PNG may need (256 MiB): larger images fail with
/// [`Error::ImageTooLarge`] before the samples are allocated.
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
}

/// Options for [`Image::png_with`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PngOptions {
    /// Convert 16-bit samples to 8 bits (smaller; PDF 1.5+ readers take 16-bit images, so it
    /// is off by default).
    pub downconvert_16: bool,
}

impl PngOptions {
    /// With [`PngOptions::downconvert_16`] set.
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
/// use lamina_build::Image;
///
/// let img = Image::rgba(2, 1, &[255, 0, 0, 255, 0, 0, 255, 128])?;
/// assert_eq!((img.width(), img.height(), img.has_alpha()), (2, 1, true));
/// # Ok::<(), lamina_build::Error>(())
/// ```
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
            }),
        }
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.data.width
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.data.height
    }

    /// Whether the image has an alpha channel that isn't fully opaque (written as `/SMask`).
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
        ))
    }

    /// A PNG, decoded with [`PngOptions::default`].
    ///
    /// # Errors
    /// As [`Image::png_with`].
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
    pub fn png_with(bytes: &[u8], options: PngOptions) -> Result<Self> {
        let mut decoder = png::Decoder::new_with_limits(
            Cursor::new(bytes),
            png::Limits {
                bytes: MAX_DECODED_IMAGE_BYTES,
            },
        );
        let mut transform = png::Transformations::EXPAND;
        if options.downconvert_16 {
            transform |= png::Transformations::STRIP_16;
        }
        decoder.set_transformations(transform);
        decoder.set_ignore_text_chunk(true);
        let mut reader = decoder.read_info().map_err(png_error)?;
        let size = reader
            .output_buffer_size()
            .filter(|s| *s <= MAX_DECODED_IMAGE_BYTES)
            .ok_or(Error::ImageTooLarge {
                limit: MAX_DECODED_IMAGE_BYTES,
            })?;
        let mut buf = vec![0; size];
        let info = reader.next_frame(&mut buf).map_err(png_error)?;
        buf.truncate(info.buffer_size());
        let bits = match info.bit_depth {
            png::BitDepth::Sixteen => 16,
            png::BitDepth::Eight => 8,
            _ => return Err(unsupported(ImageFormat::Png, "a bit depth below 8 after expansion")),
        };
        let (color, has_alpha) = match info.color_type {
            png::ColorType::Grayscale => (ImageColor::Gray, false),
            png::ColorType::GrayscaleAlpha => (ImageColor::Gray, true),
            png::ColorType::Rgb => (ImageColor::Rgb, false),
            png::ColorType::Rgba => (ImageColor::Rgb, true),
            png::ColorType::Indexed => {
                return Err(unsupported(ImageFormat::Png, "an indexed image after expansion"));
            }
        };
        let (samples, alpha) = if has_alpha {
            split_alpha(&buf, color.channels(), usize::from(bits / 8))
        } else {
            (buf, None)
        };
        Ok(Self::new(
            info.width,
            info.height,
            color,
            bits,
            Samples::Raw(samples),
            alpha,
            false,
        ))
    }

    /// Raw 8-bit RGBA samples, row by row from the top, not premultiplied. The alpha channel
    /// becomes an `/SMask` unless every pixel is opaque.
    ///
    /// # Errors
    /// [`Error::ImageDataLength`] when `rgba` is not `width × height × 4` bytes;
    /// [`Error::InvalidGeometry`] for a zero width or height.
    pub fn rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(Error::InvalidGeometry {
                what: "an image needs a width and height of at least one pixel",
            });
        }
        let expected = usize::try_from(u64::from(width) * u64::from(height) * 4).unwrap_or(usize::MAX);
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

#[expect(clippy::needless_pass_by_value, reason = "used as map_err(png_error)")]
fn png_error(e: png::DecodingError) -> Error {
    match e {
        png::DecodingError::LimitsExceeded => Error::ImageTooLarge {
            limit: MAX_DECODED_IMAGE_BYTES,
        },
        _ => Error::InvalidImage {
            format: ImageFormat::Png,
            reason: "the PNG decoder rejected the data",
        },
    }
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
                    adobe: false
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
