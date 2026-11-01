//! ADR 0018 tier 9: every minimised crash of the `content` fuzz target in
//! `fixtures/regressions/content/` is replayed through the target's entry point
//! (`common::fuzz_entry::content`), so a fixed crash stays fixed.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::fixtures_dir;
use common::fuzz_entry::{content, fixture_file};

#[test]
fn every_saved_crash_input_replays_without_a_panic() {
    let Ok(files) = std::fs::read_dir(fixtures_dir().join("regressions/content")) else {
        return;
    };
    for file in files {
        content(&std::fs::read(file.unwrap().path()).unwrap());
    }
}

#[test]
fn the_fixture_file_opens_with_one_page() {
    let doc = laminapdf_model::Document::open(fixture_file(b"q Q"), laminapdf_model::OpenOptions::default()).unwrap();
    assert_eq!(doc.pages().unwrap().len(), 1);
}

const EVERY_RESOURCE: &[u8] =
    b"/F1 9 Tf /Im1 Do /Fm1 Do /Fm2 Do /GS1 gs /Sh1 sh /Pattern cs /P1 scn /CS1 cs /P /MC1 BDC EMC";

#[test]
fn the_entry_runs_every_resource_of_the_fixture() {
    content(EVERY_RESOURCE);
}

#[test]
fn every_resource_of_the_fixture_is_found() {
    let doc =
        laminapdf_model::Document::open(fixture_file(EVERY_RESOURCE), laminapdf_model::OpenOptions::default()).unwrap();
    let mut rec = common::Recorder::default();
    common::run_with(&doc, &mut rec).unwrap();
    // Fm2 draws itself: the one warning the fixture is built to raise.
    assert_eq!(rec.codes(), ["xobject-cycle"]);
}

#[test]
fn the_entry_takes_a_whole_file_too() {
    content(&fixture_file(b"BT (a) Tj ET"));
}
