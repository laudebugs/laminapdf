//! `cargo xtask doctor`: checks that the external tools xtask and CI rely on are installed, at
//! the versions the repository pins.

use std::fs;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

/// Binaryen release for `wasm-opt`. Bump it deliberately: its output is part of the shipped .wasm.
const BINARYEN_VERSION: &str = "133";
/// The oldest Node major we support (`engines` in package.json; `.nvmrc` pins the one we develop on).
const MIN_NODE_MAJOR: u32 = 22;

/// When a tool must be present.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Need {
    /// Needed by `cargo xtask check`.
    Required,
    /// Needed by one task only (named), or not yet needed.
    Optional(&'static str),
}

struct Tool {
    name: &'static str,
    command: &'static [&'static str],
    need: Need,
    /// Text the version output must contain; `None` accepts any version.
    pinned: Option<String>,
}

/// Checks every tool and fails if a required tool is missing or any present tool is off its pin.
pub(crate) fn run(ws: &Workspace) -> Result<()> {
    let wasm_bindgen = locked_version(ws, "wasm-bindgen")?;
    let tools = [
        Tool {
            name: "rustc",
            command: &["rustc", "--version"],
            need: Need::Required,
            pinned: Some(format!("rustc {} ", toolchain_channel(ws)?)),
        },
        Tool {
            name: "pnpm",
            command: &["pnpm", "--version"],
            need: Need::Required,
            pinned: Some(package_manager_pnpm(ws)?),
        },
        Tool {
            name: "node",
            command: &["node", "--version"],
            need: Need::Required,
            pinned: None,
        },
        Tool {
            name: "cargo-deny",
            command: &["cargo", "deny", "--version"],
            need: Need::Optional("cargo xtask deny"),
            pinned: None,
        },
        Tool {
            name: "cargo-nextest",
            command: &["cargo", "nextest", "--version"],
            need: Need::Optional("faster tests; cargo test is the fallback"),
            pinned: None,
        },
        Tool {
            name: "wasm-bindgen-cli",
            command: &["wasm-bindgen", "--version"],
            need: Need::Optional("lamina-wasm; must match the wasm-bindgen crate in Cargo.lock"),
            pinned: wasm_bindgen.map(|v| format!("wasm-bindgen {v}")),
        },
        Tool {
            name: "wasm-opt (binaryen)",
            command: &["wasm-opt", "--version"],
            need: Need::Optional("lamina-wasm size optimisation"),
            pinned: Some(format!("version {BINARYEN_VERSION}")),
        },
    ];

    let mut failures = Vec::new();
    for tool in &tools {
        let (program, args) = tool.command.split_first().context("empty tool command")?;
        let output = Command::new(program).args(args).current_dir(&ws.root).output();
        let version = match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_owned(),
            _ => {
                match tool.need {
                    Need::Required => {
                        println!("✗ {:<20} missing (required)", tool.name);
                        failures.push(tool.name);
                    }
                    Need::Optional(why) => println!("- {:<20} not installed (optional: {why})", tool.name),
                }
                continue;
            }
        };
        let on_pin = tool.pinned.as_deref().is_none_or(|pin| version.contains(pin));
        let node_ok = tool.name != "node" || node_major(&version).is_some_and(|major| major >= MIN_NODE_MAJOR);
        if on_pin && node_ok {
            println!("✓ {:<20} {version}", tool.name);
        } else {
            let wanted = tool
                .pinned
                .clone()
                .unwrap_or_else(|| format!("node >= {MIN_NODE_MAJOR}"));
            println!("✗ {:<20} {version} (want: {wanted})", tool.name);
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

/// The value of a `key = "value"` line in a simple TOML file.
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
