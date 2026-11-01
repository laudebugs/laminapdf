//! Writes the lamina half of the round-trip suite (`tests/common/roundtrip.rs`) to
//! `<out-dir>`: every saved output and `manifest.json`, which `cargo xtask roundtrip` then
//! checks with qpdf, pdf.js and `PDFium`. Fails when a case's outcome or reopened facts were
//! wrong.
//!
//! ```sh
//! cargo run -p laminapdf-save --example roundtrip -- <out-dir>
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

#[path = "../tests/common/roundtrip.rs"]
mod roundtrip;

use std::path::{Path, PathBuf};

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: roundtrip <out-dir>"));
    std::fs::create_dir_all(&out).unwrap();
    let suite = roundtrip::run(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    for (file, bytes) in &suite.outputs {
        std::fs::write(out.join(file), bytes).unwrap();
    }
    let manifest = serde_json::json!({ "cases": suite.manifest });
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    println!(
        "lamina: {} saved and reopened, {} refused as expected",
        suite.saved, suite.refused
    );
    if !suite.failures.is_empty() {
        for f in &suite.failures {
            eprintln!("{f}");
        }
        std::process::exit(1);
    }
}
