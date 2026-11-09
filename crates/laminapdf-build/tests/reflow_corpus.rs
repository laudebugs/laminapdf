//! The reader-mode corpus (todo U4b) is what its generator writes: every
//! `fixtures/reflow/<name>.pdf` and `.json` equals `common::reflow::corpus()`,
//! byte for byte, and no file is left without a generator. `cargo xtask golden --update`
//! rewrites them; this only compares (ADR 0018 tier 3).
#![allow(clippy::panic, clippy::unwrap_used, reason = "test code: a panic is a test failure")]

mod common;

#[test]
fn the_corpus_matches_its_generator() {
    let dir = laminapdf_test_fixtures::fixtures_dir().join("reflow");
    let corpus = common::reflow::corpus();
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
        if !names.contains(&name) {
            stale.push(format!("{name} (no generator)"));
        }
    }
    assert!(
        stale.is_empty(),
        "fixtures/reflow differs from its generator: {stale:?}; run `cargo xtask golden --update`"
    );
}

#[test]
fn the_corpus_is_the_same_on_a_second_build() {
    let (first, second) = (common::reflow::corpus(), common::reflow::corpus());
    for (a, b) in first.iter().zip(&second) {
        // Byte comparisons without printing whole PDFs on failure.
        assert!(a.name == b.name && a.pdf == b.pdf && a.json == b.json, "{}", a.name);
    }
    assert_eq!(first.len(), second.len());
}
