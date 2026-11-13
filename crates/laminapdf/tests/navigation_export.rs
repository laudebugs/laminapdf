//! Links and outline as files: `laminapdf.links/1` and `laminapdf.outline/1` JSON, valid
//! against their committed schemas, and CSV; for `fixtures/links/links.pdf` (every link kind)
//! and the links-and-outlines golden, against goldens in `fixtures/navigation/`, and the links
//! against pypdf's facts.
//!
//! `UPDATE_GOLDEN=1 cargo test -p laminapdf --all-features --test navigation_export` rewrites
//! the schemas and the goldens.

#![cfg(feature = "schema")]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use std::path::PathBuf;

use laminapdf::{Document, OpenOptions, navigation_schemas};
use laminapdf_test_fixtures::{fixture, fixtures_dir, json};

fn update() -> bool {
    std::env::var_os("UPDATE_GOLDEN").is_some()
}

fn open(path: &str) -> Document {
    Document::open_bytes(fixture(path), OpenOptions::default()).unwrap()
}

/// `actual` against the golden `name` in `fixtures/navigation/`, which `UPDATE_GOLDEN` writes.
/// The generator's version reads `x.y.z` there, so a release changes no golden.
fn golden(name: &str, actual: &str) {
    let actual = actual.replace(
        &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
        "\"version\": \"x.y.z\"",
    );
    let path = fixtures_dir().join("navigation").join(name);
    if update() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        actual, expected,
        "fixtures/navigation/{name} is stale: run with UPDATE_GOLDEN=1"
    );
}

#[test]
fn the_committed_schemas_match_the_rust_types() {
    for (file, schema) in navigation_schemas() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas").join(file);
        let generated = serde_json::to_string_pretty(&schema).unwrap() + "\n";
        if update() {
            std::fs::write(&path, &generated).unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            generated,
            "schemas/{file} is stale: run with UPDATE_GOLDEN=1"
        );
    }
}

/// `value` against the committed schema `file`.
fn validate(file: &str, value: &serde_json::Value) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas").join(file);
    let schema: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("urn:schema", schema).unwrap();
    let index = compiler.compile("urn:schema", &mut schemas).unwrap();
    if let Err(e) = schemas.validate(value, index) {
        panic!("{file}: {e}");
    }
}

/// capability: H.08 core read
#[test]
fn every_link_kind_goes_out_as_json_and_csv() {
    let doc = open("links/links.pdf");
    let links = doc.links_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&links).unwrap();
    validate("links.schema.json", &value);
    assert_eq!(value["schema"], "laminapdf.links/1");
    golden("links.json", &links);
    golden("links.csv", &doc.links_csv().unwrap());
    // As many links on each page as pypdf finds visible.
    let facts = json("links/expected.json");
    for (page, theirs) in facts["files"]["links.pdf"].as_array().unwrap().iter().enumerate() {
        // pypdf lists every link annotation; a reader leaves out hidden ones and those without
        // a rectangle.
        let visible = theirs
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| l["hidden"] == false && !l["rect"].is_null())
            .count();
        let ours = value["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| l["pageIndex"] == page)
            .count();
        assert_eq!(ours, visible, "page {page}");
    }
}

/// capability: H.08 core read
#[test]
fn the_outline_goes_out_as_json_and_csv() {
    let doc = open("golden/links-and-outlines.pdf");
    let outline = doc.outline_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&outline).unwrap();
    validate("outline.schema.json", &value);
    assert_eq!(value["schema"], "laminapdf.outline/1");
    assert_eq!(value["items"].as_array().unwrap().len(), 4);
    golden("outline.json", &outline);
    let csv = doc.outline_csv().unwrap();
    assert!(csv.starts_with("level,title,targetPage,"));
    // A row an item, children after their parent.
    let items = |items: &serde_json::Value| -> usize {
        fn count(items: &serde_json::Value) -> usize {
            items
                .as_array()
                .unwrap()
                .iter()
                .map(|i| 1 + count(&i["children"]))
                .sum()
        }
        count(items)
    };
    assert_eq!(csv.lines().count(), 1 + items(&value["items"]));
    golden("outline.csv", &csv);
}

#[test]
fn a_document_without_links_or_an_outline_gives_empty_files() {
    let doc = open("golden/text-std14.pdf");
    let links: serde_json::Value = serde_json::from_str(&doc.links_json().unwrap()).unwrap();
    assert_eq!(links["links"], serde_json::json!([]));
    assert_eq!(doc.outline_csv().unwrap().lines().count(), 1);
}
