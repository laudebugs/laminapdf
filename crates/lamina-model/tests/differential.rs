//! ADR 0018 tier 6 (pypdf only, B2c): lamina against pypdf over `fixtures/corpus` and its
//! fixed-seed mutants (`common::mutants`), comparing the open/repair/fail outcome, the page
//! count and the outline titles with the expectations pypdf produced, precomputed in
//! `fixtures/differential/expected.json` by `cargo xtask differential --update`. No Python is
//! needed here.
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

use common::differential::{FACTS, facts as ours};
use common::{corpus, fixture, json, mutants};

/// `(file, fact)` pairs excused by `exceptions.toml`.
fn exceptions() -> BTreeSet<(String, String)> {
    let text = String::from_utf8(fixture("differential/exceptions.toml")).unwrap();
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
            let file = e["file"].as_str().unwrap().to_owned();
            e["facts"]
                .as_array()
                .unwrap()
                .iter()
                .map(move |f| (file.clone(), f.as_str().unwrap().to_owned()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn inputs() -> Vec<(String, Vec<u8>)> {
    corpus().into_iter().chain(mutants::all()).collect()
}

#[test]
fn lamina_agrees_with_pypdf_except_for_listed_exceptions() {
    let expected = json("differential/expected.json");
    let excused = exceptions();
    let mut failures = Vec::new();
    for (name, bytes) in inputs() {
        let theirs = &expected[&name];
        assert!(
            theirs.is_object(),
            "{name} missing from fixtures/differential/expected.json"
        );
        let ours = ours(&bytes);
        for fact in FACTS {
            if ours[fact] != theirs[fact] && !excused.contains(&(name.clone(), fact.to_owned())) {
                failures.push(format!(
                    "{name}: {fact}: lamina {} vs pypdf {}",
                    ours[fact], theirs[fact]
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "mutant seed {:#x}; {} disagreements:\n{}",
        mutants::SEED,
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_exception_is_still_needed() {
    let expected = json("differential/expected.json");
    let excused = exceptions();
    let mut stale = Vec::new();
    for (name, bytes) in inputs() {
        let ours = ours(&bytes);
        for fact in FACTS {
            if ours[fact] == expected[&name][fact] && excused.contains(&(name.clone(), fact.to_owned())) {
                stale.push(format!("{name}: {fact}"));
            }
        }
    }
    assert!(stale.is_empty(), "exceptions lamina no longer needs: {stale:?}");
}

#[test]
fn expectations_cover_exactly_the_inputs() {
    let expected = json("differential/expected.json");
    let listed: BTreeSet<&String> = expected.as_object().unwrap().keys().collect();
    let names: Vec<String> = inputs().into_iter().map(|(n, _)| n).collect();
    assert_eq!(
        listed,
        names.iter().collect(),
        "rerun `cargo xtask differential --update`"
    );
}
