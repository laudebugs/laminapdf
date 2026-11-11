//! Lamina PDF's reading of each file's XMP packet, as JSON: Lamina PDF's side of the nightly
//! lane's XMP differential against exiftool (`cargo xtask differential --live --xmp`, todo E1c).
//!
//! The list has one `<name>\t<path>` line per file; the output maps each name to
//! `{outcome, error?, packet, fields}`: `outcome` is `opened` or `failed` (with Lamina PDF's
//! `error` code), `packet` is `absent`, `parsed` or `malformed` (`Document::xmp`), and `fields`
//! holds the values `xmp::Packet`'s getters read (dates as ISO 8601, the PDF/A and PDF/UA
//! parts as numbers), leaving out the ones the packet doesn't have. A panic is recorded as the
//! outcome `panic` (a bug to report), not a crash of the whole run.
//!
//! ```sh
//! cargo run --release -p laminapdf-model --example xmp-facts -- <list.tsv> <out.json>
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use laminapdf_model::xmp::{Packet, PacketState};
use laminapdf_model::{Document, OpenOptions, PdfDate};
use serde_json::{Map, Value, json};

/// The fields lamina reads from `packet`, by the names xtask compares.
fn fields(packet: &Packet) -> Map<String, Value> {
    let date = |d: Option<PdfDate>| d.map(|d| json!(d.to_iso8601()));
    let authors = packet.authors();
    let values = [
        ("title", packet.title().map(Value::from)),
        ("authors", (!authors.is_empty()).then(|| json!(authors))),
        ("subject", packet.subject().map(Value::from)),
        ("keywords", packet.keywords().map(Value::from)),
        ("creator_tool", packet.creator_tool().map(Value::from)),
        ("producer", packet.producer().map(Value::from)),
        ("create_date", date(packet.create_date())),
        ("modify_date", date(packet.modify_date())),
        ("metadata_date", date(packet.metadata_date())),
        ("document_id", packet.document_id().map(Value::from)),
        ("instance_id", packet.instance_id().map(Value::from)),
        ("pdfa_part", packet.pdfa_part().map(Value::from)),
        ("pdfa_conformance", packet.pdfa_conformance().map(Value::from)),
        ("pdfua_part", packet.pdfua_part().map(Value::from)),
    ];
    values
        .into_iter()
        .filter_map(|(name, value)| Some((name.to_owned(), value?)))
        .collect()
}

/// Lamina PDF's XMP facts about one file.
fn facts(bytes: Vec<u8>) -> Value {
    let failed = |code: &str| json!({"outcome": "failed", "error": code, "packet": null, "fields": {}});
    let doc = match Document::open(bytes, OpenOptions::default()) {
        Ok(doc) => doc,
        Err(e) => return failed(e.code()),
    };
    let (packet, fields) = match doc.xmp() {
        Ok(PacketState::Parsed(packet)) => ("parsed", fields(&packet)),
        Ok(state) if state.is_malformed() => ("malformed", Map::new()),
        Ok(_) => ("absent", Map::new()),
        Err(e) => return failed(e.code()),
    };
    json!({"outcome": "opened", "packet": packet, "fields": fields})
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [list, out] = args.as_slice() else {
        panic!("usage: xmp-facts <list.tsv> <out.json>");
    };
    let list = std::fs::read_to_string(list).unwrap();
    // The panic message still goes to stderr; the run carries on with the next file.
    let mut facts_by_name = Map::new();
    for line in list.lines().filter(|l| !l.is_empty()) {
        let (name, path) = line.split_once('\t').expect("<name>\\t<path> lines");
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        let value = catch_unwind(AssertUnwindSafe(|| facts(bytes)))
            .unwrap_or_else(|_| json!({"outcome": "panic", "packet": null, "fields": {}}));
        facts_by_name.insert(name.to_owned(), value);
    }
    let report = json!({
        "tool": "lamina",
        "version": env!("CARGO_PKG_VERSION"),
        "reports_repair": false,
        "facts": Value::Object(facts_by_name),
    });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
}
