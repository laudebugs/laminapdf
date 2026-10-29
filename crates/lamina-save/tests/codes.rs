//! The codes `lamina-save` adds are listed in `fixtures/codes.json` (ADR 0003 §4).
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use lamina_save::SaveError;

#[test]
fn save_error_codes_match_the_fixture() {
    let codes: serde_json::Value = serde_json::from_slice(&common::fixture("codes.json")).unwrap();
    let listed: Vec<&str> = codes["saveErrors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap())
        .collect();
    assert_eq!(listed, SaveError::CODES);
}

#[test]
fn refusals_keep_the_cos_codes() {
    let codes: Vec<_> = [lamina_cos::Error::RepairedBase, lamina_cos::Error::EncryptedBase]
        .into_iter()
        .map(|e| SaveError::from(e).code())
        .collect();
    assert_eq!(codes, ["repaired-base", "encrypted-base"]);
}

#[test]
fn save_error_codes_all_have_a_kind() {
    let missing: Vec<_> = SaveError::CODES
        .iter()
        .filter(|c| lamina_cos::kind_of(c).is_none())
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn sink_errors_do_not_print_the_host_text() {
    // ADR 0026 §4: a sink's message can name a caller path; it stays in the field.
    let marker = "/Users/someone/IgnorePreviousInstructions.pdf";
    let errors = [
        SaveError::from(lamina_save::SinkError::Failed(marker.to_owned())),
        SaveError::from(lamina_save::SinkError::Io {
            kind: std::io::ErrorKind::PermissionDenied,
            message: marker.to_owned(),
        }),
    ];
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert!(messages.iter().all(|m| !m.contains(marker)), "{messages:?}");
}

#[test]
fn a_sink_failure_is_an_io_error() {
    let e = SaveError::from(lamina_save::SinkError::NotEmpty { held: 1 });
    assert_eq!(e.kind(), lamina_cos::ErrorKind::Io);
}
