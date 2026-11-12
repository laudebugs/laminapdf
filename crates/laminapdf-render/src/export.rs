//! [`export_pages`]: pages drawn through a [`RenderDocument`] and encoded as PNG, JPEG or TIFF
//! (the editor plan's operations H.06 and H.11). Feature `export`.

use std::io::Cursor;

use laminapdf_syntax::CancelToken;

use crate::backend::RenderDocument;
use crate::error::RenderError;
use crate::request::{RenderFlags, RenderRequest, Rgba8};

/// The file format pages are exported to.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExportFormat {
    /// PNG, RGBA (the `png` crate), with the resolution in `pHYs`.
    Png,
    /// JPEG at `quality` 1–100 (`jpeg-encoder`), RGB on the export's background, with the
    /// resolution in the JFIF header.
    Jpeg {
        /// 1 (smallest) to 100 (best); clamped to that range.
        quality: u8,
    },
    /// TIFF, RGBA, Deflate-compressed (the `tiff` crate), with the resolution in
    /// `XResolution` and `YResolution`. With `multi_page`, [`export_pages`] writes every page
    /// into one file.
    Tiff {
        /// One file holding every page, instead of one file per page.
        multi_page: bool,
    },
}

/// What [`export_pages`] writes.
///
/// ```
/// use laminapdf_render::{ExportFormat, ExportOptions};
///
/// let options = ExportOptions::new(ExportFormat::Png).with_dpi(150.0);
/// assert_eq!(options.dpi(), 150.0);
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExportOptions {
    format: ExportFormat,
    dpi: f64,
    background: Option<Rgba8>,
    flags: RenderFlags,
}

impl ExportOptions {
    /// `format` at 150 dpi, on white, with the default [`RenderFlags`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(format: ExportFormat) -> Self {
        Self {
            format,
            dpi: 150.0,
            background: Some(Rgba8::WHITE),
            flags: RenderFlags::default(),
        }
    }

    /// The resolution, in pixels per inch (72 is one pixel per point).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_dpi(mut self, dpi: f64) -> Self {
        self.dpi = dpi;
        self
    }

    /// The colour under the page; `None` keeps PNG and TIFF transparent where nothing is drawn.
    /// JPEG has no transparency, so it always goes on an opaque colour: this one, or white.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_background(mut self, background: Option<Rgba8>) -> Self {
        self.background = background;
        self
    }

    /// What is drawn besides the page content (annotations, form widgets, print intent).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_flags(mut self, flags: RenderFlags) -> Self {
        self.flags = flags;
        self
    }

    /// The format.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn format(&self) -> ExportFormat {
        self.format
    }

    /// The resolution, in pixels per inch.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn dpi(&self) -> f64 {
        self.dpi
    }

    fn request(&self, page: u32) -> RenderRequest {
        let background = match self.format {
            ExportFormat::Jpeg { .. } => Some(self.background.filter(|c| c.a == 255).unwrap_or(Rgba8::WHITE)),
            _ => self.background,
        };
        RenderRequest {
            flags: self.flags,
            background,
            ..RenderRequest::page(page, scale(self.dpi))
        }
    }
}

/// Pixels per point for `dpi`, never below the exact ratio. A render's size is the page's
/// size times the scale, rounded down, so an `f32` that lands a hair under `dpi / 72` would cost
/// a page that is a whole number of pixels its last column (Letter at 150 dpi: 1,274, not 1,275).
fn scale(dpi: f64) -> f32 {
    let exact = dpi / 72.0;
    #[expect(clippy::cast_possible_truncation, reason = "the nearest f32, nudged up below")]
    let scale = exact as f32;
    if f64::from(scale) < exact {
        scale.next_up()
    } else {
        scale
    }
}

/// Draws `pages` (0-based) of `doc` one at a time and encodes each as [`ExportOptions`] says:
/// one file per page, in order, except a multi-page TIFF, which is one file. Only one page's
/// pixels are held at a time.
///
/// ```
/// # #[cfg(feature = "hayro")] {
/// use std::sync::Arc;
/// use laminapdf_render::{
///     ByteSource, CancelToken, ExportFormat, ExportOptions, HayroBackend, RenderBackend, RenderOpenOptions,
///     export_pages,
/// };
/// # let pdf = laminapdf_test_fixtures::fixture("corpus/hello.pdf");
/// let doc = HayroBackend::new().open(Arc::new(pdf) as Arc<dyn ByteSource>, &RenderOpenOptions::new())?;
/// let files = export_pages(doc.as_ref(), &[0], &ExportOptions::new(ExportFormat::Png), &CancelToken::new())?;
/// assert!(files[0].starts_with(b"\x89PNG"));
/// # }
/// # Ok::<(), laminapdf_render::RenderError>(())
/// ```
///
/// # Errors
/// [`RenderError::InvalidRequest`] for a resolution that isn't a positive finite number; what
/// [`RenderDocument::render`] returns for a page (an index out of range, a page past
/// [`RenderLimits`](crate::RenderLimits), [`RenderError::Cancelled`]);
/// [`RenderError::TooLarge`] for a page wider or taller than JPEG's 65,535 pixels;
/// [`RenderError::InvalidRequest`] for no pages; [`RenderError::ExportFailed`] if an encoder
/// fails.
///
/// Added in 0.2.0.
pub fn export_pages(
    doc: &dyn RenderDocument,
    pages: &[u32],
    options: &ExportOptions,
    cancel: &CancelToken,
) -> Result<Vec<Vec<u8>>, RenderError> {
    if pages.is_empty() {
        return Err(RenderError::InvalidRequest {
            what: "an export needs at least one page",
        });
    }
    if !options.dpi.is_finite() || options.dpi <= 0.0 {
        return Err(RenderError::InvalidRequest {
            what: "the export resolution must be a positive number of pixels per inch",
        });
    }
    if let ExportFormat::Tiff { multi_page: true } = options.format {
        let mut out = Cursor::new(Vec::new());
        let mut tiff = tiff_encoder(&mut out)?;
        for &page in pages {
            let (width, height, rgba) = draw(doc, page, options, cancel)?;
            write_tiff_page(&mut tiff, width, height, &rgba, options.dpi)?;
        }
        return Ok(vec![out.into_inner()]);
    }
    pages
        .iter()
        .map(|&page| {
            let (width, height, rgba) = draw(doc, page, options, cancel)?;
            encode(width, height, &rgba, options)
        })
        .collect()
}

/// One page's pixels: width, height and straight RGBA.
fn draw(
    doc: &dyn RenderDocument,
    page: u32,
    options: &ExportOptions,
    cancel: &CancelToken,
) -> Result<(u32, u32, Vec<u8>), RenderError> {
    let request = options.request(page);
    let plan = doc.plan(&request)?;
    let mut rgba = vec![0; plan.byte_len()];
    doc.render(&request, &mut rgba, cancel)?;
    Ok((plan.tile.width, plan.tile.height, rgba))
}

fn encode(width: u32, height: u32, rgba: &[u8], options: &ExportOptions) -> Result<Vec<u8>, RenderError> {
    let dpi = options.dpi;
    match options.format {
        ExportFormat::Png => encode_png(width, height, rgba, dpi),
        ExportFormat::Jpeg { quality } => encode_jpeg(width, height, rgba, dpi, quality),
        ExportFormat::Tiff { .. } => {
            let mut out = Cursor::new(Vec::new());
            write_tiff_page(&mut tiff_encoder(&mut out)?, width, height, rgba, dpi)?;
            Ok(out.into_inner())
        }
    }
}

fn encode_failed(format: &'static str) -> RenderError {
    RenderError::ExportFailed { format }
}

/// The metres in an inch.
const METRES_PER_INCH: f64 = 0.0254;

/// `dpi` as a whole number of dots per metre (PNG's unit).
fn dots_per_metre(dpi: f64) -> u32 {
    let dpm = (dpi / METRES_PER_INCH).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to u32's range first"
    )]
    let dpm = dpm.clamp(1.0, f64::from(u32::MAX)) as u32;
    dpm
}

fn encode_png(width: u32, height: u32, rgba: &[u8], dpi: f64) -> Result<Vec<u8>, RenderError> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let dpm = dots_per_metre(dpi);
    encoder.set_pixel_dims(Some(png::PixelDimensions {
        xppu: dpm,
        yppu: dpm,
        unit: png::Unit::Meter,
    }));
    let mut writer = encoder.write_header().map_err(|_| encode_failed("png"))?;
    writer.write_image_data(rgba).map_err(|_| encode_failed("png"))?;
    writer.finish().map_err(|_| encode_failed("png"))?;
    Ok(out)
}

fn encode_jpeg(width: u32, height: u32, rgba: &[u8], dpi: f64, quality: u8) -> Result<Vec<u8>, RenderError> {
    const MAX_SIDE: u32 = 65_535;
    let (Ok(w), Ok(h)) = (u16::try_from(width), u16::try_from(height)) else {
        return Err(RenderError::TooLarge {
            width: u64::from(width),
            height: u64::from(height),
            limit: u64::from(MAX_SIDE),
            unit: "pixels per side",
        });
    };
    let mut out = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut out, quality.clamp(1, 100));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to u16's range first"
    )]
    let density = dpi.round().clamp(1.0, f64::from(u16::MAX)) as u16;
    encoder.set_density(jpeg_encoder::PixelDensity {
        density: (density, density),
        unit: jpeg_encoder::PixelDensityUnit::Inches,
    });
    // The page was drawn on an opaque background, so alpha is 255 everywhere and is dropped.
    encoder
        .encode(rgba, w, h, jpeg_encoder::ColorType::Rgba)
        .map_err(|_| encode_failed("jpeg"))?;
    Ok(out)
}

/// A TIFF being written into a buffer it borrows.
type Tiff<'a> = tiff::encoder::TiffEncoder<&'a mut Cursor<Vec<u8>>>;

fn tiff_encoder(out: &mut Cursor<Vec<u8>>) -> Result<Tiff<'_>, RenderError> {
    Ok(tiff::encoder::TiffEncoder::new(out)
        .map_err(|_| encode_failed("tiff"))?
        .with_compression(tiff::encoder::Compression::Deflate(
            tiff::encoder::DeflateLevel::Balanced,
        )))
}

fn write_tiff_page(tiff: &mut Tiff<'_>, width: u32, height: u32, rgba: &[u8], dpi: f64) -> Result<(), RenderError> {
    let mut image = tiff
        .new_image::<tiff::encoder::colortype::RGBA8>(width, height)
        .map_err(|_| encode_failed("tiff"))?;
    // TIFF resolutions are rationals: hundredths of a dot per inch.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to u32's range first"
    )]
    let hundredths = (dpi * 100.0).round().clamp(1.0, f64::from(u32::MAX)) as u32;
    image.resolution(
        tiff::tags::ResolutionUnit::Inch,
        tiff::encoder::Rational { n: hundredths, d: 100 },
    );
    image.write_data(rgba).map_err(|_| encode_failed("tiff"))
}
