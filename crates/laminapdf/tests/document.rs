//! `laminapdf.document/1` through the façade: a misspelt key's diagnostic carries the value's
//! JSON Pointer in its data and the closest known key in `didYouMean`; a template's missing value
//! carries its pointer.
#![cfg(feature = "document")]

use laminapdf::flow::document::{FlowDocument, fill};
use serde_json::json;

#[test]
fn a_misspelt_key_s_diagnostic_names_its_place_and_the_closest_key() {
    let document = json!({
        "schema": "laminapdf.document/1",
        "styles": { "body": { "font": "Helvetica", "size": 10 } },
        "blocks": [{ "type": "paragraph", "text": "x", "keepWithNxt": true }]
    });
    let error = laminapdf::Error::from(FlowDocument::from_json(&document).unwrap_err());
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code, "invalid-document");
    assert_eq!(diagnostic.data["pointer"], "/blocks/0/keepWithNxt");
    assert_eq!(diagnostic.did_you_mean, ["keepWithNext"]);
}

#[test]
fn a_template_s_missing_value_names_the_pointer() {
    let template = json!({
        "schema": "laminapdf.document/1",
        "styles": { "body": { "font": "Helvetica", "size": 10 } },
        "blocks": [{ "type": "paragraph", "text": { "$data": "/total" } }]
    });
    let error = laminapdf::Error::from(fill(&template, &json!({})).unwrap_err());
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code, "template-data-missing");
    assert_eq!(
        (
            diagnostic.data["pointer"].as_str(),
            diagnostic.data["placeholder"].as_str()
        ),
        (Some("/total"), Some("$data"))
    );
}

/// Text from the document and the data stays in `data`: no message, hint or `didYouMean` holds
/// a marker placed in a key the document gets wrong, a style it names, or a value a template
/// reads.
#[test]
fn no_diagnostic_text_carries_the_document_s_or_the_data_s_words() {
    const MARKER: &str = "MARKER7f3a";
    let base = |blocks: serde_json::Value| json!({ "schema": "laminapdf.document/1", "styles": { "body": { "font": "Helvetica", "size": 10 } }, "blocks": blocks });
    let errors: Vec<laminapdf::Error> = vec![
        FlowDocument::from_json(&base(json!([{ "type": "paragraph", MARKER: 1 }])))
            .unwrap_err()
            .into(),
        fill(
            &base(json!([{ "type": "paragraph", "text": { "$data": "/v" } }])),
            &json!({ "v": { MARKER: MARKER } }),
        )
        .unwrap_err()
        .into(),
        fill(
            &base(json!([{ "type": "paragraph", "text": { "$data": format!("/{MARKER}") } }])),
            &json!({}),
        )
        .unwrap_err()
        .into(),
        {
            let doc =
                FlowDocument::from_json(&base(json!([{ "type": "paragraph", "text": "x", "style": MARKER }]))).unwrap();
            let mut b = laminapdf::Builder::new(laminapdf::BuildOptions::new());
            b.document(&doc, &laminapdf::flow::document::Resources::new())
                .unwrap_err()
        },
    ];
    for error in errors {
        let d = error.diagnostic();
        let shown = format!("{} {:?} {:?}", d.message, d.hint, d.did_you_mean);
        assert!(!shown.contains(MARKER), "{} leaks: {shown}", d.code);
    }
}
