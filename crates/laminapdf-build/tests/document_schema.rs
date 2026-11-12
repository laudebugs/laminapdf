//! `laminapdf.document/1`: the committed schema is the generated one, every fixture document
//! (filled) validates against it, documents survive a round trip, and the format's errors name
//! the value at fault without quoting the document.
//! `UPDATE_GOLDEN=1 cargo test -p laminapdf-build --features schema --test document_schema`
//! rewrites the schema.
#![cfg(feature = "schema")]
#![expect(clippy::unwrap_used, clippy::panic, reason = "test code: a panic is a test failure")]

use laminapdf_build::flow::document::json_schema;
use laminapdf_build::flow::document::{FlowDocument, fill};
use laminapdf_build::{BuildOptions, Builder, Error};
use laminapdf_test_fixtures::{fixture, fixtures_dir};
use serde_json::{Value, json};

const SCHEMA: &str = "schemas/document.schema.json";

fn read(path: &str) -> Value {
    serde_json::from_slice(&fixture(path)).unwrap()
}

/// The invoice template filled from its data.
fn invoice() -> FlowDocument {
    fill(&read("documents/invoice.json"), &read("documents/invoice-data.json")).unwrap()
}

#[test]
fn the_committed_schema_is_the_generated_one() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SCHEMA);
    let generated = serde_json::to_string_pretty(&json_schema()).unwrap() + "\n";
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &generated).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(committed == generated, "{SCHEMA} is stale: run with UPDATE_GOLDEN=1");
}

#[test]
fn every_filled_fixture_document_validates_against_the_schema() {
    let mut compiler = boon::Compiler::new();
    let schema = json_schema();
    let id = schema["$id"].as_str().unwrap().to_owned();
    compiler.add_resource(&id, schema).unwrap();
    let mut schemas = boon::Schemas::new();
    let index = compiler.compile(&id, &mut schemas).unwrap();
    let documents = std::fs::read_dir(fixtures_dir().join("documents")).unwrap();
    let mut checked = 0;
    for entry in documents {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".json").filter(|s| !s.ends_with("-data")) else {
            continue;
        };
        let data = fixtures_dir().join(format!("documents/{stem}-data.json"));
        let data = std::fs::read(&data).map_or(json!({}), |b| serde_json::from_slice(&b).unwrap());
        let filled = fill(&read(&format!("documents/{name}")), &data).unwrap().to_json();
        if let Err(e) = schemas.validate(&filled, index) {
            panic!("{name} doesn't match {SCHEMA}:\n{e}");
        }
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn every_block_kind_survives_a_round_trip_through_json() {
    for document in [
        invoice(),
        FlowDocument::from_json(&read("documents/every-block.json")).unwrap(),
    ] {
        assert_eq!(FlowDocument::from_json(&document.to_json()).unwrap(), document);
    }
}

#[test]
fn the_names_a_document_uses_are_its_styles_fonts_and_its_figures_images() {
    let document = FlowDocument::from_json(&read("documents/every-block.json")).unwrap();
    assert_eq!(
        document.font_names().into_iter().collect::<Vec<_>>(),
        ["Courier", "Helvetica", "Helvetica-Bold"]
    );
    assert_eq!(document.image_names().into_iter().collect::<Vec<_>>(), ["square"]);
    let quoted = FlowDocument::from_json(&json!({
        "schema": "laminapdf.document/1",
        "styles": { "body": { "font": "regular", "size": 10, "fallbacks": ["arabic"] } },
        "blocks": [{ "type": "quote", "blocks": [
            { "type": "keep", "blocks": [{ "type": "figure", "image": "logo", "alt": "Logo" }] }
        ] }]
    }))
    .unwrap();
    assert_eq!(
        quoted.font_names().into_iter().collect::<Vec<_>>(),
        ["arabic", "regular"]
    );
    assert_eq!(quoted.image_names().into_iter().collect::<Vec<_>>(), ["logo"]);
}

#[test]
fn every_block_kind_lays_out() {
    let document = FlowDocument::from_json(&read("documents/every-block.json")).unwrap();
    let square = laminapdf_build::Image::rgba(2, 2, &[128; 16]).unwrap();
    let resources = laminapdf_build::flow::document::Resources::new().with_image("square", square);
    let mut b = Builder::new(BuildOptions::new());
    let report = b.document(&document, &resources).unwrap();
    assert!(report.pages.len() >= 3, "{} pages", report.pages.len());
    b.finish().unwrap();
}

#[test]
fn a_fault_inside_a_block_is_pointed_at_where_it_is() {
    let cell = json!({ "type": "table", "columns": ["auto"], "rows": [{ "cells": [{ "spn": 2 }] }] });
    assert_eq!(invalid(&with_block(&cell)).0, "/blocks/1/rows/0/cells/0/spn");
    let quoted = json!({ "type": "quote", "blocks": [{ "type": "paragraph", "txt": "x" }] });
    let (pointer, reason, did_you_mean) = invalid(&with_block(&quoted));
    assert_eq!(
        (pointer.as_str(), reason, did_you_mean),
        ("/blocks/1/blocks/0/txt", "unknown-key", vec!["text".to_owned()])
    );
    let span = json!({ "type": "paragraph", "spans": [{ "text": "a" }, { "code": "yes" }] });
    assert_eq!(invalid(&with_block(&span)).0, "/blocks/1/spans/1/code");
    let (pointer, _, did_you_mean) = invalid(&with_block(&json!({ "type": "paragraf" })));
    assert_eq!(
        (pointer.as_str(), did_you_mean),
        ("/blocks/1/type", vec!["paragraph".to_owned()])
    );
}

#[test]
fn headings_need_a_level_and_sizes_must_be_usable() {
    let (pointer, reason, _) = invalid(&with_block(&json!({ "type": "heading", "text": "x" })));
    assert_eq!((pointer.as_str(), reason), ("/blocks/1/level", "missing-key"));
    let (pointer, reason, _) = invalid(&with_block(&json!({ "type": "heading", "level": 7, "text": "x" })));
    assert_eq!((pointer.as_str(), reason), ("/blocks/1/level", "out-of-range"));
    let (pointer, _, _) = invalid(&with_block(&json!({ "type": "paragraph", "level": 1, "text": "x" })));
    assert_eq!(pointer, "/blocks/1/level");
    let mut zero = with_block(&json!({ "type": "rule" }));
    zero["styles"]["body"]["size"] = json!(0);
    assert_eq!(
        invalid(&zero),
        ("/styles/body/size".to_owned(), "out-of-range", Vec::new())
    );
    let span = json!({ "type": "table", "columns": ["auto"], "rows": [{ "cells": [{ "span": 0 }] }] });
    assert_eq!(invalid(&with_block(&span)).1, "out-of-range");
}

#[test]
fn a_key_s_own_text_never_reaches_did_you_mean() {
    // A key that holds backticks and serde's own words can't pass text into the suggestions.
    let key = "spaceA`, expected `INJECTED` or `spaceAfter";
    let (pointer, reason, did_you_mean) = invalid(&with_block(&json!({ "type": "paragraph", key: 1 })));
    assert_eq!(reason, "unknown-key");
    assert!(did_you_mean.iter().all(|k| !k.contains("INJECTED")), "{did_you_mean:?}");
    assert!(pointer.starts_with("/blocks/1/spaceA"), "{pointer}");
}

fn invalid(json: &Value) -> (String, &'static str, Vec<String>) {
    match FlowDocument::from_json(json).unwrap_err() {
        Error::InvalidDocument {
            pointer,
            reason,
            did_you_mean,
        } => (pointer, reason, did_you_mean),
        other => panic!("{other:?}"),
    }
}

fn with_block(block: &Value) -> Value {
    json!({
        "schema": "laminapdf.document/1",
        "styles": { "body": { "font": "Helvetica", "size": 10 } },
        "blocks": [{ "type": "rule" }, block]
    })
}

#[test]
fn a_misspelt_key_names_its_place_and_the_closest_known_key() {
    let (pointer, reason, did_you_mean) = invalid(&with_block(&json!({ "type": "paragraph", "spaceAftr": 6 })));
    assert_eq!((pointer.as_str(), reason), ("/blocks/1/spaceAftr", "unknown-key"));
    assert_eq!(did_you_mean, ["spaceAfter"]);
}

#[test]
fn a_missing_key_a_wrong_kind_and_another_schema_are_refused() {
    let (pointer, reason, _) = invalid(&with_block(&json!({ "type": "spacer" })));
    assert_eq!((pointer.as_str(), reason), ("/blocks/1/height", "missing-key"));
    let (_, reason, _) = invalid(&with_block(&json!({ "type": "spacer", "height": "tall" })));
    assert_eq!(reason, "wrong-type");
    let (pointer, reason, _) = invalid(&json!({ "schema": "laminapdf.document/2", "styles": {}, "blocks": [] }));
    assert_eq!((pointer.as_str(), reason), ("/schema", "unsupported-schema"));
}

#[test]
fn a_style_font_or_image_the_document_lacks_is_refused_where_it_is_named() {
    let document = FlowDocument::from_json(&with_block(
        &json!({ "type": "paragraph", "text": "x", "style": "bdy" }),
    ))
    .unwrap();
    let mut b = Builder::new(BuildOptions::new());
    let error = b
        .document(&document, &laminapdf_build::flow::document::Resources::new())
        .unwrap_err();
    assert!(
        matches!(&error, Error::InvalidDocument { pointer, reason: "unknown-style", did_you_mean } if pointer == "/blocks/1/style" && did_you_mean.is_empty()),
        "{error:?}"
    );
    let figure = FlowDocument::from_json(&with_block(
        &json!({ "type": "figure", "image": "logo", "alt": "Logo" }),
    ))
    .unwrap();
    let error = b
        .document(&figure, &laminapdf_build::flow::document::Resources::new())
        .unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidDocument {
            reason: "unknown-image",
            ..
        }
    ));
}

#[test]
fn no_message_carries_the_document_s_or_the_data_s_text() {
    const MARKER: &str = "MARKER-7f3a";
    let template = with_block(&json!({ "type": "paragraph", "text": { "$data": format!("/{MARKER}") } }));
    let missing = fill(&template, &json!({ "other": MARKER })).unwrap_err();
    assert!(!missing.to_string().contains(MARKER));
    let key = invalid(&with_block(&json!({ "type": "paragraph", MARKER: 1 })));
    let error = FlowDocument::from_json(&with_block(&json!({ "type": "paragraph", MARKER: 1 }))).unwrap_err();
    assert!(!error.to_string().contains(MARKER), "{error}");
    assert!(key.0.contains(MARKER), "the pointer, in data, names the key");
}

#[test]
fn a_tab_in_a_code_block_should_raise_no_tounicode_conflict_in_a_cff_font() {
    let document = FlowDocument::from_json(&json!({
        "schema": "laminapdf.document/1",
        "styles": { "body": { "font": "noto", "size": 10 } },
        "blocks": [
            { "type": "codeBlock", "text": "name\tqty\n\tindented\nab\t1" },
            { "type": "paragraph", "text": "a\tb" }
        ]
    }))
    .unwrap();
    let mut b = Builder::new(BuildOptions::new());
    let noto = b
        .font(laminapdf_text::Font::from_bytes(fixture("fonts/NotoSansSC-Regular.otf")).unwrap())
        .unwrap();
    let resources = laminapdf_build::flow::document::Resources::new().with_font("noto", &noto);
    b.document(&document, &resources).unwrap();
    let (_, warnings) = b.finish_with_warnings().unwrap();
    assert_eq!(warnings, []);
}
