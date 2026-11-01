//! SPEC case 3: every corpus file opens and agrees with pypdf's facts in `expected.json`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use common::open;
use common::{corpus, expected, fixture};
use laminapdf_cos::{Document, ObjRef};
use serde_json::Value;

/// Runs `check` on every corpus file, collecting all mismatches into one readable failure.
fn for_each_file(check: impl Fn(&str, &Document, &Value) -> Option<String>) {
    let expected = expected();
    let mut failures = Vec::new();
    for (name, bytes) in corpus() {
        let facts = &expected[&name];
        assert!(facts.is_object(), "{name} missing from expected.json");
        match open(&bytes) {
            Ok(doc) => failures.extend(check(&name, &doc, facts).map(|m| format!("{name}: {m}"))),
            Err(e) => failures.push(format!("{name}: open failed: {e}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn mismatch<T: PartialEq + std::fmt::Debug>(what: &str, ours: &T, theirs: &T) -> Option<String> {
    (ours != theirs).then(|| format!("{what}: ours {ours:?} vs expected {theirs:?}"))
}

#[test]
fn every_expected_file_is_in_the_corpus() {
    let names: Vec<String> = corpus().into_iter().map(|(n, _)| n).collect();
    let expected = expected();
    let missing: Vec<&String> = expected
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !names.contains(k))
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn page_count_matches_expected() {
    for_each_file(|_, doc, facts| {
        mismatch(
            "page_count",
            &doc.page_count().ok(),
            &facts["page_count"].as_u64().and_then(|n| usize::try_from(n).ok()),
        )
    });
}

#[test]
fn encrypted_flag_matches_expected() {
    for_each_file(|_, doc, facts| mismatch("encrypted", &Some(doc.is_encrypted()), &facts["encrypted"].as_bool()));
}

#[test]
fn first_page_mediabox_matches_expected() {
    for_each_file(|_, doc, facts| {
        let ours = doc
            .pages()
            .ok()
            .and_then(|p| p.first().and_then(|p| p.media_box))
            .map(|b| b.to_vec());
        let theirs = facts["first_page_mediabox"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_f64).collect());
        mismatch("first_page_mediabox", &ours, &theirs)
    });
}

#[test]
fn root_keys_match_expected() {
    for_each_file(|_, doc, facts| {
        let ours = doc.catalog().ok().map(|c| {
            let mut keys: Vec<String> = c
                .iter()
                .map(|(k, _)| format!("/{}", String::from_utf8_lossy(k.as_bytes())))
                .collect();
            keys.sort();
            keys
        });
        let theirs = facts["root_keys"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect());
        mismatch("root_keys", &ours, &theirs)
    });
}

#[test]
fn revisions_match_eof_marker_count() {
    for_each_file(|_, doc, facts| {
        mismatch(
            "revisions",
            &u64::try_from(doc.revisions()).ok(),
            &facts["revisions"].as_u64(),
        )
    });
}

#[test]
fn xref_stream_presence_matches_expected() {
    for_each_file(|_, doc, facts| {
        mismatch(
            "has_xref_stream",
            &Some(doc.has_xref_stream()),
            &facts["has_xref_stream"].as_bool(),
        )
    });
}

#[test]
fn header_version_matches_expected() {
    for_each_file(|_, doc, facts| {
        mismatch(
            "header_version",
            &doc.header_version(),
            &facts["header_version"].as_str(),
        )
    });
}

#[test]
fn only_hello_broken_needs_repair() {
    for_each_file(|name, doc, _| mismatch("was_repaired", &doc.was_repaired(), &(name == "hello-broken.pdf")));
}

#[test]
fn hello_broken_reports_repair() {
    assert!(open(&fixture("corpus/hello-broken.pdf")).unwrap().was_repaired());
}

#[test]
fn hello_incremental_has_two_revisions() {
    assert_eq!(open(&fixture("corpus/hello-incremental.pdf")).unwrap().revisions(), 2);
}

#[test]
fn hello_incremental_page_shows_updated_text() {
    let bytes = fixture("corpus/hello-incremental.pdf");
    let doc = open(&bytes).unwrap();
    let content = doc.page_content(&doc.pages().unwrap()[0]).unwrap();
    assert!(String::from_utf8_lossy(&content).contains("Hello, Update"));
}

#[test]
fn hello_xrefstm_resolves_catalog_from_object_stream() {
    let bytes = fixture("corpus/hello-xrefstm.pdf");
    let doc = open(&bytes).unwrap();
    let catalog = doc.resolve(ObjRef::new(1, 0)).unwrap();
    assert_eq!(
        catalog.as_dict().and_then(|d| d.get_name("Type")),
        Some(&b"Catalog"[..])
    );
}

#[test]
fn hello_xrefstm_resolves_font_from_object_stream() {
    let bytes = fixture("corpus/hello-xrefstm.pdf");
    let doc = open(&bytes).unwrap();
    let font = doc.resolve(ObjRef::new(4, 0)).unwrap();
    assert_eq!(
        font.as_dict().and_then(|d| d.get_name("BaseFont")),
        Some(&b"Helvetica"[..])
    );
}

#[test]
fn fpdf2_page_inherits_mediabox_from_pages_node() {
    let bytes = fixture("corpus/fpdf2.pdf");
    let doc = open(&bytes).unwrap();
    let page = doc.pages().unwrap().remove(0);
    assert!(!page.dict.contains_key("MediaBox") && page.media_box == Some([0.0, 0.0, 612.0, 792.0]));
}
