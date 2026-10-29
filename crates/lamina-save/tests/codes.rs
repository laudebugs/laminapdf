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
        .map(|c| c.as_str().unwrap())
        .collect();
    assert_eq!(listed, SaveError::CODES);
}

#[test]
fn the_cos_codes_it_reuses_are_cos_codes() {
    for code in [SaveError::RepairedBase.code(), SaveError::EncryptedBase.code()] {
        assert!(lamina_cos::Error::CODES.contains(&code), "{code}");
    }
}
