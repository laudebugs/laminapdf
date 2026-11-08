//! `laminapdf reveal`: the categories it finds, its text and JSON, `--fields`, `--fail-on` and
//! exit 6, `--include-hidden-text`, and batches.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers: a panic is a test failure"
)]

mod common;

use common::schemas::Validator;
use common::{arg, folder, laminapdf};
use serde_json::json;

/// The file holding something in every category, written to the folder `name`.
fn disclosing(name: &str) -> std::path::PathBuf {
    let path = folder(name).join("disclosing.pdf");
    std::fs::write(&path, laminapdf_test_fixtures::disclosing()).unwrap();
    path
}

/// A page whose sentence a black box covers, in the folder `name`.
fn covered(name: &str) -> std::path::PathBuf {
    let path = folder(name).join("covered.pdf");
    std::fs::write(&path, common::page_with_text("Pay John Smith now.", true)).unwrap();
    path
}

#[test]
fn a_file_built_to_disclose_should_list_every_category_and_validate() {
    let path = disclosing("reveal-full");
    let run = laminapdf(&["reveal", arg(&path), "--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let report = run.json();
    Validator::new().check("reveal.schema.json", &report);
    assert_eq!(
        report["found"],
        json!([
            "history",
            "metadata",
            "attachments",
            "active-content",
            "annotations",
            "forms",
            "hidden-content"
        ])
    );
    assert_eq!(report["history"]["revisions"][1]["changed"], json!([4]));
    assert_eq!(report["history"]["revisions"][0]["removedWords"], 3);
    let keys: Vec<&str> = report["metadata"]["info"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["Title", "Producer", "SourceModified"]);
    assert_eq!(report["attachments"][0]["name"], "notes.txt");
    let kinds: Vec<&str> = report["activeContent"]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["JavaScript", "Launch"]);
    assert_eq!(report["annotations"]["comments"][0]["contents"], "Fix the totals");
    assert_eq!(
        report["forms"][0],
        json!({ "name": "name", "kind": "Tx", "value": "Alice", "filled": true, "hidden": false })
    );
    assert_eq!(report["hiddenContent"]["layersOff"], json!([{ "name": "Draft notes" }]));
    assert_eq!(report["documentText"], true);
}

#[test]
fn the_text_should_have_a_line_per_category_found() {
    let path = disclosing("reveal-text");
    let run = laminapdf(&["reveal", arg(&path)]);
    let text = run.stdout_text();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 8, "{text}");
    assert!(lines[1].starts_with("  history         2 revisions"), "{text}");
    assert!(lines[7].contains("layer \"Draft notes\" off"), "{text}");
}

#[test]
fn a_plain_file_should_find_nothing_and_pass_fail_on_all() {
    let run = laminapdf(&["reveal", "fixtures/corpus/hello.pdf", "--fail-on", "all"]);
    assert_eq!(
        (run.code, run.stdout_text()),
        (0, "fixtures/corpus/hello.pdf: nothing beyond its pages\n".to_owned())
    );
    assert_eq!(
        laminapdf(&["reveal", "fixtures/corpus/hello.pdf", "--json"]).json()["found"],
        json!([])
    );
}

#[test]
fn fail_on_should_exit_6_and_still_print_the_report() {
    let path = covered("reveal-fail-on");
    let run = laminapdf(&["reveal", arg(&path), "--fail-on", "hidden-content", "--json"]);
    assert_eq!(run.code, 6);
    assert_eq!(run.json()["found"], json!(["hidden-content"]));
    assert_eq!(laminapdf(&["reveal", arg(&path), "--fail-on", "history,forms"]).code, 0);
    let full = disclosing("reveal-fail-on-all");
    assert_eq!(laminapdf(&["reveal", arg(&full), "--fail-on", "all"]).code, 6);
}

#[test]
fn an_unknown_category_should_be_a_usage_error_before_any_read() {
    let run = laminapdf(&["reveal", "no-such-file.pdf", "--fail-on", "bogus"]);
    assert_eq!(run.code, 2);
    assert!(!run.stderr.contains("input-unreadable"), "{}", run.stderr);
}

#[test]
fn hidden_text_should_show_only_when_asked() {
    let path = covered("reveal-hidden");
    let plain = laminapdf(&["reveal", arg(&path), "--json"]);
    assert!(!plain.stdout_text().contains("John"), "{}", plain.stdout_text());
    assert!(!laminapdf(&["reveal", arg(&path)]).stdout_text().contains("John"));
    let shown = laminapdf(&["reveal", arg(&path), "--json", "--include-hidden-text"]);
    let report = shown.json();
    Validator::new().check("reveal.schema.json", &report);
    let runs = report["hiddenContent"]["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["reason"], "covered");
    assert_eq!(runs[0]["page"], json!({ "index": 0, "number": 1 }));
    assert_eq!(shown.stdout_text().matches("John Smith").count(), 1);
}

#[test]
fn fields_should_keep_only_the_keys_named() {
    let path = disclosing("reveal-fields");
    let report = laminapdf(&["reveal", arg(&path), "--json", "--fields", "found,forms"]).json();
    let mut keys: Vec<&str> = report.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["forms", "found", "input", "schema"]);
}

#[test]
fn a_batch_should_exit_6_with_found_counted_and_1_once_an_input_fails() {
    let path = covered("reveal-batch");
    let run = laminapdf(&[
        "reveal",
        "fixtures/corpus/hello.pdf",
        arg(&path),
        "--fail-on",
        "hidden-content",
        "--json",
    ]);
    assert_eq!(run.code, 6, "{}", run.stderr);
    let summary: serde_json::Value = serde_json::from_str(run.stdout_text().lines().last().unwrap()).unwrap();
    assert_eq!(
        (summary["type"].as_str(), summary["found"].as_u64()),
        (Some("summary"), Some(1))
    );
    let failing = laminapdf(&[
        "reveal",
        "fixtures/corpus/hello.pdf",
        arg(&path),
        "no-such-file.pdf",
        "--fail-on",
        "hidden-content",
    ]);
    assert_eq!(failing.code, 1);
}

#[test]
fn every_fixture_should_reveal_without_error() {
    for dir in ["corpus", "golden", "signed"] {
        let root = laminapdf_test_fixtures::fixtures_dir().join(dir);
        for entry in std::fs::read_dir(&root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "pdf") {
                continue;
            }
            let run = laminapdf(&["reveal", arg(&path), "--json"]);
            // 5: an encrypted fixture without its password; anything else must read.
            assert!(
                matches!(run.code, 0 | 3 | 5),
                "{}: {} {}",
                path.display(),
                run.code,
                run.stderr
            );
        }
    }
}
