//! Text extraction by independent tools (todos B4a, B4b, B4d): `fixtures/text/extracted.json`
//! holds what pypdf, pdf.js and pdftotext extract from each text golden and whether
//! `qpdf --check` passes,
//! written by `tools/fixtures/text_facts.py`. The facts must describe the goldens' current
//! bytes, and each extractor's text must equal the text each sample drew, except the lines in
//! `fixtures/text/extraction-exceptions.json` (an extractor's known limitation, each with its
//! reason; an entry that no longer matches the facts fails too).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure (serde_json `Value` indexing returns `Null` when absent)"
)]

mod common;

use sha2::{Digest, Sha256};

/// The extractors with facts.
const EXTRACTORS: [&str; 3] = ["pypdf", "pdfjs", "pdftotext"];

fn facts() -> serde_json::Value {
    serde_json::from_slice(&laminapdf_test_fixtures::fixture("text/extracted.json")).unwrap()
}

#[test]
fn facts_describe_the_current_goldens() {
    let facts = facts();
    for (name, _, _) in common::samples::GOLDENS {
        let digest = laminapdf_test_fixtures::hex(&Sha256::digest(laminapdf_test_fixtures::fixture(&format!(
            "golden/{name}"
        ))));
        assert_eq!(
            facts["files"][name]["sha256"].as_str(),
            Some(digest.as_str()),
            "{name}: rerun tools/fixtures/text_facts.py"
        );
    }
}

/// `(file, extractor, line) → what the extractor returns instead`, from the exceptions file.
fn exceptions() -> Vec<((String, String, usize), String)> {
    let file: serde_json::Value =
        serde_json::from_slice(&laminapdf_test_fixtures::fixture("text/extraction-exceptions.json")).unwrap();
    file["exceptions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            assert!(!e["reason"].as_str().unwrap().is_empty(), "{e}");
            (
                (
                    e["file"].as_str().unwrap().to_owned(),
                    e["extractor"].as_str().unwrap().to_owned(),
                    usize::try_from(e["line"].as_u64().unwrap()).unwrap(),
                ),
                e["extracted"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn check_extractor(extractor: &str) {
    let facts = facts();
    let excused = exceptions();
    for (name, _, lines) in common::samples::GOLDENS {
        let extracted: Vec<String> = facts["files"][name][extractor]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: no {extractor} facts; rerun tools/fixtures/text_facts.py"))
            .iter()
            .map(|l| l.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(extracted.len(), lines.len(), "{name}: {extractor} found {extracted:?}");
        for (i, (got, want)) in extracted.iter().zip(lines.iter()).enumerate() {
            let key = ((*name).to_owned(), extractor.to_owned(), i);
            match excused.iter().find(|(k, _)| *k == key) {
                Some((_, instead)) => assert_eq!(got, instead, "{name} line {i}: exception is stale for {extractor}"),
                None => assert_eq!(got, want, "{name} line {i}: {extractor}"),
            }
        }
    }
}

#[test]
fn pypdf_extracts_the_input_text() {
    check_extractor("pypdf");
}

#[test]
fn pdfjs_extracts_the_input_text() {
    check_extractor("pdfjs");
}

#[test]
fn pdftotext_extracts_the_input_text() {
    check_extractor("pdftotext");
}

#[test]
fn every_exception_names_a_golden_line() {
    for ((file, extractor, line), _) in exceptions() {
        let lines = common::samples::GOLDENS
            .iter()
            .find(|(name, _, _)| *name == file)
            .map_or_else(|| panic!("{file} is not a text golden"), |(_, _, lines)| lines.len());
        assert!(line < lines, "{file} has no line {line}");
        assert!(EXTRACTORS.contains(&extractor.as_str()), "{extractor}");
    }
}

#[test]
fn exceptions_are_limited_to_known_extractor_bugs() {
    // B4d: one CID per (glyph, text) made every shaped line exact in pypdf and pdf.js except
    // the right-to-left line of several runs (4: pypdf's TJ spaces, pdf.js's item spaces)
    // and, for pdf.js, the Devanagari line (5: its zero-width test). pdftotext writes
    // right-to-left text in display order (3, 4, 7) and drops the ZapfDingbats spaces.
    let allowed = [
        ("text-shaped.pdf", "pypdf", 4),
        ("text-shaped.pdf", "pdfjs", 4),
        ("text-shaped.pdf", "pdfjs", 5),
        ("text-shaped.pdf", "pdftotext", 3),
        ("text-shaped.pdf", "pdftotext", 4),
        ("text-shaped.pdf", "pdftotext", 7),
        ("text-std14.pdf", "pdftotext", 13),
    ];
    for ((file, extractor, line), _) in exceptions() {
        assert!(
            allowed.contains(&(file.as_str(), extractor.as_str(), line)),
            "{file} {extractor} line {line}"
        );
    }
}

#[test]
fn every_exception_links_upstream() {
    let file: serde_json::Value =
        serde_json::from_slice(&laminapdf_test_fixtures::fixture("text/extraction-exceptions.json")).unwrap();
    for e in file["exceptions"].as_array().unwrap() {
        assert!(e["upstream"].as_str().unwrap().starts_with("https://"), "{e}");
    }
}

#[test]
fn qpdf_check_passes() {
    let facts = facts();
    for (name, _, _) in common::samples::GOLDENS {
        assert_eq!(facts["files"][name]["qpdf_check"].as_bool(), Some(true), "{name}");
    }
}

/// The version `pin` is set to in `text` (`name==1.2.3` or `NAME: &str = "1.2.3";`).
fn pinned(text: &str, pin: &str) -> String {
    text.lines()
        .find_map(|l| l.trim().strip_prefix(pin))
        .map(|rest| rest.trim_matches(|c| c == '"' || c == ';' || c == ' ').to_owned())
        .unwrap()
}

#[test]
fn facts_were_written_with_the_pinned_pypdf() {
    let requirements =
        String::from_utf8(laminapdf_test_fixtures::fixture("../tools/fixtures/requirements.txt")).unwrap();
    assert_eq!(
        facts()["pypdf_version"].as_str(),
        Some(pinned(&requirements, "pypdf==").as_str()),
        "fixtures/text/extracted.json was written with another pypdf than tools/fixtures/requirements.txt pins; rerun tools/fixtures/text_facts.py"
    );
}

#[test]
fn facts_were_written_with_the_pinned_qpdf() {
    let doctor = String::from_utf8(laminapdf_test_fixtures::fixture("../tools/xtask/src/doctor.rs")).unwrap();
    assert_eq!(
        facts()["qpdf_version"].as_str(),
        Some(pinned(&doctor, "pub(crate) const QPDF_VERSION: &str =").as_str()),
        "fixtures/text/extracted.json was written with another qpdf than tools/xtask/src/doctor.rs pins; rerun tools/fixtures/text_facts.py"
    );
}

#[test]
fn facts_were_written_with_the_pinned_poppler() {
    let doctor = String::from_utf8(laminapdf_test_fixtures::fixture("../tools/xtask/src/doctor.rs")).unwrap();
    assert_eq!(
        facts()["poppler_version"].as_str(),
        Some(pinned(&doctor, "pub(crate) const POPPLER_VERSION: &str =").as_str()),
        "fixtures/text/extracted.json was written with another pdftotext than tools/xtask/src/doctor.rs pins; rerun tools/fixtures/text_facts.py"
    );
}

#[test]
fn pdfjs_scripts_pin_the_doctor_version() {
    let doctor = String::from_utf8(laminapdf_test_fixtures::fixture("../tools/xtask/src/doctor.rs")).unwrap();
    let pinned_version = pinned(&doctor, "const PDFJS_DIST_VERSION: &str =");
    for script in [
        "../tools/fixtures/pdfjs_text.mjs",
        "../tools/fixtures/pdfjs_build.mjs",
        "../tools/fixtures/pdfjs_structure.mjs",
        "../tools/corpus-facts/pdfjs-operators.mjs",
        "../tools/roundtrip/pdfjs.mjs",
    ] {
        let text = String::from_utf8(laminapdf_test_fixtures::fixture(script)).unwrap();
        assert_eq!(pinned(&text, "const PINNED ="), pinned_version, "{script}");
    }
}

#[test]
fn facts_were_written_with_the_pinned_pdfjs() {
    let script = String::from_utf8(laminapdf_test_fixtures::fixture("../tools/fixtures/pdfjs_text.mjs")).unwrap();
    assert_eq!(
        facts()["pdfjs_version"].as_str(),
        Some(pinned(&script, "const PINNED =").as_str()),
        "fixtures/text/extracted.json was written with another pdf.js than tools/fixtures/pdfjs_text.mjs pins"
    );
}
