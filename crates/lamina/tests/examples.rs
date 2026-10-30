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
#[path = "../examples/vectors.rs"]
mod vectors;

use std::path::Path;

type Build = fn() -> Result<Vec<u8>, Box<dyn std::error::Error>>;

/// Every example, by the name of its golden.
const EXAMPLES: [(&str, Build); 6] = [
    ("hello", hello::build),
    ("invoice", invoice::build),
    ("multilingual", multilingual::build),
    ("vectors", vectors::build),
    ("links_and_outlines", links_and_outlines::build),
    ("generate_then_edit", generate_then_edit::build),
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
fn every_golden_has_an_example() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/examples");
    let orphans: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| !EXAMPLES.iter().any(|(name, _)| *f == format!("{name}.pdf")))
        .collect();
    assert!(orphans.is_empty(), "goldens without an example: {orphans:?}");
}
