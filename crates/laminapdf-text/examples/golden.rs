//! Regenerates `laminapdf-text`'s byte-exact goldens (`fixtures/golden/text-*.pdf`) from the
//! samples in `tests/common/samples.rs` into a directory, and prints the files it wrote.
//! `cargo xtask golden --update` runs it on `fixtures/golden/`; tests never write goldens
//! (ADR 0018 tier 3).
//!
//! ```sh
//! cargo run -p laminapdf-text --example golden-text -- <out-dir>
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
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: golden <out-dir>"));
    std::fs::create_dir_all(&out).unwrap();
    for (name, build, _) in common::samples::GOLDENS {
        let bytes = build().bytes;
        let path = out.join(name);
        std::fs::write(&path, &bytes).unwrap();
        println!("{} ({} bytes)", path.display(), bytes.len());
    }
}
