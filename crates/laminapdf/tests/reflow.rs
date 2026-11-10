//! Reader mode through the façade (`Document::reflow`) and the JSON the bindings send: the
//! `mixed` file of `fixtures/reflow/` with each block's kind as a `kind` key, its box as
//! `[x, y, width, height]` in page space, a crop's `alt`, and the checks' measures; an untagged
//! file's JSON names its approximate source and why, and a scan's page image says why.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
#![cfg(feature = "reflow")]

use laminapdf::{CancelToken, Document, OpenOptions};
use serde_json::{Value, json};

fn mixed() -> Value {
    let bytes = std::fs::read(laminapdf_test_fixtures::fixtures_dir().join("reflow/mixed.pdf")).unwrap();
    let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    serde_json::to_value(doc.reflow(&CancelToken::new()).unwrap()).unwrap()
}

#[test]
fn the_json_names_each_blocks_kind_and_a_crops_alt_and_box() {
    let got = mixed();
    assert_eq!((&got["available"], &got["source"]), (&json!(true), &json!("tags")));
    assert!(got.get("reason").is_none());
    let blocks = got["blocks"].as_array().unwrap();
    let kinds: Vec<&str> = blocks.iter().map(|b| b["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        [
            "heading",
            "paragraph",
            "figure",
            "caption",
            "formula",
            "quote",
            "list-item",
            "list-item",
            "table"
        ]
    );
    assert_eq!(blocks[0]["level"], 1);
    assert_eq!(blocks[2]["alt"], "Four coloured squares in a grid");
    assert_eq!(blocks[2]["bbox"], json!([56.0, 180.0, 160.0, 110.0]));
    assert_eq!(blocks[2]["text"], "");
    assert!(blocks[6]["marker"].is_string() && blocks[6]["depth"] == 0);
    assert_eq!(got["checks"]["agreement"], 1.0);
}

#[test]
fn an_untagged_file_reads_approximately_and_says_why() {
    let bytes = std::fs::read(laminapdf_test_fixtures::fixtures_dir().join("golden/examples/hello.pdf")).unwrap();
    let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    let got = serde_json::to_value(doc.reflow(&CancelToken::new()).unwrap()).unwrap();
    assert_eq!(
        (&got["available"], &got["source"], &got["reason"]),
        (&json!(true), &json!("approximate"), &json!("untagged"))
    );
    assert_eq!(got["blocks"][0]["text"], "Hello, world!");
    assert_eq!(got["checks"]["pageImages"], 0);
}

#[test]
fn a_page_image_names_why_in_the_json() {
    let mut builder = laminapdf::Builder::new(laminapdf::BuildOptions::default());
    let mut page = builder.page(laminapdf::PageSize::A4);
    let gray = laminapdf::Image::rgba(2, 2, &[128; 16]).unwrap();
    page.surface()
        .image(&gray, laminapdf::RectPt::from_points(0.0, 0.0, 595.0, 842.0));
    page.finish().unwrap();
    let mut doc = builder.finish().unwrap();
    let saved = doc.save(&laminapdf::SaveOptions::new()).unwrap();
    let doc = Document::open_bytes(saved.bytes, OpenOptions::default()).unwrap();
    let got = serde_json::to_value(doc.reflow(&CancelToken::new()).unwrap()).unwrap();
    assert_eq!(got["blocks"][0]["kind"], "page-image");
    assert_eq!(got["blocks"][0]["why"], "scanned");
}
