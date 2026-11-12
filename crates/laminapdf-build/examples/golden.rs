//! Regenerates `laminapdf-build`'s byte-exact goldens in `fixtures/golden/` into a directory,
//! and prints the files it wrote: `vectors.pdf` and `images.pdf` (the code is in
//! `tests/common/samples.rs`), the text and navigation goldens `build-text-latin.pdf`,
//! `build-text-multiscript.pdf`, `links-and-outlines.pdf` and `metadata.pdf`
//! (`text_and_navigation.rs`), and the tagged `tagged-report.pdf` (`tagged.rs`). `cargo xtask
//! golden --update` runs it on `fixtures/golden/`; tests never write goldens (ADR 0018 tier 3).
//!
//! ```sh
//! cargo run -p laminapdf-build --example golden-build -- <out-dir>
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
    for (name, build) in common::samples::GOLDENS {
        let bytes = build();
        let path = out.join(name);
        std::fs::write(&path, &bytes).unwrap();
        println!("{} ({} bytes)", path.display(), bytes.len());
    }
}
