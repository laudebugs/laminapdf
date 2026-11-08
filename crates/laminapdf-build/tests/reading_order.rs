//! The reading-order corpus (todo U2b) is what its generator writes: every
//! `fixtures/reading-order/<name>.pdf` and `.json` equals `common::reading_order::corpus()`,
//! byte for byte, and no file is left without a generator. `cargo xtask golden --update`
//! rewrites them; this only compares (ADR 0018 tier 3).
#![allow(clippy::panic, clippy::unwrap_used, reason = "test code: a panic is a test failure")]

mod common;

#[test]
fn the_corpus_matches_its_generator() {
    let dir = laminapdf_test_fixtures::fixtures_dir().join("reading-order");
    let corpus = common::reading_order::corpus();
    let mut stale = Vec::new();
    let mut names = Vec::new();
    for sample in &corpus {
        for (ext, bytes) in [("pdf", sample.pdf.as_slice()), ("json", sample.json.as_bytes())] {
            let name = format!("{}.{ext}", sample.name);
            if std::fs::read(dir.join(&name)).unwrap_or_default() != bytes {
                stale.push(name.clone());
            }
            names.push(name);
        }
    }
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let name = entry
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .unwrap_or_default();
        if name != "threshold.json" && !names.contains(&name) {
            stale.push(format!("{name} (no generator)"));
        }
    }
    assert!(
        stale.is_empty(),
        "fixtures/reading-order differs from its generator: {stale:?}; run `cargo xtask golden --update`"
    );
}

#[test]
fn the_corpus_is_the_same_on_a_second_build() {
    let (first, second) = (common::reading_order::corpus(), common::reading_order::corpus());
    for (a, b) in first.iter().zip(&second) {
        // Byte comparisons without printing whole PDFs on failure.
        assert!(a.name == b.name && a.pdf == b.pdf && a.json == b.json, "{}", a.name);
    }
    assert_eq!(first.len(), second.len());
}

#[test]
fn every_corpus_file_opens_and_interprets_without_warnings() {
    for sample in common::reading_order::corpus() {
        let doc = common::reopen(&sample.pdf);
        let pages = doc.pages().unwrap().len();
        let content: Vec<&str> = (0..pages).flat_map(|i| common::record(&doc, i).warnings).collect();
        let codes: Vec<&str> = doc.warnings().iter().map(laminapdf_cos::Warning::code).collect();
        assert_eq!((codes, content), (vec![], vec![]), "{}", sample.name);
    }
}

#[test]
fn the_corpus_covers_the_layouts_the_todo_asks_for() {
    let names: Vec<&str> = common::reading_order::layouts().iter().map(|l| l.name).collect();
    for wanted in [
        "one-column",
        "two-columns",
        "three-columns",
        "paper",
        "sidebar",
        "table",
        "header-footer",
        "rotated",
        "arabic",
        "hebrew-two-columns",
        "chinese",
        "pull-quote",
        "narrow-gutter",
        "wrapped-grid",
    ] {
        assert!(names.contains(&wanted), "missing layout {wanted}");
    }
}
