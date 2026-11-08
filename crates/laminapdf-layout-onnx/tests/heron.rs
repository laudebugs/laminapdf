//! Heron on two example pages, rendered by hayro, against the reference run that
//! `tools/models/heron_reference.py` recorded with ONNX Runtime's Python package and PDFium.
//! Ignored by default: it needs the weights and the ONNX Runtime library.
//!
//! ```sh
//! cargo xtask models fetch docling-layout-heron
//! LAMINA_PDF_HERON_MODEL=target/models/docling-layout-heron/<sha256>/model.onnx \
//! LAMINA_PDF_ONNXRUNTIME=/path/to/libonnxruntime.so.1.31.1 \
//!   cargo test -p laminapdf-layout-onnx --test heron -- --ignored
//! ```
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use std::sync::Arc;

use laminapdf_extract::{LayoutModel, PageRaster, RegionLabel};
use laminapdf_layout_onnx::HeronLayout;
use laminapdf_model::{Pt, RectPt};
use laminapdf_render::{ByteSource, CancelToken, HayroBackend, RenderBackend, RenderOpenOptions, RenderRequest};
use laminapdf_test_fixtures::fixture;

/// The renders differ (hayro here, PDFium there), so boxes match within this overlap.
const MIN_IOU: f64 = 0.8;
/// Regions the reference scored this well must be found; weaker ones may come and go.
const STABLE: f64 = 0.5;

fn env_path(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name}; see this file's header"))
}

fn raster(name: &str, dpi: u32) -> PageRaster {
    let source: Arc<dyn ByteSource> = Arc::new(fixture(&format!("golden/examples/{name}")));
    let doc = HayroBackend::new().open(source, &RenderOpenOptions::default()).unwrap();
    let page = doc.page(0).unwrap();
    #[expect(clippy::cast_precision_loss, reason = "a dpi is small")]
    let req = RenderRequest::page(0, dpi as f32 / 72.0);
    let plan = doc.plan(&req).unwrap();
    let mut pixels = vec![0; plan.byte_len()];
    doc.render(&req, &mut pixels, &CancelToken::new()).unwrap();
    let page_box = RectPt::new(
        Pt::new(0.0),
        Pt::new(0.0),
        Pt::new(page.width.into()),
        Pt::new(page.height.into()),
    );
    PageRaster::new(plan.tile.width, plan.tile.height, dpi, pixels, page_box).unwrap()
}

fn name_of(label: RegionLabel) -> &'static str {
    match label {
        RegionLabel::Caption => "caption",
        RegionLabel::Footnote => "footnote",
        RegionLabel::Formula => "formula",
        RegionLabel::ListItem => "list-item",
        RegionLabel::PageFooter => "page-footer",
        RegionLabel::PageHeader => "page-header",
        RegionLabel::Picture => "picture",
        RegionLabel::SectionHeader => "section-header",
        RegionLabel::Table => "table",
        RegionLabel::Text => "text",
        RegionLabel::Title => "title",
        _ => "other",
    }
}

/// Labels the model swaps between on near-identical renders: a page's title is often scored
/// almost evenly as a title and a section header.
fn family(label: &str) -> &str {
    match label {
        "title" | "section-header" => "heading",
        other => other,
    }
}

fn iou(a: &RectPt, b: [f64; 4]) -> f64 {
    let w = (a.x1.get().min(b[2]) - a.x0.get().max(b[0])).max(0.0);
    let h = (a.y1.get().min(b[3]) - a.y0.get().max(b[1])).max(0.0);
    let union = a.width().get() * a.height().get() + (b[2] - b[0]) * (b[3] - b[1]) - w * h;
    if union > 0.0 { w * h / union } else { 0.0 }
}

#[test]
#[ignore = "needs the Heron weights and ONNX Runtime (see the file's header)"]
fn heron_should_find_the_reference_regions() {
    let weights = std::fs::read(env_path("LAMINA_PDF_HERON_MODEL")).unwrap();
    let heron = HeronLayout::new(&weights, env_path("LAMINA_PDF_ONNXRUNTIME")).unwrap();
    let reference: serde_json::Value = serde_json::from_str(include_str!("reference/heron.json")).unwrap();
    let dpi = u32::try_from(reference["dpi"].as_u64().unwrap()).unwrap();
    for (name, expected) in reference["pages"].as_object().unwrap() {
        let found = heron.detect(&raster(name, dpi), &CancelToken::new()).unwrap();
        for region in expected.as_array().unwrap() {
            if region["score"].as_f64().unwrap() < STABLE {
                continue;
            }
            let label = region["label"].as_str().unwrap();
            let b: Vec<f64> = region["bbox"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            let best = found
                .iter()
                .filter(|r| family(name_of(r.label)) == family(label))
                .map(|r| iou(&r.bbox, [b[0], b[1], b[2], b[3]]))
                .fold(0.0, f64::max);
            assert!(
                best >= MIN_IOU,
                "{name}: {label} at {b:?} best IoU {best:.3}; found {found:?}"
            );
        }
    }
}

#[test]
fn heron_should_refuse_other_weights() {
    let err = HeronLayout::new(b"not the model", "/nonexistent/libonnxruntime.so").unwrap_err();
    assert!(err.to_string().starts_with("the weights' SHA-256 is"), "{err}");
}
