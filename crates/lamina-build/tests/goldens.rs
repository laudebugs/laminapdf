//! ADR 0018 tier 3: every golden `lamina-build` owns is rebuilt byte for byte, twice the same,
//! and reads back cleanly: lamina's reader and model open it without warnings and
//! `lamina-content` interprets every page without content warnings.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::samples::GOLDENS;
use common::{fixture, record, reopen};

#[test]
fn every_golden_is_rebuilt_byte_for_byte() {
    let stale: Vec<&str> = GOLDENS
        .iter()
        .filter(|(name, build)| build() != fixture(&format!("golden/{name}")))
        .map(|(name, _)| *name)
        .collect();
    assert!(
        stale.is_empty(),
        "stale goldens (run `cargo xtask golden --update`): {stale:?}"
    );
}

#[test]
fn every_golden_is_the_same_on_a_second_build() {
    for (name, build) in GOLDENS {
        assert!(build() == build(), "{name}");
    }
}

#[test]
fn every_golden_opens_and_interprets_without_warnings() {
    for (name, _) in GOLDENS {
        let doc = reopen(&fixture(&format!("golden/{name}")));
        let pages = doc.pages().unwrap().len();
        let content: Vec<&str> = (0..pages).flat_map(|i| record(&doc, i).warnings).collect();
        for i in 0..pages {
            doc.pages().unwrap().get(i).unwrap().frame().unwrap();
        }
        let codes: Vec<&str> = doc.warnings().iter().map(lamina_cos::Warning::code).collect();
        assert_eq!((codes, content), (vec![], vec![]), "{name}");
    }
}

#[test]
fn imaging_demo_content_matches_the_fixture_stream() {
    let golden = fixture("golden/imaging-demo.pdf");
    let doc = lamina_cos::Document::open(golden, lamina_cos::OpenOptions::default()).unwrap();
    let page = doc.pages().unwrap().remove(0);
    assert_eq!(doc.page_content(&page).unwrap(), common::legacy::imaging_demo_content());
}
