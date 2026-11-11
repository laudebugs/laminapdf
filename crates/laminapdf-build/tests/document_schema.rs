//! `laminapdf.document/1`: the committed schema is the generated one, every fixture document
//! (filled) validates against it, documents survive a round trip, and the format's errors name
//! the value at fault without quoting the document.
//! `UPDATE_GOLDEN=1 cargo test -p laminapdf-build --features schema --test document_schema`
//! rewrites the schema.
#![cfg(feature = "schema")]
#![expect(clippy::unwrap_used, clippy::panic, reason = "test code: a panic is a test failure")]

use laminapdf_build::flow::document::json_schema;
use laminapdf_build::flow::{FlowDocument, fill};
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
fn a_document_survives_a_round_trip_through_json() {
    let document = invoice();
    assert_eq!(FlowDocument::from_json(&document.to_json()).unwrap(), document);
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
        .document(&document, &laminapdf_build::flow::Resources::new())
        .unwrap_err();
    assert!(
        matches!(&error, Error::InvalidDocument { pointer, reason: "unknown-style", did_you_mean } if pointer == "/blocks/1/style" && did_you_mean == &["body"]),
        "{error:?}"
    );
    let figure = FlowDocument::from_json(&with_block(
        &json!({ "type": "figure", "image": "logo", "alt": "Logo" }),
    ))
    .unwrap();
    let error = b
        .document(&figure, &laminapdf_build::flow::Resources::new())
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
