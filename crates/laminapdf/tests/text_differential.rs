//! The text differential (todo B9c; ADR 0018 tier 6): Lamina PDF's `page_texts` against pdf.js and
//! pdftotext, page by page, on `fixtures/corpus` and the text goldens. Their output is
//! precomputed in `fixtures/differential/text.json` (`cargo xtask differential --text --update`,
//! `tools/differential/text.mjs`); this test needs neither tool.
//!
//! **Comparison rule:** whitespace is removed on both sides before comparing (pdf.js, pdftotext
//! and Lamina PDF each insert spaces and line breaks by their own heuristics), so a page agrees when
//! the same characters come out in the same order. Every disagreement must be listed in
//! `fixtures/differential/text-exceptions.json` with a reason, and every listed one must still
//! disagree (a fixed one is removed from the list).
#![cfg(feature = "extract")]
#![allow(clippy::unwrap_used, reason = "test code: a panic is a test failure")]

use std::path::{Path, PathBuf};

use laminapdf::extract::TextOptions;
use laminapdf::{Document, OpenOptions};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(rel: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(rel)).unwrap()).unwrap()
}

/// The text with every whitespace character and bidi control (U+200E, U+200F, U+202A–U+202E,
/// U+2066–U+2069; pdftotext wraps right-to-left runs in embeddings) removed.
fn squeeze(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
        .collect()
}

/// Lamina PDF's pages, squeezed; none when Lamina PDF can't extract the file (every page then
/// disagrees, and must be listed).
fn lamina_pages(rel: &str) -> Vec<String> {
    let bytes = std::fs::read(root().join(rel)).unwrap();
    let Ok(doc) = Document::open_bytes(bytes, OpenOptions::default()) else {
        return Vec::new();
    };
    doc.page_texts(&TextOptions::new())
        .map(|pages| pages.iter().map(|p| squeeze(&p.lines().join("\n"))).collect())
        .unwrap_or_default()
}

/// capability: H.07 core read
#[test]
fn page_text_agrees_with_pdfjs_and_pdftotext() {
    let expected = json("fixtures/differential/text.json");
    let exceptions = json("fixtures/differential/text-exceptions.json");
    let listed: Vec<(String, u64, String)> = exceptions["exceptions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            assert!(
                e["reason"].as_str().is_some_and(|r| !r.is_empty()),
                "every exception has a reason: {e}"
            );
            (
                e["file"].as_str().unwrap().to_owned(),
                e["page"].as_u64().unwrap(),
                e["tool"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let mut unexplained = Vec::new();
    let mut disagreeing = Vec::new();
    for (file, tools) in expected["files"].as_object().unwrap() {
        let ours = lamina_pages(file);
        for tool in ["pdfjs", "pdftotext"] {
            // A tool that failed on the file (null) is a disagreement too: listed with page 0.
            let Some(theirs) = tools[tool].as_array() else {
                let key = (file.clone(), 0, tool.to_owned());
                disagreeing.push(key.clone());
                if !listed.contains(&key) {
                    unexplained.push(format!("{file}: {tool} read nothing (null in text.json)"));
                }
                continue;
            };
            let theirs: Vec<String> = theirs.iter().map(|p| squeeze(p.as_str().unwrap())).collect();
            if ours.len() > theirs.len() {
                unexplained.push(format!(
                    "{file}: lamina has {} pages, {tool} {}",
                    ours.len(),
                    theirs.len()
                ));
            }
            for (i, theirs) in theirs.iter().enumerate() {
                let page = i as u64;
                if ours.get(i) == Some(theirs) {
                    continue;
                }
                let key = (file.clone(), page, tool.to_owned());
                disagreeing.push(key.clone());
                if !listed.contains(&key) {
                    unexplained.push(format!(
                        "{file} page {page} vs {tool}:\n  lamina: {:?}\n  {tool}: {theirs:?}",
                        ours.get(i).map(|s| s.chars().take(200).collect::<String>())
                    ));
                }
            }
        }
    }
    let stale: Vec<_> = listed.iter().filter(|l| !disagreeing.contains(l)).collect();
    assert!(
        stale.is_empty(),
        "exceptions that no longer disagree (remove them): {stale:?}"
    );
    assert!(unexplained.is_empty(), "{}", unexplained.join("\n"));
}
