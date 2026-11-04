//! The codes `laminapdf-build` adds are listed in `fixtures/codes.json` under `buildErrors` (v2,
//! ADR 0026). Their kinds are checked per variant by the unit tests in `src/error.rs`, their
//! entries and pages by `crates/laminapdf-cos/tests/codes.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure (serde_json `Value` indexing returns `Null` when absent)"
)]

mod common;

use laminapdf_build::Error;

#[test]
fn build_error_codes_match_the_fixture() {
    let codes: serde_json::Value = serde_json::from_slice(&laminapdf_test_fixtures::fixture("codes.json")).unwrap();
    let listed: Vec<&str> = codes["buildErrors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["code"].as_str().unwrap())
        .collect();
    assert_eq!(listed, Error::CODES);
}

#[test]
fn cos_errors_keep_their_codes() {
    assert_eq!(Error::from(laminapdf_cos::Error::RepairedBase).code(), "repaired-base");
}
