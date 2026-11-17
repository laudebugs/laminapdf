//! Lamina PDF's objects and fonts for every file in a list, as JSON: Lamina PDF's side of the
//! nightly lane's inspection differential against qpdf and pdffonts (`cargo xtask differential
//! --live --inspect`), in the shape `tests/common/inspect_facts.rs` writes, which the PR lane's
//! `tests/inspect_differential.rs` compares with committed facts too.
//!
//! The list has one `<name>\t<path>` line per file; the output maps each name to
//! `{outcome, error?, objects, xref, fonts, appearance_fonts, annotations_without_appearance}`.
//! `outcome` is `opened`, `repaired` (the cross-reference table was rebuilt, so the object
//! places are Lamina PDF's own) or `failed` (with Lamina PDF's `error` code). A panic is
//! recorded as the outcome `panic` (a bug to report), not a crash of the whole run.
//!
//! The last two facts let the differential's exceptions match what pdffonts reads differently.
//! `appearance_fonts` lists the objects of the fonts reached only through annotations'
//! appearance streams, not through the content, whose fonts the document's properties list.
//! `annotations_without_appearance` lists the subtypes of the annotations that have no normal
//! appearance, which poppler draws itself.
//!
//! It is not one of the façade's golden examples (`examples/*.rs`), so it lives in a subfolder
//! with its own `path` in `Cargo.toml`.
//!
//! ```sh
//! cargo run --release -p laminapdf --example inspect-facts -- <list.tsv> <out.json>
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use laminapdf::{Document, OpenOptions};
use serde_json::{Map, Value, json};

#[path = "../../tests/common/inspect_facts.rs"]
mod inspect_facts;

/// The warning that says the stored xref was rebuilt (ADR 0003 §1: Repair).
const REPAIRED: &str = "xref-repaired";

/// Lamina PDF's inspection facts about one file.
fn facts(bytes: Vec<u8>) -> Value {
    let failed = |e: &laminapdf::Error| json!({"outcome": "failed", "error": e.error_code().as_str()});
    let doc = match Document::open_bytes(bytes, OpenOptions::default()) {
        Ok(doc) => doc,
        Err(e) => return failed(&e),
    };
    let objects = match inspect_facts::objects(&doc) {
        Ok(objects) => objects,
        Err(e) => return failed(&e),
    };
    let fonts = match inspect_facts::fonts(&doc) {
        Ok(fonts) => fonts,
        Err(e) => return failed(&e),
    };
    let repaired = doc
        .warnings()
        .next_batch(u32::MAX)
        .iter()
        .any(|w| w.warning_code().as_str() == REPAIRED);
    let outcome = if repaired { "repaired" } else { "opened" };
    json!({
        "outcome": outcome,
        "objects": objects,
        "xref": inspect_facts::xref(&doc),
        "fonts": fonts,
        "appearance_fonts": appearance_fonts(&doc, &fonts),
        "annotations_without_appearance": annotations_without_appearance(&doc),
    })
}

/// The subtypes of the annotations on every page that have no normal appearance, each once;
/// none when the annotations can't be read.
fn annotations_without_appearance(doc: &Document) -> Value {
    let mut subtypes: Vec<String> = (0..doc.page_count().unwrap_or(0))
        .filter_map(|index| doc.page_annotations(index).ok())
        .flatten()
        .filter(|annot| !annot.has_appearance)
        .map(|annot| annot.subtype)
        .collect();
    subtypes.sort();
    subtypes.dedup();
    json!(subtypes)
}

/// The objects of `fonts` (every page's) that the pages' content doesn't reach; none when the
/// properties can't be read, so nothing is excused by them.
fn appearance_fonts(doc: &Document, fonts: &Value) -> Value {
    let Ok(properties) = doc.properties() else {
        return json!([]);
    };
    let content: Vec<String> = properties
        .fonts
        .iter()
        .filter_map(|f| f.object.map(|r| format!("{} {}", r.num, r.gen_nr)))
        .collect();
    let objects = fonts
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f["object"].as_str());
    objects.filter(|o| !content.iter().any(|c| c == o)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [list, out] = args.as_slice() else {
        panic!("usage: inspect-facts <list.tsv> <out.json>");
    };
    let list = std::fs::read_to_string(list).unwrap();
    // The panic message still goes to stderr; the run carries on with the next file.
    let mut all = Map::new();
    for line in list.lines().filter(|l| !l.is_empty()) {
        let (name, path) = line.split_once('\t').expect("<name>\\t<path> lines");
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        let value = catch_unwind(AssertUnwindSafe(|| facts(bytes))).unwrap_or_else(|_| json!({"outcome": "panic"}));
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
