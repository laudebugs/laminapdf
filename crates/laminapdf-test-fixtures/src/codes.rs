//! What `fixtures/codes.json` says about each code, for the tests that check every crate's
//! `kind()` against it (ADR 0026 §2), and for `laminapdf-cos`'s `tests/codes.rs`.

pub use crate::code_groups::NOT_CODE_GROUPS;

/// The parsed `fixtures/codes.json`.
///
/// # Panics
/// When the file can't be read or isn't JSON.
#[must_use]
pub fn codes_json() -> serde_json::Value {
    crate::json("codes.json")
}

/// The kind `codes.json` gives `code`, in whichever group it is listed.
#[must_use]
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
///
/// # Panics
/// When any kind differs.
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
#[must_use]
pub fn distinct(codes: impl IntoIterator<Item = &'static str>) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for code in codes {
        if !out.contains(&code) {
            out.push(code);
        }
    }
    out
}
