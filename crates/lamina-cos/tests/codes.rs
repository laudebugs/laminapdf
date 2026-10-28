//! ADR 0003 §4: warning, error and limit codes are listed once in `fixtures/codes.json`, and
//! the Rust API exposes exactly those (a TS test will check the same file from B6).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use lamina_cos::{Error, Limit, Warning};

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
fn warning_codes_match_the_fixture() {
    assert_eq!(listed("warnings"), Warning::CODES);
}

#[test]
fn error_codes_match_the_fixture() {
    assert_eq!(listed("errors"), Error::CODES);
}

#[test]
fn syntax_error_codes_are_all_error_codes() {
    assert!(lamina_syntax::Error::CODES.iter().all(|c| Error::CODES.contains(c)));
}

#[test]
fn limit_names_match_the_fixture() {
    let names: Vec<_> = Limit::ALL.iter().map(|l| l.name()).collect();
    assert_eq!(listed("limits"), names);
}
