//! Pages copied in from another document (`InsertPagesFrom`) render as they did in their
//! source: up to three pages of each of ten corpus files are copied into `hello.pdf`, the result
//! is saved, and each copy is rendered beside its source page. hayro runs in every test run;
//! PDFium's run is ignored, as its other tests are (`cargo xtask render-golden` runs it).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{open, pdfium, render};
use laminapdf::{Document, InsertPagesFrom, OpenOptions, PagePosition, SaveOptions};
use laminapdf_render::{HayroBackend, RenderBackend, RenderOpenOptions, RenderRequest};
use laminapdf_test_fixtures::fixture;

/// The corpus files whose pages are copied: every unencrypted, unsigned one that opens.
const SOURCES: [&str; 10] = [
    "fpdf2.pdf",
    "hello-incremental.pdf",
    "hello-xrefstm.pdf",
    "hello.pdf",
    "imaging-demo.pdf",
    "outline.pdf",
    "pdflib.pdf",
    "raw.pdf",
    "reportlab.pdf",
    "text-example.pdf",
];

/// The pages copied per file, to keep the run short.
const MAX_PAGES: usize = 3;

/// `hello.pdf` with up to [`MAX_PAGES`] pages of `source` copied in after its page, saved:
/// the bytes and how many pages were copied.
fn imported(source: &[u8]) -> (Vec<u8>, usize) {
    let mut doc = Document::open_bytes(fixture("corpus/hello.pdf"), OpenOptions::default()).unwrap();
    let id = doc.add_source(source.to_vec(), None).unwrap();
    let pages: Vec<_> = doc.source_pages(&id).unwrap().into_iter().take(MAX_PAGES).collect();
    let count = pages.len();
    doc.apply(vec![InsertPagesFrom::new(id, pages, PagePosition::End).into()])
        .unwrap();
    (doc.save(&SaveOptions::new()).unwrap().bytes, count)
}

/// Every copy rendered by `backend` as its source page is.
fn copies_render_as_their_sources(backend: &dyn RenderBackend) {
    for name in SOURCES {
        let source = fixture(&format!("corpus/{name}"));
        let (bytes, count) = imported(&source);
        let before = open(backend, source, &RenderOpenOptions::new()).unwrap();
        let after = open(backend, bytes, &RenderOpenOptions::new()).unwrap();
        for page in 0..u32::try_from(count).unwrap() {
            let want = render(before.as_ref(), &RenderRequest::page(page, 1.0)).unwrap();
            let got = render(after.as_ref(), &RenderRequest::page(page + 1, 1.0)).unwrap();
            assert_eq!(
                (got.width, got.height),
                (want.width, want.height),
                "{} {name} page {}",
                backend.name(),
                page + 1
            );
            let differing = got
                .rgba
                .chunks(4)
                .zip(want.rgba.chunks(4))
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(differing, 0, "{} {name} page {}", backend.name(), page + 1);
        }
    }
}

#[test]
fn copied_pages_render_as_their_sources_in_hayro() {
    copies_render_as_their_sources(&HayroBackend::new());
}

#[test]
#[ignore = "needs PDFium: cargo xtask render-golden"]
fn copied_pages_render_as_their_sources_in_pdfium() {
    copies_render_as_their_sources(&pdfium());
}
