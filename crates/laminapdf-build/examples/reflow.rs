//! Writes the reader-mode corpus (todo U4b; the files are in
//! `tests/common/reflow.rs`) into a directory: `<name>.pdf` and `<name>.json` per file,
//! and prints the files it wrote. `cargo xtask golden --update` runs it on
//! `fixtures/reflow/`; tests never write fixtures (ADR 0018 tier 3).
//!
//! ```sh
//! cargo run -p laminapdf-build --example reflow -- <out-dir>
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

#[path = "../tests/common/mod.rs"]
mod common;

use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: reflow <out-dir>"));
    std::fs::create_dir_all(&out).unwrap();
    for sample in common::reflow::corpus() {
        for (ext, bytes) in [("pdf", sample.pdf.as_slice()), ("json", sample.json.as_bytes())] {
            let path = out.join(format!("{}.{ext}", sample.name));
            std::fs::write(&path, bytes).unwrap();
            println!("{} ({} bytes)", path.display(), bytes.len());
        }
    }
}
