//! Writes the differential suite's inputs (every `fixtures/corpus` file and its fixed-seed
//! mutants) to a directory, for `cargo xtask differential --update`, which runs pypdf over
//! them. The mutants come from `tests/common/mutants.rs`, the same code the differential
//! test uses, so pypdf's facts and lamina's answers are about the same bytes.
//!
//! ```sh
//! cargo run -p lamina-model --example differential-inputs -- <out-dir>
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
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: differential-inputs <out-dir>"));
    std::fs::create_dir_all(&out).unwrap();
    let mut written = 0;
    for (name, bytes) in common::corpus().into_iter().chain(common::mutants::all()) {
        std::fs::write(out.join(&name), bytes).unwrap();
        written += 1;
    }
    println!(
        "wrote {written} files to {} (mutant seed {:#x})",
        out.display(),
        common::mutants::SEED
    );
}
