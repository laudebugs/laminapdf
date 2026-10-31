//! `cargo xtask render-golden [--update]` (ADR 0018 tier 8, todos B5c and C2b): renders every page of
//! the committed example goldens (`fixtures/golden/examples/*.pdf`, which `cargo xtask
//! examples` and `crates/lamina/tests/examples.rs` keep equal to the examples' output) with
//! PDFium (`tools/render/pdfium.py`, pypdfium2 pinned in `tools/render/requirements.txt`;
//! `$LAMINA_PYTHON` or `python3`) at 72 and 150 dpi, then runs `tools/render` (`render-golden`,
//! with hayro; its own workspace and lockfile, built here in release mode after its fmt,
//! clippy and tests): each page's 72 dpi renders against
//! `fixtures/render-golden/<example>/<page>-<engine>.png`, and hayro against PDFium at 150
//! dpi, with the limits in `fixtures/render-golden/thresholds.toml`. Failures write expected |
//! actual | diff images into `target/xtask/render-golden/diff/`. Then [`corpus`]: lamina-render's
//! PDFium and hayro adapters on `fixtures/corpus/` against `fixtures/golden/render/`.

use std::fs;

use anyhow::{Context, Result};

use crate::step;
use crate::workspace::Workspace;

/// The render tool's manifest: its own workspace, outside the root one.
pub(crate) const MANIFEST: &str = "tools/render/Cargo.toml";

/// Runs the render goldens; `update` rewrites the PNGs.
pub(crate) fn run(ws: &Workspace, update: bool) -> Result<()> {
    let out = ws.root.join("target/xtask/render-golden");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    // The tool is outside the root workspace, so the fast lane's fmt, clippy and tests never
    // see it: check it here, where it is built anyway.
    let manifest = ws.root.join(MANIFEST).to_string_lossy().into_owned();
    ws.cargo(&["fmt", "--manifest-path", &manifest, "--check"])?;
    ws.cargo(&[
        "clippy",
        "--manifest-path",
        &manifest,
        "--locked",
        "--release",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])?;
    ws.cargo(&["test", "--manifest-path", &manifest, "--locked", "--release", "-q"])?;
    let pdfs = ws.root.join("fixtures/golden/examples");
    let pdfium = out.join("pdfium");
    let script = ws.root.join("tools/render/pdfium.py");
    crate::run(
        &ws.root,
        &crate::harness::python(),
        &[
            script.as_os_str(),
            pdfs.as_os_str(),
            pdfium.as_os_str(),
            "72".as_ref(),
            "150".as_ref(),
        ],
    )?;
    let golden = ws.root.join("fixtures/render-golden");
    let report = out.join("diff");
    let (pdfs, pdfium, golden, report) = (
        pdfs.to_string_lossy().into_owned(),
        pdfium.to_string_lossy().into_owned(),
        golden.to_string_lossy().into_owned(),
        report.to_string_lossy().into_owned(),
    );
    let mut args = vec![
        "run",
        "--manifest-path",
        &manifest,
        "--locked",
        "--release",
        "-q",
        "--",
        &pdfs,
        &pdfium,
        &golden,
        &report,
    ];
    if update {
        args.push("--update");
    }
    ws.cargo(&args)?;
    if update {
        step("Render goldens rewritten; review `git diff --stat fixtures/render-golden` and the PNGs");
    }
    corpus(ws, update)
}

/// Part 3 (todo C2b, ADR 0004): `lamina-render`'s backends on `fixtures/corpus/`, against
/// `fixtures/golden/render/<backend>/`. Fetches the pinned PDFium, then runs
/// `lamina-render-pdfium`'s ignored tests (goldens, determinism, errors, tile and rotation
/// geometry, page sizes against lamina), or with `update` its `goldens` example, which rewrites
/// the PNGs. The fast lane never runs these: they need the library, and the goldens are
/// written on macOS arm64, the CI runners' platform.
fn corpus(ws: &Workspace, update: bool) -> Result<()> {
    let dir = crate::pdfium::fetch(ws)?;
    let dir = dir.to_string_lossy().into_owned();
    let env = [(PDFIUM_DIR_ENV, dir.as_str())];
    if update {
        crate::run_env(
            &ws.root,
            "cargo",
            &[
                "run",
                "--release",
                "-q",
                "-p",
                "lamina-render-pdfium",
                "--example",
                "goldens",
            ],
            &env,
        )?;
        step("Corpus render goldens rewritten; review `git diff --stat fixtures/golden/render` and the PNGs");
        Ok(())
    } else {
        crate::run_env(
            &ws.root,
            "cargo",
            &["test", "-p", "lamina-render-pdfium", "--locked", "--", "--ignored"],
            &env,
        )
    }
}

/// The variable `lamina-render-pdfium` reads the library folder from.
pub(crate) const PDFIUM_DIR_ENV: &str = "LAMINA_PDFIUM_DIR";
