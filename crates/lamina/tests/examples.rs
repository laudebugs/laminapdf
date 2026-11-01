//! ADR 0018 tier 3 for the façade's examples: each example's document is byte-identical to
//! its golden in `fixtures/golden/examples/` and to a second run (ADR 0001 determinism).
//! `cargo xtask golden --update` (or `cargo xtask examples`) regenerates the goldens by running
//! the examples; this test only compares.
#![expect(clippy::panic, reason = "test helper: a panic is a test failure")]

#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/generate_then_edit.rs"]
mod generate_then_edit;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/hello.rs"]
mod hello;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/invoice.rs"]
mod invoice;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/links_and_outlines.rs"]
mod links_and_outlines;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/multilingual.rs"]
mod multilingual;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/pdf_a3_invoice_with_xml.rs"]
mod pdf_a3_invoice_with_xml;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/pdf_a_archive.rs"]
mod pdf_a_archive;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/pdf_ua_report.rs"]
mod pdf_ua_report;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/vectors.rs"]
mod vectors;

use std::path::Path;

type Build = fn() -> Result<Vec<u8>, Box<dyn std::error::Error>>;

/// Every example, by the name of its golden (`<example>-<variant>` for an example that writes
/// several).
const EXAMPLES: [(&str, Build); 10] = [
    ("hello", hello::build),
    ("invoice", invoice::build),
    ("multilingual", multilingual::build),
    ("vectors", vectors::build),
    ("links_and_outlines", links_and_outlines::build),
    ("generate_then_edit", generate_then_edit::build),
    ("pdf_ua_report", pdf_ua_report::build),
    ("pdf_a_archive-2b", || pdf_a_archive::build(pdf_a_archive::Level::A2b)),
    ("pdf_a_archive-2u", || pdf_a_archive::build(pdf_a_archive::Level::A2u)),
    ("pdf_a3_invoice_with_xml", pdf_a3_invoice_with_xml::build),
];

fn golden(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/golden/examples/{name}.pdf"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}; run `cargo xtask golden --update`", path.display()))
}

#[test]
fn every_example_matches_its_golden() {
    let stale: Vec<&str> = EXAMPLES
        .iter()
        .filter(|(name, build)| build().unwrap() != golden(name))
        .map(|(name, _)| *name)
        .collect();
    assert!(
        stale.is_empty(),
        "examples differ from fixtures/golden/examples/: {stale:?}"
    );
}

#[test]
fn every_example_gives_the_same_bytes_twice() {
    let differ: Vec<&str> = EXAMPLES
        .iter()
        .filter(|(_, build)| build().unwrap() != build().unwrap())
        .map(|(name, _)| *name)
        .collect();
    assert!(differ.is_empty(), "nondeterministic examples: {differ:?}");
}

#[test]
fn every_example_reopens_without_warnings() {
    for (name, build) in EXAMPLES {
        let doc = lamina::Document::open_bytes(build().unwrap(), lamina::OpenOptions::default().with_strict(true))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(doc.page_count().unwrap() > 0, "{name}");
        assert_eq!(doc.warnings().remaining(), 0, "{name}");
    }
}

#[test]
fn the_tagged_invoice_leaves_nothing_untagged() {
    assert_eq!(invoice::builder().unwrap().tagging_diagnostics(), []);
}

#[test]
fn the_pdf_ua_report_leaves_nothing_untagged() {
    assert_eq!(pdf_ua_report::builder().unwrap().tagging_diagnostics(), []);
}

#[test]
fn every_golden_has_an_example() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/examples");
    let orphans: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| !EXAMPLES.iter().any(|(name, _)| *f == format!("{name}.pdf")))
        .collect();
    assert!(orphans.is_empty(), "goldens without an example: {orphans:?}");
}

/// The examples are listed here (to build them in process), in `Cargo.toml` (`[[example]]`, for
/// `required-features`) and found by `cargo xtask examples` in the folder: all must agree.
#[test]
fn every_example_file_is_listed_here_and_in_the_manifest() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<String> = std::fs::read_dir(root.join("examples"))
        .unwrap()
        .filter_map(|e| {
            let path = e.unwrap().path();
            (path.extension()? == "rs").then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    files.sort();
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let mut in_manifest: Vec<String> = manifest
        .split("[[example]]")
        .skip(1)
        // An entry with its own `path` (examples/differential/) is a tool, not a golden example.
        .filter(|block| !block.contains("\npath = "))
        .filter_map(|block| block.split('"').nth(1).map(str::to_owned))
        .collect();
    in_manifest.sort();
    // A golden `<example>-<variant>` comes from `<example>.rs`.
    let mut here: Vec<String> = EXAMPLES
        .iter()
        .map(|(name, _)| name.split_once('-').map_or(*name, |(stem, _)| stem).to_owned())
        .collect();
    here.sort();
    here.dedup();
    assert_eq!((&here, &in_manifest), (&files, &files));
}
