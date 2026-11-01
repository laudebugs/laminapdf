//! ADR 0018 tier 9: every minimised fuzz crash in `fixtures/regressions/<target>/` is
//! replayed through the same entry point its fuzz target calls (`common::fuzz_entry`), so a
//! fixed crash stays fixed. Inputs are raw bytes, exactly what the fuzzer fed the target.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::fixtures_dir;
use common::fuzz_entry::{FLATE_SEED_PARAMS, TARGETS, flate, page_tree, wrap_page_tree};

#[test]
fn every_saved_crash_input_replays_without_a_panic() {
    let root = fixtures_dir().join("regressions");
    for (target, entry) in TARGETS {
        let dir = root.join(target);
        let Ok(files) = std::fs::read_dir(&dir) else { continue };
        for file in files {
            let path = file.unwrap().path();
            entry(&std::fs::read(&path).unwrap());
        }
    }
}

#[test]
fn every_regressions_folder_is_a_fuzz_target() {
    let Ok(dirs) = std::fs::read_dir(fixtures_dir().join("regressions")) else {
        return;
    };
    for dir in dirs {
        let name = dir.unwrap().file_name().to_string_lossy().into_owned();
        // `content` and the save targets (`ops_json`, `save_roundtrip`) are replayed by
        // laminapdf-content's and laminapdf-save's own `tests/regressions.rs`.
        let elsewhere = matches!(
            name.as_str(),
            "content" | "ops_json" | "save_roundtrip" | "extract_text"
        );
        assert!(
            elsewhere || TARGETS.iter().any(|(t, _)| *t == name),
            "fixtures/regressions/{name} is no fuzz target"
        );
    }
}

#[test]
fn the_page_tree_wrapper_reaches_the_intermediate_node_page() {
    let doc = common::open(&wrap_page_tree(b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>")).unwrap();
    let pages = doc.pages().unwrap();
    let inherited = pages.get(1).unwrap().crop_box().unwrap().map(|r| r.to_array());
    assert_eq!((pages.len(), inherited), (2, Some([0.0, 0.0, 10.0, 10.0])));
}

#[test]
fn the_page_tree_entry_accepts_a_tree_body() {
    page_tree(b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>");
}

#[test]
fn a_flate_seed_is_a_stream_followed_by_parameters() {
    let mut seed = laminapdf_syntax::filters::flate_encode(b"hello");
    seed.extend(FLATE_SEED_PARAMS);
    flate(&seed);
}
