//! `cargo xtask desktop-sidecar [--release | --debug] [--e2e] [--universal]`: builds the desktop app's engine
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
//! The host's target only, except for the Mac App Store build (ADR 0014 §11, D8c): with
//! `--universal`, or when the Tauri CLI builds `universal-apple-darwin`, the engine and PDFium's
//! library are built or fetched for both Mac architectures and joined with `lipo`, as
//! `laminapdf-desktop-engine-universal-apple-darwin` and one `libpdfium.dylib`. `--e2e` builds the
//! engine the end-to-end tests run, with its `e2e` feature (it obeys their forced panic).
//!
//! A release engine links with fat LTO in one codegen unit ([`RELEASE_SIZE`]), which only this
//! build sets: the rest of the workspace's release builds keep thin LTO for their build times.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

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
    /// `--universal`: the engine and PDFium for both Mac architectures, on macOS.
    universal: bool,
}

fn parse(args: &[String]) -> Result<Options> {
    let mut options = Options::default();
    for arg in args {
        match arg.as_str() {
            "--release" if options.release.is_none() => options.release = Some(true),
            "--debug" if options.release.is_none() => options.release = Some(false),
            "--e2e" if !options.e2e => options.e2e = true,
            "--universal" if !options.universal => options.universal = true,
            _ => bail!("usage: cargo xtask desktop-sidecar [--release | --debug] [--e2e] [--universal]"),
        }
    }
    Ok(options)
}

/// Whether the Tauri CLI that ran this hook builds for release: inside a hook (`platform` set)
/// unless `debug` is `true`; outside Tauri, never.
fn tauri_wants_release(platform: Option<&str>, debug: Option<&str>) -> bool {
    platform.is_some() && debug != Some("true")
}

/// The release engine's link settings, on top of the workspace's release profile: fat LTO in one
/// codegen unit. Against thin LTO's 16 units they took the stripped macOS engine from 9,173,472
/// to 6,195,968 bytes (gzip: 3,609,127 to 2,914,255) on 2026-11-20, when engine growth had taken
/// the Linux x86-64 `.deb` over the desktop plan's 16 MB budget (§4). They change only how the
/// code is optimised across crates, not what it does. Cargo keeps these artifacts apart from the
/// plain release profile's, so the two don't rebuild each other.
const RELEASE_SIZE: [&str; 4] = [
    "--config",
    "profile.release.lto=\"fat\"",
    "--config",
    "profile.release.codegen-units=1",
];

/// The Tauri CLI's name for the two Mac architectures in one app.
const UNIVERSAL: &str = "universal-apple-darwin";

/// The two Mac architectures a universal build joins: the Rust target and the architecture name
/// `std::env::consts::ARCH` (and `pdfium.rs`'s table) uses.
const MAC_ARCHES: [(&str, &str); 2] = [("aarch64-apple-darwin", "aarch64"), ("x86_64-apple-darwin", "x86_64")];

/// The arguments of the engine's `cargo build`, for `target` or the host's. Only the engine that
/// ships (release, without the `e2e` feature) takes [`RELEASE_SIZE`]: the end-to-end engine isn't
/// installed, and a fat link would only slow the suites.
fn cargo_args(release: bool, e2e: bool, target: Option<&'static str>) -> Vec<&'static str> {
    let mut cargo = vec!["build", "--locked", "-p", ENGINE];
    if let Some(target) = target {
        cargo.extend(["--target", target]);
    }
    if release {
        cargo.push("--release");
        if !e2e {
            cargo.extend(RELEASE_SIZE);
        }
    }
    if e2e {
        cargo.extend(["--features", "e2e"]);
    }
    cargo
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
    let asked = env::var("TAURI_ENV_TARGET_TRIPLE").ok();
    let universal = options.universal || asked.as_deref() == Some(UNIVERSAL);
    if universal && !cfg!(target_os = "macos") {
        bail!("a universal engine is for macOS only");
    }
    let triple = if universal {
        UNIVERSAL.to_owned()
    } else {
        host_triple()?
    };
    if let Some(asked) = asked
        && asked != triple
    {
        bail!("the Tauri CLI builds for {asked}, but desktop-sidecar builds {triple} only");
    }

    let exe = env::consts::EXE_SUFFIX;
    let profile = if release { "release" } else { "debug" };
    let app = ws.root.join("apps/desktop/src-tauri");
    let binaries = app.join("binaries");
    fs::create_dir_all(&binaries).with_context(|| format!("creating {}", binaries.display()))?;
    let built = if universal {
        universal_engine(ws, release, options.e2e, profile, &binaries)?
    } else {
        ws.cargo(&cargo_args(release, options.e2e, None))?;
        ws.target_dir.join(profile).join(format!("{ENGINE}{exe}"))
    };
    let sidecar = binaries.join(format!("{ENGINE}-{triple}{exe}"));
    fs::copy(&built, &sidecar).with_context(|| format!("copying {} to {}", built.display(), sidecar.display()))?;
    step(&format!("engine ({profile}) → {}", sidecar.display()));
    if cfg!(target_os = "macos") {
        sign_ad_hoc(&app, &sidecar)?;
    }

    let resources = app.join("resources/pdfium");
    if resources.exists() {
        fs::remove_dir_all(&resources).with_context(|| format!("removing {}", resources.display()))?;
    }
    fs::create_dir_all(&resources).with_context(|| format!("creating {}", resources.display()))?;
    let lib = if universal {
        universal_pdfium(ws, &resources)?
    } else {
        let lib = pdfium::fetch(ws)?;
        copy_libraries(&lib, &resources)?;
        lib
    };
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

/// Builds the engine for both Mac architectures and joins them into one fat binary; returns its
/// path. The runner's own CPU builds both: the other target needs `rustup target add`. Each
/// architecture's engine also goes into `binaries` under its own triple, where Tauri's build
/// script looks for it while it compiles the app for that architecture; the bundler takes the
/// universal one.
fn universal_engine(ws: &Workspace, release: bool, e2e: bool, profile: &str, binaries: &Path) -> Result<PathBuf> {
    let mut parts = Vec::new();
    for (triple, _) in MAC_ARCHES {
        ws.cargo(&cargo_args(release, e2e, Some(triple)))?;
        let part = ws.target_dir.join(triple).join(profile).join(ENGINE);
        fs::copy(&part, binaries.join(format!("{ENGINE}-{triple}")))
            .with_context(|| format!("copying {}", part.display()))?;
        parts.push(part);
    }
    let dir = ws.target_dir.join(UNIVERSAL).join(profile);
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let fat = dir.join(ENGINE);
    lipo(&parts, &fat)?;
    Ok(fat)
}

/// Fetches PDFium for both Mac architectures and joins each library into `resources`; returns
/// the first architecture's folder, which holds the licence notices both builds share.
fn universal_pdfium(ws: &Workspace, resources: &Path) -> Result<PathBuf> {
    let [(_, first), (_, second)] = MAC_ARCHES;
    let (one, two) = (
        pdfium::fetch_for(ws, "macos", first)?,
        pdfium::fetch_for(ws, "macos", second)?,
    );
    let mut joined = 0;
    for entry in fs::read_dir(&one).with_context(|| format!("reading {}", one.display()))? {
        let path = entry?.path();
        if let (true, Some(name)) = (path.extension().is_some_and(|ext| ext == "dylib"), path.file_name()) {
            lipo(&[path.clone(), two.join(name)], &resources.join(name))?;
            joined += 1;
        }
    }
    if joined == 0 {
        bail!("no PDFium library in {}", one.display());
    }
    Ok(one)
}

/// Joins `inputs` (one per architecture) into the universal file `output`.
fn lipo(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let mut args = vec![OsStr::new("-create")];
    args.extend(inputs.iter().map(|path| path.as_os_str()));
    args.extend([OsStr::new("-output"), output.as_os_str()]);
    let cwd = output.parent().unwrap_or(Path::new("."));
    crate::run(cwd, "lipo", &args)
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
    use super::{MAC_ARCHES, Options, RELEASE_SIZE, UNIVERSAL, cargo_args, developer_id, parse, tauri_wants_release};

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn only_the_shipped_release_engine_should_link_with_fat_lto() {
        let has_size = |args: &[&str]| RELEASE_SIZE.iter().all(|flag| args.contains(flag));
        let has_none = |args: &[&str]| RELEASE_SIZE.iter().all(|flag| !args.contains(flag));
        assert!(has_size(&cargo_args(true, false, None)), "a release engine");
        assert!(has_none(&cargo_args(false, false, None)), "a debug engine");
        assert!(has_none(&cargo_args(false, true, None)), "a debug e2e engine");
        assert!(has_none(&cargo_args(true, true, None)), "a release e2e engine");
    }

    #[test]
    fn the_flags_should_be_read_in_any_order() {
        assert_eq!(
            parse(&args(&["--e2e", "--debug"])).unwrap(),
            Options {
                release: Some(false),
                e2e: true,
                universal: false
            }
        );
    }

    #[test]
    fn a_universal_build_should_name_both_mac_architectures_and_pass_the_target_on() {
        assert!(parse(&args(&["--release", "--universal"])).unwrap().universal);
        assert_eq!(
            MAC_ARCHES.map(|(triple, _)| triple),
            ["aarch64-apple-darwin", "x86_64-apple-darwin"]
        );
        for (triple, arch) in MAC_ARCHES {
            assert!(triple.starts_with(arch), "{triple} is {arch}'s target");
            let cargo = cargo_args(true, false, Some(triple));
            assert!(cargo.windows(2).any(|pair| pair == ["--target", triple]));
        }
        assert_eq!(UNIVERSAL, "universal-apple-darwin", "the Tauri CLI's name for it");
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
