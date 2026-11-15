//! `cargo xtask desktop-sidecar [--release | --debug] [--e2e]`: builds the desktop app's engine
//! binary and fetches PDFium, where `tauri.bundle.conf.json` bundles them from:
//!
//! - `apps/desktop/src-tauri/binaries/laminapdf-desktop-engine-<target triple>[.exe]`: Tauri's
//!   naming for an `externalBin` sidecar; it drops the triple when it copies the binary next to
//!   the app's own executable.
//! - `apps/desktop/src-tauri/resources/pdfium/`: the pinned PDFium library (`fetch-pdfium`) with
//!   its licence notices, which ship with every bundle (ADR 0004 §3).
//!
//! On macOS the engine is signed ad hoc with `apps/desktop/src-tauri/engine.entitlements`, for dev
//! builds that run it unbundled; Tauri signs it again as it bundles. When `APPLE_SIGNING_IDENTITY`
//! names a Developer ID (the release build, D6c), PDFium's library is signed with it too: Tauri
//! signs the app and its sidecar but not the files in `resources/`, and the hardened runtime's
//! library validation loads only libraries with the engine's own Team ID.
//!
//! Both folders are git-ignored. The profile is `--release` or `--debug`, else what the Tauri CLI
//! asked for, else debug. The CLI's `beforeDevCommand`/`beforeBuildCommand` set
//! `TAURI_ENV_PLATFORM`, and `TAURI_ENV_DEBUG=true` only for a debug build: a release build leaves
//! `TAURI_ENV_DEBUG` unset (Tauri CLI 2.12, checked after release builds had shipped the
//! debug engine).
//! The host's target only: the universal macOS engine comes with code signing. `--e2e` builds the
//! engine the end-to-end tests run, with its `e2e` feature (it obeys their forced panic).

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;
use crate::{pdfium, step};

/// What the command line asked for.
#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    /// `--release` or `--debug`; `None` follows the Tauri CLI.
    release: Option<bool>,
    /// `--e2e`: the end-to-end tests' engine.
    e2e: bool,
}

fn parse(args: &[String]) -> Result<Options> {
    let mut options = Options::default();
    for arg in args {
        match arg.as_str() {
            "--release" if options.release.is_none() => options.release = Some(true),
            "--debug" if options.release.is_none() => options.release = Some(false),
            "--e2e" if !options.e2e => options.e2e = true,
            _ => bail!("usage: cargo xtask desktop-sidecar [--release | --debug] [--e2e]"),
        }
    }
    Ok(options)
}

/// Whether the Tauri CLI that ran this hook builds for release: inside a hook (`platform` set)
/// unless `debug` is `true`; outside Tauri, never.
fn tauri_wants_release(platform: Option<&str>, debug: Option<&str>) -> bool {
    platform.is_some() && debug != Some("true")
}

/// The engine's package and binary name.
pub(crate) const ENGINE: &str = "laminapdf-desktop-engine";

pub(crate) fn run(ws: &Workspace, args: &[String]) -> Result<()> {
    let options = parse(args)?;
    let release = options.release.unwrap_or_else(|| {
        tauri_wants_release(
            env::var("TAURI_ENV_PLATFORM").ok().as_deref(),
            env::var("TAURI_ENV_DEBUG").ok().as_deref(),
        )
    });
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
    if options.e2e {
        cargo.extend(["--features", "e2e"]);
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
    if cfg!(target_os = "macos")
        && let Some(identity) = developer_id(env::var("APPLE_SIGNING_IDENTITY").ok().as_deref())
    {
        sign_libraries(&resources, identity)?;
    }
    step(&format!("PDFium {} → {}", pdfium::TAG, resources.display()));
    Ok(())
}

/// The Developer ID `APPLE_SIGNING_IDENTITY` names, if it names one: unset, empty and Tauri's ad-hoc
/// `-` mean an ad-hoc signature.
fn developer_id(identity: Option<&str>) -> Option<&str> {
    identity.map(str::trim).filter(|id| !id.is_empty() && *id != "-")
}

/// Signs the engine ad hoc with `engine.entitlements`, so dev builds run the binary the way a
/// release signs it, without the hardened runtime: its library validation would refuse PDFium, which
/// has no Team ID in an ad-hoc build. The sandbox itself is the engine's Seatbelt profile and needs
/// no entitlement (ADR 0008 §5).
fn sign_ad_hoc(app: &Path, sidecar: &Path) -> Result<()> {
    let entitlements = app.join("engine.entitlements");
    crate::run(
        app,
        "codesign",
        &[
            OsStr::new("--force"),
            OsStr::new("--sign"),
            OsStr::new("-"),
            OsStr::new("--identifier"),
            OsStr::new("com.laminapdf.desktop.engine"),
            OsStr::new("--entitlements"),
            entitlements.as_os_str(),
            sidecar.as_os_str(),
        ],
    )
}

/// Signs each library in `dir` (PDFium's) with the Developer ID `identity` and a secure timestamp.
fn sign_libraries(dir: &Path, identity: &str) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "dylib") {
            crate::run(
                dir,
                "codesign",
                &[
                    OsStr::new("--force"),
                    OsStr::new("--sign"),
                    OsStr::new(identity),
                    OsStr::new("--timestamp"),
                    path.as_os_str(),
                ],
            )?;
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{Options, developer_id, parse, tauri_wants_release};

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn the_flags_should_be_read_in_any_order() {
        assert_eq!(
            parse(&args(&["--e2e", "--debug"])).unwrap(),
            Options {
                release: Some(false),
                e2e: true
            }
        );
    }

    #[test]
    fn both_profiles_or_an_unknown_flag_should_be_refused() {
        assert!(parse(&args(&["--release", "--debug"])).is_err() && parse(&args(&["--fast"])).is_err());
    }

    #[test]
    fn only_a_named_identity_should_be_a_developer_id() {
        let id = "Developer ID Application: A Person (35KBR7UHPZ)";
        assert_eq!(
            [None, Some(""), Some(" "), Some("-"), Some(id)].map(developer_id),
            [None, None, None, None, Some(id)]
        );
    }

    #[test]
    fn tauri_wants_release_should_follow_the_cli_hooks_environment() {
        // tauri build: TAURI_ENV_PLATFORM set, TAURI_ENV_DEBUG unset; --debug and dev: "true".
        assert!(tauri_wants_release(Some("darwin"), None));
        assert!(!tauri_wants_release(Some("darwin"), Some("true")));
        assert!(!tauri_wants_release(None, None));
    }
}
