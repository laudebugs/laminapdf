//! The Cargo workspace as xtask sees it: crate layers and the core-crate portability rules.
//!
//! Every crate under `crates/` declares its layer in its manifest:
//!
//! ```toml
//! [package.metadata.lamina]
//! layer = "core"   # "core" | "edge" | "app"
//! wasm = true      # optional, edge crates only: also lint/build for wasm32 (core crates always are)
//! ```
//!
//! Core crates are bytes in, bytes out (docs/guidelines/code-organization.md §2), and xtask enforces it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// `std` paths core crates must not use: file system, network, processes, threads.
const FORBIDDEN_PATHS: [&str; 4] = ["std::fs", "std::net", "std::process", "std::thread"];
/// Clock types core crates must not use (time is passed in by the caller).
const FORBIDDEN_IDENTS: [&str; 2] = ["SystemTime", "Instant"];
/// Runtime, binding and platform crates that belong at the edges, never in a core crate.
const EDGE_ONLY_DEPS: [&str; 13] = [
    "wasm-bindgen",
    "js-sys",
    "web-sys",
    "tokio",
    "async-std",
    "tauri",
    "napi",
    "napi-derive",
    "uniffi",
    "getrandom",
    "rand",
    "libc",
    "windows-sys",
];

/// A workspace crate under `crates/`.
struct Crate {
    name: String,
    dir: PathBuf,
    layer: Layer,
    wasm: bool,
    /// Normal and build dependencies (dev-dependencies are not shipped and may use anything).
    deps: Vec<String>,
}

#[derive(PartialEq, Eq)]
enum Layer {
    Core,
    Edge,
    App,
}

/// The repository root and its crates.
pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
    crates: Vec<Crate>,
}

impl Workspace {
    /// Reads `cargo metadata` and the `[package.metadata.lamina]` table of every crate in `crates/`.
    pub(crate) fn load(root: PathBuf) -> Result<Self> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
            .current_dir(&root)
            .output()
            .context("failed to run cargo metadata")?;
        if !output.status.success() {
            bail!("cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        let metadata: Value = serde_json::from_slice(&output.stdout).context("cargo metadata returned bad JSON")?;
        let crates_dir = root.join("crates");
        let mut crates = Vec::new();
        for package in metadata.get("packages").and_then(Value::as_array).into_iter().flatten() {
            let manifest = PathBuf::from(str_field(package, "/manifest_path"));
            let Some(dir) = manifest.parent() else { continue };
            if !dir.starts_with(&crates_dir) {
                continue; // tools/xtask, apps/*/src-tauri: not subject to the crate-layer rules
            }
            crates.push(Crate::from_metadata(package, dir)?);
        }
        Ok(Self { root, crates })
    }

    /// Names of the core crates.
    pub(crate) fn core_crates(&self) -> impl Iterator<Item = &str> {
        self.crates
            .iter()
            .filter(|c| c.layer == Layer::Core)
            .map(|c| c.name.as_str())
    }

    /// Names of the crates that must build for wasm32: every core crate plus edge crates with `wasm = true`.
    pub(crate) fn wasm_crates(&self) -> impl Iterator<Item = &str> {
        self.crates
            .iter()
            .filter(|c| c.layer == Layer::Core || c.wasm)
            .map(|c| c.name.as_str())
    }

    /// Checks every core crate for forbidden APIs and dependencies, and for `#![forbid(unsafe_code)]`.
    pub(crate) fn check_core_sources(&self) -> Result<()> {
        let mut problems = Vec::new();
        for krate in self.crates.iter().filter(|c| c.layer == Layer::Core) {
            for dep in &krate.deps {
                if EDGE_ONLY_DEPS.contains(&dep.as_str()) {
                    problems.push(format!("{}: depends on edge-only crate `{dep}`", krate.name));
                }
            }
            let lib = krate.dir.join("src/lib.rs");
            let lib_src = fs::read_to_string(&lib).with_context(|| format!("reading {}", lib.display()))?;
            if !lib_src.contains("#![forbid(unsafe_code)]") {
                problems.push(format!("{}: missing #![forbid(unsafe_code)]", lib.display()));
            }
            for file in rust_files(&krate.dir.join("src"))? {
                let source = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
                for (number, line) in source.lines().enumerate() {
                    if let Some(found) = forbidden_api(line) {
                        problems.push(format!("{}:{}: uses `{found}`", file.display(), number + 1));
                    }
                }
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            bail!(
                "core crates must be bytes in, bytes out (docs/guidelines/code-organization.md §2):\n  {}",
                problems.join("\n  ")
            )
        }
    }

    /// Runs `cargo args…` at the workspace root.
    pub(crate) fn cargo(&self, args: &[&str]) -> Result<()> {
        crate::run(&self.root, "cargo", args)
    }

    /// Runs `pnpm args…` at the workspace root (after checking `pnpm install` has been run).
    pub(crate) fn pnpm(&self, args: &[&str]) -> Result<()> {
        if !self.root.join("node_modules").is_dir() {
            bail!("node_modules is missing: run `pnpm install` at the repository root first");
        }
        crate::run(&self.root, "pnpm", args)
    }
}

impl Crate {
    fn from_metadata(package: &Value, dir: &Path) -> Result<Self> {
        let name = str_field(package, "/name").to_owned();
        let layer = match package.pointer("/metadata/lamina/layer").and_then(Value::as_str) {
            Some("core") => Layer::Core,
            Some("edge") => Layer::Edge,
            Some("app") => Layer::App,
            other => {
                bail!("{name}: [package.metadata.lamina] layer must be \"core\", \"edge\" or \"app\" (found {other:?})")
            }
        };
        let deps = package
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|dep| dep.get("kind").and_then(Value::as_str) != Some("dev"))
            .filter_map(|dep| dep.get("name").and_then(Value::as_str).map(str::to_owned))
            .collect();
        Ok(Self {
            name,
            dir: dir.to_path_buf(),
            wasm: layer == Layer::Edge
                && package.pointer("/metadata/lamina/wasm").and_then(Value::as_bool) == Some(true),
            layer,
            deps,
        })
    }
}

/// The string at JSON `pointer` in `value`, or `""` when absent.
fn str_field<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or_default()
}

/// Returns the forbidden path or identifier `line` uses, if any. Comment lines are skipped.
///
/// This is a grep, not a parser: `use std::{fs, …}` slips through, which review and the
/// cross-target builds cover. It keeps the spike's `check-targets.sh` rule cheap and obvious.
fn forbidden_api(line: &str) -> Option<&'static str> {
    let code = line.trim_start();
    if code.starts_with("//") {
        return None;
    }
    FORBIDDEN_PATHS
        .into_iter()
        .find(|path| code.contains(path))
        .or_else(|| FORBIDDEN_IDENTS.into_iter().find(|ident| contains_word(code, ident)))
}

/// Whether `word` occurs in `text` as a whole identifier.
fn contains_word(text: &str, word: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(start, _)| {
        let before = text.get(..start).and_then(|s| s.chars().next_back());
        let after = text.get(start + word.len()..).and_then(|s| s.chars().next());
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

/// All `.rs` files under `dir`, recursively.
fn rust_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::forbidden_api;

    #[test]
    fn flags_std_fs_import() {
        assert_eq!(forbidden_api("use std::fs::File;"), Some("std::fs"));
    }

    #[test]
    fn flags_clock_identifier() {
        assert_eq!(forbidden_api("let t = Instant::now();"), Some("Instant"));
    }

    #[test]
    fn ignores_identifier_that_only_contains_a_clock_name() {
        assert_eq!(forbidden_api("let x = InstantReplay::new();"), None);
    }

    #[test]
    fn ignores_comment_lines() {
        assert_eq!(forbidden_api("    /// never calls std::fs"), None);
    }
}
