//! Writes the differential suite's inputs to a directory, for `cargo xtask differential
//! --update`, which runs pypdf over them: every `fixtures/corpus` file and its fast-set
//! mutants, or with `--mutants <n>` only the full lane's `n` mutants. The mutants come from
//! `tests/common/mutants.rs`, the same code the differential test uses, so pypdf's facts and
//! lamina's answers are about the same bytes.
//!
//! ```sh
//! cargo run -p lamina-model --example differential-inputs -- <out-dir> [--mutants <n>]
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

const USAGE: &str = "usage: differential-inputs <out-dir> [--mutants <n>]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (out, full) = match args.as_slice() {
        [out] => (out, None),
        [out, flag, n] if flag == "--mutants" => (out, Some(n.parse::<usize>().expect(USAGE))),
        _ => panic!("{USAGE}"),
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let (inputs, seed) = match full {
        None => (
            common::corpus().into_iter().chain(common::mutants::all()).collect(),
            common::mutants::SEED,
        ),
        Some(n) => (common::mutants::full(n), common::mutants::FULL_SEED),
    };
    let mut written = 0;
    for (name, bytes) in inputs {
        std::fs::write(out.join(&name), bytes).unwrap();
        written += 1;
    }
    println!("wrote {written} files to {} (mutant seed {seed:#x})", out.display());
}
