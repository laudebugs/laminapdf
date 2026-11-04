//! Byte-exact goldens (ADR 0018 tier 3): `fixtures/golden/text-*.pdf` are rebuilt by the
//! samples in `tests/common/samples.rs`; `cargo xtask golden --update` regenerates them.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

#[test]
fn text_goldens_are_byte_identical() {
    for (name, build, _) in common::samples::GOLDENS {
        assert!(
            build().bytes == laminapdf_test_fixtures::fixture(&format!("golden/{name}")),
            "{name} differs; run `cargo xtask golden --update`"
        );
    }
}
