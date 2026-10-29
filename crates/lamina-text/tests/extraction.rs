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
    for (name, _) in common::samples::GOLDENS {
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
    for (name, build) in common::samples::GOLDENS {
        let extracted: Vec<String> = facts["files"][name]["pypdf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(extracted, build().lines, "{name}");
    }
}

#[test]
fn qpdf_check_passes() {
    let facts = facts();
    for (name, _) in common::samples::GOLDENS {
        assert_eq!(facts["files"][name]["qpdf_check"].as_bool(), Some(true), "{name}");
    }
}
