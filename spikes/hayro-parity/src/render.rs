//! The two renderers behind one shape: load a document once, render page `n`
//! (0-based) to an opaque RGB bitmap at a given DPI.
//!
//! Settings, kept deliberately close to each engine's defaults so the numbers
//! describe what a lamina user would get out of the box:
//! - Page area: the CropBox (intersected with the MediaBox) with `/Rotate`
//!   applied. hayro: `Page::render_dimensions` + `initial_transform(true)`;
//!   PDFium: `FPDF_GetPageWidthF/HeightF` and `FPDF_RenderPageBitmap`.
//! - Size: `floor(points * dpi / 72)` per axis from each engine's own page size,
//!   so a disagreement on the page box shows up as a size mismatch.
//! - White background, annotations on (hayro `render_annotations`, PDFium
//!   `FPDF_ANNOT`), PDFium form data on (`FPDF_FFLDraw`), both anti-aliased as
//!   they ship (PDFium smooth text/image/path on, no LCD text; hayro has no knobs).
//! - Fonts: hayro's `embed-fonts` defaults (Foxit standard-14 CFF fonts, no
//!   system fonts); PDFium's built-in Foxit fonts plus the system font mapper.
//! - Very large pages are scaled down so the longest side is at most
//!   [`MAX_SIDE`] pixels, by the same rule in both engines.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings};
use pdfium_render::prelude::*;

/// Longest side of a rendered page, in pixels (hayro's canvas is `u16`; also bounds the SSIM buffers).
pub const MAX_SIDE: f32 = 3000.0;

/// An opaque RGB bitmap (alpha composited over white).
pub struct Rgb {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Pixel scale for a page of `w`×`h` points at `dpi`, capped at [`MAX_SIDE`].
fn scale_for(w: f32, h: f32, dpi: f32) -> f32 {
    let scale = dpi / 72.0;
    let longest = w.max(h) * scale;
    if longest > MAX_SIDE {
        scale * MAX_SIDE / longest
    } else {
        scale
    }
}

pub struct HayroDoc {
    pdf: Pdf,
    settings: InterpreterSettings,
    warnings: Arc<Mutex<Vec<String>>>,
}

impl HayroDoc {
    pub fn load(bytes: Vec<u8>) -> Result<Self> {
        let pdf = Pdf::new(bytes).map_err(|e| anyhow!("{e:?}"))?;
        let warnings = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&warnings);
        let settings = InterpreterSettings {
            warning_sink: Arc::new(move |w| {
                if let Ok(mut v) = sink.lock() {
                    v.push(format!("{w:?}"));
                }
            }),
            ..Default::default()
        };
        Ok(Self {
            pdf,
            settings,
            warnings,
        })
    }

    pub fn page_count(&self) -> usize {
        self.pdf.pages().len()
    }

    /// Renders page `index`; also returns the distinct interpreter warnings it raised.
    pub fn render(&self, index: usize, dpi: f32) -> Result<(Rgb, Vec<String>)> {
        let page = self.pdf.pages().get(index).context("no such page")?;
        let (w, h) = page.render_dimensions();
        let scale = scale_for(w, h, dpi);
        if (w * scale) < 1.0 || (h * scale) < 1.0 {
            return Err(anyhow!("empty page box {w}x{h}"));
        }
        if let Ok(mut v) = self.warnings.lock() {
            v.clear();
        }
        // A fresh cache per page keeps pages independent (a crash or a slow page
        // doesn't depend on what was rendered before) at a small speed cost.
        let cache = RenderCache::new();
        let pixmap = hayro::render(
            page,
            &cache,
            &self.settings,
            &RenderSettings::default(),
            &PixmapSettings {
                x_scale: scale,
                y_scale: scale,
                bg_color: WHITE,
            },
        );
        let premul = pixmap.data_as_u8_slice();
        let mut data = Vec::with_capacity(premul.len() / 4 * 3);
        for px in premul.as_chunks::<4>().0 {
            // Premultiplied over white: c + (255 - a).
            let inv = 255 - px[3];
            data.extend(px[..3].iter().map(|&c| c.saturating_add(inv)));
        }
        let mut warnings = self.warnings.lock().map(|v| v.clone()).unwrap_or_default();
        warnings.sort();
        warnings.dedup();
        let img = Rgb {
            width: u32::from(pixmap.width()),
            height: u32::from(pixmap.height()),
            data,
        };
        Ok((img, warnings))
    }
}

pub fn bind_pdfium(lib_dir: &Path) -> Result<Pdfium> {
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(lib_dir))
        .map_err(|e| anyhow!("bind pdfium in {}: {e:?}", lib_dir.display()))?;
    Ok(Pdfium::new(bindings))
}

pub struct PdfiumDoc<'a> {
    doc: PdfDocument<'a>,
}

impl<'a> PdfiumDoc<'a> {
    pub fn load(pdfium: &'a Pdfium, bytes: &'a [u8]) -> Result<Self> {
        let doc = pdfium
            .load_pdf_from_byte_slice(bytes, None)
            .map_err(|e| anyhow!("{e:?}"))?;
        Ok(Self { doc })
    }

    pub fn page_count(&self) -> usize {
        usize::try_from(self.doc.pages().len()).unwrap_or(0)
    }

    pub fn render(&self, index: usize, dpi: f32) -> Result<Rgb> {
        let idx = PdfPageIndex::try_from(index).context("page index")?;
        let page = self.doc.pages().get(idx).map_err(|e| anyhow!("page: {e:?}"))?;
        let (w, h) = (page.width().value, page.height().value);
        let scale = scale_for(w, h, dpi);
        let (pw, ph) = ((w * scale) as i32, (h * scale) as i32);
        if pw < 1 || ph < 1 {
            return Err(anyhow!("empty page box {w}x{h}"));
        }
        let config = PdfRenderConfig::new()
            .set_fixed_size(pw, ph)
            .set_clear_color(PdfColor::WHITE)
            .render_annotations(true)
            .render_form_data(true);
        let bitmap = page.render_with_config(&config).map_err(|e| anyhow!("render: {e:?}"))?;
        let rgba = bitmap.as_rgba_bytes();
        let (width, height) = (bitmap.width() as u32, bitmap.height() as u32);
        let mut data = Vec::with_capacity(rgba.len() / 4 * 3);
        for px in rgba.as_chunks::<4>().0 {
            // Straight alpha over white (the bitmap is cleared to opaque white, so a == 255
            // in practice).
            let a = u32::from(px[3]);
            data.extend(
                px[..3]
                    .iter()
                    .map(|&c| ((u32::from(c) * a + 255 * (255 - a)) / 255) as u8),
            );
        }
        Ok(Rgb { width, height, data })
    }
}
