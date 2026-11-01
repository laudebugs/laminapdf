//! Writes the differential suite's inputs to a directory, for `cargo xtask differential
//! [--full] --update`, which runs pypdf over them: the fast set (every `fixtures/corpus` file
//! and its four mutants), or with `--full` the full lane's mutants. The inputs come from
//! `tests/common/mutants.rs`, the same code the differential test uses, so pypdf's facts and
//! Lamina PDF's answers are about the same bytes.
//!
//! With `--nightly <N>`, it writes the nightly lane's inputs for `cargo xtask differential
//! --live`: every corpus file, the first `N` mutants of the fast set's series (the fast set is
//! its prefix) and the full lane's set, so every mutant either lane has exceptions for is in it.
//!
//! ```sh
//! cargo run -p laminapdf-model --example differential-inputs -- <out-dir> [--full | --nightly <N>]
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

const USAGE: &str = "usage: differential-inputs <out-dir> [--full | --nightly <N>]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (out, inputs, seed) = match args.as_slice() {
        [out] => (out, common::mutants::fast_inputs(), common::mutants::SEED),
        [out, flag] if flag == "--full" => (out, common::mutants::full(), common::mutants::FULL_SEED),
        [out, flag, n] if flag == "--nightly" => {
            let per_file: usize = n.parse().expect(USAGE);
            let inputs = common::corpus()
                .into_iter()
                .chain(common::mutants::series_m(per_file))
                .chain(common::mutants::full())
                .collect();
            (out, inputs, common::mutants::SEED)
        }
        _ => panic!("{USAGE}"),
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let mut written = 0;
    for (name, bytes) in inputs {
        std::fs::write(out.join(&name), bytes).unwrap();
        written += 1;
    }
    println!("wrote {written} files to {} (mutant seed {seed:#x})", out.display());
}
