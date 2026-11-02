//! ADR 0018 tier 3: every golden `laminapdf-build` owns is rebuilt byte for byte and twice the
//! same. Those goldens and the frozen `hello.pdf` and `imaging-demo.pdf` read back cleanly:
//! Lamina PDF's reader and model open them without warnings, and `laminapdf-content`
//! interprets every page without content warnings.
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

/// The frozen goldens `laminapdf-save` builds (its `tests/common/legacy.rs`), read back here too.
const READ_BACK_ONLY: [&str; 2] = ["hello.pdf", "imaging-demo.pdf"];

#[test]
fn every_golden_opens_and_interprets_without_warnings() {
    for name in GOLDENS.iter().map(|(name, _)| *name).chain(READ_BACK_ONLY) {
        let doc = reopen(&fixture(&format!("golden/{name}")));
        let pages = doc.pages().unwrap().len();
        let content: Vec<&str> = (0..pages).flat_map(|i| record(&doc, i).warnings).collect();
        for i in 0..pages {
            doc.pages().unwrap().get(i).unwrap().frame().unwrap();
        }
        let codes: Vec<&str> = doc.warnings().iter().map(laminapdf_cos::Warning::code).collect();
        assert_eq!((codes, content), (vec![], vec![]), "{name}");
    }
}
