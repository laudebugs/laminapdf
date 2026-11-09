//! Reader mode through the façade (`Document::reflow`) and the JSON the bindings send: the
//! `mixed` file of `fixtures/reflow/` with each block's kind as a `kind` key, its box as
//! `[x, y, width, height]` in page space, a crop's `alt`, and the checks' measures.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
#![cfg(feature = "extract")]
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
    assert_eq!(got["available"], true);
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
fn an_untagged_file_says_why_with_no_blocks() {
    let bytes = std::fs::read(laminapdf_test_fixtures::fixtures_dir().join("golden/examples/hello.pdf")).unwrap();
    let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    let got = serde_json::to_value(doc.reflow(&CancelToken::new()).unwrap()).unwrap();
    assert_eq!(
        (&got["available"], &got["reason"], &got["blocks"]),
        (&json!(false), &json!("untagged"), &json!([]))
    );
}
