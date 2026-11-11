//! `laminapdf.document/1` through the façade: a misspelt key's diagnostic carries the value's
//! JSON Pointer in its data and the closest known key in `didYouMean`; a template's missing value
//! carries its pointer.
#![cfg(feature = "document")]
#![expect(clippy::unwrap_used, reason = "test code: a panic is a test failure")]

use laminapdf::flow::{FlowDocument, fill};
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
