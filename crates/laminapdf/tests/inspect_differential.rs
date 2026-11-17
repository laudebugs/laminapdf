//! The inspector's objects and fonts against qpdf's and pdffonts' on the committed PDFs
//! (`fixtures/differential/inspect.json`, written by `cargo xtask differential --live --inspect
//! --update` with the tools at `doctor.rs`'s pins). The nightly lane runs the same comparison
//! with the tools themselves over the external corpus too.
#![expect(clippy::unwrap_used, reason = "test code: a panic is a test failure")]

use laminapdf::{Document, OpenOptions};
use laminapdf_test_fixtures::{fixture, json};
use serde_json::Value;

#[path = "common/inspect_facts.rs"]
mod inspect_facts;

/// Each committed file opened, with the tools' facts about it.
fn files() -> Vec<(String, Document, Value)> {
    let facts = json("differential/inspect.json");
    let files = facts.get("files").and_then(Value::as_object).unwrap();
    assert!(files.len() >= 20, "the committed facts cover {} files", files.len());
    files
        .iter()
        .map(|(name, facts)| {
            let path = name.strip_prefix("fixtures/").unwrap();
            let doc = Document::open_bytes(fixture(path), OpenOptions::default()).unwrap();
            (name.clone(), doc, facts.clone())
        })
        .collect()
}

/// capability: J.09 core read
#[test]
fn every_committed_files_objects_and_places_match_qpdf() {
    for (name, doc, facts) in files() {
        assert_eq!(
            facts["outcome"], "opened",
            "{name}: qpdf repaired it, so its places are qpdf's own"
        );
        assert_eq!(inspect_facts::objects(&doc).unwrap(), facts["objects"], "{name}");
    }
}

/// capability: J.09 core read
#[test]
fn every_committed_files_newest_cross_reference_section_matches_qpdf() {
    let mut kinds = std::collections::BTreeSet::new();
    for (name, doc, facts) in files() {
        let xref = inspect_facts::xref(&doc);
        assert_eq!(xref, facts["xref"], "{name}");
        kinds.insert(xref["kind"].as_str().unwrap().to_owned());
    }
    // A table, a cross-reference stream and a hybrid file are all among them.
    assert_eq!(kinds.len(), 3, "{kinds:?}");
}

/// capability: J.08 core read
#[test]
fn every_committed_files_fonts_match_pdffonts() {
    let mut with_fonts = 0;
    for (name, doc, facts) in files() {
        let fonts = inspect_facts::fonts(&doc).unwrap();
        assert_eq!(fonts, facts["fonts"], "{name}");
        with_fonts += usize::from(fonts.as_array().is_some_and(|f| !f.is_empty()));
    }
    assert!(with_fonts >= 20, "only {with_fonts} files with fonts compared");
}
