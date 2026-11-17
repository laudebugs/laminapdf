//! `cargo xtask annots-parity [--update] [--corpus]`: the annotation appearance parity suite
//! (todo E4g; ADR 0018 tiers 6 and 8). `tools/render`'s `annots-parity` writes the matrix (every
//! annotation subtype Lamina PDF draws × colour × opacity × border, on pages turned 0°, 90°,
//! 180° and 270°, through the façade), PDFium (`tools/render/pdfium.py`, pypdfium2;
//! `$LAMINA_PDF_PYTHON` or `python3`) and pdf.js (`tools/render/pdfjs-render.mjs`, its own pnpm
//! project, installed here when missing) render it at 72 and 150 dpi, and `annots-parity
//! compare` checks each engine's 72 dpi pages against `fixtures/render-golden/annots-parity/`
//! and hayro and pdf.js against PDFium at 150 dpi, with the limits in
//! `fixtures/render-golden/annots-parity.toml`. Failures write expected | actual | diff images
//! into `target/xtask/annots-parity/diff/`; `--update` rewrites the goldens.
//!
//! `--corpus` adds the corpus edit test: every annotation of the pdf.js corpus's
//! `annotation-*.pdf` files (`cargo xtask corpus fetch`) moved by 1 pt and saved incrementally
//! (`annots-parity move`, which also reopens each file in Lamina PDF and hayro), then each saved
//! file checked by `qpdf --check` and rendered by PDFium. A file Lamina PDF may not save
//! incrementally as it is (signed, encrypted, repaired) is skipped with its reason.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::workspace::Workspace;
use crate::{render, step};

/// Where the suite writes, relative to the workspace root.
const OUT_DIR: &str = "target/xtask/annots-parity";
/// The goldens' resolution and the parity checks' (`tools/render/src/annots_compare.rs`).
const DPIS: [&str; 2] = ["72", "150"];
/// The matrix file's name, which `annots-parity compare` reads PDFium's renders under.
const MATRIX: &str = "annots-matrix";

/// What `cargo xtask annots-parity` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    /// Rewrite the goldens.
    pub(crate) update: bool,
    /// Also run the corpus edit test.
    pub(crate) corpus: bool,
}

/// Parses `[--update] [--corpus]`.
pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut parsed = Args::default();
    for arg in args {
        match arg.as_str() {
            "--update" => parsed.update = true,
            "--corpus" => parsed.corpus = true,
            _ => bail!("usage: cargo xtask annots-parity [--update] [--corpus]"),
        }
    }
    Ok(parsed)
}

/// Runs the suite.
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let out = ws.root.join(OUT_DIR);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    // The corpus test runs even when the matrix fails, so one night reports both.
    let matrix = matrix(ws, &out, args.update);
    let corpus = if args.corpus { corpus(ws, &out) } else { Ok(()) };
    match (matrix, corpus) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
        (Err(m), Err(c)) => bail!("{m:#}\n{c:#}"),
    }
}

/// The matrix: generated, rendered by PDFium and pdf.js, compared by `annots-parity compare`.
fn matrix(ws: &Workspace, out: &Path, update: bool) -> Result<()> {
    let pdfs = out.join("pdf");
    fs::create_dir_all(&pdfs)?;
    let matrix = pdfs.join(format!("{MATRIX}.pdf"));
    tool(ws, &["matrix".as_ref(), matrix.as_os_str()])?;
    let renders = out.join("renders");
    let pdfium = renders.join("pdfium");
    let mut args = vec![
        ws.root.join("tools/render/pdfium.py").into_os_string(),
        pdfs.into_os_string(),
        pdfium.into_os_string(),
    ];
    args.extend(DPIS.map(Into::into));
    crate::run(&ws.root, &crate::harness::python(), &args)?;
    install_pdfjs(ws)?;
    let script = ws.root.join("tools/render/pdfjs-render.mjs");
    for dpi in DPIS {
        let dir = renders.join("pdfjs").join(dpi);
        crate::run(
            &ws.root,
            "node",
            &[script.as_os_str(), matrix.as_os_str(), dir.as_os_str(), dpi.as_ref()],
        )?;
    }
    let golden = ws.root.join("fixtures/render-golden/annots-parity");
    let thresholds = ws.root.join("fixtures/render-golden/annots-parity.toml");
    let report = out.join("diff");
    let mut args = vec![
        "compare".as_ref(),
        matrix.as_os_str(),
        renders.as_os_str(),
        golden.as_os_str(),
        thresholds.as_os_str(),
        report.as_os_str(),
    ];
    if update {
        args.push("--update".as_ref());
    }
    tool(ws, &args).with_context(|| format!("annots-parity failed; diff images in {}", report.display()))?;
    if update {
        step(
            "Annotation parity goldens rewritten; review `git diff --stat fixtures/render-golden/annots-parity` and the PNGs",
        );
    }
    Ok(())
}

/// `tools/render`'s pdf.js, installed from its lockfile when missing.
fn install_pdfjs(ws: &Workspace) -> Result<()> {
    let dir = ws.root.join("tools/render");
    if dir.join("node_modules/pdfjs-dist/package.json").is_file() {
        return Ok(());
    }
    crate::run(&dir, "pnpm", &["install", "--frozen-lockfile"])
}

/// Runs `tools/render`'s `annots-parity` (its own workspace, built in release mode).
fn tool(ws: &Workspace, args: &[&std::ffi::OsStr]) -> Result<()> {
    let manifest = ws.root.join(render::MANIFEST);
    let mut all: Vec<&std::ffi::OsStr> = vec![
        "run".as_ref(),
        "--manifest-path".as_ref(),
        manifest.as_os_str(),
        "--locked".as_ref(),
        "--release".as_ref(),
        "-q".as_ref(),
        "--bin".as_ref(),
        "annots-parity".as_ref(),
        "--".as_ref(),
    ];
    all.extend(args);
    crate::run(&ws.root, "cargo", &all)
}

/// One file of `annots-parity move`'s report.
#[derive(Debug, Deserialize)]
struct Entry {
    result: Report,
    /// The saved file, alone in its folder (`<n>/<n>.pdf`).
    saved: Option<PathBuf>,
}

/// `tools/render/src/annots_move.rs`'s `Report`.
#[derive(Debug, Deserialize)]
struct Report {
    outcome: String,
    reason: Option<String>,
    moved: usize,
    popups: usize,
    locked: usize,
    not_editable: usize,
    problems: Vec<String>,
}

/// The corpus edit test over the pdf.js corpus's `annotation-*.pdf` files.
fn corpus(ws: &Workspace, out: &Path) -> Result<()> {
    let inputs: Vec<(String, PathBuf)> = crate::corpus::verified_inputs(ws, &["pdfjs".to_owned()])?
        .into_iter()
        .filter(|(name, _)| is_annotation_file(name))
        .collect();
    if inputs.is_empty() {
        bail!("no annotation-*.pdf in the pdf.js corpus");
    }
    let dir = out.join("corpus");
    fs::create_dir_all(&dir)?;
    let list = dir.join("list.tsv");
    let lines: Vec<String> = inputs
        .iter()
        .map(|(name, path)| format!("{name}\t{}", path.display()))
        .collect();
    fs::write(&list, lines.join("\n") + "\n")?;
    tool(ws, &["move".as_ref(), list.as_os_str(), dir.as_os_str()])?;
    let report: BTreeMap<String, Entry> = serde_json::from_str(&fs::read_to_string(dir.join("report.json"))?)?;
    let mut failed = Vec::new();
    let (mut moved, mut skipped) = (0, 0);
    for (name, entry) in &report {
        let mut problems = entry.result.problems.clone();
        if let Some(saved) = &entry.saved {
            problems.extend(check_saved(ws, saved).err().map(|e| format!("{e:#}")));
        }
        let r = &entry.result;
        let line = match (r.outcome.as_str(), &r.reason) {
            ("skipped", reason) => {
                skipped += 1;
                format!("skipped: {}", reason.as_deref().unwrap_or("no reason given"))
            }
            _ => format!(
                "{} moved, {} Popups with them, {} locked, {} not editable",
                r.moved, r.popups, r.locked, r.not_editable
            ),
        };
        if problems.is_empty() {
            moved += usize::from(r.outcome == "moved");
            println!("  {name}: {line}");
        } else {
            println!("  {name}: {line}; FAILED: {}", problems.join("; "));
            failed.push(name.clone());
        }
    }
    if !failed.is_empty() {
        bail!("corpus edit test failed for {}", failed.join(", "));
    }
    step(&format!(
        "corpus edit test: {moved} files moved, saved and checked by Lamina PDF, hayro, qpdf and PDFium; {skipped} skipped"
    ));
    Ok(())
}

/// `qpdf --check` and PDFium on the saved file `saved`, alone in its folder.
fn check_saved(ws: &Workspace, saved: &Path) -> Result<()> {
    let (Some(dir), Some(file)) = (saved.parent(), saved.file_name()) else {
        bail!("{}: not a saved file", saved.display());
    };
    crate::roundtrip::qpdf_check(dir, &[file.to_string_lossy().into_owned()])?;
    let script = ws.root.join("tools/render/pdfium.py");
    let renders = dir.join("pdfium");
    let output = std::process::Command::new(crate::program_path(&crate::harness::python()))
        .args([script.as_os_str(), dir.as_os_str(), renders.as_os_str(), "72".as_ref()])
        .output()
        .context("starting pdfium.py")?;
    if !output.status.success() {
        bail!("PDFium: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(())
}

/// Whether a corpus input (`pdfjs/test/pdfs/annotation-ink.pdf`) is one of pdf.js's annotation
/// test files.
fn is_annotation_file(name: &str) -> bool {
    let path = Path::new(name);
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("annotation-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_args_should_read_both_flags_in_any_order() {
        assert_eq!(
            parse_args(&args(&["--corpus", "--update"])).unwrap(),
            Args {
                update: true,
                corpus: true
            }
        );
        assert_eq!(parse_args(&[]).unwrap(), Args::default());
    }

    #[test]
    fn parse_args_should_refuse_anything_else() {
        assert!(parse_args(&args(&["--sample", "3"])).is_err());
    }

    #[test]
    fn only_pdfjs_annotation_files_are_moved() {
        assert!(is_annotation_file("pdfjs/test/pdfs/annotation-caret-ink.pdf"));
        assert!(!is_annotation_file("pdfjs/test/pdfs/alphatrans.pdf"));
        assert!(!is_annotation_file("pdfjs/test/pdfs/annotation-caret-ink.pdf.link"));
    }
}
