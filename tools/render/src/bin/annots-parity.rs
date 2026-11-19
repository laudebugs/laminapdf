//! `annots-parity`: the annotation appearance parity suite's Rust half. `cargo xtask
//! annots-parity [--update] [--corpus]` runs it between the other renderers:
//!
//! ```sh
//! annots-parity settings
//! annots-parity matrix <out.pdf>
//! annots-parity compare <matrix.pdf> <renders-dir> <golden-dir> <thresholds.toml> <report-dir> [--update]
//! annots-parity move <list.tsv> <out-dir>
//! annots-parity check-saved <dir> <thresholds.toml>
//! ```
//!
//! - `settings` prints what xtask must render, as one JSON line: the matrix file's stem and the
//!   resolutions of the goldens and the parity checks, so xtask keeps no copy of them.
//! - `matrix` writes the matrix ([`annots_matrix`]): every subtype under each property set on
//!   pages turned 0°, 90°, 180° and 270°, made twice to check that the bytes don't change.
//! - `compare` checks the renders ([`annots_compare::run`]): each engine's page against its
//!   golden, pdf.js and hayro against PDFium; failures write expected | actual | diff images into
//!   `<report-dir>`. Exits 1 on any failure. `--update` rewrites the goldens.
//! - `move` is the corpus edit test's edit ([`annots_move`]) for every `<name>\t<path>` line of
//!   the list: for each file it edits, `<out-dir>/<n>/` (`n` the line's number) gets
//!   `original.pdf` and `saved.pdf`, for the renderers to draw side by side; `<out-dir>/report.json`
//!   maps each name to what happened and that folder.
//! - `check-saved` compares the renders of one such folder ([`annots_compare::compare_saved`]);
//!   exits 1 when the saved file draws differently.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use lamina_render::settings::{GOLDEN_DPI, PARITY_DPI};
use lamina_render::{annots_compare, annots_matrix, annots_move};
use serde_json::json;

const USAGE: &str = "usage: annots-parity settings | matrix <out.pdf> | compare <matrix.pdf> <renders-dir> \
                     <golden-dir> <thresholds.toml> <report-dir> [--update] | move <list.tsv> <out-dir> | \
                     check-saved <dir> <thresholds.toml>";

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
        [command] if command == "settings" => {
            let settings = json!({
                "matrix": annots_compare::MATRIX,
                "golden_dpi": GOLDEN_DPI,
                "parity_dpi": PARITY_DPI,
            });
            println!("{settings}");
            Ok(true)
        }
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
            Ok(report_failures(&failures))
        }
        [command, list, out] if command == "move" => move_files(list, Path::new(out)).map(|()| true),
        [command, dir, thresholds] if command == "check-saved" => Ok(report_failures(&annots_compare::compare_saved(
            Path::new(dir),
            Path::new(thresholds),
        )?)),
        _ => bail!(USAGE),
    }
}

fn report_failures(failures: &[String]) -> bool {
    for failure in failures {
        eprintln!("  {failure}");
    }
    failures.is_empty()
}

/// Writes the matrix, made twice: the same build must write the same bytes.
fn matrix(out: &str) -> Result<()> {
    let bytes = annots_matrix::generate()?;
    if annots_matrix::generate()? != bytes {
        bail!("the matrix differs between two runs: something in it is not deterministic");
    }
    fs::write(out, bytes).with_context(|| format!("writing {out}"))?;
    println!(
        "annots-parity: {} pages: {} looks × {} turns, {} marks each, {} stamp pages of {} marks, a filled form and {} content pages",
        annots_matrix::PAGES,
        annots_matrix::LOOKS.len(),
        annots_matrix::TURNS.len(),
        annots_matrix::SUBTYPES.len(),
        annots_matrix::STAMP_PAGES,
        annots_matrix::STAMP_SUBTYPES.len(),
        annots_matrix::CONTENT_PAGES
    );
    for look in &annots_matrix::LOOKS {
        println!("  {}", look.name);
    }
    Ok(())
}

/// The corpus edit test's edit over every file of `list`.
fn move_files(list: &str, out: &Path) -> Result<()> {
    let text = fs::read_to_string(list).with_context(|| format!("reading {list}"))?;
    let mut report = BTreeMap::new();
    for (n, line) in text.lines().filter(|l| !l.is_empty()).enumerate() {
        let (name, path) = line.split_once('\t').context("a `<name>\\t<path>` line")?;
        let original = fs::read(path).with_context(|| format!("reading {path}"))?;
        let (moved, saved) = annots_move::move_all(&original);
        let dir = saved
            .map(|bytes| -> Result<PathBuf> {
                let dir = out.join(n.to_string());
                fs::create_dir_all(&dir)?;
                fs::write(dir.join("original.pdf"), &original)?;
                fs::write(dir.join("saved.pdf"), bytes)?;
                Ok(dir)
            })
            .transpose()?;
        report.insert(name.to_owned(), json!({ "result": moved, "saved": dir }));
    }
    fs::write(out.join("report.json"), serde_json::to_string_pretty(&report)? + "\n")?;
    Ok(())
}
