//! Flattening (todo E5e) keeps a page's look: each page of `fixtures/annotations/flatten.pdf`,
//! drawn by hayro with its annotations, matches the same page once annotations are flattened
//! into its content. Hayro draws an annotation's appearance without its `/CA`, `NoView` or
//! `/OC`, and draws nothing for one without an appearance, so this test flattens the ones it
//! draws as ISO 32000-2 says: a state appearance, a hidden square (left in place), a note with
//! its Popup and reply, a turned appearance and a tagged square. PDFium's test in
//! `laminapdf-render-pdfium` flattens every one.
#![cfg(feature = "hayro")]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "tests: a panic is a test failure; pixel counts are small"
)]

use std::sync::Arc;

use laminapdf::{Document, FlattenIntent, FlattenScope, OpenOptions, SaveOptions};
use laminapdf_render::{ByteSource, CancelToken, HayroBackend, RenderBackend, RenderOpenOptions, RenderRequest};
use laminapdf_test_fixtures::fixture;

/// Page `page` at 72 dpi, with its annotations.
fn render(bytes: Vec<u8>, page: u32) -> Vec<u8> {
    let source: Arc<dyn ByteSource> = Arc::new(bytes);
    let doc = HayroBackend::new().open(source, &RenderOpenOptions::new()).unwrap();
    let req = RenderRequest::page(page, 1.0);
    let plan = doc.plan(&req).unwrap();
    let mut out = vec![0; plan.byte_len()];
    doc.render(&req, &mut out, &CancelToken::new()).unwrap();
    out
}

/// The share of pixels that differ by more than a rounding step.
fn differing(a: &[u8], b: &[u8]) -> f64 {
    let pixels = a.len() / 4;
    let changed = a
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| x.abs_diff(*y) > 2))
        .count();
    changed as f64 / pixels as f64
}

fn flattened() -> Vec<u8> {
    let mut doc = Document::open_bytes(fixture("annotations/flatten.pdf"), OpenOptions::new()).unwrap();
    let ids = ["a11.0", "a12.0", "a19.0", "a22.0", "a60.0"]
        .map(|i| i.parse().unwrap())
        .to_vec();
    doc.flatten_annotations(&FlattenScope::Annotations(ids), FlattenIntent::View)
        .unwrap();
    doc.save(&SaveOptions::new()).unwrap().bytes
}

#[test]
fn each_flattened_page_looks_as_it_did() {
    let after = flattened();
    for page in 0..2 {
        let share = differing(
            &render(fixture("annotations/flatten.pdf"), page),
            &render(after.clone(), page),
        );
        assert!(share < 0.001, "page {page}: {share}");
    }
}
