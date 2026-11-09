//! ADR 0018 tier 9: every minimised crash of the `markdown` fuzz target in
//! `fixtures/regressions/markdown/` is replayed through the target's entry point
//! (`fuzz_entry::markdown`), so a fixed crash stays fixed; so are the Markdown fixtures and
//! `fuzz/seeds/markdown/`, the target's seeds.
#![cfg(feature = "markdown")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::expect_used,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

#[allow(
    unreachable_pub,
    reason = "the fuzz workspace calls the entry point as a public item"
)]
#[path = "common/fuzz_entry.rs"]
mod fuzz_entry;

use laminapdf_test_fixtures::fixtures_dir;

fn replay(dir: &str) {
    let Ok(files) = std::fs::read_dir(fixtures_dir().join(dir)) else {
        return;
    };
    for file in files {
        let path = file.unwrap().path();
        if path.is_file() {
            fuzz_entry::markdown(&std::fs::read(path).unwrap());
        }
    }
}

#[test]
fn every_saved_crash_input_replays_without_a_panic() {
    replay("regressions/markdown");
}

#[test]
fn every_markdown_fixture_and_seed_keeps_the_invariants() {
    replay("markdown");
    replay("../fuzz/seeds/markdown");
}
