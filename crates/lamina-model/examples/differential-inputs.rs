//! Writes the differential suite's inputs (every `fixtures/corpus` file and its fixed-seed
//! mutants) to a directory: the PR lane's set for `cargo xtask differential --update`, which
//! runs pypdf over them, or a larger one (`--per-file N`) for the nightly lane's live run
//! (`cargo xtask differential --live`). The mutants come from `tests/common/mutants.rs`, the
//! same code the differential test uses, so the tools' facts and lamina's answers are about
//! the same bytes.
//!
//! ```sh
//! cargo run -p lamina-model --example differential-inputs -- <out-dir> [--per-file <N>]
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

const USAGE: &str = "usage: differential-inputs <out-dir> [--per-file <N>]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (out, per_file) = match args.as_slice() {
        [out] => (out, common::mutants::PER_FILE),
        [out, flag, n] if flag == "--per-file" => (out, n.parse().expect(USAGE)),
        _ => panic!("{USAGE}"),
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let mut written = 0;
    for (name, bytes) in common::corpus().into_iter().chain(common::mutants::first(per_file)) {
        std::fs::write(out.join(&name), bytes).unwrap();
        written += 1;
    }
    println!(
        "wrote {written} files to {} (mutant seed {:#x}, {per_file} per file)",
        out.display(),
        common::mutants::SEED
    );
}
