//! Render goldens (ADR 0004, todo C2b): every page of every `fixtures/corpus/` file, rendered by
//! each backend at 96 DPI on white, against that backend's own committed PNG in
//! `fixtures/golden/render/<backend>/` (SSIM ≥ 0.99; cross-backend parity is X3b's nightly diff).
//! Files a backend can't render are listed with a reason in `fixtures/golden/render/skip.json`.
//! A test never writes a golden: `cargo xtask render-golden --update` does
//! (`examples/goldens.rs`). Ignored: needs the PDFium library, and goldens are only stable on
//! the platform that wrote them (macOS arm64, the CI runners').
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

mod common;

use common::{golden_dir, golden_name, read_png, render_file, ssim};

/// The SSIM a render must reach against its own backend's golden.
const MIN_SSIM: f64 = 0.99;

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn every_corpus_page_matches_its_golden() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for backend in common::backends() {
        let skip = common::skipped(backend.name());
        let mut rendered = std::collections::BTreeSet::new();
        for file in common::corpus() {
            if skip.contains_key(&file) {
                continue;
            }
            for (i, bitmap) in render_file(backend.as_ref(), &file).iter().enumerate() {
                rendered.insert(golden_name(&file, i));
                let path = golden_dir(backend.name()).join(golden_name(&file, i));
                if !path.is_file() {
                    failures.push(format!(
                        "{}: missing (run cargo xtask render-golden --update)",
                        path.display()
                    ));
                    continue;
                }
                let golden = read_png(&path);
                if (golden.width, golden.height) != (bitmap.width, bitmap.height) {
                    failures.push(format!(
                        "{}: {}×{} rendered, {}×{} golden",
                        path.display(),
                        bitmap.width,
                        bitmap.height,
                        golden.width,
                        golden.height
                    ));
                    continue;
                }
                let score = ssim(bitmap, &golden);
                if score < MIN_SSIM {
                    failures.push(format!("{}: SSIM {score:.4} < {MIN_SSIM}", path.display()));
                }
                checked += 1;
            }
        }
        // A golden of a page that no longer renders (a lost page) fails too.
        for entry in std::fs::read_dir(golden_dir(backend.name())).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            let png = std::path::Path::new(&name)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("png"));
            if png && !rendered.contains(&name) {
                failures.push(format!("{}/{name}: no page renders to it any more", backend.name()));
            }
        }
    }
    assert!(checked > 0, "no golden was checked");
    assert!(
        failures.is_empty(),
        "{} render golden(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn every_skip_names_a_corpus_file() {
    let corpus = common::corpus();
    for backend in ["pdfium", "hayro"] {
        for (file, reason) in common::skipped(backend) {
            assert!(
                corpus.contains(&file),
                "skip.json lists {file} for {backend}, which isn't in fixtures/corpus"
            );
            assert!(!reason.trim().is_empty(), "{backend} {file}: give a reason");
        }
    }
}
