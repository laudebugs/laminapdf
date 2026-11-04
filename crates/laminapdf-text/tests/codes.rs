//! The codes `laminapdf-text` adds are listed in `fixtures/codes.json` (v2, ADR 0003 §4 and
//! ADR 0026). Their kinds are checked per variant by the unit tests in `src/error.rs`, their
//! entries and pages by `crates/laminapdf-cos/tests/codes.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure (serde_json `Value` indexing returns `Null` when absent)"
)]

mod common;

use laminapdf_text::{Error, Warning};

fn listed(key: &str) -> Vec<String> {
    let codes: serde_json::Value = serde_json::from_slice(&laminapdf_test_fixtures::fixture("codes.json")).unwrap();
    codes[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn text_error_codes_match_the_fixture() {
    assert_eq!(listed("textErrors"), Error::CODES);
}

#[test]
fn text_warning_codes_match_the_fixture() {
    assert_eq!(listed("textWarnings"), Warning::CODES);
}

#[test]
fn cos_errors_keep_their_codes() {
    assert_eq!(Error::from(laminapdf_cos::Error::RepairedBase).code(), "repaired-base");
}

#[test]
fn messages_carry_no_font_names_or_text() {
    // ADR 0026 §4: document-derived strings go in fields, never in Display.
    let marker = "IgnorePreviousInstructions";
    let errors = [
        Error::UnknownAxis {
            font: marker.to_owned(),
            tag: laminapdf_text::OpenTypeTag::WEIGHT,
        },
        Error::FontNotEmbeddable {
            font: marker.to_owned(),
            fs_type: laminapdf_text::FsType::new(2),
        },
        Error::MissingGlyph {
            ch: 'x',
            font: marker.to_owned(),
        },
        Error::GlyphOutOfRange {
            gid: 9,
            count: 3,
            font: marker.to_owned(),
        },
        Error::Subset {
            font: marker.to_owned(),
            reason: "malformed font",
        },
        Error::InvalidFeature {
            feature: marker.to_owned(),
        },
        Error::FontMismatch {
            font: marker.to_owned(),
            shaped_with: marker.to_owned(),
        },
    ];
    let warning = Warning::ToUnicodeConflict {
        font: marker.to_owned(),
        gid: 1,
        kept: marker.to_owned(),
        dropped: marker.to_owned(),
    };
    let messages: Vec<String> = errors
        .iter()
        .map(ToString::to_string)
        .chain([warning.to_string()])
        .collect();
    assert!(
        messages.iter().all(|m| !m.contains(marker) && !m.contains("wght")),
        "{messages:?}"
    );
}
