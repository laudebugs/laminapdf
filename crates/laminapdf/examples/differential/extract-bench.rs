//! Lamina PDF's Markdown for every PDF in a list: Lamina PDF's side of the extraction benchmark
//! (`cargo xtask extract-bench`, todo L3f), which scores the files with the benchmark's own
//! tests (`tools/bench/olmocr_score.py`).
//!
//! The list has one `<out.md>\t<pdf>` line per file, the `<name>\t<path>` lines of xtask's
//! differential runner (`tools/xtask/src/runner.rs`), which runs it in chunks under time limits
//! and reruns a chunk that crashes or hangs one file at a time. Each PDF is extracted with the
//! defaults (`--tables lattice-and-stream` also looks for tables without lines) and written as
//! Markdown without page markers; a file that fails to open or extract, or panics, gets an
//! empty Markdown file, and its outcome goes in the report (`facts`: `{outcome, error?,
//! millis, statuses?}` per Markdown file, `statuses` being each page's text-layer status), so
//! one bad file costs one file's tests. It is not one of the
//! façade's golden examples (`examples/*.rs`), so it lives in a subfolder with its own `path`
//! in `Cargo.toml`.
//!
//! ```sh
//! cargo run --release -p laminapdf --example extract-bench -- [--tables lattice-and-stream] <list.tsv> <out.json>
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use laminapdf::extract::{ExtractOptions, MarkdownOptions, TableMode};
use laminapdf::{Document, OpenOptions};
use serde_json::{Map, Value, json};

/// The Markdown of `bytes` and each page's text-layer status, or the error code that stopped it.
fn markdown(bytes: Vec<u8>, options: &ExtractOptions) -> Result<(String, Vec<&'static str>), &'static str> {
    let doc = Document::open_bytes(bytes, OpenOptions::default()).map_err(|e| e.error_code().as_str())?;
    let extraction = doc.extract(options).map_err(|e| e.error_code().as_str())?;
    let statuses = extraction.pages.iter().map(|p| p.text_layer.status.as_str()).collect();
    Ok((extraction.to_markdown(&MarkdownOptions::new()).text, statuses))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (list, summary, tables) = match args.as_slice() {
        [list, summary] => (list, summary, TableMode::Lattice),
        [flag, mode, list, summary] if flag == "--tables" && mode == "lattice-and-stream" => {
            (list, summary, TableMode::LatticeAndStream)
        }
        _ => panic!("usage: extract-bench [--tables lattice-and-stream] <list.tsv> <out.json>"),
    };
    let options = ExtractOptions::new().with_tables(tables);
    let list = std::fs::read_to_string(list).unwrap();
    let mut all = Map::new();
    // The panic message still goes to stderr; the run carries on with the next file.
    for line in list.lines().filter(|l| !l.is_empty()) {
        let (out, pdf) = line.split_once('\t').expect("<out.md>\\t<pdf> lines");
        let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("reading {pdf}: {e}"));
        let start = Instant::now();
        let result = catch_unwind(AssertUnwindSafe(|| markdown(bytes, &options)));
        let millis = start.elapsed().as_millis();
        let (text, outcome) = match result {
            Ok(Ok((text, statuses))) => (
                text,
                json!({"outcome": "extracted", "millis": millis, "statuses": statuses}),
            ),
            Ok(Err(code)) => (
                String::new(),
                json!({"outcome": "failed", "error": code, "millis": millis}),
            ),
            Err(_) => (String::new(), json!({"outcome": "panic", "millis": millis})),
        };
        if let Some(dir) = std::path::Path::new(out).parent() {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(out, text).unwrap();
        all.insert(out.to_owned(), outcome);
    }
    let report = json!({
        "tool": "laminapdf",
        "version": env!("CARGO_PKG_VERSION"),
        "tables": if tables == TableMode::LatticeAndStream { "lattice-and-stream" } else { "lattice" },
        "reports_repair": false,
        "facts": Value::Object(all),
    });
    std::fs::write(summary, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
}
