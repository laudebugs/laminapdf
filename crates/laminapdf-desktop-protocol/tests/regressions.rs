//! ADR 0018 tier 9: every minimised crash of the `engine_protocol_decode` fuzz target in
//! `fixtures/regressions/engine_protocol_decode/` is replayed through the target's entry point,
//! so a fixed crash stays fixed; so are `fuzz/seeds/engine_protocol_decode/`, the target's seeds.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

#[expect(
    unreachable_pub,
    reason = "the fuzz workspace calls the entry point as a public item"
)]
#[path = "common/fuzz_entry.rs"]
mod fuzz_entry;

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn replay(dir: &Path) -> usize {
    let Ok(files) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0;
    for file in files {
        let path = file.unwrap().path();
        if path.is_file() {
            fuzz_entry::engine_protocol_decode(&std::fs::read(path).unwrap());
            count += 1;
        }
    }
    count
}

#[test]
fn every_saved_crash_input_replays_without_a_panic() {
    replay(&root().join("fixtures/regressions/engine_protocol_decode"));
}

#[test]
fn every_seed_replays_without_a_panic() {
    assert!(
        replay(&root().join("fuzz/seeds/engine_protocol_decode")) > 0,
        "no seeds"
    );
}
