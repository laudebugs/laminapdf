//! Flattening (todo E5e) keeps a page's look in PDFium: each page of
//! `fixtures/annotations/flatten.pdf`, and of the pdf.js corpus's annotation files, rendered with
//! its annotations matches the page rendered after they are flattened into its content, on
//! screen and printed (SSIM ≥ 0.99 on luma, the render goldens' threshold). Ignored: needs the
//! PDFium library (`cargo xtask render-golden` runs it), and the corpus test needs
//! `cargo xtask corpus fetch --source pdfjs`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{Bitmap, open, pdfium, render};
use laminapdf::{Document, FlattenIntent, FlattenScope, OpenOptions, SaveOptions};
use laminapdf_render::{RenderFlags, RenderOpenOptions, RenderRequest};
use laminapdf_test_fixtures::image::ssim;
use laminapdf_test_fixtures::{fixture, fixtures_dir};

/// The pdf.js files whose flattened pages differ from their annotated ones, and why.
const DIFFERENT: [(&str, &str); 3] = [
    (
        "annotation-line-without-appearance-empty-Rect.pdf",
        "its line has no appearance: Lamina PDF draws its own, which differs from the one PDFium makes up",
    ),
    (
        "annotation-polyline-polygon-without-appearance.pdf",
        "its shapes have no appearance: Lamina PDF draws its own, which differs from the ones PDFium makes up",
    ),
    (
        "annotation-square-circle-without-appearance.pdf",
        "its shapes have no appearance: Lamina PDF draws its own, which differs from the ones PDFium makes up",
    ),
];

/// The fixture's annotations but two: its translucent square, since PDFium draws an
/// annotation's appearance without its `/CA`, which flattening applies (ISO 32000-2 §12.5.2);
/// and its square without an appearance, which Lamina PDF and PDFium each draw their own way.
const COMPARED: [&str; 9] = [
    "a11.0", "a12.0", "a13.0", "a14.0", "a15.0", "a17.0", "a19.0", "a22.0", "a60.0",
];

/// Every page of `bytes` at 72 dpi, with annotations and widgets, printed or on screen.
fn pages(bytes: Vec<u8>, print: bool) -> Vec<Bitmap> {
    let backend = pdfium();
    let doc = open(&backend, bytes, &RenderOpenOptions::new()).unwrap();
    let flags = RenderFlags {
        print,
        ..RenderFlags::default()
    };
    (0..doc.page_count())
        .map(|i| render(doc.as_ref(), &RenderRequest::page(i, 1.0).with_flags(flags)).unwrap())
        .collect()
}

/// `bytes` with the comments of `scope` flattened for `intent`.
fn flattened(bytes: Vec<u8>, scope: &FlattenScope, intent: FlattenIntent) -> laminapdf::Result<Vec<u8>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    doc.flatten_annotations(scope, intent)?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

/// The lowest SSIM of a page before and after flattening `scope`, for each intent.
fn worst(bytes: &[u8], scope: &FlattenScope) -> laminapdf::Result<f64> {
    let mut worst = 1.0_f64;
    for (intent, print) in [(FlattenIntent::View, false), (FlattenIntent::Print, true)] {
        let after = flattened(bytes.to_vec(), scope, intent)?;
        for (before, after) in pages(bytes.to_vec(), print).iter().zip(pages(after, print).iter()) {
            worst = worst.min(ssim(before, after));
        }
    }
    Ok(worst)
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn the_fixtures_flattened_pages_look_as_they_did() {
    let scope = FlattenScope::Annotations(COMPARED.map(|id| id.parse().unwrap()).to_vec());
    let worst = worst(&fixture("annotations/flatten.pdf"), &scope).unwrap();
    assert!(worst >= 0.99, "SSIM {worst}");
}

#[test]
#[ignore = "needs the PDFium library and the pdf.js corpus: cargo xtask corpus fetch --source pdfjs"]
fn the_pdfjs_annotation_files_flatten_and_look_as_they_did() {
    let dir = fixtures_dir().join("../target/corpus/pdfjs/test/pdfs");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}; run cargo xtask corpus fetch --source pdfjs", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| {
            n.starts_with("annotation-")
                && std::path::Path::new(n)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        })
        .collect();
    names.sort();
    let mut failures = Vec::new();
    for name in &names {
        if DIFFERENT.iter().any(|(listed, _)| listed == name) {
            continue;
        }
        let bytes = std::fs::read(dir.join(name)).unwrap();
        match worst(&bytes, &FlattenScope::All) {
            Ok(score) if score >= 0.99 => {}
            Ok(score) => failures.push(format!("{name}: SSIM {score:.4}")),
            Err(e) => failures.push(format!("{name}: {}", e.code())),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} files:\n{}",
        failures.len(),
        names.len(),
        failures.join("\n")
    );
}
