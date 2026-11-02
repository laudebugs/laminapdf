//! Lamina PDF's page text for every file in a list, as JSON: Lamina PDF's side of the nightly lane's
//! live text differential (`cargo xtask differential --live --text`, todo B9f), extracted as
//! the PR lane's `tests/text_differential.rs` does (`page_texts`, lines joined by newlines).
//!
//! The list has one `<name>\t<path>` line per file; the output maps each name to
//! `{outcome, error?, pages, page_errors?, warnings}`: `outcome` is `opened`, `repaired` or
//! `failed` (with Lamina PDF's `error` code), `pages` holds each page's text as extracted, or
//! `null` for a page whose extraction failed (its code is in `page_errors`, by page index),
//! and `warnings` lists the distinct warning codes raised. xtask removes whitespace before
//! comparing. A panic is recorded as the outcome `panic` (a bug to report), not a crash of
//! the whole run. It is not one of the façade's golden examples (`examples/*.rs`), so it lives
//! in a subfolder with its own `path` in `Cargo.toml`.
//!
//! ```sh
//! cargo run --release -p laminapdf --example text-facts -- <list.tsv> <out.json>
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

use laminapdf::extract::TextOptions;
use laminapdf::{Document, OpenOptions};
use serde_json::{Map, Value, json};

/// The warning that says the stored xref was rebuilt (ADR 0003 §1: Repair).
const REPAIRED: &str = "xref-repaired";

/// Lamina PDF's text facts about one file.
fn facts(bytes: Vec<u8>) -> Value {
    let doc = match Document::open_bytes(bytes, OpenOptions::new()) {
        Ok(doc) => doc,
        Err(e) => return json!({"outcome": "failed", "error": e.error_code().as_str(), "pages": null}),
    };
    let options = TextOptions::new();
    let mut page_errors = Map::new();
    let pages: Vec<Value> = if let Ok(pages) = doc.page_texts(&options) {
        pages.iter().map(|p| json!(p.lines().join("\n"))).collect()
    } else {
        // One page failed: read them one by one to find which (each call walks the page tree,
        // so only on this path). The count comes from the same walk `page_text` does: asking
        // for a page past the end answers with it, where `page_count` would also build every
        // page's facts (boxes, rotation) and fail on one bad page.
        let count = match doc.page_text(u32::MAX, &options) {
            Err(laminapdf::Error::PageIndexOutOfRange { count, .. }) => count,
            Err(e) => return json!({"outcome": "failed", "error": e.error_code().as_str(), "pages": null}),
            Ok(_) => panic!("a page at index u32::MAX"),
        };
        (0..count)
            .map(|i| match doc.page_text(i, &options) {
                Ok(page) => json!(page.lines().join("\n")),
                Err(e) => {
                    page_errors.insert(i.to_string(), json!(e.error_code().as_str()));
                    Value::Null
                }
            })
            .collect()
    };
    let mut cursor = doc.warnings();
    let warnings: BTreeSet<&'static str> = cursor
        .next_batch(u32::MAX)
        .iter()
        .map(|w| w.warning_code().as_str())
        .collect();
    let outcome = if warnings.contains(REPAIRED) {
        "repaired"
    } else {
        "opened"
    };
    let mut out = Map::new();
    out.insert("outcome".to_owned(), json!(outcome));
    out.insert("pages".to_owned(), Value::Array(pages));
    out.insert("warnings".to_owned(), json!(warnings));
    if !page_errors.is_empty() {
        out.insert("page_errors".to_owned(), Value::Object(page_errors));
    }
    Value::Object(out)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [list, out] = args.as_slice() else {
        panic!("usage: text-facts <list.tsv> <out.json>");
    };
    let list = std::fs::read_to_string(list).unwrap();
    // The panic message still goes to stderr; the run carries on with the next file.
    let mut all = Map::new();
    for line in list.lines().filter(|l| !l.is_empty()) {
        let (name, path) = line.split_once('\t').expect("<name>\\t<path> lines");
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        let value = catch_unwind(AssertUnwindSafe(|| facts(bytes)))
            .unwrap_or_else(|_| json!({"outcome": "panic", "pages": null}));
        all.insert(name.to_owned(), value);
    }
    let report = json!({
        "tool": "lamina",
        "version": env!("CARGO_PKG_VERSION"),
        "reports_repair": true,
        "facts": Value::Object(all),
    });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
}
