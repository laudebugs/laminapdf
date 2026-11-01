//! ADR 0018 tier 6 (pypdf only, B2c): Lamina PDF against pypdf over `fixtures/corpus` and its
//! fixed-seed mutants (`common::mutants`), comparing the open/repair/fail outcome, the page
//! count and the outline titles with the expectations pypdf produced, precomputed by
//! `cargo xtask differential [--full] --update`. No Python is needed here.
//!
//! Two sets: the fast set (the corpus and four mutants per file; `differential/expected.json`,
//! `exceptions.toml`) and the full lane's set (`mutants::FULL_PER_FILE` mutants per file;
//! `differential/full.json`, `full-exceptions.toml`), checked by the ignored test at the end,
//! which `cargo xtask differential --full` runs (X4).
//!
//! A disagreement is either a bug in Lamina PDF (fix it) or a deliberate difference, listed in the
//! set's exceptions file with the ADR 0003 rule behind Lamina PDF's answer.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use std::collections::BTreeSet;
use std::sync::OnceLock;

use common::differential::{FACTS, facts as ours};
use common::{fixture, json, mutants};
/// The tools of the nightly lane's live differential (`tools/xtask/src/live.rs`).
const TOOLS: [&str; 4] = ["pdfjs", "pdfium", "qpdf", "pypdf"];

/// The `(file, fact)` pairs an exceptions file excuses. Each entry names one input (`file`)
/// or several that differ for the same reason (`files`), a non-empty `facts` list, its
/// ADR 0003 rule and a reason. A pair listed twice is an error: it would hide a stale entry.
/// An entry with `tools` (the nightly lane's live differential) applies here only when it lists
/// `pypdf`, this lane's only tool; an unknown tool name is an error.
fn parse_exceptions(text: &str) -> Result<BTreeSet<(String, String)>, String> {
    let table: toml::Table = text.parse().map_err(|e| format!("not TOML: {e}"))?;
    let entries = match table.get("exception") {
        None => return Ok(BTreeSet::new()),
        Some(entries) => entries.as_array().ok_or("`exception` must be an array of tables")?,
    };
    let strings = |value: Option<&toml::Value>, key: &str| -> Result<Vec<String>, String> {
        let list = value
            .and_then(toml::Value::as_array)
            .ok_or_else(|| format!("`{key}` must be an array"))?;
        if list.is_empty() {
            return Err(format!("`{key}` is empty"));
        }
        list.iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("`{key}` holds a non-string"))
            })
            .collect()
    };
    let mut excused = BTreeSet::new();
    for e in entries {
        let rule = e.get("rule").and_then(toml::Value::as_str).unwrap_or_default();
        if !rule.starts_with("ADR 0003") {
            return Err(format!("every exception names its ADR 0003 rule: {e:?}"));
        }
        if e.get("reason")
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .is_empty()
        {
            return Err(format!("every exception gives a reason: {e:?}"));
        }
        let files = match (e.get("file"), e.get("files")) {
            (Some(file), None) => vec![file.as_str().ok_or("`file` must be a string")?.to_owned()],
            (None, Some(_)) => strings(e.get("files"), "files")?,
            _ => return Err(format!("an exception has exactly one of `file` and `files`: {e:?}")),
        };
        if e.get("tools").is_some() {
            let tools = strings(e.get("tools"), "tools")?;
            if let Some(bad) = tools.iter().find(|t| !TOOLS.contains(&t.as_str())) {
                return Err(format!("unknown tool {bad:?}; tools are {TOOLS:?}"));
            }
            if !tools.iter().any(|t| t == "pypdf") {
                continue;
            }
        }
        let facts = strings(e.get("facts"), "facts")?;
        if let Some(bad) = facts.iter().find(|f| !FACTS.contains(&f.as_str())) {
            return Err(format!("unknown fact {bad:?}; facts are {FACTS:?}"));
        }
        for file in &files {
            for fact in &facts {
                if !excused.insert((file.clone(), fact.clone())) {
                    return Err(format!("{file}: {fact} is excused twice"));
                }
            }
        }
    }
    Ok(excused)
}

/// What comparing one input set with pypdf's facts found.
#[derive(Debug, Default)]
struct Report {
    /// Inputs without facts, and facts for files that aren't inputs.
    coverage: Vec<String>,
    /// Facts on which Lamina PDF and pypdf disagree and no exception excuses.
    failures: Vec<String>,
    /// Exceptions Lamina PDF no longer needs, or for files that aren't inputs.
    stale: Vec<String>,
}

/// Compares Lamina PDF with `fixtures/<expected>` over `inputs`, excusing what
/// `fixtures/<exceptions>` lists. One pass: each input is opened once.
fn compare(expected: &str, exceptions: &str, inputs: &[(String, Vec<u8>)]) -> Report {
    let facts = json(expected);
    let facts = facts.as_object().unwrap();
    let excused = parse_exceptions(&String::from_utf8(fixture(exceptions)).unwrap())
        .unwrap_or_else(|e| panic!("fixtures/{exceptions}: {e}"));
    let names: BTreeSet<&String> = inputs.iter().map(|(n, _)| n).collect();
    let mut report = Report::default();
    report.coverage.extend(
        facts
            .keys()
            .filter(|k| !names.contains(k))
            .map(|k| format!("{k}: in fixtures/{expected} but not an input")),
    );
    report.stale.extend(
        excused
            .iter()
            .filter(|(file, _)| !names.contains(file))
            .map(|(file, fact)| format!("{file}: {fact} (not an input)")),
    );
    for (name, bytes) in inputs {
        let Some(theirs) = facts.get(name) else {
            report
                .coverage
                .push(format!("{name}: missing from fixtures/{expected}"));
            continue;
        };
        let ours = ours(bytes);
        for fact in FACTS {
            let excuse = excused.contains(&(name.clone(), fact.to_owned()));
            match (ours[fact] == theirs[fact], excuse) {
                (false, false) => report.failures.push(format!(
                    "{name}: {fact}: lamina {} vs pypdf {}",
                    ours[fact], theirs[fact]
                )),
                (true, true) => report.stale.push(format!("{name}: {fact}")),
                _ => {}
            }
        }
    }
    report
}

fn assert_clean(report: &Report, seed: u64, update: &str) {
    assert!(
        report.coverage.is_empty(),
        "rerun `{update}`:\n{}",
        report.coverage.join("\n")
    );
    assert!(
        report.failures.is_empty(),
        "mutant seed {seed:#x}; {} disagreements:\n{}",
        report.failures.len(),
        report.failures.join("\n")
    );
    assert!(
        report.stale.is_empty(),
        "exceptions lamina no longer needs: {:?}",
        report.stale
    );
}

/// The fast set's report, computed once for its three tests.
fn fast() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| {
        compare(
            "differential/expected.json",
            "differential/exceptions.toml",
            &mutants::fast_inputs(),
        )
    })
}

#[test]
fn expectations_cover_exactly_the_inputs() {
    assert!(
        fast().coverage.is_empty(),
        "rerun `cargo xtask differential --update`: {:?}",
        fast().coverage
    );
}

#[test]
fn lamina_agrees_with_pypdf_except_for_listed_exceptions() {
    let failures = &fast().failures;
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
    assert!(
        fast().stale.is_empty(),
        "exceptions lamina no longer needs: {:?}",
        fast().stale
    );
}

/// The full lane's set in one test (ignored in the fast lane): coverage, agreement and stale
/// exceptions, in one pass over its mutants.
#[test]
#[ignore = "full lane: run with `cargo xtask differential --full`"]
fn full_set_agrees_with_pypdf() {
    let report = compare(
        "differential/full.json",
        "differential/full-exceptions.toml",
        &mutants::full(),
    );
    assert_clean(&report, mutants::FULL_SEED, "cargo xtask differential --full --update");
}

#[test]
fn exceptions_name_a_rule_and_a_reason() {
    let text =
        "[[exception]]\nfile = \"a.pdf\"\nfacts = [\"outcome\"]\nrule = \"ADR 0003 §1: Repair\"\nreason = \"why\"\n";
    assert_eq!(parse_exceptions(text).unwrap().len(), 1);
    assert!(parse_exceptions(&text.replace("ADR 0003", "ADR 0018")).is_err());
    assert!(parse_exceptions(&text.replace("\"why\"", "\"\"")).is_err());
}

#[test]
fn exceptions_reject_empty_lists() {
    let base = "rule = \"ADR 0003 §1: Repair\"\nreason = \"why\"\n";
    let no_files = format!("[[exception]]\nfiles = []\nfacts = [\"outcome\"]\n{base}");
    let no_facts = format!("[[exception]]\nfile = \"a.pdf\"\nfacts = []\n{base}");
    assert!(parse_exceptions(&no_files).is_err() && parse_exceptions(&no_facts).is_err());
}

#[test]
fn exceptions_reject_a_pair_listed_twice() {
    let entry = "[[exception]]\nfiles = [\"a.pdf\", \"b.pdf\"]\nfacts = [\"outcome\"]\nrule = \"ADR 0003 §1: Repair\"\nreason = \"why\"\n";
    let twice = format!("{entry}{}", entry.replace("\"a.pdf\", ", ""));
    assert!(parse_exceptions(&twice).is_err());
}

#[test]
fn exceptions_reject_unknown_facts_and_both_file_keys() {
    let base = "rule = \"ADR 0003 §1: Repair\"\nreason = \"why\"\n";
    let unknown = format!("[[exception]]\nfile = \"a.pdf\"\nfacts = [\"colour\"]\n{base}");
    let both = format!("[[exception]]\nfile = \"a.pdf\"\nfiles = [\"b.pdf\"]\nfacts = [\"outcome\"]\n{base}");
    assert!(parse_exceptions(&unknown).is_err() && parse_exceptions(&both).is_err());
}

#[test]
fn exceptions_for_other_tools_should_not_excuse_pypdf() {
    let base = "file = \"a.pdf\"\nfacts = [\"outcome\"]\nrule = \"ADR 0003 §1: Repair\"\nreason = \"why\"\n";
    let qpdf_only = format!("[[exception]]\ntools = [\"qpdf\"]\n{base}");
    let with_pypdf = format!("[[exception]]\ntools = [\"qpdf\", \"pypdf\"]\n{base}");
    assert_eq!(
        (
            parse_exceptions(&qpdf_only).unwrap().len(),
            parse_exceptions(&with_pypdf).unwrap().len()
        ),
        (0, 1)
    );
}

#[test]
fn exceptions_reject_unknown_tools() {
    let text = "[[exception]]\ntools = [\"mupdf\"]\nfile = \"a.pdf\"\nfacts = [\"outcome\"]\nrule = \"ADR 0003 §1: Repair\"\nreason = \"why\"\n";
    assert!(parse_exceptions(text).is_err());
}
