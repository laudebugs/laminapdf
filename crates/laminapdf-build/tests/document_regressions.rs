//! ADR 0018 tier 9: the `document` fuzz target's seeds (`fuzz/seeds/document/`) and any saved
//! crash input (`fixtures/regressions/document/`) replay through its entry point
//! (`document_fuzz_entry::document`) without a panic.
#![cfg(feature = "serde")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

#[expect(
    unreachable_pub,
    reason = "the fuzz workspace calls the entry point as a public item"
)]
#[path = "common/document_fuzz_entry.rs"]
mod document_fuzz_entry;

use laminapdf_test_fixtures::fixtures_dir;

fn replay(dir: &std::path::Path) -> usize {
    let Ok(files) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0;
    for file in files {
        let path = file.unwrap().path();
        if path.is_file() {
            document_fuzz_entry::document(&std::fs::read(path).unwrap());
            count += 1;
        }
    }
    count
}

#[test]
fn every_seed_and_saved_crash_input_replays_without_a_panic() {
    replay(&fixtures_dir().join("regressions/document"));
    assert!(replay(&fixtures_dir().join("../fuzz/seeds/document")) > 0, "the seeds are there");
}
