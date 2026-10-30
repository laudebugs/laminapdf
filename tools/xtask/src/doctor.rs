//! External tools xtask shells out to, pinned in one place (ADR 0015, ADR 0019).
//!
//! Tasks call [`require`] before using a tool, so an off-pin version fails with the fix command
//! instead of producing different results. `cargo xtask doctor` checks them all, and
//! `cargo xtask doctor --install` runs the fix commands that are safe to automate (`cargo install`).
//! CI installs the same versions (`.github/workflows/ci.yml`, `nightly.yml`); keep them in sync.

use std::fs;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

/// cargo-deny release (`cargo xtask deny`, `check rust`).
const CARGO_DENY_VERSION: &str = "0.20.2";
/// cargo-nextest release (`cargo xtask test`; plain `cargo test` is the fallback).
const CARGO_NEXTEST_VERSION: &str = "0.9.150";
/// cargo-fuzz release (`cargo xtask fuzz`, `.github/workflows/nightly.yml`); it runs on nightly.
const CARGO_FUZZ_VERSION: &str = "0.13.2";
/// qpdf release for the nightly differential (`cargo xtask differential --live`; the nightly
/// workflow downloads this release's binaries, checked by SHA-256), the round-trip suite
/// (`cargo xtask roundtrip`) and the committed `qpdf --check` facts (`tools/fixtures/text_facts.py`,
/// `fixtures/encrypted/`; tests read the facts, so only regenerating them needs it).
pub(crate) const QPDF_VERSION: &str = "12.4.2";
/// Poppler release behind the committed `pdftotext` facts (`tools/fixtures/text_facts.py`,
/// todo B4d; tests read the facts, so only regenerating them needs it).
/// `crates/lamina-text/tests/extraction.rs` checks the facts were written with it.
pub(crate) const POPPLER_VERSION: &str = "26.10.0";
/// Binaryen release for `wasm-opt`. Its output is part of the shipped .wasm, so bump it deliberately
/// and review the wasm-opt feature list (ADR 0002) whenever this or `rust-toolchain.toml` changes.
pub(crate) const BINARYEN_VERSION: &str = "133";
/// twiggy release for the size reports of `cargo xtask size` (the top-items diff when a WASM
/// module grows more than 5 %).
const TWIGGY_VERSION: &str = "0.8.0";
/// HarfBuzz release behind the shaping goldens (`fixtures/shaping/`, written by
/// `tools/shaping-goldens/goldens.py` with `hb-shape`); `golden --update` skips them on
/// another version. `crates/lamina-text/tests/shaping_goldens.rs` checks the script's pin.
pub(crate) const HB_SHAPE_VERSION: &str = "14.3.0";
/// pdfjs-dist release behind the pdf.js facts (`tools/fixtures/pdfjs_text.mjs`,
/// `tools/corpus-facts/pdfjs-operators.mjs`): an npm package installed in a scratch
/// directory, so `doctor` can't probe it; `crates/lamina-text/tests/extraction.rs` checks the
/// scripts' pins against this one.
const PDFJS_DIST_VERSION: &str = "6.4.299";
/// The oldest Node major we support (`engines` in package.json; `.nvmrc` pins the one we develop on).
const MIN_NODE_MAJOR: u32 = 22;

/// Which versions of a tool are acceptable.
enum Pin {
    /// Exactly this version (compared with the version token of the tool's output).
    Exact(String),
    /// Any version whose major is at least this.
    MinMajor(u32),
    /// Any version (nothing to pin against yet).
    Any,
}

/// A pinned external tool.
pub(crate) struct Tool {
    name: &'static str,
    /// Command that prints the version.
    version_command: &'static [&'static str],
    pin: Pin,
    /// How to install the pinned version; `cargo install …` commands are run by `doctor --install`.
    fix: String,
    /// `None` when every task needs it; otherwise what needs it.
    optional_for: Option<&'static str>,
}

/// What [`Tool::probe`] found. The strings are the tool's version output.
pub(crate) enum Probe {
    Missing,
    OffPin(String),
    Ok(String),
}

impl Tool {
    pub(crate) fn probe(&self, ws: &Workspace) -> Probe {
        let Some((program, args)) = self.version_command.split_first() else {
            return Probe::Missing;
        };
        let output = Command::new(crate::program_path(program))
            .args(args)
            .current_dir(&ws.root)
            .output();
        let version = match output {
            // Some tools (poppler's `pdftotext -v`) print their version on stderr.
            Ok(out) if out.status.success() && out.stdout.is_empty() => {
                String::from_utf8_lossy(&out.stderr).trim().to_owned()
            }
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_owned(),
            _ => return Probe::Missing,
        };
        if self.accepts(&version) {
            Probe::Ok(version)
        } else {
            Probe::OffPin(version)
        }
    }

    fn accepts(&self, output: &str) -> bool {
        let found = version_token(output);
        match &self.pin {
            Pin::Exact(want) => found == Some(want.as_str()),
            Pin::MinMajor(min) => found.and_then(major).is_some_and(|m| m >= *min),
            Pin::Any => true,
        }
    }

    fn wanted(&self) -> String {
        match &self.pin {
            Pin::Exact(want) => want.clone(),
            Pin::MinMajor(min) => format!("{min}.x or newer"),
            Pin::Any => "any version".to_owned(),
        }
    }

    /// The error for a tool that is not usable as found.
    pub(crate) fn problem(&self, probe: &Probe) -> String {
        match probe {
            Probe::Ok(_) => format!("{} is fine", self.name),
            Probe::Missing => format!("{} is not installed; run: {}", self.name, self.fix),
            Probe::OffPin(found) => format!("{} is {found}, want {}; run: {}", self.name, self.wanted(), self.fix),
        }
    }

    /// The `cargo install …` arguments of the fix, if the fix is a cargo install.
    fn cargo_install_args(&self) -> Option<Vec<&str>> {
        let rest = self.fix.strip_prefix("cargo ")?;
        rest.starts_with("install ").then(|| rest.split_whitespace().collect())
    }
}

pub(crate) fn cargo_deny() -> Tool {
    Tool {
        name: "cargo-deny",
        version_command: &["cargo", "deny", "--version"],
        pin: Pin::Exact(CARGO_DENY_VERSION.to_owned()),
        fix: format!("cargo install cargo-deny --locked --version {CARGO_DENY_VERSION}"),
        optional_for: None,
    }
}

pub(crate) fn cargo_nextest() -> Tool {
    Tool {
        name: "cargo-nextest",
        version_command: &["cargo", "nextest", "--version"],
        pin: Pin::Exact(CARGO_NEXTEST_VERSION.to_owned()),
        fix: format!("cargo install cargo-nextest --locked --version {CARGO_NEXTEST_VERSION}"),
        optional_for: Some("faster tests; cargo test is the fallback"),
    }
}

pub(crate) fn cargo_fuzz() -> Tool {
    Tool {
        name: "cargo-fuzz",
        version_command: &["cargo", "fuzz", "--version"],
        pin: Pin::Exact(CARGO_FUZZ_VERSION.to_owned()),
        fix: format!("cargo install cargo-fuzz --locked --version {CARGO_FUZZ_VERSION}"),
        optional_for: Some("cargo xtask fuzz (also needs `rustup toolchain install nightly`)"),
    }
}

pub(crate) fn qpdf() -> Tool {
    Tool {
        name: "qpdf",
        version_command: &["qpdf", "--version"],
        pin: Pin::Exact(QPDF_VERSION.to_owned()),
        fix: format!("install qpdf {QPDF_VERSION} from https://github.com/qpdf/qpdf/releases (or brew install qpdf)"),
        optional_for: Some(
            "cargo xtask differential --live (the nightly lane), cargo xtask roundtrip and regenerating fixture facts \
             (tools/fixtures/text_facts.py, fixtures/encrypted/)",
        ),
    }
}

/// Every tool, with pins read from the repository where they live (toolchain file, package.json, Cargo.lock).
fn all_tools(ws: &Workspace) -> Result<Vec<Tool>> {
    let channel = toolchain_channel(ws)?;
    let pnpm = package_manager_pnpm(ws)?;
    let wasm_bindgen = locked_version(ws, "wasm-bindgen")?;
    Ok(vec![
        Tool {
            name: "rustc",
            version_command: &["rustc", "--version"],
            pin: Pin::Exact(channel),
            fix: "rustup show  # installs the toolchain pinned in rust-toolchain.toml".to_owned(),
            optional_for: None,
        },
        Tool {
            name: "node",
            version_command: &["node", "--version"],
            pin: Pin::MinMajor(MIN_NODE_MAJOR),
            fix: "nvm install  # or any version manager reading .nvmrc".to_owned(),
            optional_for: None,
        },
        Tool {
            name: "pnpm",
            version_command: &["pnpm", "--version"],
            fix: format!("corepack enable  # or: npm install -g pnpm@{pnpm}"),
            pin: Pin::Exact(pnpm),
            optional_for: None,
        },
        cargo_deny(),
        cargo_nextest(),
        cargo_fuzz(),
        qpdf(),
        pdftotext(),
        hb_shape(),
        wasm_bindgen_tool(wasm_bindgen),
        wasm_opt(),
        twiggy(),
    ])
}

/// wasm-bindgen-cli at exactly the `wasm-bindgen` crate's version in Cargo.lock (it brings
/// `wasm-bindgen-test-runner`).
pub(crate) fn wasm_bindgen(ws: &Workspace) -> Result<Tool> {
    Ok(wasm_bindgen_tool(locked_version(ws, "wasm-bindgen")?))
}

fn wasm_bindgen_tool(locked: Option<String>) -> Tool {
    Tool {
        name: "wasm-bindgen-cli",
        version_command: &["wasm-bindgen", "--version"],
        fix: match &locked {
            Some(v) => format!("cargo install wasm-bindgen-cli --locked --version {v}"),
            None => "not needed until a crate depends on wasm-bindgen".to_owned(),
        },
        pin: locked.map_or(Pin::Any, Pin::Exact),
        optional_for: Some("cargo xtask wasm (lamina-wasm); must match the wasm-bindgen crate in Cargo.lock"),
    }
}

/// binaryen's `wasm-opt` on `PATH`. Optional: `cargo xtask wasm` downloads the pinned release
/// into `target/tools/` (checked against its SHA-256) when `PATH` has no matching one.
pub(crate) fn wasm_opt() -> Tool {
    Tool {
        name: "wasm-opt",
        version_command: &["wasm-opt", "--version"],
        pin: Pin::Exact(BINARYEN_VERSION.to_owned()),
        fix: format!(
            "nothing to do: cargo xtask wasm downloads binaryen version_{BINARYEN_VERSION} into target/tools/"
        ),
        optional_for: Some("cargo xtask wasm (fetched into target/tools/ when missing)"),
    }
}

/// twiggy, for the top-items diff of `cargo xtask size`.
pub(crate) fn twiggy() -> Tool {
    Tool {
        name: "twiggy",
        version_command: &["twiggy", "--version"],
        pin: Pin::Exact(TWIGGY_VERSION.to_owned()),
        fix: format!("cargo install twiggy --locked --version {TWIGGY_VERSION}"),
        optional_for: Some("cargo xtask size (the top-items diff when a WASM module grows more than 5 %)"),
    }
}

/// Poppler's `pdftotext` at [`POPPLER_VERSION`] (optional: only regenerating the text
/// extraction facts uses it).
pub(crate) fn pdftotext() -> Tool {
    Tool {
        name: "pdftotext",
        version_command: &["pdftotext", "-v"],
        pin: Pin::Exact(POPPLER_VERSION.to_owned()),
        fix: format!("brew install poppler  # or a poppler {POPPLER_VERSION} release"),
        optional_for: Some("regenerating the text extraction facts (tools/fixtures/text_facts.py)"),
    }
}

/// HarfBuzz's `hb-shape` at [`HB_SHAPE_VERSION`] (optional: only `golden --update` uses it).
pub(crate) fn hb_shape() -> Tool {
    Tool {
        name: "hb-shape",
        version_command: &["hb-shape", "--version"],
        pin: Pin::Exact(HB_SHAPE_VERSION.to_owned()),
        fix: format!("brew install harfbuzz  # or a HarfBuzz {HB_SHAPE_VERSION} release"),
        optional_for: Some("regenerating the shaping goldens (tools/shaping-goldens/goldens.py)"),
    }
}

/// Fails unless `tool` is installed at its pinned version, naming the fix command.
pub(crate) fn require(ws: &Workspace, tool: &Tool) -> Result<()> {
    match tool.probe(ws) {
        Probe::Ok(_) => Ok(()),
        probe => bail!("{}", tool.problem(&probe)),
    }
}

/// What `doctor` does with a tool, given what it found and whether `--install` was passed.
#[derive(Debug, PartialEq, Eq)]
enum Action {
    /// Installed at the pinned version.
    Ok,
    /// Run the `cargo install` fix.
    Install,
    /// Optional and missing: report it, not an error.
    Note,
    /// Report it as a problem.
    Fail,
}

fn action(probe: &Probe, optional: bool, install: bool, installable: bool) -> Action {
    match probe {
        Probe::Ok(_) => Action::Ok,
        // --install fixes every tool it can, optional ones included.
        _ if install && installable => Action::Install,
        Probe::Missing if optional => Action::Note,
        _ => Action::Fail,
    }
}

/// `cargo xtask doctor [--install]`: reports every tool; fails on a missing required or off-pin tool.
pub(crate) fn run(ws: &Workspace, install: bool) -> Result<()> {
    let mut failures = Vec::new();
    for tool in all_tools(ws)? {
        let probe = tool.probe(ws);
        let install_args = tool.cargo_install_args();
        match action(&probe, tool.optional_for.is_some(), install, install_args.is_some()) {
            Action::Ok => {
                if let Probe::Ok(version) = &probe {
                    println!("✓ {:<17} {version}", tool.name);
                }
            }
            Action::Install => {
                println!("… {:<17} installing: {}", tool.name, tool.fix);
                crate::run(&ws.root, "cargo", &install_args.unwrap_or_default())?;
            }
            Action::Note => {
                let why = tool.optional_for.unwrap_or_default();
                println!(
                    "- {:<17} not installed (optional: {why})\n  {:<17} {}",
                    tool.name, "", tool.fix
                );
            }
            Action::Fail => {
                println!("✗ {}", tool.problem(&probe));
                failures.push(tool.name);
            }
        }
    }
    println!(
        "- {:<17} {PDFJS_DIST_VERSION} (npm; not probed: install it in a scratch directory for the pdf.js facts scripts)",
        "pdfjs-dist"
    );
    if failures.is_empty() {
        Ok(())
    } else {
        bail!("doctor found problems with: {}", failures.join(", "))
    }
}

/// `[toolchain] channel` in `rust-toolchain.toml`.
fn toolchain_channel(ws: &Workspace) -> Result<String> {
    let text = fs::read_to_string(ws.root.join("rust-toolchain.toml")).context("reading rust-toolchain.toml")?;
    parse_channel(&text)
}

fn parse_channel(text: &str) -> Result<String> {
    let table: toml::Table = toml::from_str(text).context("rust-toolchain.toml is not valid TOML")?;
    table
        .get("toolchain")
        .and_then(|t| t.get("channel"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .context("rust-toolchain.toml has no [toolchain] channel")
}

/// The pnpm version from the root package.json `packageManager` field (`pnpm@X.Y.Z`).
fn package_manager_pnpm(ws: &Workspace) -> Result<String> {
    let text = fs::read_to_string(ws.root.join("package.json")).context("reading package.json")?;
    let json: serde_json::Value = serde_json::from_str(&text).context("package.json is not JSON")?;
    json.get("packageManager")
        .and_then(serde_json::Value::as_str)
        .and_then(|pm| pm.strip_prefix("pnpm@"))
        // Corepack may append a hash: pnpm@11.20.0+sha512.…
        .map(|v| v.split('+').next().unwrap_or(v).to_owned())
        .context("package.json packageManager must be pnpm@<version>")
}

/// The version of package `name` in Cargo.lock, if the workspace depends on it.
fn locked_version(ws: &Workspace, name: &str) -> Result<Option<String>> {
    let text = fs::read_to_string(ws.root.join("Cargo.lock")).context("reading Cargo.lock")?;
    let lock: toml::Table = toml::from_str(&text).context("Cargo.lock is not valid TOML")?;
    Ok(lock
        .get("package")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .find(|p| p.get("name").and_then(toml::Value::as_str) == Some(name))
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned))
}

/// The first whitespace-separated token of a version output that is a dotted number, without a
/// leading `v`: `rustc 1.99.1 (58cf8f9c3 2026-10-13)` → `1.99.1`, `wasm-opt version 133 (…)` → `133`.
fn version_token(output: &str) -> Option<&str> {
    output
        .split_whitespace()
        .map(|t| t.strip_prefix('v').unwrap_or(t))
        .find(|t| t.starts_with(|c: char| c.is_ascii_digit()) && t.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

/// `24.19.0` → 24.
fn major(version: &str) -> Option<u32> {
    version.split('.').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{Action, Pin, Probe, Tool, action, parse_channel, version_token};

    fn tool(pin: Pin) -> Tool {
        Tool {
            name: "t",
            version_command: &[],
            pin,
            fix: String::new(),
            optional_for: None,
        }
    }

    #[test]
    fn version_token_skips_hash_and_date() {
        assert_eq!(version_token("rustc 1.99.1 (58cf8f9c3 2026-10-13)"), Some("1.99.1"));
    }

    #[test]
    fn version_token_strips_leading_v() {
        assert_eq!(version_token("v24.19.0"), Some("24.19.0"));
    }

    #[test]
    fn exact_pin_rejects_version_that_only_starts_with_pin() {
        assert!(!tool(Pin::Exact("0.20.2".into())).accepts("cargo-deny 0.20.20"));
    }

    #[test]
    fn exact_pin_rejects_longer_binaryen_release() {
        assert!(!tool(Pin::Exact("133".into())).accepts("wasm-opt version 1330 (version_1330)"));
    }

    #[test]
    fn exact_pin_accepts_same_version() {
        assert!(tool(Pin::Exact("0.9.150".into())).accepts("cargo-nextest 0.9.150 (abc123 2026-10-01)"));
    }

    #[test]
    fn min_major_pin_rejects_older_node() {
        assert!(!tool(Pin::MinMajor(22)).accepts("v20.11.0"));
    }

    #[test]
    fn install_flag_installs_missing_optional_tool() {
        assert_eq!(action(&Probe::Missing, true, true, true), Action::Install);
    }

    #[test]
    fn missing_optional_tool_is_only_noted_without_install_flag() {
        assert_eq!(action(&Probe::Missing, true, false, true), Action::Note);
    }

    #[test]
    fn off_pin_optional_tool_fails_without_install_flag() {
        assert_eq!(action(&Probe::OffPin("x".into()), true, false, true), Action::Fail);
    }

    #[test]
    fn channel_ignores_inline_comment() {
        let text = "[toolchain]\nchannel = \"1.99.1\" # bump with MSRV\n";
        assert_eq!(parse_channel(text).unwrap(), "1.99.1");
    }
}
