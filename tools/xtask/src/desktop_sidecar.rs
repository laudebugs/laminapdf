//! `cargo xtask desktop-sidecar [--release | --debug]`: builds the desktop app's engine
//! binary and fetches PDFium, where `tauri.bundle.conf.json` bundles them from:
//!
//! - `apps/desktop/src-tauri/binaries/laminapdf-desktop-engine-<target triple>[.exe]`: Tauri's
//!   naming for an `externalBin` sidecar; it drops the triple when it copies the binary next to
//!   the app's own executable.
//! - `apps/desktop/src-tauri/resources/pdfium/`: the pinned PDFium library (`fetch-pdfium`) with
//!   its licence notices, which ship with every bundle (ADR 0004 §3).
//!
//! On macOS the engine is signed ad hoc with `apps/desktop/src-tauri/engine.entitlements`.
//!
//! Both folders are git-ignored. The profile is `--release` or `--debug`, else what the Tauri CLI
//! asked for (`TAURI_ENV_DEBUG` in its `beforeDevCommand`/`beforeBuildCommand`), else debug.
//! The host's target only: the universal macOS engine comes with code signing.

use std::env;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;
use crate::{pdfium, step};

/// The engine's package and binary name.
pub(crate) const ENGINE: &str = "laminapdf-desktop-engine";

pub(crate) fn run(ws: &Workspace, args: &[String]) -> Result<()> {
    let release = match args {
        [] => env::var("TAURI_ENV_DEBUG").is_ok_and(|debug| debug == "false"),
        [flag] if flag == "--release" => true,
        [flag] if flag == "--debug" => false,
        _ => bail!("usage: cargo xtask desktop-sidecar [--release | --debug]"),
    };
    let triple = host_triple()?;
    if let Ok(asked) = env::var("TAURI_ENV_TARGET_TRIPLE")
        && asked != triple
    {
        bail!("the Tauri CLI builds for {asked}, but desktop-sidecar builds the host's {triple} only");
    }
    let mut cargo = vec!["build", "--locked", "-p", ENGINE];
    if release {
        cargo.push("--release");
    }
    ws.cargo(&cargo)?;

    let exe = env::consts::EXE_SUFFIX;
    let profile = if release { "release" } else { "debug" };
    let built = ws.target_dir.join(profile).join(format!("{ENGINE}{exe}"));
    let app = ws.root.join("apps/desktop/src-tauri");
    let binaries = app.join("binaries");
    fs::create_dir_all(&binaries).with_context(|| format!("creating {}", binaries.display()))?;
    let sidecar = binaries.join(format!("{ENGINE}-{triple}{exe}"));
    fs::copy(&built, &sidecar).with_context(|| format!("copying {} to {}", built.display(), sidecar.display()))?;
    step(&format!("engine ({profile}) → {}", sidecar.display()));
    if cfg!(target_os = "macos") {
        sign_ad_hoc(&app, &sidecar)?;
    }

    let lib = pdfium::fetch(ws)?;
    let resources = app.join("resources/pdfium");
    if resources.exists() {
        fs::remove_dir_all(&resources).with_context(|| format!("removing {}", resources.display()))?;
    }
    fs::create_dir_all(&resources).with_context(|| format!("creating {}", resources.display()))?;
    copy_libraries(&lib, &resources)?;
    if let Some(unpacked) = lib.parent() {
        crate::util::copy_dir(&unpacked.join("licenses"), &resources.join("licenses"))?;
        let licence = unpacked.join("LICENSE");
        fs::copy(&licence, resources.join("LICENSE")).with_context(|| format!("copying {}", licence.display()))?;
    }
    step(&format!("PDFium {} → {}", pdfium::TAG, resources.display()));
    Ok(())
}

/// Signs the engine ad hoc with `engine.entitlements`, so dev builds run the binary the way a
/// release signs it (D6c then signs with the Developer ID and the hardened runtime, which an ad-hoc
/// signature can't use: library validation would refuse PDFium, which has no Team ID). The
/// sandbox itself is the engine's Seatbelt profile and needs no entitlement (ADR 0008 §5).
fn sign_ad_hoc(app: &Path, sidecar: &Path) -> Result<()> {
    let entitlements = app.join("engine.entitlements");
    crate::run(
        app,
        "codesign",
        &[
            "--force".as_ref(),
            "--sign".as_ref(),
            "-".as_ref(),
            "--identifier".as_ref(),
            "com.laminapdf.desktop.engine".as_ref(),
            "--entitlements".as_ref(),
            entitlements.as_os_str(),
            sidecar.as_os_str(),
        ],
    )
}

/// The host's target triple, from `rustc -vV`.
fn host_triple() -> Result<String> {
    let info = crate::util::stdout("rustc", &["-vV"]).context("running rustc -vV")?;
    crate::harness::host_triple(&info).context("rustc -vV printed no host line")
}

/// Copies the shared libraries in `from` (PDFium's `.dylib`, `.so` or `.dll`) into `to`.
fn copy_libraries(from: &Path, to: &Path) -> Result<()> {
    let mut copied = 0;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let path = entry?.path();
        let is_library = path
            .extension()
            .is_some_and(|ext| ext == "dylib" || ext == "so" || ext == "dll");
        if let (true, Some(name)) = (is_library, path.file_name()) {
            fs::copy(&path, to.join(name)).with_context(|| format!("copying {}", path.display()))?;
            copied += 1;
        }
    }
    if copied == 0 {
        bail!("no PDFium library in {}", from.display());
    }
    Ok(())
}
