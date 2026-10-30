//! `cargo xtask render-golden [--update]` (ADR 0018 tier 8, todo B5c): runs the `lamina`
//! examples, renders every page with PDFium (`tools/render/pdfium.py`, pypdfium2 pinned in
//! `tools/render/requirements.txt`; `$LAMINA_PYTHON` or `python3`) at 72 and 150 dpi, then
//! runs `tools/render` (`render-golden`, with hayro, in release mode): each page's 72 dpi
//! renders against `fixtures/render-golden/<example>/<page>-<engine>.png`, and hayro against
//! PDFium at 150 dpi, with the limits in `fixtures/render-golden/thresholds.toml`. Failures
//! write expected | actual | diff images into `target/xtask/render-golden/diff/`.

use std::fs;

use anyhow::{Context, Result};

use crate::workspace::Workspace;
use crate::{examples, step};

/// Runs the render goldens; `update` rewrites the PNGs.
pub(crate) fn run(ws: &Workspace, update: bool) -> Result<()> {
    let out = ws.root.join("target/xtask/render-golden");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    let pdfs = out.join("pdf");
    examples::run_all(ws, &pdfs)?;
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
        "--locked",
        "--release",
        "-q",
        "-p",
        "render-golden",
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
    Ok(())
}
