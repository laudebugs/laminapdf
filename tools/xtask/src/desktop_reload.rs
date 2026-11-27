//! `cargo xtask desktop-reload [--gate]` (todo E6f): the desktop's touched-page reload after a
//! content edit, on files of about 1, 10 and 100 MB with native PDFium
//! (`crates/laminapdf-desktop-engine/examples/reload.rs`). It builds the engine in release mode,
//! fetches the pinned PDFium (`fetch-pdfium`), and runs the example, which writes its figures to
//! `target/desktop-reload/` and, with `--gate`, fails when a median is over its budget by more than
//! 20 % (the E6 plan's §6a). The table goes in the job summary whether or not the gate passed.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::util::summary;
use crate::workspace::Workspace;

const USAGE: &str = "usage: cargo xtask desktop-reload [--gate]";

/// `cargo xtask desktop-reload [--gate]`.
pub(crate) fn run(ws: &Workspace, args: &[String]) -> Result<()> {
    let gate = match args {
        [] => false,
        [flag] if flag == "--gate" => true,
        _ => bail!("{USAGE}"),
    };
    let pdfium = crate::pdfium::fetch(ws)?;
    ws.cargo(&["build", "--locked", "--release", "-p", "laminapdf-desktop-engine"])?;
    let engine = ws
        .root
        .join("target/release")
        .join(format!("laminapdf-desktop-engine{}", std::env::consts::EXE_SUFFIX));
    let out = ws.root.join("target/desktop-reload");
    let (engine, out, pdfium) = (
        engine.to_string_lossy().into_owned(),
        out.to_string_lossy().into_owned(),
        pdfium.to_string_lossy().into_owned(),
    );
    let mut example = vec![
        "run",
        "--locked",
        "--release",
        "-p",
        "laminapdf-desktop-engine",
        "--example",
        "reload",
        "--",
        "--engine",
        &engine,
        "--out",
        &out,
    ];
    if gate {
        example.push("--gate");
    }
    let passed = crate::run_env(
        &ws.root,
        "cargo",
        &example,
        &[(crate::render::PDFIUM_DIR_ENV, pdfium.as_str())],
    );
    if let Ok(table) = fs::read_to_string(Path::new(&out).join("reload.md")) {
        summary(&format!("## Desktop: content edit shown\n\n{table}"))?;
    }
    passed.context("the desktop reload missed a budget by more than 20 %")
}
