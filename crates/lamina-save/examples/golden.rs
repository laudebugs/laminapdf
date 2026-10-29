//! Regenerates every byte-exact golden in `fixtures/golden/` from its source (the generation
//! code in `tests/common/samples.rs`) into a directory, and prints the files it wrote.
//! `cargo xtask golden --update` runs it on `fixtures/golden/`; tests never write goldens
//! (ADR 0018 tier 3).
//!
//! ```sh
//! cargo run -p lamina-save --example golden -- <out-dir>
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

/// The code that builds one golden.
type Build = fn() -> lamina_cos::Result<Vec<u8>>;

/// Every golden: its file name under `fixtures/golden/` and the code that builds it.
const GOLDENS: [(&str, Build); 4] = [
    ("hello.pdf", common::samples::hello),
    ("imaging-demo.pdf", common::samples::imaging_demo),
    ("incremental-set-info.pdf", common::samples::incremental_set_info),
    ("full-rewrite-hello.pdf", common::samples::full_rewrite_hello),
];

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: golden <out-dir>"));
    std::fs::create_dir_all(&out).unwrap();
    for (name, build) in GOLDENS {
        let bytes = build().unwrap();
        let path = out.join(name);
        std::fs::write(&path, &bytes).unwrap();
        println!("{} ({} bytes)", path.display(), bytes.len());
    }
}
