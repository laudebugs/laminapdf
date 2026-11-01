//! What `fixtures/codes.json` says about each code, for the tests that check every crate's
//! `kind()` against it (ADR 0026 §2). Shared by the unit tests of `laminapdf-syntax`,
//! `laminapdf-cos`, `laminapdf-ops` and `laminapdf-save` through `#[path]`, and by `tests/codes.rs`.
#![allow(dead_code, unreachable_pub, reason = "each user takes a different subset")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helper: a panic here is a test failure"
)]

use std::path::Path;

#[path = "code_groups.rs"]
pub mod code_groups;

/// Top-level keys of `codes.json` that are not groups of codes.
pub use code_groups::NOT_CODE_GROUPS;

/// The parsed `fixtures/codes.json` (crates sit two levels below the repository root).
pub fn codes_json() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codes.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap()
}

/// The kind `codes.json` gives `code`, in whichever group it is listed.
pub fn listed_kind(codes: &serde_json::Value, code: &str) -> Option<String> {
    codes
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, _)| !NOT_CODE_GROUPS.contains(&key.as_str()))
        .filter_map(|(_, group)| group.as_array())
        .flatten()
        .find(|entry| entry["code"].as_str() == Some(code))
        .and_then(|entry| entry["kind"].as_str().map(str::to_owned))
}

/// Fails, naming each one, for every `(code, kind)` whose kind is not the one `codes.json`
/// lists.
pub fn assert_kinds_match(pairs: impl IntoIterator<Item = (&'static str, &'static str)>) {
    let codes = codes_json();
    let wrong: Vec<String> = pairs
        .into_iter()
        .filter_map(|(code, kind)| {
            let listed = listed_kind(&codes, code);
            (listed.as_deref() != Some(kind)).then(|| format!("{code}: kind() is {kind}, codes.json says {listed:?}"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "kind() disagrees with fixtures/codes.json:\n  {}",
        wrong.join("\n  ")
    );
}

/// The distinct codes of `codes`, in first-seen order (to compare with a `CODES` list).
pub fn distinct(codes: impl IntoIterator<Item = &'static str>) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for code in codes {
        if !out.contains(&code) {
            out.push(code);
        }
    }
    out
}
