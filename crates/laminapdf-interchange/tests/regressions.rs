//! Replays every saved crash input of the `xfdf` and `fdf` fuzz targets
//! (`fixtures/regressions/<target>/`) through the target's own entry point.

#![allow(clippy::panic, clippy::unwrap_used, reason = "test code: a panic is a test failure")]

mod common;

use common::fuzz_entry::TARGETS;

#[test]
fn every_saved_crash_input_runs_clean() {
    for (name, entry) in TARGETS {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/regressions/{name}"));
        let Ok(inputs) = std::fs::read_dir(&dir) else {
            continue;
        };
        for input in inputs {
            entry(&std::fs::read(input.unwrap().path()).unwrap());
        }
    }
}
