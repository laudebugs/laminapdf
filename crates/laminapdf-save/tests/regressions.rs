//! The save fuzz targets (`ops_json`, `save_roundtrip`, `copy_pages`; ADR 0018 tier 9): every
//! saved crash input in `fixtures/regressions/<target>/` replays through the entry point its
//! target calls, and so does every seed the targets start from (the op samples, and the fixture
//! PDFs, behind the seed control bytes for `save_roundtrip`), so the fast lane runs what the
//! fuzzer runs.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use common::fuzz_entry::{SAVE_SEED_CONTROL, TARGETS, copy_pages, op_log, ops_json, save_roundtrip};

fn files(dir: &str) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(common::fixtures_dir().join(dir)) else {
        return Vec::new();
    };
    let mut out: Vec<_> = entries.map(|e| e.unwrap().path()).collect();
    out.sort();
    out
}

#[test]
fn every_saved_crash_input_replays_without_a_panic() {
    for (target, entry) in TARGETS {
        for path in files(&format!("regressions/{target}")) {
            entry(&std::fs::read(&path).unwrap());
        }
    }
}

#[test]
fn every_fixture_pdf_runs_through_copy_pages() {
    for dir in ["corpus", "golden", "signed"] {
        for path in files(dir)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
        {
            copy_pages(&std::fs::read(&path).unwrap());
        }
    }
}

#[test]
fn every_op_sample_runs_through_ops_json() {
    for path in files("ops/v1") {
        ops_json(&std::fs::read(&path).unwrap());
    }
}

#[test]
fn every_fixture_pdf_runs_through_save_roundtrip_with_each_mode() {
    for dir in ["corpus", "golden", "signed"] {
        for path in files(dir)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
        {
            for flags in [0u8, 1, 2, 4 | 2, 8, 16 | 2, 32 | 2] {
                let mut seed = SAVE_SEED_CONTROL.to_vec();
                seed[0] = flags;
                seed.extend(std::fs::read(&path).unwrap());
                save_roundtrip(&seed);
            }
        }
    }
}

#[test]
fn a_short_input_is_ignored() {
    save_roundtrip(b"abc");
    ops_json(&fixture("corpus/hello.pdf"));
}

#[test]
fn a_bare_op_sample_is_read_as_a_one_op_log() {
    let log = op_log(&fixture("ops/v1/rotate-page.json")).unwrap();
    assert_eq!(log.ops().len(), 1);
}

#[test]
fn a_log_longer_than_the_default_undo_depth_is_undone_completely() {
    // Each op sets a different title, so each is its own undo entry.
    let ops: Vec<String> = (0..600)
        .map(|i| format!(r#"{{ "op": "setInfo", "args": {{ "patch": {{ "title": "t{i}" }} }} }}"#))
        .collect();
    let log = format!(r#"{{ "v": 1, "ops": [{}] }}"#, ops.join(","));
    assert_eq!(op_log(log.as_bytes()).map(|l| l.ops().len()), Some(600));
    ops_json(log.as_bytes());
}
