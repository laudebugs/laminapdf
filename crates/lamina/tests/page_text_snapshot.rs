//! The shared `page.text()` snapshot (todo B9c): `fixtures/text/page-text.json` holds what
//! `Document::page_text` returns for the text fixtures, and `@lamina/pdf`'s
//! `test/text.test.ts` builds the same JSON through the WASM and compares it with the same file,
//! so Rust and TypeScript give the same text, runs and quads.
//!
//! Numbers are rounded the same way on both sides: quads go through `f32` (the WASM's
//! `Float32Array`) and then to 0.01 pt; bounds to 0.01 pt; sizes to 0.001.
//! `cargo xtask golden --update` rewrites the file through `write_snapshot` (ignored; it writes to
//! `$LAMINA_SNAPSHOT_OUT`, never into `fixtures/`).
#![cfg(feature = "extract")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_arithmetic,
    clippy::format_push_string,
    reason = "test code: a panic is a test failure; rounding is the point; the writer builds text"
)]

use std::path::{Path, PathBuf};

use lamina::extract::TextOptions;
use lamina::{Document, OpenOptions};
use serde_json::{Value, json};

/// The fixtures the snapshot covers: every text golden (five scripts, CFF, standard 14,
/// variable fonts, built text) and the examples with text.
const FILES: &[&str] = &[
    "golden/text-latin.pdf",
    "golden/text-shaped.pdf",
    "golden/text-cff.pdf",
    "golden/text-std14.pdf",
    "golden/text-variable-collection.pdf",
    "golden/build-text-latin.pdf",
    "golden/build-text-multiscript.pdf",
    "golden/examples/hello.pdf",
    "golden/examples/invoice.pdf",
    "golden/examples/multilingual.pdf",
];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn round(x: f64, step: f64) -> f64 {
    let r = (x / step).round() * step;
    // -0 and 0 print differently; the snapshot only knows 0.
    if r == 0.0 { 0.0 } else { (r * 1000.0).round() / 1000.0 }
}

/// A glyph coordinate as TypeScript sees it: through `f32`, then to 0.01.
fn coord(x: f64) -> f64 {
    #[expect(clippy::cast_possible_truncation, reason = "the WASM sends quads as f32")]
    let x = f64::from(x as f32);
    round(x, 0.01)
}

fn page_json(text: &lamina::extract::PageText) -> Value {
    let runs: Vec<Value> = text
        .runs
        .iter()
        .map(|r| {
            json!({
                "text": r.text,
                "font": r.font_name.as_ref(),
                "size": round(r.font_size, 0.001),
                "vertical": r.vertical,
                "mcid": r.mcid,
                "actualText": r.actual_text,
                "invisible": r.invisible,
                "bounds": [
                    round(r.bounds.x0.get(), 0.01),
                    round(r.bounds.y0.get(), 0.01),
                    round(r.bounds.width().get(), 0.01),
                    round(r.bounds.height().get(), 0.01),
                ],
                "glyphs": [r.glyphs.start, r.glyphs.end],
            })
        })
        .collect();
    let glyphs: Vec<Value> = text
        .glyphs
        .iter()
        .map(|g| {
            let quad: Vec<f64> = g
                .quad
                .iter()
                .flat_map(|p| [coord(p.x.get()), coord(p.y.get())])
                .collect();
            json!({ "text": g.text, "quad": quad })
        })
        .collect();
    json!({ "lines": text.lines(), "unmapped": text.unmapped, "runs": runs, "glyphs": glyphs })
}

fn snapshot() -> Value {
    let mut files = serde_json::Map::new();
    for rel in FILES {
        let bytes = std::fs::read(fixtures().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
        let pages: Vec<Value> = doc
            .page_texts(&TextOptions::new())
            .unwrap()
            .iter()
            .map(page_json)
            .collect();
        files.insert((*rel).to_owned(), Value::Array(pages));
    }
    json!({ "files": files })
}

/// The file's text: pretty at the top, one line per run and per glyph (about a third of the
/// fully pretty size, and still a readable diff).
fn text(value: &Value) -> String {
    let compact = |v: &Value| serde_json::to_string(v).unwrap();
    let list = |items: &[Value], indent: &str| -> String {
        if items.is_empty() {
            return "[]".to_owned();
        }
        let rows: Vec<String> = items.iter().map(|i| format!("{indent}  {}", compact(i))).collect();
        format!("[\n{}\n{indent}]", rows.join(",\n"))
    };
    let mut out = String::from("{\n  \"files\": {\n");
    let files = value["files"].as_object().unwrap();
    let mut first_file = true;
    for (name, pages) in files {
        if !first_file {
            out.push_str(",\n");
        }
        first_file = false;
        out.push_str(&format!("    {}: [\n", compact(&Value::String(name.clone()))));
        let pages = pages.as_array().unwrap();
        for (i, page) in pages.iter().enumerate() {
            out.push_str(&format!(
                "      {{\n        \"lines\": {},\n        \"unmapped\": {},\n        \"runs\": {},\n        \"glyphs\": {}\n      }}",
                list(page["lines"].as_array().unwrap(), "        "),
                page["unmapped"],
                list(page["runs"].as_array().unwrap(), "        "),
                list(page["glyphs"].as_array().unwrap(), "        "),
            ));
            out.push_str(if i + 1 < pages.len() { ",\n" } else { "\n" });
        }
        out.push_str("    ]");
    }
    out.push_str("\n  }\n}\n");
    out
}

#[test]
fn page_text_matches_the_shared_snapshot() {
    let path = fixtures().join("text/page-text.json");
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        text(&snapshot()) == committed,
        "{} is stale: run `cargo xtask golden --update`",
        path.display()
    );
}

/// Writes the snapshot to `$LAMINA_SNAPSHOT_OUT` (run by `cargo xtask golden --update`).
#[test]
#[ignore = "run by cargo xtask golden"]
fn write_snapshot() {
    let out = std::env::var("LAMINA_SNAPSHOT_OUT").unwrap();
    std::fs::write(out, text(&snapshot())).unwrap();
}
