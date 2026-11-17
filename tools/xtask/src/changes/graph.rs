//! The repository's two dependency graphs, which `cargo xtask changes` walks to work out what a job
//! reads (todos X10a, X12): the Cargo workspace's crates and their path dependencies (`cargo
//! metadata --no-deps`), and the pnpm workspace's packages and their `workspace:` dependencies (the
//! `package.json` files under `apps/` and `packages/`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// Both graphs, read once per `cargo xtask changes`.
#[derive(Debug)]
pub(crate) struct Graph {
    /// The repository's root, in the same form as [`Graph::crates`]' directories.
    root: PathBuf,
    /// Each crate's directory and the directories of its path dependencies of any kind (normal,
    /// build, dev).
    crates: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// The same, through normal and build dependencies only: what a crate's library and binaries
    /// build from, without its tests' and examples' dev-dependencies.
    runtime: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// Each crate's directory, by name.
    crate_names: BTreeMap<String, PathBuf>,
    /// Every workspace package, by name.
    packages: BTreeMap<String, WorkspacePackage>,
}

/// A pnpm workspace package, as [`Graph::package_dirs`] walks them.
#[derive(Debug)]
pub(crate) struct WorkspacePackage {
    /// Its directory, relative to the root and ending in `/`.
    pub(crate) dir: String,
    /// The names of the workspace packages it depends on (`workspace:` versions, any kind).
    pub(crate) deps: Vec<String>,
}

/// One form for every path, so a dependency's `path` meets its package's directory on every OS.
fn normal(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

impl Graph {
    /// Reads both graphs of the repository at `root`.
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let metadata = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(["metadata", "--no-deps", "--offline", "--format-version", "1"])
            .current_dir(root)
            .output()
            .context("failed to run cargo metadata")?;
        if !metadata.status.success() {
            bail!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&metadata.stderr).trim()
            );
        }
        let metadata: Value = serde_json::from_slice(&metadata.stdout).context("reading cargo metadata")?;
        Ok(Self::from_parts(&metadata, root, workspace_packages(root)?))
    }

    /// The graphs from `cargo metadata`'s JSON and the workspace packages already read.
    pub(crate) fn from_parts(metadata: &Value, root: &Path, packages: BTreeMap<String, WorkspacePackage>) -> Self {
        let empty = Vec::new();
        let list = metadata.get("packages").and_then(Value::as_array).unwrap_or(&empty);
        let mut crates = BTreeMap::new();
        let mut runtime = BTreeMap::new();
        let mut crate_names = BTreeMap::new();
        for package in list {
            let Some(dir) = package
                .get("manifest_path")
                .and_then(Value::as_str)
                .and_then(|manifest| Path::new(manifest).parent())
                .map(normal)
            else {
                continue;
            };
            let deps = package.get("dependencies").and_then(Value::as_array).unwrap_or(&empty);
            let path_of = |d: &Value| {
                d.get("path")
                    .and_then(Value::as_str)
                    .map(|path| normal(Path::new(path)))
            };
            let paths = deps.iter().filter_map(path_of).collect();
            let non_dev = deps
                .iter()
                .filter(|d| d.get("kind").and_then(Value::as_str) != Some("dev"))
                .filter_map(path_of)
                .collect();
            if let Some(name) = package.get("name").and_then(Value::as_str) {
                crate_names.insert(name.to_owned(), dir.clone());
            }
            runtime.insert(dir.clone(), non_dev);
            crates.insert(dir, paths);
        }
        Self {
            root: normal(root),
            crates,
            runtime,
            crate_names,
            packages,
        }
    }

    /// The directories, relative to the root and ending in `/`, of `roots` and every crate they
    /// reach through path dependencies. A root `cargo metadata` doesn't list is an error, so a
    /// renamed crate can't quietly leave a job's inputs.
    pub(crate) fn crate_dirs<'a>(&self, roots: impl IntoIterator<Item = &'a str>) -> Result<BTreeSet<String>> {
        self.reach(&self.crates, roots)
    }

    /// As [`Graph::crate_dirs`], through normal and build dependencies only: what the roots'
    /// libraries are built from, which an example reaches when it uses only those.
    pub(crate) fn runtime_crate_dirs<'a>(&self, roots: impl IntoIterator<Item = &'a str>) -> Result<BTreeSet<String>> {
        self.reach(&self.runtime, roots)
    }

    /// The directories of `roots` and every crate they reach in `edges`.
    fn reach<'a>(
        &self,
        edges: &BTreeMap<PathBuf, Vec<PathBuf>>,
        roots: impl IntoIterator<Item = &'a str>,
    ) -> Result<BTreeSet<String>> {
        let mut todo = Vec::new();
        for name in roots {
            let dir = self
                .crate_names
                .get(name)
                .with_context(|| format!("cargo metadata lists no crate named {name}"))?;
            todo.push(dir.clone());
        }
        let mut seen = BTreeSet::new();
        while let Some(dir) = todo.pop() {
            if seen.insert(dir.clone()) {
                todo.extend(edges.get(&dir).into_iter().flatten().cloned());
            }
        }
        Ok(seen
            .iter()
            .filter_map(|dir| {
                let relative = dir.strip_prefix(&self.root).ok()?;
                Some(format!("{}/", relative.to_str()?.replace('\\', "/")))
            })
            .collect())
    }

    /// The directory `start` (a package's, ending in `/`) and those of every workspace package it
    /// reaches.
    pub(crate) fn package_dirs(&self, start: &str) -> BTreeSet<String> {
        let mut todo: Vec<&str> = self
            .packages
            .values()
            .filter(|package| package.dir == start)
            .flat_map(|package| package.deps.iter().map(String::as_str))
            .collect();
        let mut dirs = BTreeSet::from([start.to_owned()]);
        while let Some(name) = todo.pop() {
            if let Some(package) = self.packages.get(name)
                && dirs.insert(package.dir.clone())
            {
                todo.extend(package.deps.iter().map(String::as_str));
            }
        }
        dirs
    }
}

/// Every workspace package under `apps/` and `packages/`, by name.
fn workspace_packages(root: &Path) -> Result<BTreeMap<String, WorkspacePackage>> {
    let mut found = BTreeMap::new();
    for parent in ["apps", "packages"] {
        let Ok(entries) = std::fs::read_dir(root.join(parent)) else {
            continue;
        };
        for entry in entries {
            let entry = entry?;
            let Ok(text) = std::fs::read_to_string(entry.path().join("package.json")) else {
                continue;
            };
            let manifest: Value = serde_json::from_str(&text)
                .with_context(|| format!("reading {}/package.json", entry.path().display()))?;
            let Some(name) = manifest.get("name").and_then(Value::as_str) else {
                continue;
            };
            let deps = ["dependencies", "devDependencies", "peerDependencies"]
                .iter()
                .filter_map(|kind| manifest.get(kind).and_then(Value::as_object))
                .flatten()
                .filter(|(_, version)| version.as_str().is_some_and(|v| v.starts_with("workspace:")))
                .map(|(dep, _)| dep.clone())
                .collect();
            let dir = format!("{parent}/{}/", entry.file_name().to_string_lossy());
            found.insert(name.to_owned(), WorkspacePackage { dir, deps });
        }
    }
    Ok(found)
}

/// `paths` sorted, without duplicates or any path inside another directory in the list.
pub(crate) fn outermost(paths: impl IntoIterator<Item = String>) -> Vec<String> {
    let sorted: BTreeSet<String> = paths.into_iter().collect();
    let mut kept: Vec<String> = Vec::new();
    for path in sorted {
        if !kept.iter().any(|k| k.ends_with('/') && path.starts_with(k.as_str())) {
            kept.push(path);
        }
    }
    kept
}

/// Whether any of `paths` that isn't docs lies in `inputs` (directories ending in `/`, or files).
pub(crate) fn touches(paths: &[&str], inputs: &[String]) -> bool {
    paths
        .iter()
        .any(|path| !super::is_docs(path) && inputs.iter().any(|input| path.starts_with(input.as_str())))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use super::{Graph, WorkspacePackage, outermost, touches};

    fn pkg(dir: &str, deps: &[&str]) -> WorkspacePackage {
        WorkspacePackage {
            dir: dir.to_owned(),
            deps: deps.iter().map(|&d| d.to_owned()).collect(),
        }
    }

    fn graph(packages: BTreeMap<String, WorkspacePackage>) -> Graph {
        let metadata = serde_json::json!({ "packages": [
            { "name": "app", "manifest_path": "/r/crates/app/Cargo.toml",
              "dependencies": [{ "name": "core", "path": "/r/crates/core" }, { "name": "serde" }] },
            { "name": "core", "manifest_path": "/r/crates/core/Cargo.toml",
              "dependencies": [{ "name": "fixtures", "path": "/r/crates/fixtures", "kind": "dev" }] },
            { "name": "fixtures", "manifest_path": "/r/crates/fixtures/Cargo.toml", "dependencies": [] },
            { "name": "cli", "manifest_path": "/r/crates/cli/Cargo.toml",
              "dependencies": [{ "name": "core", "path": "/r/crates/core" }] },
        ]});
        Graph::from_parts(&metadata, Path::new("/r"), packages)
    }

    #[test]
    fn crate_dirs_should_follow_path_dependencies_from_the_roots_only() {
        let dirs = graph(BTreeMap::new()).crate_dirs(["app"]).unwrap();
        assert_eq!(
            dirs.into_iter().collect::<Vec<_>>(),
            ["crates/app/", "crates/core/", "crates/fixtures/"]
        );
    }

    #[test]
    fn runtime_crate_dirs_should_skip_dev_dependencies() {
        let dirs = graph(BTreeMap::new()).runtime_crate_dirs(["app"]).unwrap();
        assert_eq!(dirs.into_iter().collect::<Vec<_>>(), ["crates/app/", "crates/core/"]);
    }

    #[test]
    fn crate_dirs_should_refuse_a_root_cargo_metadata_doesnt_list() {
        assert!(graph(BTreeMap::new()).crate_dirs(["gone"]).is_err());
    }

    #[test]
    fn package_dirs_should_follow_workspace_dependencies_from_the_start() {
        let packages = BTreeMap::from([
            ("desktop".to_owned(), pkg("apps/desktop/", &["ui"])),
            ("ui".to_owned(), pkg("packages/ui/", &["tokens"])),
            ("tokens".to_owned(), pkg("packages/tokens/", &[])),
            ("site-kit".to_owned(), pkg("packages/site-kit/", &["tokens"])),
        ]);
        assert_eq!(
            graph(packages)
                .package_dirs("apps/desktop/")
                .into_iter()
                .collect::<Vec<_>>(),
            ["apps/desktop/", "packages/tokens/", "packages/ui/"]
        );
    }

    #[test]
    fn touches_should_skip_docs_and_paths_outside_the_inputs() {
        let inputs = ["crates/a/".to_owned(), "Cargo.lock".to_owned()];
        assert_eq!(
            [
                touches(&["crates/a/src/lib.rs"], &inputs),
                touches(&["Cargo.lock"], &inputs),
                touches(&["crates/b/src/lib.rs", "docs/README.md"], &inputs),
            ],
            [true, true, false]
        );
    }

    #[test]
    fn outermost_should_drop_duplicates_and_paths_inside_another() {
        assert_eq!(
            outermost(
                [
                    "apps/desktop/src-tauri/",
                    "apps/desktop/",
                    "Cargo.lock",
                    "Cargo.lock",
                    "apps/desktops"
                ]
                .map(String::from)
            ),
            ["Cargo.lock", "apps/desktop/", "apps/desktops"]
        );
    }
}
