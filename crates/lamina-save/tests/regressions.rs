//! The save fuzz targets (`ops_json`, `save_roundtrip`; ADR 0018 tier 9): every saved crash
//! input in `fixtures/regressions/<target>/` replays through the entry point its target calls,
//! and so does every seed the targets start from (the op samples, and the fixture PDFs behind
//! the seed control bytes), so the fast lane runs what the fuzzer runs.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use common::fuzz_entry::{SAVE_SEED_CONTROL, TARGETS, ops_json, save_roundtrip};

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
