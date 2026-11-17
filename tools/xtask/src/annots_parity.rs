//! `cargo xtask annots-parity [--update] [--corpus]`: the annotation appearance parity suite
//! (ADR 0018 tiers 6 to 8). `tools/render`'s `annots-parity` writes the matrix (every annotation
//! subtype Lamina PDF draws × colour × opacity × border, on pages turned 0°, 90°, 180° and 270°,
//! through the façade), PDFium (`tools/render/pdfium.py`, pypdfium2; `$LAMINA_PDF_PYTHON` or
//! `python3`) and pdf.js (`tools/render/pdfjs-render.mjs`, its own pnpm project, installed here
//! when missing) render it at the resolutions `annots-parity settings` names, and
//! `annots-parity compare` checks each engine's pages against
//! `fixtures/render-golden/annots-parity/` and hayro and pdf.js against PDFium, with the limits
//! in `fixtures/render-golden/annots-parity.toml`. Failures write expected | actual | diff
//! images into `target/xtask/annots-parity/diff/`; `--update` rewrites the goldens.
//!
//! `--corpus` adds the corpus edit test over the pdf.js corpus's `annotation-*.pdf` files
//! (`cargo xtask corpus fetch`): `annots-parity move` moves every annotation by 1 pt, saves
//! incrementally and reopens each file in Lamina PDF; then `qpdf --check` checks each saved file,
//! PDFium, pdf.js and hayro draw it beside its original (`annots-parity check-saved`), and every
//! file's outcome must be the one pinned in `fixtures/render-golden/annots-parity-corpus.toml`.
//!
//! The goldens are exact, and made on macOS arm64; elsewhere the matrix half fails on
//! anti-aliasing, so run it there (the nightly lane does).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::workspace::Workspace;
use crate::{render, step};

/// Where the suite writes, relative to the workspace root.
const OUT_DIR: &str = "target/xtask/annots-parity";
/// The outcome each corpus file must have.
const EXPECTED: &str = "fixtures/render-golden/annots-parity-corpus.toml";
/// The limits.
const THRESHOLDS: &str = "fixtures/render-golden/annots-parity.toml";

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

/// What `annots-parity settings` prints: the one source of the matrix's name and resolutions.
#[derive(Debug, Deserialize)]
struct Settings {
    matrix: String,
    golden_dpi: u32,
    parity_dpi: u32,
}

/// Runs the suite.
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let out = ws.root.join(OUT_DIR);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    install_pdfjs(ws)?;
    let settings = settings(ws)?;
    // The corpus test runs even when the matrix fails, so one night reports both.
    let matrix = matrix(ws, &out, &settings, args.update);
    let corpus = if args.corpus {
        corpus(ws, &out, &settings)
    } else {
        Ok(())
    };
    match (matrix, corpus) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
        (Err(m), Err(c)) => bail!("{m:#}\n{c:#}"),
    }
}

/// `tools/render`'s pdf.js and canvas, installed from its lockfile when missing, as
/// tools/differential's are.
fn install_pdfjs(ws: &Workspace) -> Result<()> {
    if ws
        .root
        .join("tools/render/node_modules/pdfjs-dist/package.json")
        .is_file()
    {
        return Ok(());
    }
    ws.pnpm(&["--dir", "tools/render", "install", "--frozen-lockfile"])
}

/// The matrix: generated, rendered by PDFium and pdf.js, compared by `annots-parity compare`.
fn matrix(ws: &Workspace, out: &Path, settings: &Settings, update: bool) -> Result<()> {
    let pdfs = out.join("pdf");
    fs::create_dir_all(&pdfs)?;
    let matrix = pdfs.join(format!("{}.pdf", settings.matrix));
    tool(ws, &["matrix".as_ref(), matrix.as_os_str()])?;
    let renders = out.join("renders");
    let dpis = [settings.golden_dpi, settings.parity_dpi];
    pdfium(ws, &pdfs, &renders.join("pdfium"), &dpis)?;
    for dpi in dpis {
        let dir = renders.join("pdfjs").join(dpi.to_string()).join(&settings.matrix);
        pdfjs(ws, &matrix, &dir, dpi)?;
    }
    let golden = ws.root.join("fixtures/render-golden/annots-parity");
    let thresholds = ws.root.join(THRESHOLDS);
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

/// PDFium's renders of every PDF in `pdfs` at each of `dpis`, into `out/<dpi>/<stem>/`.
fn pdfium(ws: &Workspace, pdfs: &Path, out: &Path, dpis: &[u32]) -> Result<()> {
    let mut args = vec![
        ws.root.join("tools/render/pdfium.py").into_os_string(),
        pdfs.as_os_str().to_owned(),
        out.as_os_str().to_owned(),
    ];
    args.extend(dpis.iter().map(|d| d.to_string().into()));
    crate::run(&ws.root, &crate::harness::python(), &args)
}

/// pdf.js's renders of `pdf` at `dpi`, into `out/<page>.ppm`.
fn pdfjs(ws: &Workspace, pdf: &Path, out: &Path, dpi: u32) -> Result<()> {
    let script = ws.root.join("tools/render/pdfjs-render.mjs");
    let dpi = dpi.to_string();
    crate::run(
        &ws.root,
        "node",
        &[script.as_os_str(), pdf.as_os_str(), out.as_os_str(), dpi.as_ref()],
    )
}

/// `annots-parity settings`.
fn settings(ws: &Workspace) -> Result<Settings> {
    let output = Command::new("cargo")
        .args(tool_args(ws))
        .arg("settings")
        .current_dir(&ws.root)
        .output()
        .context("starting cargo")?;
    if !output.status.success() {
        bail!(
            "annots-parity settings failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    serde_json::from_slice(&output.stdout).context("annots-parity settings")
}

/// `cargo run` of `tools/render`'s `annots-parity` (its own workspace, built in release mode),
/// up to the `--` before its arguments.
fn tool_args(ws: &Workspace) -> Vec<std::ffi::OsString> {
    let manifest = ws.root.join(render::MANIFEST);
    let mut args: Vec<std::ffi::OsString> = ["run", "--manifest-path"].map(Into::into).to_vec();
    args.push(manifest.into_os_string());
    args.extend(["--locked", "--release", "-q", "--bin", "annots-parity", "--"].map(Into::into));
    args
}

/// Runs `annots-parity args…`.
fn tool(ws: &Workspace, args: &[&std::ffi::OsStr]) -> Result<()> {
    let mut all = tool_args(ws);
    all.extend(args.iter().map(|a| (*a).to_owned()));
    crate::run(&ws.root, "cargo", &all)
}

/// One file of `annots-parity move`'s report.
#[derive(Debug, Deserialize)]
struct Entry {
    result: Report,
    /// The folder holding `original.pdf` and `saved.pdf`.
    saved: Option<PathBuf>,
}

/// `tools/render/src/annots_move.rs`'s `Report`; `outcome` is its `Outcome`, by name.
#[derive(Debug, Deserialize)]
struct Report {
    outcome: String,
    moved: usize,
    popups: usize,
    locked: usize,
    not_editable: usize,
    problems: Vec<String>,
}

/// `annots-parity-corpus.toml`: each file's expected outcome.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    files: BTreeMap<String, String>,
}

/// The corpus edit test over the pdf.js corpus's `annotation-*.pdf` files.
fn corpus(ws: &Workspace, out: &Path, settings: &Settings) -> Result<()> {
    let inputs: Vec<(String, PathBuf)> = crate::corpus::verified_inputs(ws, &["pdfjs".to_owned()])?
        .into_iter()
        .filter(|(name, _)| is_annotation_file(name))
        .collect();
    let expected: Expected =
        toml::from_str(&fs::read_to_string(ws.root.join(EXPECTED))?).with_context(|| format!("parsing {EXPECTED}"))?;
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
    let mut failed = unlisted(&report, &expected);
    for (name, entry) in &report {
        let mut problems = entry.result.problems.clone();
        if let Some(saved) = &entry.saved {
            problems.extend(
                check_saved(ws, saved, settings.golden_dpi)
                    .err()
                    .map(|e| format!("{e:#}")),
            );
        }
        let want = expected.files.get(name).map_or("(not listed)", String::as_str);
        if entry.result.outcome != want {
            problems.push(format!("expected {want}"));
        }
        let r = &entry.result;
        let line = format!(
            "{}: {} moved, {} Popups with them, {} locked, {} not editable",
            r.outcome, r.moved, r.popups, r.locked, r.not_editable
        );
        if problems.is_empty() {
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
        "corpus edit test: {} files, each with its outcome in {EXPECTED}",
        report.len()
    ));
    Ok(())
}

/// Files of the corpus that `expected` doesn't list, and files it lists that the corpus lacks.
fn unlisted(report: &BTreeMap<String, Entry>, expected: &Expected) -> Vec<String> {
    let missing = expected
        .files
        .keys()
        .filter(|name| !report.contains_key(*name))
        .map(|name| format!("{name} (listed in {EXPECTED}, not in the corpus)"));
    let extra = report
        .keys()
        .filter(|name| !expected.files.contains_key(*name))
        .map(|name| format!("{name} (in the corpus, not listed in {EXPECTED})"));
    missing.chain(extra).collect()
}

/// `qpdf --check` on the saved file in `dir`, then PDFium, pdf.js and hayro drawing it beside
/// its original at `dpi` (`annots-parity check-saved`).
fn check_saved(ws: &Workspace, dir: &Path, dpi: u32) -> Result<()> {
    crate::roundtrip::qpdf_check(dir, &["saved.pdf".to_owned()])?;
    pdfium(ws, dir, &dir.join("pdfium"), &[dpi]).context("PDFium")?;
    for stem in ["original", "saved"] {
        let pdf = dir.join(format!("{stem}.pdf"));
        pdfjs(ws, &pdf, &dir.join("pdfjs").join(dpi.to_string()).join(stem), dpi).context("pdf.js")?;
    }
    let thresholds = ws.root.join(THRESHOLDS);
    tool(ws, &["check-saved".as_ref(), dir.as_os_str(), thresholds.as_os_str()])
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

    fn entry() -> Entry {
        serde_json::from_str(
            r#"{"result": {"outcome": "moved", "moved": 1, "popups": 0, "locked": 0, "not_editable": 0,
                "problems": []}, "saved": null}"#,
        )
        .unwrap()
    }

    fn expected(names: &[&str]) -> Expected {
        Expected {
            files: names.iter().map(|n| ((*n).to_owned(), "moved".to_owned())).collect(),
        }
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
    }

    #[test]
    fn parse_args_should_default_to_the_matrix_alone() {
        assert_eq!(parse_args(&[]).unwrap(), Args::default());
    }

    #[test]
    fn parse_args_should_refuse_anything_else() {
        assert!(parse_args(&args(&["--sample", "3"])).is_err());
    }

    #[test]
    fn a_pdfjs_annotation_file_is_moved() {
        assert!(is_annotation_file("pdfjs/test/pdfs/annotation-caret-ink.pdf"));
    }

    #[test]
    fn another_corpus_file_is_not() {
        assert!(!is_annotation_file("pdfjs/test/pdfs/alphatrans.pdf"));
    }

    #[test]
    fn a_link_file_is_not() {
        assert!(!is_annotation_file("pdfjs/test/pdfs/annotation-caret-ink.pdf.link"));
    }

    #[test]
    fn a_corpus_file_without_an_expected_outcome_fails() {
        let report = BTreeMap::from([("a.pdf".to_owned(), entry())]);
        assert_eq!(unlisted(&report, &expected(&[])).len(), 1);
    }

    #[test]
    fn an_expected_file_missing_from_the_corpus_fails() {
        assert_eq!(unlisted(&BTreeMap::new(), &expected(&["a.pdf"])).len(), 1);
    }

    #[test]
    fn a_corpus_matching_its_list_passes() {
        let report = BTreeMap::from([("a.pdf".to_owned(), entry())]);
        assert_eq!(unlisted(&report, &expected(&["a.pdf"])), Vec::<String>::new());
    }
}
