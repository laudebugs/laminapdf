//! ADR 0018 tier 4: every corpus file opens (or fails with a coded error), and its page count
//! and outline titles equal pypdf's facts in `fixtures/corpus/expected.json`, except the
//! deliberate differences listed in `fixtures/corpus/exceptions.toml`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::exercise::exercise;
use common::{corpus, fixture, json, open, outline_titles};
use lamina_model::Error;
use serde_json::Value;

/// `(file, fact)` pairs that `exceptions.toml` excuses, each with its ADR 0003 rule.
fn exceptions() -> Vec<(String, String)> {
    let text = String::from_utf8(fixture("corpus/exceptions.toml")).unwrap();
    let table: toml::Table = text.parse().unwrap();
    table
        .get("exception")
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .flat_map(|e| {
            let rule = e["rule"].as_str().unwrap();
            assert!(
                rule.starts_with("ADR 0003"),
                "every exception names its ADR 0003 rule: {e:?}"
            );
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

fn check_fact(fact: &str, ours: impl Fn(&lamina_model::Document) -> Option<Value>) {
    let expected = json("corpus/expected.json");
    let excused = exceptions();
    let mut failures = Vec::new();
    for (name, bytes) in corpus() {
        if excused.contains(&(name.clone(), fact.to_owned())) {
            continue;
        }
        let theirs = &expected[&name][fact];
        assert!(!theirs.is_null(), "{name}: {fact} missing from expected.json");
        match open(&bytes) {
            Ok(doc) => {
                let ours = ours(&doc);
                if ours.as_ref() != Some(theirs) {
                    failures.push(format!("{name}: {fact}: ours {ours:?} vs pypdf {theirs}"));
                }
            }
            Err(e) => failures.push(format!("{name}: open failed: {e}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_corpus_file_opens_or_fails_with_a_coded_error() {
    for (name, bytes) in corpus() {
        if let Err(e) = open(&bytes) {
            assert!(Error::CODES.contains(&e.code()), "{name}: uncoded error {e:?}");
        }
    }
}

#[test]
fn every_corpus_file_is_in_expected_json() {
    let expected = json("corpus/expected.json");
    for (name, _) in corpus() {
        assert!(expected[&name].is_object(), "{name} missing from expected.json");
    }
}

/// capability: C.04 core read
#[test]
fn page_count_matches_pypdf() {
    check_fact("page_count", |doc| doc.pages().ok().map(|p| p.len().into()));
}

/// capability: I.01 core read
#[test]
fn outline_titles_match_pypdf() {
    check_fact("outline_titles", |doc| outline_titles(doc).ok().map(Value::from));
}

#[test]
fn every_view_reads_every_corpus_file_without_errors() {
    for (name, bytes) in corpus() {
        let doc = open(&bytes).unwrap();
        assert_eq!(exercise(&doc), 0, "{name}");
    }
}

#[test]
fn exceptions_name_corpus_files() {
    let names: Vec<String> = corpus().into_iter().map(|(n, _)| n).collect();
    for (file, fact) in exceptions() {
        assert!(names.contains(&file), "{file}");
        assert!(["page_count", "outline_titles"].contains(&fact.as_str()), "{fact}");
    }
}
