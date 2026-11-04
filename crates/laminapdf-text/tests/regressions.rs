//! ADR 0018 tier 9: every minimised crash of the `extract_text` fuzz target in
//! `fixtures/regressions/extract_text/` is replayed through the target's entry point
//! (`common::fuzz_entry::extract_text`), so a fixed crash stays fixed.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::fuzz_entry::{extract_text, fixture_file};

use laminapdf_test_fixtures::fixtures_dir;

#[test]
fn every_saved_crash_input_replays_without_a_panic() {
    let Ok(files) = std::fs::read_dir(fixtures_dir().join("regressions/extract_text")) else {
        return;
    };
    for file in files {
        extract_text(&std::fs::read(file.unwrap().path()).unwrap());
    }
}

#[test]
fn the_fixture_file_extracts_both_fonts() {
    let cmap = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap
1 begincodespacerange <0000> <FFFF> endcodespacerange
1 beginbfchar <0001> <0041> endbfchar endcmap CMapName currentdict /CMap defineresource pop end end
BT /F1 12 Tf 72 700 Td (AB) Tj /F2 12 Tf <0001> Tj ET";
    let doc = laminapdf_model::Document::open(fixture_file(cmap), laminapdf_model::OpenOptions::default()).unwrap();
    let pages = doc.pages().unwrap();
    let text = laminapdf_text::TextExtractor::new()
        .page(&doc, &pages.get(0).unwrap())
        .unwrap();
    assert_eq!(text.lines(), ["\u{391}fiA"]);
}

#[test]
fn whole_files_and_other_bytes_run_through_the_entry() {
    extract_text(&std::fs::read(fixtures_dir().join("golden/text-shaped.pdf")).unwrap());
    extract_text(b"BT /F2 9 Tf <00010002> Tj /F1 9 Tf (A) ' ET");
    extract_text(b"");
}
