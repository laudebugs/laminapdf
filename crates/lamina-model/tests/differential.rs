//! ADR 0018 tier 6 (pypdf only, B2c): lamina against pypdf over `fixtures/corpus` and its
//! fixed-seed mutants (`common::mutants`), comparing the open/repair/fail outcome, the page
//! count and the outline titles with the expectations pypdf produced, precomputed in
//! `fixtures/differential/expected.json` by `cargo xtask differential --update`. No Python is
//! needed here.
//!
//! The full lane's larger set (`cargo xtask differential --mutants 500`, X4) is the ignored
//! test at the end, with its facts in `mutants-500.json` and its exceptions in
//! `mutants-500-exceptions.toml`.
//!
//! A disagreement is either a lamina bug (fix it) or a deliberate difference, listed in
//! `fixtures/differential/exceptions.toml` with the ADR 0003 rule behind lamina's answer.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use std::collections::BTreeSet;

use common::{corpus, fixture, json, mutants, open, outline_titles};
use serde_json::{Value, json};

/// lamina's facts about `bytes`, in the shape `expected.json` stores pypdf's, extracted the
/// same way as `tools/corpus-facts/facts.py` does: the outcome is about opening the file and
/// walking its pages; an unreadable outline is its own fact (`null`) and leaves the outcome.
fn ours(bytes: &[u8]) -> Value {
    let failed = json!({"outcome": "failed", "page_count": null, "outline_titles": null});
    let Ok(doc) = open(bytes) else { return failed };
    let Ok(pages) = doc.pages() else { return failed };
    let outcome = if doc.was_repaired() { "repaired" } else { "opened" };
    json!({
        "outcome": outcome,
        "page_count": pages.len(),
        "outline_titles": outline_titles(&doc).ok(),
    })
}

/// `(file, fact)` pairs excused by an exceptions file (`fixtures/<rel>`). An entry names one
/// input (`file`) or several that differ for the same reason (`files`).
fn exceptions(rel: &str) -> BTreeSet<(String, String)> {
    let text = String::from_utf8(fixture(rel)).unwrap();
    let table: toml::Table = text.parse().unwrap();
    table
        .get("exception")
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .flat_map(|e| {
            assert!(
                e["rule"].as_str().unwrap().starts_with("ADR 0003"),
                "every exception names its ADR 0003 rule: {e:?}"
            );
            assert!(!e["reason"].as_str().unwrap().is_empty(), "{e:?}");
            let files: Vec<String> = match (e.get("file"), e.get("files")) {
                (Some(file), None) => vec![file.as_str().unwrap().to_owned()],
                (None, Some(files)) => files
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|f| f.as_str().unwrap().to_owned())
                    .collect(),
                _ => panic!("an exception has exactly one of `file` and `files`: {e:?}"),
            };
            let facts: Vec<String> = e["facts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| f.as_str().unwrap().to_owned())
                .collect();
            files
                .into_iter()
                .flat_map(move |file| facts.clone().into_iter().map(move |f| (file.clone(), f)))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// One input set, with the files holding pypdf's facts about it and the excused differences,
/// and the seed of its mutants.
struct Set {
    expected: String,
    exceptions: String,
    inputs: Vec<(String, Vec<u8>)>,
    seed: u64,
}

/// The fast lane's set: the corpus and its four mutants per file.
fn fast() -> Set {
    Set {
        expected: "differential/expected.json".to_owned(),
        exceptions: "differential/exceptions.toml".to_owned(),
        inputs: corpus().into_iter().chain(mutants::all()).collect(),
        seed: mutants::SEED,
    }
}

/// The full lane's set: `$LAMINA_DIFFERENTIAL_MUTANTS` mutants (`cargo xtask differential
/// --mutants <n>` sets it; 500 when unset), facts in `differential/mutants-<n>.json` and
/// exceptions in `differential/mutants-<n>-exceptions.toml`.
fn full() -> Set {
    let n: usize = std::env::var("LAMINA_DIFFERENTIAL_MUTANTS")
        .map_or(Ok(500), |n| n.parse())
        .unwrap();
    Set {
        expected: format!("differential/mutants-{n}.json"),
        exceptions: format!("differential/mutants-{n}-exceptions.toml"),
        inputs: mutants::full(n),
        seed: mutants::FULL_SEED,
    }
}

/// Every fact on which lamina and pypdf disagree and `exceptions.toml` doesn't excuse.
fn disagreements(set: &Set) -> Vec<String> {
    let expected = json(&set.expected);
    let excused = exceptions(&set.exceptions);
    let mut failures = Vec::new();
    for (name, bytes) in &set.inputs {
        let theirs = &expected[name];
        assert!(theirs.is_object(), "{name} missing from fixtures/{}", set.expected);
        let ours = ours(bytes);
        for fact in FACTS {
            if ours[fact] != theirs[fact] && !excused.contains(&(name.clone(), fact.to_owned())) {
                failures.push(format!(
                    "{name}: {fact}: lamina {} vs pypdf {}",
                    ours[fact], theirs[fact]
                ));
            }
        }
    }
    failures
}

/// Exceptions on which lamina now agrees with pypdf, or for files that aren't in the set.
fn stale_exceptions(set: &Set) -> Vec<String> {
    let expected = json(&set.expected);
    let excused = exceptions(&set.exceptions);
    let names: BTreeSet<&String> = set.inputs.iter().map(|(n, _)| n).collect();
    let mut stale: Vec<String> = excused
        .iter()
        .filter(|(file, _)| !names.contains(file))
        .map(|(file, fact)| format!("{file}: {fact} (not an input)"))
        .collect();
    for (name, bytes) in &set.inputs {
        let ours = ours(bytes);
        for fact in FACTS {
            if ours[fact] == expected[name][fact] && excused.contains(&(name.clone(), fact.to_owned())) {
                stale.push(format!("{name}: {fact}"));
            }
        }
    }
    stale
}

/// The names in `set.expected` and in `set.inputs`, for an exact-coverage check.
fn coverage(set: &Set) -> (BTreeSet<String>, BTreeSet<String>) {
    let expected = json(&set.expected);
    let listed = expected.as_object().unwrap().keys().cloned().collect();
    let names = set.inputs.iter().map(|(n, _)| n.clone()).collect();
    (listed, names)
}

const FACTS: [&str; 3] = ["outcome", "page_count", "outline_titles"];

#[test]
fn lamina_agrees_with_pypdf_except_for_listed_exceptions() {
    let set = fast();
    let failures = disagreements(&set);
    assert!(
        failures.is_empty(),
        "mutant seed {:#x}; {} disagreements:\n{}",
        set.seed,
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_exception_is_still_needed() {
    let stale = stale_exceptions(&fast());
    assert!(stale.is_empty(), "exceptions lamina no longer needs: {stale:?}");
}

#[test]
fn expectations_cover_exactly_the_inputs() {
    let (listed, names) = coverage(&fast());
    assert_eq!(listed, names, "rerun `cargo xtask differential --update`");
}

/// The full lane's set in one test (it is ignored in the fast lane): coverage, agreement and
/// stale exceptions.
#[test]
#[ignore = "full lane: run with `cargo xtask differential --mutants 500`"]
fn full_mutant_set_agrees_with_pypdf() {
    let set = full();
    let (listed, names) = coverage(&set);
    assert_eq!(listed, names, "rerun `cargo xtask differential --mutants <n> --update`");
    let failures = disagreements(&set);
    assert!(
        failures.is_empty(),
        "mutant seed {:#x}; {} disagreements:\n{}",
        set.seed,
        failures.len(),
        failures.join("\n")
    );
    let stale = stale_exceptions(&set);
    assert!(stale.is_empty(), "exceptions lamina no longer needs: {stale:?}");
}
