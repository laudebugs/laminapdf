//! The codes `lamina-text` adds are listed in `fixtures/codes.json` (ADR 0003 §4).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure (serde_json `Value` indexing returns `Null` when absent)"
)]

mod common;

use lamina_text::{Error, Warning};

fn listed(key: &str) -> Vec<String> {
    let codes: serde_json::Value = serde_json::from_slice(&common::fixture("codes.json")).unwrap();
    codes[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_owned())
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
    assert_eq!(Error::from(lamina_cos::Error::RepairedBase).code(), "repaired-base");
}
