//! ADR 0018 tier 9: every minimised crash of the `extract` fuzz target in
//! `fixtures/regressions/extract/` is replayed through the target's entry point
//! (`common::fuzz_entry::extract`), so a fixed crash stays fixed; so is every fixture PDF,
//! the target's seeds. The `extract_images` target's are replayed the same way.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::fuzz_entry::{extract, extract_images};
use laminapdf_test_fixtures::fixtures_dir;

fn replay(dir: &str) {
    replay_with(dir, extract);
}

fn replay_with(dir: &str, entry: fn(&[u8])) {
    let Ok(files) = std::fs::read_dir(fixtures_dir().join(dir)) else {
        return;
    };
    for file in files {
        let path = file.unwrap().path();
        if path.is_file() {
            entry(&std::fs::read(path).unwrap());
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

#[test]
fn every_saved_image_crash_input_and_fixture_keeps_the_image_invariants() {
    replay_with("regressions/extract_images", extract_images);
    replay_with("corpus", extract_images);
    replay_with("images", extract_images);
}

#[test]
fn every_filter_colour_space_and_depth_keeps_the_image_invariants() {
    // The entry's first six bytes choose the image's dictionary; the rest is its stream.
    let patterns: [&[u8]; 4] = [b"", &[0xff; 64], b"\x00\x11\x22\x33\x44\x55\x66\x77\x88\x99", b"~>\x80"];
    for filter in 0..8u8 {
        for space in 0..6u8 {
            for bits in 0..5u8 {
                for (i, pattern) in patterns.iter().enumerate() {
                    let size = u8::try_from(i * 7).unwrap();
                    let input = [&[filter, space, bits, size, size, u8::from(i % 2 == 1)][..], pattern].concat();
                    extract_images(&input);
                }
            }
        }
    }
}
