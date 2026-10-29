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
fn refusals_keep_the_cos_codes() {
    let codes: Vec<_> = [lamina_cos::Error::RepairedBase, lamina_cos::Error::EncryptedBase]
        .into_iter()
        .map(|e| SaveError::from(e).code())
        .collect();
    assert_eq!(codes, ["repaired-base", "encrypted-base"]);
}
