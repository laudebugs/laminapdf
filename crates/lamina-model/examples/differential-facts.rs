//! lamina's facts (outcome, page count, outline titles) for every file in a list, as JSON:
//! lamina's side of the nightly lane's live differential (`cargo xtask differential --live`),
//! computed by the same code as the PR lane's `tests/differential.rs`.
//!
//! The list has one `<name>\t<path>` line per file; the output maps each name to its facts.
//! A panic while reading a file is recorded as the outcome `panic` (a bug to report), not a
//! crash of the whole run.
//!
//! ```sh
//! cargo run --release -p lamina-model --example differential-facts -- <list.tsv> <out.json>
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

#[path = "../tests/common/mod.rs"]
mod common;

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::{Map, Value, json};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [list, out] = args.as_slice() else {
        panic!("usage: differential-facts <list.tsv> <out.json>");
    };
    let list = std::fs::read_to_string(list).unwrap();
    // The panic message still goes to stderr; the run carries on with the next file.
    let mut facts = Map::new();
    for line in list.lines().filter(|l| !l.is_empty()) {
        let (name, path) = line.split_once('\t').expect("<name>\\t<path> lines");
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        let value = catch_unwind(AssertUnwindSafe(|| common::differential::facts(&bytes)))
            .unwrap_or_else(|_| json!({"outcome": "panic", "page_count": null, "outline_titles": null}));
        facts.insert(name.to_owned(), value);
    }
    let report = json!({
        "tool": "lamina",
        "version": env!("CARGO_PKG_VERSION"),
        "reports_repair": true,
        "facts": Value::Object(facts),
    });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
}
