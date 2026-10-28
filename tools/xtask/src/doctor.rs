//! External tools xtask shells out to, pinned in one place (ADR 0015, ADR 0019).
//!
//! Tasks call [`require`] before using a tool, so an off-pin version fails with the fix command
//! instead of producing different results. `cargo xtask doctor` checks them all, and
//! `cargo xtask doctor --install` runs the fix commands that are safe to automate (`cargo install`).
//! CI installs the same versions (`.github/workflows/ci.yml`); keep the two in sync.

use std::fs;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

/// cargo-deny release (`cargo xtask deny`, `check rust`).
const CARGO_DENY_VERSION: &str = "0.20.2";
/// cargo-nextest release (`cargo xtask test`; plain `cargo test` is the fallback when it is absent).
const CARGO_NEXTEST_VERSION: &str = "0.9.150";
/// Binaryen release for `wasm-opt`. Its output is part of the shipped .wasm, so bump it deliberately
/// and review the wasm-opt feature list (ADR 0002) whenever this or `rust-toolchain.toml` changes.
const BINARYEN_VERSION: &str = "133";
/// The oldest Node major we support (`engines` in package.json; `.nvmrc` pins the one we develop on).
const MIN_NODE_MAJOR: u32 = 22;

/// A pinned external tool.
pub(crate) struct Tool {
    name: &'static str,
    /// Command that prints the version.
    version_command: &'static [&'static str],
    /// Text the version output must contain (`None`: any version, checked by [`Tool::accepts`]).
    pin: Option<String>,
    /// How to install the pinned version; `cargo install …` commands are run by `doctor --install`.
    fix: String,
    /// `None` when every task needs it; otherwise what needs it.
    optional_for: Option<&'static str>,
}

/// What [`Tool::probe`] found.
enum Probe {
    Missing,
    OffPin(String),
    Ok(String),
}

impl Tool {
    fn probe(&self, ws: &Workspace) -> Probe {
        let Some((program, args)) = self.version_command.split_first() else {
            return Probe::Missing;
        };
        let output = Command::new(program).args(args).current_dir(&ws.root).output();
        let version = match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_owned(),
            _ => return Probe::Missing,
        };
        if self.accepts(&version) {
            Probe::Ok(version)
        } else {
            Probe::OffPin(version)
        }
    }

    fn accepts(&self, version: &str) -> bool {
        match &self.pin {
            Some(pin) => version.contains(pin.as_str()),
            // Node is the only range check: any major from MIN_NODE_MAJOR up.
            None => self.name != "node" || node_major(version).is_some_and(|major| major >= MIN_NODE_MAJOR),
        }
    }

    fn wanted(&self) -> String {
        self.pin
            .clone()
            .unwrap_or_else(|| format!("v{MIN_NODE_MAJOR} or newer"))
    }
}

pub(crate) fn cargo_deny() -> Tool {
    Tool {
        name: "cargo-deny",
        version_command: &["cargo", "deny", "--version"],
        pin: Some(format!("cargo-deny {CARGO_DENY_VERSION}")),
        fix: format!("cargo install cargo-deny --locked --version {CARGO_DENY_VERSION}"),
        optional_for: None,
    }
}

pub(crate) fn cargo_nextest() -> Tool {
    Tool {
        name: "cargo-nextest",
        version_command: &["cargo", "nextest", "--version"],
        pin: Some(format!("cargo-nextest {CARGO_NEXTEST_VERSION}")),
        fix: format!("cargo install cargo-nextest --locked --version {CARGO_NEXTEST_VERSION}"),
        optional_for: Some("faster tests; cargo test is the fallback"),
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
            pin: Some(format!("rustc {channel} ")),
            fix: "rustup show  # installs the toolchain pinned in rust-toolchain.toml".to_owned(),
            optional_for: None,
        },
        Tool {
            name: "node",
            version_command: &["node", "--version"],
            pin: None,
            fix: "nvm install  # or any version manager reading .nvmrc".to_owned(),
            optional_for: None,
        },
        Tool {
            name: "pnpm",
            version_command: &["pnpm", "--version"],
            fix: format!("corepack enable  # or: npm install -g pnpm@{pnpm}"),
            pin: Some(pnpm),
            optional_for: None,
        },
        cargo_deny(),
        cargo_nextest(),
        Tool {
            name: "wasm-bindgen-cli",
            version_command: &["wasm-bindgen", "--version"],
            fix: match &wasm_bindgen {
                Some(v) => format!("cargo install wasm-bindgen-cli --locked --version {v}"),
                None => "not needed until a crate depends on wasm-bindgen".to_owned(),
            },
            pin: wasm_bindgen.map(|v| format!("wasm-bindgen {v}")),
            optional_for: Some("lamina-wasm (Round B); must match the wasm-bindgen crate in Cargo.lock"),
        },
        Tool {
            name: "wasm-opt",
            version_command: &["wasm-opt", "--version"],
            pin: Some(format!("version {BINARYEN_VERSION}")),
            fix: format!(
                "download binaryen version_{BINARYEN_VERSION} from https://github.com/WebAssembly/binaryen/releases"
            ),
            optional_for: Some("lamina-wasm size optimisation (Round B)"),
        },
    ])
}

/// Fails unless `tool` is installed at its pinned version, naming the fix command.
pub(crate) fn require(ws: &Workspace, tool: &Tool) -> Result<()> {
    match tool.probe(ws) {
        Probe::Ok(_) => Ok(()),
        Probe::Missing => bail!("{} is not installed; run: {}", tool.name, tool.fix),
        Probe::OffPin(found) => bail!("{} is {found}, want {}; run: {}", tool.name, tool.wanted(), tool.fix),
    }
}

/// Whether `tool` is installed at its pinned version. Fails if it is installed at another version.
pub(crate) fn available(ws: &Workspace, tool: &Tool) -> Result<bool> {
    match tool.probe(ws) {
        Probe::Ok(_) => Ok(true),
        Probe::Missing => Ok(false),
        Probe::OffPin(found) => bail!("{} is {found}, want {}; run: {}", tool.name, tool.wanted(), tool.fix),
    }
}

/// `cargo xtask doctor [--install]`: reports every tool; fails on a missing required or off-pin tool.
pub(crate) fn run(ws: &Workspace, install: bool) -> Result<()> {
    let mut failures = Vec::new();
    for tool in all_tools(ws)? {
        let problem = match tool.probe(ws) {
            Probe::Ok(version) => {
                println!("✓ {:<17} {version}", tool.name);
                continue;
            }
            Probe::Missing => match tool.optional_for {
                Some(why) => {
                    println!(
                        "- {:<17} not installed (optional: {why})\n  {:<17} {}",
                        tool.name, "", tool.fix
                    );
                    continue;
                }
                None => "missing".to_owned(),
            },
            Probe::OffPin(found) => format!("{found}, want {}", tool.wanted()),
        };
        println!("✗ {:<17} {problem}\n  {:<17} fix: {}", tool.name, "", tool.fix);
        if install && tool.fix.starts_with("cargo install ") {
            let args: Vec<&str> = tool.fix.split_whitespace().skip(1).collect();
            crate::run(&ws.root, "cargo", &args)?;
        } else {
            failures.push(tool.name);
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        bail!("doctor found problems with: {}", failures.join(", "))
    }
}

/// The `channel` in `rust-toolchain.toml`.
fn toolchain_channel(ws: &Workspace) -> Result<String> {
    let text = fs::read_to_string(ws.root.join("rust-toolchain.toml")).context("reading rust-toolchain.toml")?;
    toml_string(&text, "channel").context("rust-toolchain.toml has no channel")
}

/// The pnpm version from the root package.json `packageManager` field (`pnpm@X.Y.Z`).
fn package_manager_pnpm(ws: &Workspace) -> Result<String> {
    let text = fs::read_to_string(ws.root.join("package.json")).context("reading package.json")?;
    let json: serde_json::Value = serde_json::from_str(&text).context("package.json is not JSON")?;
    json.get("packageManager")
        .and_then(serde_json::Value::as_str)
        .and_then(|pm| pm.strip_prefix("pnpm@"))
        .map(str::to_owned)
        .context("package.json packageManager must be pnpm@<version>")
}

/// The version of `name` in Cargo.lock, if the workspace depends on it.
fn locked_version(ws: &Workspace, name: &str) -> Result<Option<String>> {
    let lock = fs::read_to_string(ws.root.join("Cargo.lock")).context("reading Cargo.lock")?;
    let needle = format!("name = \"{name}\"");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == needle {
            return Ok(lines.next().and_then(|next| toml_string(next, "version")));
        }
    }
    Ok(None)
}

/// The value of the first `key = "value"` line in a simple TOML file.
fn toml_string(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
    })
}

/// `v24.19.0` → 24.
fn node_major(version: &str) -> Option<u32> {
    version.strip_prefix('v')?.split('.').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{node_major, toml_string};

    #[test]
    fn node_major_reads_leading_number() {
        assert_eq!(node_major("v24.19.0"), Some(24));
    }

    #[test]
    fn toml_string_reads_quoted_value() {
        assert_eq!(
            toml_string("[toolchain]\nchannel = \"1.99.1\"\n", "channel").as_deref(),
            Some("1.99.1")
        );
    }
}
