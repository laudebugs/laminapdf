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

use lamina_render::{
    ByteSource, CancelToken, HayroBackend, RenderBackend, RenderDocument, RenderError, RenderOpenOptions, RenderRequest,
};
use lamina_render_pdfium::PdfiumBackend;

/// Render goldens' resolution.
pub const DPI: f32 = 96.0;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixture(path: &str) -> Vec<u8> {
    let p = root().join("fixtures").join(path);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The golden folder of `backend`.
pub fn golden_dir(backend: &str) -> PathBuf {
    root().join("fixtures/golden/render").join(backend)
}

/// Every PDF of `fixtures/corpus/`, by file name.
pub fn corpus() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root().join("fixtures/corpus"))
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
        serde_json::from_slice(&std::fs::read(root().join("fixtures/golden/render/skip.json")).unwrap()).unwrap();
    v[backend]
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, r)| (k.clone(), r.as_str().unwrap().to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

/// PDFium, bound from `LAMINA_PDFIUM_DIR` (`cargo xtask fetch-pdfium`).
pub fn pdfium() -> PdfiumBackend {
    PdfiumBackend::from_env().unwrap_or_else(|e| {
        panic!("{e}: run `cargo xtask fetch-pdfium` and set LAMINA_PDFIUM_DIR (or use cargo xtask render-golden)")
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

/// One rendered bitmap.
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    /// Straight RGBA8.
    pub rgba: Vec<u8>,
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

pub fn write_png(path: &Path, bitmap: &Bitmap) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), bitmap.width, bitmap.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    encoder.write_header().unwrap().write_image_data(&bitmap.rgba).unwrap();
}

pub fn read_png(path: &Path) -> Bitmap {
    let decoder = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    ));
    let mut reader = decoder.read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba, "{}", path.display());
    rgba.truncate(info.buffer_size());
    Bitmap {
        width: info.width,
        height: info.height,
        rgba,
    }
}

/// Mean SSIM of the two bitmaps' luma over 8 × 8 windows (stride 4). 1.0 is identical.
pub fn ssim(a: &Bitmap, b: &Bitmap) -> f64 {
    assert_eq!((a.width, a.height), (b.width, b.height), "sizes differ");
    let luma = |bm: &Bitmap| -> Vec<f64> {
        bm.rgba
            .chunks(4)
            .map(|p| 0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]))
            .collect()
    };
    let (la, lb) = (luma(a), luma(b));
    let (w, h) = (a.width as usize, a.height as usize);
    let (c1, c2) = ((0.01 * 255.0f64).powi(2), (0.03 * 255.0f64).powi(2));
    let (mut total, mut windows) = (0.0, 0usize);
    let size = 8.min(w).min(h);
    let mut y = 0;
    while y + size <= h {
        let mut x = 0;
        while x + size <= w {
            let n = (size * size) as f64;
            let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for yy in y..y + size {
                for xx in x..x + size {
                    let (p, q) = (la[yy * w + xx], lb[yy * w + xx]);
                    sa += p;
                    sb += q;
                    saa += p * p;
                    sbb += q * q;
                    sab += p * q;
                }
            }
            let (ma, mb) = (sa / n, sb / n);
            let (va, vb, cov) = (saa / n - ma * ma, sbb / n - mb * mb, sab / n - ma * mb);
            total += ((2.0 * ma * mb + c1) * (2.0 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2));
            windows += 1;
            x += 4;
        }
        y += 4;
    }
    if windows == 0 { 1.0 } else { total / windows as f64 }
}
