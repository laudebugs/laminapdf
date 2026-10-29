//! The lamina-only half of the round-trip suite (ADR 0018 tier 7; todo B3c), in the fast lane:
//! every fixture × proving op × mode × overrides saves (or is refused) as expected and reopens
//! with the expected facts. The full lane also checks the outputs with qpdf, pdf.js and `PDFium`
//! (`cargo xtask roundtrip`).

mod common;

#[test]
fn every_case_saves_or_refuses_as_expected_and_reopens_in_lamina() {
    let suite = common::roundtrip::run(&common::fixtures_dir().join(".."));
    assert_eq!(suite.failures, Vec::<String>::new());
    assert!(
        suite.saved > 300 && suite.refused > 100,
        "{} saved, {} refused",
        suite.saved,
        suite.refused
    );
}
