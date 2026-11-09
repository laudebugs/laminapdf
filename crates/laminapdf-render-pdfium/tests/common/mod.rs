//! Shared by the tests and `examples/goldens.rs`: the render-golden corpus, both backends, PNG
//! reading and writing, and the comparison (SSIM on luma, ADR 0004 / C2b: ≥ 0.99 against the
//! backend's own golden).
#![allow(
    dead_code,
    unreachable_pub,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    reason = "test helpers (SSIM uses its paper's names): a panic is a test failure; not every test uses every helper"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use laminapdf_render::{
    ByteSource, CancelToken, HayroBackend, PageAppearance, RenderBackend, RenderDocument, RenderError,
    RenderOpenOptions, RenderRequest, Rgba8,
};
use laminapdf_render_pdfium::PdfiumBackend;
pub use laminapdf_test_fixtures::image::Bitmap;
use laminapdf_test_fixtures::{fixture, fixtures_dir};

/// Render goldens' resolution.
pub const DPI: f32 = 96.0;

/// The golden folder of `backend`.
pub fn golden_dir(backend: &str) -> PathBuf {
    fixtures_dir().join("golden/render").join(backend)
}

/// Every PDF of `fixtures/corpus/`, by file name.
pub fn corpus() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir().join("corpus"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| Path::new(n).extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")))
        .collect();
    names.sort();
    names
}

/// `fixtures/golden/render/skip.json`: per backend, the corpus files it doesn't render, each
/// with its reason.
pub fn skipped(backend: &str) -> BTreeMap<String, String> {
    let v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixtures_dir().join("golden/render/skip.json")).unwrap()).unwrap();
    v[backend]
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, r)| (k.clone(), r.as_str().unwrap().to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

/// PDFium, bound from `LAMINA_PDF_PDFIUM_DIR` (`cargo xtask fetch-pdfium`).
pub fn pdfium() -> PdfiumBackend {
    PdfiumBackend::from_env().unwrap_or_else(|e| {
        panic!("{e}: run `cargo xtask fetch-pdfium` and set LAMINA_PDF_PDFIUM_DIR (or use cargo xtask render-golden)")
    })
}

/// Both backends, PDFium first.
pub fn backends() -> Vec<Box<dyn RenderBackend>> {
    vec![Box::new(pdfium()), Box::new(HayroBackend::new())]
}

pub fn open(
    backend: &dyn RenderBackend,
    bytes: Vec<u8>,
    options: &RenderOpenOptions,
) -> Result<Box<dyn RenderDocument>, RenderError> {
    let source: Arc<dyn ByteSource> = Arc::new(bytes);
    backend.open(source, options)
}

pub fn render(doc: &dyn RenderDocument, req: &RenderRequest) -> Result<Bitmap, RenderError> {
    let plan = doc.plan(req)?;
    let mut rgba = vec![0; plan.byte_len()];
    doc.render(req, &mut rgba, &CancelToken::new())?;
    Ok(Bitmap {
        width: plan.tile.width,
        height: plan.tile.height,
        rgba,
    })
}

/// Every page of `file` at the golden resolution, on white.
pub fn render_file(backend: &dyn RenderBackend, file: &str) -> Vec<Bitmap> {
    let doc = open(backend, fixture(&format!("corpus/{file}")), &RenderOpenOptions::new())
        .unwrap_or_else(|e| panic!("{} can't open {file}: {e}", backend.name()));
    (0..doc.page_count())
        .map(|i| {
            render(doc.as_ref(), &RenderRequest::page(i, DPI / 72.0))
                .unwrap_or_else(|e| panic!("{} can't render {file} page {}: {e}", backend.name(), i + 1))
        })
        .collect()
}

/// `<stem>-p<N>.png`, N 1-based.
pub fn golden_name(file: &str, index: usize) -> String {
    format!("{}-p{}.png", file.trim_end_matches(".pdf"), index + 1)
}

/// The page appearance fixtures (`tools/fixtures/appearance.py`): one page each.
pub const APPEARANCE_FILES: [&str; 5] = ["text", "vector", "photo", "mixed", "annotated"];

/// Each page appearance by its golden name; High contrast as white ink on black paper.
pub const APPEARANCES: [(&str, PageAppearance); 4] = [
    ("original", PageAppearance::Original),
    ("sepia", PageAppearance::Sepia),
    ("dark", PageAppearance::Dark),
    (
        "high-contrast",
        PageAppearance::HighContrast {
            ink: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            paper: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        },
    ),
];

/// The appearance golden folder of `backend`.
pub fn appearance_dir(backend: &str) -> PathBuf {
    fixtures_dir().join("golden/render-appearance").join(backend)
}

/// `fixtures/appearance/<stem>.pdf`'s page at the golden resolution, on white, in `appearance`.
pub fn render_appearance(backend: &dyn RenderBackend, stem: &str, appearance: PageAppearance) -> Bitmap {
    let doc = open(
        backend,
        fixture(&format!("appearance/{stem}.pdf")),
        &RenderOpenOptions::new(),
    )
    .unwrap_or_else(|e| panic!("{} can't open {stem}: {e}", backend.name()));
    let req = RenderRequest::page(0, DPI / 72.0).with_appearance(appearance);
    render(doc.as_ref(), &req).unwrap_or_else(|e| panic!("{} can't render {stem}: {e}", backend.name()))
}

/// A page-space rectangle (points, top-left origin) in golden-resolution pixels, `inset` in.
pub fn points_to_pixels([x0, y0, x1, y1]: [f32; 4], inset: u32) -> [u32; 4] {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "small positive points"
    )]
    let px = |v: f32| (v * DPI / 72.0).round() as u32;
    [px(x0) + inset, px(y0) + inset, px(x1) - inset, px(y1) - inset]
}
