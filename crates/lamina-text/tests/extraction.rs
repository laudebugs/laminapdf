//! Text extraction by independent tools (todo B4a): `fixtures/text/extracted.json` holds what
//! pypdf extracts from each text golden and whether `qpdf --check` passes, written by
//! `tools/fixtures/text_facts.py`. The facts must describe the goldens' current bytes, and
//! pypdf's text must equal the text each sample drew.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure (serde_json `Value` indexing returns `Null` when absent)"
)]

mod common;

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

fn facts() -> serde_json::Value {
    serde_json::from_slice(&common::fixture("text/extracted.json")).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[test]
fn facts_describe_the_current_goldens() {
    let facts = facts();
    for (name, _, _) in common::samples::GOLDENS {
        let digest = hex(&Sha256::digest(common::fixture(&format!("golden/{name}"))));
        assert_eq!(
            facts["files"][name]["sha256"].as_str(),
            Some(digest.as_str()),
            "{name}: rerun tools/fixtures/text_facts.py"
        );
    }
}

#[test]
fn pypdf_extracts_the_input_text() {
    let facts = facts();
    for (name, _, lines) in common::samples::GOLDENS {
        let extracted: Vec<String> = facts["files"][name]["pypdf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(extracted, lines, "{name}");
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
    let requirements = String::from_utf8(common::fixture("../tools/fixtures/requirements.txt")).unwrap();
    assert_eq!(
        facts()["pypdf_version"].as_str(),
        Some(pinned(&requirements, "pypdf==").as_str()),
        "fixtures/text/extracted.json was written with another pypdf than tools/fixtures/requirements.txt pins; rerun tools/fixtures/text_facts.py"
    );
}

#[test]
fn facts_were_written_with_the_pinned_qpdf() {
    let doctor = String::from_utf8(common::fixture("../tools/xtask/src/doctor.rs")).unwrap();
    assert_eq!(
        facts()["qpdf_version"].as_str(),
        Some(pinned(&doctor, "pub(crate) const QPDF_VERSION: &str =").as_str()),
        "fixtures/text/extracted.json was written with another qpdf than tools/xtask/src/doctor.rs pins; rerun tools/fixtures/text_facts.py"
    );
}
