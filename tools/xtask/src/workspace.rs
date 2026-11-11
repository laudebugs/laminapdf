//! The Cargo workspace as xtask sees it: crate layers and the core-crate portability rules.
//!
//! Every crate under `crates/` declares its layer in its manifest:
//!
//! ```toml
//! [package.metadata.laminapdf]
//! layer = "core"   # "core" | "edge" | "app" | "dev"
//! wasm = true      # optional, edge crates only: also lint/build for wasm32 (core crates always are)
//! ```
//!
//! Core crates are bytes in, bytes out (docs/guidelines/code-organization.md §2), and xtask enforces it.
//! Dev crates (`laminapdf-test-fixtures`) hold test helpers: only tests and tools use them
//! (crates as dev-dependencies, xtask for the `codes.json` rules), and they are never published.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
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
/// `(parent, edge-only crate)` edges that are allowed anyway, each with its reason. Nothing else
/// reached through the parent, and no other path to the edge-only crate, is exempt.
const ALLOWED_EDGE_ONLY_EDGES: [(&str, &str, &str); 1] = [(
    "cpufeatures",
    "libc",
    "RustCrypto's CPU-feature detection (aes, sha2; B2b): reads hardware capabilities only \
         (getauxval, sysctlbyname) on aarch64 Linux/Android/Apple; no file system, network, clocks or threads",
)];

/// `cargo metadata`'s `target_directory`, or `<root>/target` if it is missing.
fn target_directory(metadata: &Value, root: &Path) -> PathBuf {
    metadata
        .get("target_directory")
        .and_then(Value::as_str)
        .map_or_else(|| root.join("target"), PathBuf::from)
}

/// A workspace crate under `crates/`.
struct Crate {
    name: String,
    dir: PathBuf,
    layer: Layer,
    wasm: bool,
    /// Whether `cargo publish` may upload it (`publish` in its manifest, not `false`).
    publishable: bool,
    /// Its `description`, as cargo resolves it (workspace inheritance applied); empty if none.
    description: String,
}

/// A package in a crate's dependency graph ([`crate_graph`]).
struct Node {
    name: String,
    /// Proc-macros run on the host at build time; their dependencies are not shipped.
    proc_macro: bool,
    /// Ids of normal (shipped) dependencies; dev- and build-dependencies are left out.
    normal_deps: Vec<String>,
}

#[derive(PartialEq, Eq)]
enum Layer {
    Core,
    Edge,
    App,
    Dev,
}

/// The repository root and its crates.
pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
    /// Cargo's build output directory (`cargo metadata`'s `target_directory`): `<root>/target`
    /// unless `CARGO_TARGET_DIR` (or `build.target-dir`) moves it, as the self-hosted CI
    /// runners do (isolate-rust-home). Only cargo's own output lives there: xtask's reports and
    /// scratch folders (`target/xtask/`, `target/nightly/`, `target/examples-out/`) stay under
    /// `<root>/target`, where the workflows upload them from.
    pub(crate) target_dir: PathBuf,
    crates: Vec<Crate>,
}

impl Workspace {
    /// Reads `cargo metadata` and the
    /// `[package.metadata.laminapdf]` table of every crate in `crates/`.
    pub(crate) fn load(root: PathBuf) -> Result<Self> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--locked"])
            .current_dir(&root)
            .output()
            .context("failed to run cargo metadata")?;
        if !output.status.success() {
            bail!("cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        let metadata: Value = serde_json::from_slice(&output.stdout).context("cargo metadata returned bad JSON")?;
        let target_dir = target_directory(&metadata, &root);
        let crates_dir = root.join("crates");
        let mut crates = Vec::new();
        let members: BTreeSet<&str> = metadata
            .get("workspace_members")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        for package in metadata.get("packages").and_then(Value::as_array).into_iter().flatten() {
            if !members.contains(str_field(package, "/id")) {
                continue; // a dependency, not a workspace crate
            }
            let manifest = PathBuf::from(str_field(package, "/manifest_path"));
            let Some(dir) = manifest.parent() else { continue };
            if !dir.starts_with(&crates_dir) {
                continue; // tools/xtask, apps/*/src-tauri: not subject to the crate-layer rules
            }
            crates.push(Crate::from_metadata(package, dir)?);
        }
        Ok(Self {
            root,
            target_dir,
            crates,
        })
    }

    /// The crates `cargo publish --workspace` uploads (ADR 0017 §7).
    pub(crate) fn publishable_crates(&self) -> impl Iterator<Item = &str> {
        self.crates.iter().filter(|c| c.publishable).map(|c| c.name.as_str())
    }

    /// The crates `cargo publish --workspace` uploads: name, folder and description.
    pub(crate) fn publishable_crate_details(&self) -> impl Iterator<Item = (&str, &Path, &str)> {
        self.crates
            .iter()
            .filter(|c| c.publishable)
            .map(|c| (c.name.as_str(), c.dir.as_path(), c.description.as_str()))
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
            // The crate's own dependency graph, with only the features it (and its own
            // dependencies) enable: the workspace-wide resolve also has the features edge crates
            // turn on (laminapdf-wasm's `tsify` on laminapdf-ops), which a core build never sees.
            let (graph, root) = crate_graph(&self.root, &krate.name)?;
            for path in edge_only_paths(&graph, &root) {
                problems.push(format!("{}: depends on edge-only crate: {path}", krate.name));
            }
            let lib = krate.dir.join("src/lib.rs");
            let lib_src = fs::read_to_string(&lib).with_context(|| format!("reading {}", lib.display()))?;
            if !forbids_unsafe_at_crate_root(&lib_src) {
                problems.push(format!(
                    "{}: missing crate-level #![forbid(unsafe_code)]",
                    lib.display()
                ));
            }
            for file in rust_files(&krate.dir.join("src"))? {
                let source = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
                for (number, line) in code_lines(&source) {
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
        self.require_node_modules()?;
        crate::run(&self.root, "pnpm", args)
    }

    /// Fails with an actionable message when `pnpm install` hasn't been run at the root.
    pub(crate) fn require_node_modules(&self) -> Result<()> {
        if !self.root.join("node_modules").is_dir() {
            bail!("node_modules is missing: run `pnpm install` at the repository root first");
        }
        Ok(())
    }
}

impl Crate {
    fn from_metadata(package: &Value, dir: &Path) -> Result<Self> {
        let name = str_field(package, "/name").to_owned();
        let layer = match package.pointer("/metadata/laminapdf/layer").and_then(Value::as_str) {
            Some("core") => Layer::Core,
            Some("edge") => Layer::Edge,
            Some("app") => Layer::App,
            Some("dev") => Layer::Dev,
            other => {
                bail!(
                    "{name}: [package.metadata.laminapdf] layer must be \"core\", \"edge\", \"app\" or \"dev\" (found {other:?})"
                )
            }
        };
        // cargo metadata: `publish` is null for any registry, `[]` for `publish = false`.
        let publishable = package
            .get("publish")
            .is_none_or(|p| p.as_array().is_none_or(|r| !r.is_empty()));
        if layer == Layer::Dev && publishable {
            bail!("{name}: a dev crate holds test helpers and must set `publish = false`");
        }
        // `cargo xtask check rust` documents every crate with --all-features; docs.rs uses the
        // default features unless told otherwise, and an intra-doc link to a feature-gated item
        // then fails its build (laminapdf-source and laminapdf-render, 0.2.0-alpha.2).
        let docs_rs_all_features = package
            .pointer("/metadata/docs/rs/all-features")
            .and_then(Value::as_bool)
            == Some(true);
        if publishable && !docs_rs_all_features {
            bail!(
                "{name}: a published crate must set `all-features = true` under [package.metadata.docs.rs], \
                 so docs.rs builds the docs `cargo xtask check rust` checks"
            );
        }
        Ok(Self {
            name,
            dir: dir.to_path_buf(),
            wasm: layer == Layer::Edge
                && package.pointer("/metadata/laminapdf/wasm").and_then(Value::as_bool) == Some(true),
            layer,
            publishable,
            description: package
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        })
    }
}

/// `source`'s lines (0-based number, text) without the insides of raw string literals, which
/// are text, not code: the generated explain pages quote `std::fs::read(…)` in their examples.
/// A line that opens a raw string keeps the code before it.
fn code_lines(source: &str) -> Vec<(usize, &str)> {
    let mut lines = Vec::new();
    let mut closing: Option<String> = None;
    for (number, line) in source.lines().enumerate() {
        if let Some(end) = &closing {
            if line.contains(end.as_str()) {
                closing = None;
            }
            continue;
        }
        match raw_string_start(line) {
            Some((at, hashes)) => {
                let end = format!("\"{}", "#".repeat(hashes));
                let rest = line.get(at..).unwrap_or_default();
                // Closed on the same line (after its opening quote)?
                let body = rest.split_once('"').map_or("", |(_, body)| body);
                if !body.contains(end.as_str()) {
                    closing = Some(end);
                }
                lines.push((number, line.get(..at).unwrap_or_default()));
            }
            None => lines.push((number, line)),
        }
    }
    lines
}

/// Where a raw string literal (`r"`, `r#"`, …) starts in `line`, with its number of `#`s.
fn raw_string_start(line: &str) -> Option<(usize, usize)> {
    line.match_indices('r').find_map(|(at, _)| {
        let before = line.get(..at).and_then(|b| b.chars().next_back());
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        let rest = line.get(at + 1..)?;
        let hashes = rest.chars().take_while(|c| *c == '#').count();
        rest.get(hashes..)?.starts_with('"').then_some((at, hashes))
    })
}

/// `name`'s normal dependency graph for every target, as a build of it alone resolves its
/// features (`cargo tree -p name -e normal,no-proc-macro --target all`), keyed by
/// `name version`; the root's key is returned with it.
fn crate_graph(root: &Path, name: &str) -> Result<(BTreeMap<String, Node>, String)> {
    let output = Command::new(crate::program_path("cargo"))
        .args([
            "tree",
            "--locked",
            "-p",
            name,
            "-e",
            "normal,no-proc-macro",
            "--target",
            "all",
            "--prefix",
            "depth",
        ])
        .current_dir(root)
        .output()
        .context("running cargo tree")?;
    if !output.status.success() {
        bail!(
            "cargo tree -p {name} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(parse_tree(&String::from_utf8_lossy(&output.stdout)))
}

/// The graph of `cargo tree --prefix depth` output (`2lamina-syntax v0.0.0 (path)`, repeats
/// marked `(*)`), and its root's key.
fn parse_tree(text: &str) -> (BTreeMap<String, Node>, String) {
    let mut graph: BTreeMap<String, Node> = BTreeMap::new();
    let mut stack: Vec<String> = Vec::new();
    let mut root = String::new();
    for line in text.lines() {
        let digits = line.bytes().take_while(u8::is_ascii_digit).count();
        let Some(depth) = line.get(..digits).and_then(|d| d.parse::<usize>().ok()) else {
            continue;
        };
        let mut tokens = line.get(digits..).unwrap_or_default().split_whitespace();
        let (Some(name), Some(version)) = (tokens.next(), tokens.next()) else {
            continue;
        };
        let id = format!("{name} {version}");
        graph.entry(id.clone()).or_insert_with(|| Node {
            name: name.to_owned(),
            proc_macro: false,
            normal_deps: Vec::new(),
        });
        stack.truncate(depth);
        if let Some(parent) = stack.last().and_then(|p| graph.get_mut(p))
            && !parent.normal_deps.contains(&id)
        {
            parent.normal_deps.push(id.clone());
        }
        if depth == 0 {
            root.clone_from(&id);
        }
        stack.push(id);
    }
    (graph, root)
}

/// Every edge-only crate reachable from `root` through normal dependencies, as a path
/// (`laminapdf-x → helper → getrandom`). Proc-macros are not followed: their dependencies run on the
/// host at build time and are not shipped. Dev- and build-dependencies are not in the graph.
fn edge_only_paths(graph: &BTreeMap<String, Node>, root: &str) -> Vec<String> {
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    let mut queue = VecDeque::from([root]);
    let mut found = Vec::new();
    while let Some(id) = queue.pop_front() {
        let Some(node) = graph.get(id) else { continue };
        if id != root && EDGE_ONLY_DEPS.contains(&node.name.as_str()) {
            found.push(path_to(graph, &parent, root, id));
        }
        if node.proc_macro {
            continue;
        }
        for dep in &node.normal_deps {
            let dep_name = graph.get(dep).map_or(dep.as_str(), |d| d.name.as_str());
            if ALLOWED_EDGE_ONLY_EDGES
                .iter()
                .any(|(from, to, _reason)| *from == node.name && *to == dep_name)
            {
                continue;
            }
            if dep != root && !parent.contains_key(dep.as_str()) {
                parent.insert(dep, id);
                queue.push_back(dep);
            }
        }
    }
    found
}

/// The crate names from `root` to `id`, following `parent` links.
fn path_to(graph: &BTreeMap<String, Node>, parent: &BTreeMap<&str, &str>, root: &str, id: &str) -> String {
    let mut names = Vec::new();
    let mut current = id;
    loop {
        names.push(graph.get(current).map_or(current, |n| n.name.as_str()));
        match parent.get(current) {
            Some(&up) if current != root => current = up,
            _ => break,
        }
    }
    names.reverse();
    names.join(" → ")
}

/// Whether `lib.rs` declares `#![forbid(unsafe_code)]` among its crate-level inner attributes.
///
/// Only the header counts: the inner attributes before the first item, skipping comments, doc
/// comments and blank lines. A commented-out attribute, one mentioned in doc text, or one inside an
/// inline `mod` does not satisfy it. Multi-line attributes are joined before matching.
fn forbids_unsafe_at_crate_root(source: &str) -> bool {
    let mut in_block_comment = false;
    let mut attribute = String::new();
    for line in source.lines() {
        let code = line.trim();
        if !attribute.is_empty() {
            attribute.push_str(code);
        } else if in_block_comment || code.starts_with("/*") {
            in_block_comment = !code.contains("*/");
            continue;
        } else if code.is_empty() || code.starts_with("//") {
            continue;
        } else if code.starts_with("#![") {
            attribute.push_str(code);
        } else {
            return false; // the first item: the crate-level header is over
        }
        if attribute.matches('[').count() == attribute.matches(']').count() {
            if is_forbid_unsafe(&attribute) {
                return true;
            }
            attribute.clear();
        }
    }
    false
}

/// `#![forbid(unsafe_code)]`, `#![forbid(missing_docs, unsafe_code)]` (whitespace-insensitive).
fn is_forbid_unsafe(attribute: &str) -> bool {
    let compact: String = attribute.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.split("//").next().unwrap_or_default();
    compact
        .strip_prefix("#![forbid(")
        .and_then(|rest| rest.strip_suffix(")]"))
        .is_some_and(|lints| lints.split(',').any(|lint| lint == "unsafe_code"))
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
    #[test]
    fn code_lines_should_skip_raw_string_bodies() {
        let source =
            "let a = 1;\nlet page = r#\"# title\nstd::fs::read(\"x\")\n\"#;\nuse std::fs;\nlet b = r\"one line\";\n";
        let lines: Vec<&str> = super::code_lines(source).into_iter().map(|(_, l)| l).collect();
        assert_eq!(lines, ["let a = 1;", "let page = ", "use std::fs;", "let b = "]);
    }

    use std::collections::BTreeMap;

    use std::path::Path;

    use super::{
        Crate, Node, edge_only_paths, forbidden_api, forbids_unsafe_at_crate_root, parse_tree, target_directory,
    };

    #[test]
    fn parse_tree_links_each_line_to_the_line_above_it_one_level_up() {
        let (graph, root) =
            parse_tree("0core v0.0.0 (/p)\n1helper v1.0.0\n2getrandom v0.3.0\n1other v2.0.0\n2helper v1.0.0 (*)\n");
        assert_eq!(
            (root.as_str(), edge_only_paths(&graph, &root)),
            ("core v0.0.0", vec!["core → helper → getrandom".to_owned()])
        );
    }

    #[test]
    fn a_dev_crate_should_need_publish_false() {
        let package = |publish: serde_json::Value| serde_json::json!({ "name": "t", "publish": publish, "metadata": { "laminapdf": { "layer": "dev" } } });
        let dir = Path::new("crates/t");
        assert!(Crate::from_metadata(&package(serde_json::json!([])), dir).is_ok());
        assert!(Crate::from_metadata(&package(serde_json::Value::Null), dir).is_err());
    }

    #[test]
    fn a_published_crate_should_need_docs_rs_all_features() {
        let package = |docs_rs: serde_json::Value| serde_json::json!({ "name": "t", "publish": null, "metadata": { "laminapdf": { "layer": "core" }, "docs": { "rs": docs_rs } } });
        let dir = Path::new("crates/t");
        assert!(Crate::from_metadata(&package(serde_json::json!({ "all-features": true })), dir).is_ok());
        assert!(Crate::from_metadata(&package(serde_json::json!({})), dir).is_err());
        assert!(Crate::from_metadata(&package(serde_json::Value::Null), dir).is_err());
    }

    #[test]
    fn target_directory_should_follow_cargo_metadata() {
        let metadata = serde_json::json!({ "target_directory": "/cache/lamina-target/x" });
        assert_eq!(
            target_directory(&metadata, Path::new("/r")),
            Path::new("/cache/lamina-target/x")
        );
    }

    #[test]
    fn target_directory_should_default_to_root_target() {
        let metadata = serde_json::json!({});
        assert_eq!(target_directory(&metadata, Path::new("/r")), Path::new("/r/target"));
    }

    fn graph(nodes: &[(&str, bool, &[&str])]) -> BTreeMap<String, Node> {
        nodes
            .iter()
            .map(|(name, proc_macro, deps)| {
                let normal_deps = deps.iter().map(|d| (*d).to_owned()).collect();
                (
                    (*name).to_owned(),
                    Node {
                        name: (*name).to_owned(),
                        proc_macro: *proc_macro,
                        normal_deps,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn edge_only_rule_finds_transitive_dependency() {
        let g = graph(&[
            ("core", false, &["helper"]),
            ("helper", false, &["getrandom"]),
            ("getrandom", false, &[]),
        ]);
        assert_eq!(edge_only_paths(&g, "core"), ["core → helper → getrandom"]);
    }

    #[test]
    fn edge_only_rule_allows_libc_only_through_cpufeatures() {
        let g = graph(&[
            ("core", false, &["cpufeatures", "helper"]),
            ("cpufeatures", false, &["libc"]),
            ("helper", false, &["libc"]),
            ("libc", false, &[]),
        ]);
        assert_eq!(edge_only_paths(&g, "core"), ["core → helper → libc"]);
    }

    #[test]
    fn edge_only_rule_ignores_proc_macro_dependencies() {
        let g = graph(&[
            ("core", false, &["derive"]),
            ("derive", true, &["libc"]),
            ("libc", false, &[]),
        ]);
        assert_eq!(edge_only_paths(&g, "core"), Vec::<String>::new());
    }

    #[test]
    fn forbid_check_accepts_crate_level_attribute_after_docs() {
        let src = "//! Crate docs mention #![forbid(unsafe_code)].\n#![forbid(unsafe_code)]\n\npub fn f() {}\n";
        assert!(forbids_unsafe_at_crate_root(src));
    }

    #[test]
    fn forbid_check_accepts_attribute_in_lint_list() {
        assert!(forbids_unsafe_at_crate_root(
            "#![forbid(\n    missing_docs,\n    unsafe_code,\n)]\n"
        ));
    }

    #[test]
    fn forbid_check_rejects_commented_out_attribute() {
        assert!(!forbids_unsafe_at_crate_root(
            "// #![forbid(unsafe_code)]  re-enable later\npub fn f() {}\n"
        ));
    }

    #[test]
    fn forbid_check_rejects_attribute_only_in_doc_text() {
        assert!(!forbids_unsafe_at_crate_root(
            "//! Use `#![forbid(unsafe_code)]` in core crates.\npub fn f() {}\n"
        ));
    }

    #[test]
    fn forbid_check_rejects_attribute_in_block_comment() {
        assert!(!forbids_unsafe_at_crate_root(
            "/*\n#![forbid(unsafe_code)]\n*/\npub fn f() {}\n"
        ));
    }

    #[test]
    fn forbid_check_rejects_attribute_inside_inline_module() {
        assert!(!forbids_unsafe_at_crate_root(
            "mod inner {\n    #![forbid(unsafe_code)]\n}\n"
        ));
    }

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
