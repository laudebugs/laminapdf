use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::nightly;

/// A fresh folder for one test's fixtures.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-proof-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(root: &Path, path: &str, value: &Value) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_string(value).unwrap()).unwrap();
}

fn facts(outcome: &str) -> Value {
    json!({"outcome": outcome, "page_count": null, "outline_titles": null})
}

/// The budgets a fixture night ran with.
struct Night {
    differential_sample: Option<u64>,
    render_sample: Option<u64>,
    render_pages: Option<u32>,
    fuzz_seconds: u64,
    /// Fuzz targets left out of the summaries.
    fuzz_missing: &'static [&'static str],
}

const FULL: Night = Night {
    differential_sample: None,
    render_sample: None,
    render_pages: Some(20),
    fuzz_seconds: 600,
    fuzz_missing: &[],
};

/// Lamina PDF opens `pdfjs/a.pdf`, repairs `pdfjs/b.pdf`, refuses `pdfbox/c.pdf` (which pdf.js
/// opens) and `pdfbox/d.pdf` (which every tool fails), panics on `qpdf/e.pdf` and opens the
/// mutant `m1.pdf`.
fn differential(root: &Path, night: &Night) {
    let lamina = json!({"tool": "lamina", "version": "0.2.0", "reports_repair": true, "facts": {
        "pdfjs/a.pdf": facts("opened"), "pdfjs/b.pdf": facts("repaired"), "pdfbox/c.pdf": facts("failed"),
        "pdfbox/d.pdf": facts("failed"), "qpdf/e.pdf": facts("panic"), "m1.pdf": facts("opened"),
    }});
    let pdfjs = json!({"tool": "pdfjs", "version": "5.4.296", "facts": {
        "pdfbox/c.pdf": facts("opened"), "pdfbox/d.pdf": facts("failed"),
    }});
    let qpdf = json!({"tool": "qpdf", "version": "12.2.0", "facts": {
        "pdfbox/c.pdf": facts("failed"), "pdfbox/d.pdf": facts("crash"),
    }});
    write(root, "target/nightly/differential/lamina.json", &lamina);
    write(root, "target/nightly/differential/pdfjs.json", &pdfjs);
    write(root, "target/nightly/differential/qpdf.json", &qpdf);
    let tally = json!({"agree": 4, "disagree": 1, "excused": 1});
    let report = json!({
        "commit": "abc", "inputs": 6, "external": 5, "sample": night.differential_sample,
        "tools": [{"tool": "pdfjs", "version": "5.4.296", "reports_repair": false, "files": 6, "timeouts": 1,
            "crashes": 0, "facts": {"outcome": tally, "page_count": tally, "outline_titles": tally}}],
        "lamina_errors": ["qpdf/e.pdf"], "missing": [], "broken_readers": [],
        "disagreements": [
            {"file": "pdfbox/c.pdf", "fact": "outcome", "kind": "split"},
            {"file": "pdfbox/c.pdf", "fact": "page_count", "kind": "failure"},
        ],
    });
    write(root, "target/nightly/differential/report.json", &report);
}

fn everything_else(root: &Path, night: &Night) {
    write(
        root,
        "target/nightly/differential-text/report.json",
        &json!({
            "sample": night.differential_sample,
            "tools": [{"tool": "pdfjs", "version": "5.4.296"}, {"tool": "pdftotext", "version": "25.10.0"}],
            "sources": {"pdfjs": {"files": 2, "pages": 9, "compared": 8, "agree": 7, "tools": {}}},
        }),
    );
    write(
        root,
        "target/nightly/render-diff/report.json",
        &json!({
            "sampling": {"dpi": 96, "pages": night.render_pages, "sample": night.render_sample, "sources": []},
            "sources": {
                "all": {"rows": 10, "classes": {"pass": 8, "minor": 1, "fail": 1}},
                "pdfjs": {"rows": 10, "classes": {"pass": 8, "minor": 1, "fail": 1}},
            },
            "record": {"benches": {"render-diff/all": 9000, "render-diff/pdfjs": 9000}},
        }),
    );
    for target in FUZZ_TARGETS.iter().filter(|t| !night.fuzz_missing.contains(t)) {
        let run = json!({"seconds": night.fuzz_seconds, "executions": 1000, "corpus_files": 40,
            "crashed": *target == "lex"});
        write(root, &format!("target/xtask/fuzz/{target}.json"), &run);
    }
    write(
        root,
        "target/xtask/verapdf/report.json",
        &json!({"version": "1.28.2", "validations": [
            {"file": "pdf_ua_report.pdf", "profile": "ua1", "compliant": true, "rules_passed": 106, "rules_failed": 0},
        ]}),
    );
    write(
        root,
        "target/xtask/roundtrip/summary.json",
        &json!({"cases": 30, "saved": 26, "refused": 4,
        "tools": {"qpdf": {"version": "qpdf version 12.2.0", "passed": true},
            "pdfjs": {"version": null, "passed": false}}}),
    );
    write(
        root,
        "target/xtask/pdf-runtimes/runtimes-linux.json",
        &json!({"passed": ["node"], "failed": ["bun"]}),
    );
    write(
        root,
        "target/xtask/pdf-runtimes/runtimes-webkit.json",
        &json!({"passed": ["webkit"], "failed": []}),
    );
}

fn run_info() -> Run {
    Run {
        commit: "abc".to_owned(),
        date: "2026-11-08T03:00:00+00:00".to_owned(),
        ran: "2026-11-08".to_owned(),
        runner: "r".to_owned(),
        lamina: "0.2.0-alpha.5".to_owned(),
        jobs: BTreeMap::new(),
    }
}

/// The record of a whole night with `night`'s budgets.
fn night_record(name: &str, night: &Night) -> Record {
    let root = scratch(name);
    differential(&root, night);
    everything_else(&root, night);
    let record = record(&root, run_info());
    fs::remove_dir_all(&root).unwrap();
    record
}

#[test]
fn corpus_should_count_each_outcome_per_source() {
    let corpus = night_record("corpus", &FULL).corpus.unwrap();
    let pdfbox = &corpus["pdfbox"];
    assert_eq!(
        (
            (corpus["pdfjs"].opened, corpus["pdfjs"].repaired),
            (pdfbox.files, pdfbox.refused, pdfbox.refused_alone),
            corpus["qpdf"].errors.panic,
            corpus["mutants"].opened,
        ),
        ((1, 1), (2, 2, 1), 1, 1)
    );
}

#[test]
fn differential_should_keep_each_tools_tallies_and_count_failures() {
    let d = night_record("differential", &FULL).differential.unwrap();
    let pdfjs = &d.tools["pdfjs"];
    assert_eq!(
        (
            pdfjs.timeouts,
            pdfjs.facts["outcome"].agree,
            d.failures,
            d.lamina_errors
        ),
        (1, 4, 1, 1)
    );
}

#[test]
fn every_other_section_should_read_its_report() {
    let r = night_record("sections", &FULL);
    let render = r.render.unwrap();
    let runtimes = r.runtimes.unwrap();
    assert_eq!(
        (
            r.text.unwrap().sources["pdfjs"].agree,
            (
                render.sources["pdfjs"].pass_minor_bp,
                render.sources["pdfjs"].classes["fail"]
            ),
            (
                r.fuzz.unwrap().len(),
                r.verapdf.unwrap().validations.len(),
                r.roundtrip.unwrap().saved
            ),
            (runtimes.passed, runtimes.failed),
        ),
        (
            7,
            (9000, 1),
            (FUZZ_TARGETS.len(), 1, 26),
            (vec!["node".to_owned(), "webkit".to_owned()], vec!["bun".to_owned()]),
        )
    );
}

#[test]
fn a_full_night_should_be_full() {
    let r = night_record("full", &FULL);
    assert!(r.full && !r.quick);
}

#[test]
fn a_missing_input_should_be_null_and_its_job_says_why() {
    let root = scratch("missing");
    differential(&root, &FULL);
    let mut info = run_info();
    info.jobs =
        parse_jobs(r#"{"differential": {"result": "success", "outputs": {}}, "verapdf": {"result": "failure"}}"#)
            .unwrap();
    let r = record(&root, info);
    fs::remove_dir_all(&root).unwrap();
    let value = serde_json::to_value(&r).unwrap();
    assert_eq!(
        (
            value["verapdf"].clone(),
            value["fuzz"].clone(),
            value["jobs"]["verapdf"].clone(),
            r.full
        ),
        (Value::Null, Value::Null, json!("failure"), false)
    );
}

#[test]
fn an_unreadable_report_should_be_null() {
    let root = scratch("unreadable");
    fs::create_dir_all(root.join("target/xtask/verapdf")).unwrap();
    fs::write(root.join("target/xtask/verapdf/report.json"), "{\"version\":").unwrap();
    let r = record(&root, run_info());
    fs::remove_dir_all(&root).unwrap();
    assert!(r.verapdf.is_none());
}

#[test]
fn a_sampled_differential_should_not_be_full() {
    let night = Night {
        differential_sample: Some(300),
        ..FULL
    };
    assert!(!night_record("sampled", &night).full);
}

#[test]
fn a_sampled_or_shorter_render_diff_should_not_be_full() {
    let sampled = Night {
        render_sample: Some(100),
        ..FULL
    };
    let every_page = Night {
        render_pages: None,
        ..FULL
    };
    assert!(!night_record("render-sampled", &sampled).full && !night_record("render-pages", &every_page).full);
}

#[test]
fn a_short_or_missing_fuzz_target_should_not_be_full() {
    let short = Night {
        fuzz_seconds: 30,
        ..FULL
    };
    let missing = Night {
        fuzz_missing: &["extract"],
        ..FULL
    };
    assert!(!night_record("fuzz-short", &short).full && !night_record("fuzz-missing", &missing).full);
}

#[test]
fn record_should_append_nothing_for_a_night_that_is_not_full() {
    let night = Night {
        fuzz_seconds: 30,
        ..FULL
    };
    let history = scratch("history-short");
    let appended = append(&history, &night_record("append-short", &night)).unwrap();
    let exists = history.join("history.jsonl").exists();
    fs::remove_dir_all(&history).unwrap();
    assert!(!appended && !exists);
}

#[test]
fn record_should_append_a_full_night_that_reads_back_as_the_same_record() {
    let history = scratch("history-full");
    let r = night_record("append-full", &FULL);
    append(&history, &r).unwrap();
    let back = nightly::read_history(&history.join("history.jsonl")).unwrap();
    fs::remove_dir_all(&history).unwrap();
    let parsed: Vec<Record> = back.into_iter().map(|v| serde_json::from_value(v).unwrap()).collect();
    assert_eq!(parsed, [r]);
}

#[test]
fn the_record_should_have_no_benches_so_no_benchmark_regresses_against_it() {
    let value = serde_json::to_value(night_record("benches", &FULL)).unwrap();
    let now = BTreeMap::from([("decode/flate".to_owned(), 1_000_000)]);
    assert!(value.get("benches").is_none() && nightly::regressions(&[value], "r", false, &now).is_empty());
}

#[test]
fn record_should_need_history() {
    assert!(parse_args(&["--record".to_owned()]).is_err());
}

#[test]
fn parse_args_should_read_every_flag() {
    let args: Vec<String> = [
        "--from",
        "a",
        "--jobs",
        "{}",
        "--history",
        "h",
        "--runner",
        "r",
        "--record",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(
        parse_args(&args).unwrap(),
        Args {
            from: Some("a".into()),
            jobs: Some("{}".to_owned()),
            history: Some("h".into()),
            runner: Some("r".to_owned()),
            record: true,
        }
    );
}

#[test]
fn the_summary_should_mark_a_missing_section() {
    let root = scratch("table");
    let r = record(&root, run_info());
    fs::remove_dir_all(&root).unwrap();
    assert!(summary::table(&r).contains("| fuzz | **missing** |"));
}
