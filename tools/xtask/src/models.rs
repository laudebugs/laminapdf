//! `cargo xtask models check | fetch <name>`: the model manifest (`tools/models/models.toml`)
//! and its licence gate (ADR 0030 §6, todo L3c).
//!
//! `check` (fast, offline; part of `check docs`) fails on a model whose code or weights
//! licence is outside [`ALLOWED`], on a malformed entry, and on a weights file anywhere in the
//! repository. `fetch` downloads a model's weights into `target/models/<name>/<sha256>/` and
//! keeps them only when their SHA-256 and size match the manifest.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::step;
use crate::tracked::Tracked;
use crate::util::sha256_hex;
use crate::workspace::Workspace;

/// The manifest, relative to the repository root.
pub(crate) const MANIFEST: &str = "tools/models/models.toml";

/// The licences a model's code and weights may have: permissive ones, as ADR 0019's policy
/// admits for dependencies, plus the Community Data Licence Agreement's permissive versions,
/// which cover data and models trained on it.
const ALLOWED: [&str; 7] = [
    "Apache-2.0",
    "MIT",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "CDLA-Permissive-1.0",
    "CDLA-Permissive-2.0",
];

/// File extensions that hold model weights, which never belong in the repository.
const WEIGHT_EXTENSIONS: [&str; 10] = [
    "onnx",
    "safetensors",
    "pt",
    "pth",
    "ckpt",
    "gguf",
    "tflite",
    "mlmodel",
    "mlpackage",
    "pdparams",
];

/// The kinds of model a plug-in trait takes.
const KINDS: [&str; 2] = ["layout", "ocr"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    #[serde(default)]
    model: Vec<Model>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    name: String,
    kind: String,
    description: String,
    revision: String,
    url: String,
    file: String,
    bytes: u64,
    sha256: String,
    weights_licence: String,
    weights_licence_source: String,
    code_licence: String,
    code_licence_source: String,
    training_data: Option<String>,
    read_on: String,
}

fn parse(text: &str) -> Result<Manifest> {
    toml::from_str(text).with_context(|| format!("{MANIFEST} is not a valid manifest"))
}

fn read(root: &Path) -> Result<Manifest> {
    let path = root.join(MANIFEST);
    parse(&fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?)
}

/// Whether an SPDX expression of `AND`/`OR` terms (no parentheses) is allowed: every `AND`
/// term, and at least one choice of every `OR`, must be in [`ALLOWED`].
fn licence_allowed(expression: &str) -> bool {
    let expression = expression.trim();
    !expression.is_empty()
        && expression
            .split(" AND ")
            .all(|term| term.split(" OR ").any(|choice| ALLOWED.contains(&choice.trim())))
}

/// Every problem with `manifest`'s entries.
fn problems(manifest: &Manifest) -> Vec<String> {
    let mut out = Vec::new();
    let mut names = BTreeSet::new();
    for m in &manifest.model {
        let at = |what: &str| format!("{}: {what}", m.name);
        if !names.insert(m.name.as_str()) {
            out.push(at("listed twice"));
        }
        if !KINDS.contains(&m.kind.as_str()) {
            out.push(at(&format!("kind `{}` is not one of {KINDS:?}", m.kind)));
        }
        for (what, licence) in [("weights", &m.weights_licence), ("code", &m.code_licence)] {
            if !licence_allowed(licence) {
                out.push(at(&format!(
                    "{what} licence `{licence}` is not allowed (allowed: {})",
                    ALLOWED.join(", ")
                )));
            }
        }
        for (what, source) in [
            ("weights_licence_source", &m.weights_licence_source),
            ("code_licence_source", &m.code_licence_source),
        ] {
            if !source.contains("https://") {
                out.push(at(&format!("{what} must link the licence it was read from")));
            }
        }
        if m.sha256.len() != 64
            || !m
                .sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            out.push(at("sha256 must be 64 lowercase hex digits"));
        }
        if !m.url.starts_with("https://") || !m.url.contains(&m.revision) {
            out.push(at("url must be https and name the pinned revision"));
        }
        if m.training_data.as_deref().is_some_and(|t| !t.contains("https://")) {
            out.push(at("training_data must link the data's licence"));
        }
        if m.file.contains('/')
            || m.file.is_empty()
            || m.bytes == 0
            || m.read_on.len() != 10
            || m.description.is_empty()
        {
            out.push(at("file, bytes, read_on (YYYY-MM-DD) and description must be set"));
        }
    }
    out
}

/// Tracked files that look like model weights.
fn weight_files<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| WEIGHT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .map(Path::to_path_buf)
        .collect()
}

/// `cargo xtask models check`.
pub(crate) fn check(root: &Path) -> Result<()> {
    step("models check: tools/models/models.toml's licences and pins; no weights in the repository");
    let mut found = problems(&read(root)?);
    let tracked = Tracked::from_git(root)?;
    found.extend(weight_files(tracked.files()).into_iter().map(|p| {
        format!(
            "{}: a weights file is in the repository; fetch weights instead",
            p.display()
        )
    }));
    if !found.is_empty() {
        bail!("models check failed:\n  {}", found.join("\n  "));
    }
    Ok(())
}

/// `cargo xtask models fetch <name>`: the weights' path once they are in the cache.
pub(crate) fn fetch(ws: &Workspace, name: &str) -> Result<PathBuf> {
    let manifest = read(&ws.root)?;
    let Some(model) = manifest.model.iter().find(|m| m.name == name) else {
        let known: Vec<&str> = manifest.model.iter().map(|m| m.name.as_str()).collect();
        bail!("no model `{name}` in {MANIFEST} (listed: {})", known.join(", "));
    };
    let dir = ws.root.join("target/models").join(&model.name).join(&model.sha256);
    let target = dir.join(&model.file);
    if target.is_file() && sha256_hex(&fs::read(&target)?) == model.sha256 {
        step(&format!("models: {name} is cached at {}", target.display()));
        return Ok(target);
    }
    fs::create_dir_all(&dir)?;
    let partial = target.with_extension("part");
    step(&format!("models: fetching {name} ({} MB)", model.bytes / 1_000_000));
    let status = std::process::Command::new(crate::program_path("curl"))
        .args(["-fsSL", "--retry", "3", "-o"])
        .arg(&partial)
        .arg(&model.url)
        .status()
        .context("failed to start curl")?;
    if !status.success() {
        bail!("downloading {} failed with {status}", model.url);
    }
    let bytes = fs::read(&partial)?;
    let digest = sha256_hex(&bytes);
    if digest != model.sha256 || u64::try_from(bytes.len()).ok() != Some(model.bytes) {
        fs::remove_file(&partial)?;
        bail!(
            "{name}: the download's SHA-256 is {digest} ({} bytes); the manifest pins {} ({} bytes)",
            bytes.len(),
            model.sha256,
            model.bytes
        );
    }
    fs::rename(&partial, &target)?;
    step(&format!("models: {name} at {}", target.display()));
    Ok(target)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{licence_allowed, parse, problems, weight_files};

    const GOOD: &str = r#"
[[model]]
name = "m"
kind = "layout"
description = "a model"
revision = "abc"
url = "https://example.com/abc/model.onnx"
file = "model.onnx"
bytes = 10
sha256 = "59c81a3a2923042d85034ffc487f8f47e4854117e879aef89b2b9f728fb4922a"
weights_licence = "Apache-2.0"
weights_licence_source = "https://example.com/card"
code_licence = "MIT AND Apache-2.0"
code_licence_source = "https://example.com/LICENSE"
read_on = "2026-11-08"
"#;

    #[test]
    fn the_committed_manifest_should_pass() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest = parse(&std::fs::read_to_string(root.join(super::MANIFEST)).unwrap()).unwrap();
        assert_eq!(problems(&manifest), Vec::<String>::new());
    }

    #[test]
    fn a_good_entry_should_have_no_problems() {
        assert_eq!(problems(&parse(GOOD).unwrap()), Vec::<String>::new());
    }

    #[test]
    fn a_restricted_weights_licence_should_fail() {
        let text = GOOD.replace(
            r#"weights_licence = "Apache-2.0""#,
            r#"weights_licence = "CC-BY-NC-SA-4.0""#,
        );
        let found = problems(&parse(&text).unwrap());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("weights licence `CC-BY-NC-SA-4.0` is not allowed"));
    }

    #[test]
    fn licence_expressions_should_need_every_and_term_and_one_or_choice() {
        assert!(licence_allowed("MIT OR Apache-2.0"));
        assert!(licence_allowed("MIT AND Apache-2.0"));
        assert!(licence_allowed("GPL-3.0 OR MIT"));
        assert!(!licence_allowed("MIT AND GPL-3.0"));
        assert!(!licence_allowed("AGPL-3.0"));
        assert!(!licence_allowed(""));
    }

    #[test]
    fn an_unpinned_url_or_bad_hash_should_fail() {
        let text = GOOD
            .replace(
                "https://example.com/abc/model.onnx",
                "https://example.com/main/model.onnx",
            )
            .replace("59c81a3a", "59C81A3A");
        assert_eq!(problems(&parse(&text).unwrap()).len(), 2);
    }

    #[test]
    fn weights_files_in_the_tree_should_be_found() {
        let paths = [
            Path::new("a/model.ONNX"),
            Path::new("b/w.safetensors"),
            Path::new("c/readme.md"),
        ];
        assert_eq!(weight_files(paths).len(), 2);
    }
}
