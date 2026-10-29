//! Writes the differential suite's inputs to a directory, for `cargo xtask differential
//! [--full] --update`, which runs pypdf over them: the fast set (every `fixtures/corpus` file
//! and its four mutants), or with `--full` the full lane's mutants. The inputs come from
//! `tests/common/mutants.rs`, the same code the differential test uses, so pypdf's facts and
//! lamina's answers are about the same bytes.
//!
//! ```sh
//! cargo run -p lamina-model --example differential-inputs -- <out-dir> [--full]
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

const USAGE: &str = "usage: differential-inputs <out-dir> [--full]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (out, full) = match args.as_slice() {
        [out] => (out, false),
        [out, flag] if flag == "--full" => (out, true),
        _ => panic!("{USAGE}"),
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let (inputs, seed) = if full {
        (common::mutants::full(), common::mutants::FULL_SEED)
    } else {
        (common::mutants::fast_inputs(), common::mutants::SEED)
    };
    let mut written = 0;
    for (name, bytes) in inputs {
        std::fs::write(out.join(&name), bytes).unwrap();
        written += 1;
    }
    println!("wrote {written} files to {} (mutant seed {seed:#x})", out.display());
}
