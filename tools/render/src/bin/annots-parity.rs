//! `annots-parity`: the annotation appearance parity suite's Rust half. `cargo xtask
//! annots-parity [--update] [--corpus]` runs it between the other renderers:
//!
//! ```sh
//! annots-parity matrix <out.pdf>
//! annots-parity compare <matrix.pdf> <renders-dir> <golden-dir> <thresholds.toml> <report-dir> [--update]
//! annots-parity move <list.tsv> <out-dir>
//! ```
//!
//! - `matrix` writes the matrix ([`annots_matrix`]): every subtype under each property set on
//!   pages turned 0°, 90°, 180° and 270°, made twice to check that the bytes don't change.
//! - `compare` checks the renders ([`annots_compare`]): each engine's page against its golden,
//!   pdf.js and hayro against PDFium; failures write expected | actual | diff images into
//!   `<report-dir>`. Exits 1 on any failure. `--update` rewrites the goldens.
//! - `move` is the corpus edit test ([`annots_move`]) for every `<name>\t<path>` line of the
//!   list: each saved file goes to `<out-dir>/<n>/<n>.pdf` (`n` the line's number, so `pdfium.py`
//!   can render one file per folder), and `<out-dir>/report.json` maps each name to what
//!   happened and the saved file.

#[path = "../annots_compare.rs"]
mod annots_compare;
#[path = "../annots_matrix.rs"]
mod annots_matrix;
#[path = "../annots_move.rs"]
mod annots_move;
#[path = "../compare.rs"]
mod compare;
#[path = "../hayro_render.rs"]
mod hayro_render;
#[path = "../image.rs"]
mod image;
#[path = "../thresholds.rs"]
#[expect(
    dead_code,
    reason = "shared with render-golden, whose thresholds have one parity check"
)]
mod thresholds;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use serde_json::json;

const USAGE: &str = "usage: annots-parity matrix <out.pdf> | compare <matrix.pdf> <renders-dir> <golden-dir> \
                     <thresholds.toml> <report-dir> [--update] | move <list.tsv> <out-dir>";

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("annots-parity: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let update = args.iter().any(|a| a == "--update");
    args.retain(|a| a != "--update");
    match args.as_slice() {
        [command, out] if command == "matrix" => matrix(out).map(|()| true),
        [command, matrix, renders, golden, thresholds, report] if command == "compare" => {
            let failures = annots_compare::run(&annots_compare::Run {
                matrix: matrix.into(),
                renders: renders.into(),
                golden: golden.into(),
                thresholds: thresholds.into(),
                report: report.into(),
                update,
            })?;
            for failure in &failures {
                eprintln!("  {failure}");
            }
            Ok(failures.is_empty())
        }
        [command, list, out] if command == "move" => move_files(list, &PathBuf::from(out)).map(|()| true),
        _ => bail!(USAGE),
    }
}

/// Writes the matrix, made twice: the same build must write the same bytes.
fn matrix(out: &str) -> Result<()> {
    let bytes = annots_matrix::generate()?;
    if annots_matrix::generate()? != bytes {
        bail!("the matrix differs between two runs: something in it is not deterministic");
    }
    fs::write(out, bytes).with_context(|| format!("writing {out}"))?;
    println!(
        "annots-parity: {} pages: {} looks × {} turns, {} marks each",
        annots_matrix::PAGES,
        annots_matrix::LOOKS.len(),
        annots_matrix::TURNS.len(),
        annots_matrix::SUBTYPES.len()
    );
    for look in &annots_matrix::LOOKS {
        println!("  {}", look.name);
    }
    Ok(())
}

/// The corpus edit test over every file of `list`.
fn move_files(list: &str, out: &std::path::Path) -> Result<()> {
    let text = fs::read_to_string(list).with_context(|| format!("reading {list}"))?;
    let mut report = BTreeMap::new();
    for (n, line) in text.lines().filter(|l| !l.is_empty()).enumerate() {
        let (name, path) = line.split_once('\t').context("a `<name>\\t<path>` line")?;
        let (moved, saved) = annots_move::move_all(fs::read(path).with_context(|| format!("reading {path}"))?);
        let saved = saved
            .map(|bytes| -> Result<PathBuf> {
                let dir = out.join(n.to_string());
                fs::create_dir_all(&dir)?;
                let file = dir.join(format!("{n}.pdf"));
                fs::write(&file, bytes)?;
                Ok(file)
            })
            .transpose()?;
        report.insert(name.to_owned(), json!({ "result": moved, "saved": saved }));
    }
    fs::write(out.join("report.json"), serde_json::to_string_pretty(&report)? + "\n")?;
    Ok(())
}
