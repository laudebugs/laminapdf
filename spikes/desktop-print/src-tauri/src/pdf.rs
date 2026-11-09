//! PDFium through `pdfium-render`, bound once to the pinned build in `LAMINA_PDF_PDFIUM_DIR`.

use std::path::Path;
use std::sync::OnceLock;

use pdfium_render::prelude::{PdfDocument, PdfRenderConfig, Pdfium, PdfPageRenderRotation};

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();

/// The process-wide bindings.
pub fn pdfium() -> Result<&'static Pdfium, String> {
    if let Some(p) = PDFIUM.get() {
        return Ok(p);
    }
    let dir = std::env::var_os("LAMINA_PDF_PDFIUM_DIR").ok_or("set LAMINA_PDF_PDFIUM_DIR (cargo xtask fetch-pdfium)")?;
    let lib = Pdfium::pdfium_platform_library_name_at_path(Path::new(&dir));
    let bindings = Pdfium::bind_to_library(lib).map_err(|e| e.to_string())?;
    Ok(PDFIUM.get_or_init(|| Pdfium::new(bindings)))
}

/// A second, raw binding to the same library, for PDFium calls `pdfium-render` doesn't wrap
/// (path C on Windows: `FPDF_RenderPage` onto a printer DC). `pdfium-render` 0.9.4 keeps its
/// own bindings private; loading the library again reuses the loaded copy.
pub fn raw_bindings() -> Result<Box<dyn pdfium_render::prelude::PdfiumLibraryBindings>, String> {
    pdfium()?; // initialises the library once
    let dir = std::env::var_os("LAMINA_PDF_PDFIUM_DIR").ok_or("set LAMINA_PDF_PDFIUM_DIR")?;
    Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(Path::new(&dir))).map_err(|e| e.to_string())
}

/// A page's displayed size in points (`/Rotate` applied).
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct PageSize {
    pub w: f32,
    pub h: f32,
}

impl PageSize {
    pub fn landscape(self) -> bool {
        self.w > self.h
    }
}

/// Straight RGBA8, rows of `width × 4`.
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Raster {
    /// BGRA, as GDI's `StretchDIBits` and cairo's ARGB32 (little-endian) want it. Pages are
    /// opaque, so premultiplying changes nothing.
    pub fn bgra(&self) -> Vec<u8> {
        let mut out = self.rgba.clone();
        for px in out.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        out
    }
}

pub fn open(bytes: &[u8]) -> Result<PdfDocument<'static>, String> {
    pdfium()?.load_pdf_from_byte_vec(bytes.to_vec(), None).map_err(|e| e.to_string())
}

pub fn page_sizes(bytes: &[u8]) -> Result<Vec<PageSize>, String> {
    let doc = open(bytes)?;
    Ok(doc
        .pages()
        .iter()
        .map(|p| PageSize { w: p.width().value, h: p.height().value })
        .collect())
}

/// Page `index` at `dpi` on white, with the print intent (annotations' Print flags honoured).
pub fn render(doc: &PdfDocument<'_>, index: u16, dpi: f32) -> Result<Raster, String> {
    let page = doc.pages().get(i32::from(index)).map_err(|e| e.to_string())?;
    let (w, h) = (page.width().value, page.height().value);
    let (pw, ph) = ((w * dpi / 72.0).round() as i32, (h * dpi / 72.0).round() as i32);
    render_at(&page, pw.max(1), ph.max(1), true)
}

/// The page at exactly `width × height` pixels (the comparison renders both PDFs this way).
pub fn render_at(
    page: &pdfium_render::prelude::PdfPage<'_>,
    width: i32,
    height: i32,
    print: bool,
) -> Result<Raster, String> {
    let config = PdfRenderConfig::new()
        .set_target_size(width, height)
        .rotate(PdfPageRenderRotation::None, false)
        .render_form_data(true)
        .set_format(pdfium_render::prelude::PdfBitmapFormat::BGRA)
        .use_print_quality(print);
    let bitmap = page.render_with_config(&config).map_err(|e| e.to_string())?;
    let image = bitmap.as_rgba_bytes();
    Ok(Raster { width: bitmap.width() as u32, height: bitmap.height() as u32, rgba: image })
}

/// The printable fixtures, by name, from `fixtures/`.
pub const FIXTURES: [&str; 3] = ["mixed-50", "mixed-sizes", "vectors"];
