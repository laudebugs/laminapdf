//! Writes the reading-order corpus (todo U2b; the layouts are in
//! `tests/common/reading_order.rs`) into a directory: `<name>.pdf` and `<name>.json` per file,
//! and prints the files it wrote. `cargo xtask golden --update` runs it on
//! `fixtures/reading-order/`; tests never write fixtures (ADR 0018 tier 3).
//!
//! ```sh
//! cargo run -p lamina-build --example reading_order -- <out-dir>
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
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: reading_order <out-dir>"));
    std::fs::create_dir_all(&out).unwrap();
    for sample in common::reading_order::corpus() {
        for (ext, bytes) in [("pdf", sample.pdf.as_slice()), ("json", sample.json.as_bytes())] {
            let path = out.join(format!("{}.{ext}", sample.name));
            std::fs::write(&path, bytes).unwrap();
            println!("{} ({} bytes)", path.display(), bytes.len());
        }
    }
}
