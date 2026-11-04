//! ADR 0018 tier 9: every minimised crash of the `extract` fuzz target in
//! `fixtures/regressions/extract/` is replayed through the target's entry point
//! (`common::fuzz_entry::extract`), so a fixed crash stays fixed; so is every fixture PDF,
//! the target's seeds.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::fuzz_entry::extract;
use laminapdf_test_fixtures::fixtures_dir;

fn replay(dir: &str) {
    let Ok(files) = std::fs::read_dir(fixtures_dir().join(dir)) else {
        return;
    };
    for file in files {
        let path = file.unwrap().path();
        if path.is_file() {
            extract(&std::fs::read(path).unwrap());
        }
    }
}

#[test]
fn every_saved_crash_input_replays_without_a_panic() {
    replay("regressions/extract");
}

#[test]
fn every_corpus_and_reading_order_file_keeps_the_invariants() {
    replay("corpus");
    replay("reading-order");
}
