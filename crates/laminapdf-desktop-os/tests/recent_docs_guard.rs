//! `SHAddToRecentDocs` with a null `pv` empties every app's Recent Items on Windows, not just
//! Lamina PDF's. This guard reads the crate's sources, on every OS, and fails the build unless
//! each call passes a path (`SHARD_PATHW`) and a pointer that can't be null (`Some(…)`).

use std::path::Path;

/// Every call of `SHAddToRecentDocs` in the crate's sources: the text from the name to the
/// statement's end.
fn calls() -> Vec<String> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let files = std::fs::read_dir(src).into_iter().flatten().filter_map(Result::ok);
    for entry in files {
        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let mut rest = text.as_str();
        while let Some(at) = rest.find("SHAddToRecentDocs(") {
            let call = &rest[at..];
            let end = call.find(';').unwrap_or(call.len());
            found.push(call[..end].to_owned());
            rest = &call[end..];
        }
    }
    found
}

#[test]
fn every_call_should_pass_a_path_never_null() {
    let calls = calls();
    let bad: Vec<&String> = calls
        .iter()
        .filter(|call| !call.contains("SHARD_PATHW") || !call.contains("Some(") || call.contains("None"))
        .collect();
    assert!(!calls.is_empty(), "the guard found no call to check");
    assert!(
        bad.is_empty(),
        "SHAddToRecentDocs must take a path, never null: {bad:?}"
    );
}
