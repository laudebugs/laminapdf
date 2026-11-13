//! What the desktop app's jobs read (the full lane's `desktop` and `desktop-e2e`; todos X10a,
//! X10b): the crates and packages it builds, worked out from `cargo metadata` and the
//! `package.json` files rather than listed by hand, and a key that names those inputs' contents,
//! so a leg that passed on the same inputs needn't run again.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::tracked::git;

/// The crates the desktop jobs compile: the app and its engine, the WebAssembly modules the
/// frontend's packages are built from (`cargo xtask desktop-frontend`), and the task runner, whose
/// path dependencies (the test fixtures behind `fixtures large-pdf`) the jobs build too.
fn root_crates() -> BTreeSet<&'static str> {
    let wasm = crate::wasm::MODULES.iter().map(|module| module.krate);
    ["laminapdf-desktop", "laminapdf-desktop-engine", "xtask"]
        .into_iter()
        .chain(wasm)
        .collect()
}

/// The package whose workspace dependencies make the rest of the frontend's inputs.
const ROOT_PACKAGE: &str = "apps/desktop/";

/// Inputs no dependency graph names: the app itself; the manifests, lockfiles, toolchains and
/// shared configs; the fixtures the scenarios open; the task runner, whose desktop-frontend,
/// desktop-sidecar and fixtures tasks reach most of its modules (wasm-opt's pin in doctor.rs among
/// them), so all of it counts; and the workflow and actions that run the jobs.
const FIXED: [&str; 20] = [
    ROOT_PACKAGE,
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/",
    "package.json",
    ".npmrc",
    ".nvmrc",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "tsconfig.base.json",
    "biome.json",
    "fixtures/golden/examples/",
    "fixtures/corpus/",
    "fixtures/encrypted/",
    "tools/xtask/",
    ".github/workflows/full.yml",
    ".github/actions/install-tools/",
    ".github/actions/linux-webkit/",
    ".github/actions/node-pnpm/",
];

/// Every path prefix the desktop jobs read, sorted, none inside another.
pub(crate) fn inputs(root: &Path) -> Result<Vec<String>> {
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
    let crates = crate_dirs(&metadata, root, &root_crates())?;
    let packages = package_dirs(&workspace_packages(root)?, ROOT_PACKAGE);
    Ok(outermost(
        FIXED.iter().map(|&p| p.to_owned()).chain(crates).chain(packages),
    ))
}

/// The directories, relative to `root` and ending in `/`, of `roots` and every crate they reach
/// through path dependencies of any kind (normal, build, dev).
fn crate_dirs(metadata: &Value, root: &Path, roots: &BTreeSet<&str>) -> Result<BTreeSet<String>> {
    let empty = Vec::new();
    let packages = metadata.get("packages").and_then(Value::as_array).unwrap_or(&empty);
    // One form for every path, so a dependency's `path` meets its package's directory on every OS.
    let normal = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let dir_of = |manifest: &str| Path::new(manifest).parent().map(normal);
    // Each package's directory and the directories of its path dependencies.
    let mut graph: BTreeMap<std::path::PathBuf, Vec<std::path::PathBuf>> = BTreeMap::new();
    let mut start = Vec::new();
    for package in packages {
        let Some(dir) = package.get("manifest_path").and_then(Value::as_str).and_then(dir_of) else {
            continue;
        };
        let deps = package.get("dependencies").and_then(Value::as_array).unwrap_or(&empty);
        let paths = deps
            .iter()
            .filter_map(|d| d.get("path").and_then(Value::as_str))
            .map(|path| normal(Path::new(path)))
            .collect();
        if package
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| roots.contains(name))
        {
            start.push(dir.clone());
        }
        graph.insert(dir, paths);
    }
    if start.len() != roots.len() {
        bail!("cargo metadata is missing some of the desktop's crates: {roots:?}");
    }
    let mut seen = BTreeSet::new();
    while let Some(dir) = start.pop() {
        if seen.insert(dir.clone()) {
            start.extend(graph.get(&dir).into_iter().flatten().cloned());
        }
    }
    let root = normal(root);
    Ok(seen
        .iter()
        .filter_map(|dir| {
            let relative = dir.strip_prefix(&root).ok()?;
            Some(format!("{}/", relative.to_str()?.replace('\\', "/")))
        })
        .collect())
}

/// A pnpm workspace package, as [`package_dirs`] walks them.
#[derive(Debug)]
struct WorkspacePackage {
    /// Its directory, relative to the root and ending in `/`.
    dir: String,
    /// The names of the workspace packages it depends on (`workspace:` versions, any kind).
    deps: Vec<String>,
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

/// The directories of the package in `start` and every workspace package it reaches.
fn package_dirs(packages: &BTreeMap<String, WorkspacePackage>, start: &str) -> BTreeSet<String> {
    let mut todo: Vec<&str> = packages
        .values()
        .filter(|package| package.dir == start)
        .flat_map(|package| package.deps.iter().map(String::as_str))
        .collect();
    let mut dirs = BTreeSet::from([start.to_owned()]);
    while let Some(name) = todo.pop() {
        if let Some(package) = packages.get(name)
            && dirs.insert(package.dir.clone())
        {
            todo.extend(package.deps.iter().map(String::as_str));
        }
    }
    dirs
}

/// `paths` sorted, without duplicates or any path inside another directory in the list.
fn outermost(paths: impl IntoIterator<Item = String>) -> Vec<String> {
    let sorted: BTreeSet<String> = paths.into_iter().collect();
    let mut kept: Vec<String> = Vec::new();
    for path in sorted {
        if !kept.iter().any(|k| k.ends_with('/') && path.starts_with(k.as_str())) {
            kept.push(path);
        }
    }
    kept
}

/// Whether any of `paths` lies in `inputs`, so the full lane builds and tests the app.
pub(crate) fn touches(paths: &[&str], inputs: &[String]) -> bool {
    paths
        .iter()
        .any(|path| !super::is_docs(path) && inputs.iter().any(|input| path.starts_with(input.as_str())))
}

/// A digest of every file under `inputs` at `HEAD` (its path, mode and blob id), the same on
/// every OS: a desktop leg that passed with it passes again on the same inputs.
pub(crate) fn key(root: &Path, inputs: &[String]) -> Result<String> {
    let mut args = vec!["ls-tree", "-r", "--full-tree", "HEAD", "--"];
    args.extend(inputs.iter().map(String::as_str));
    let listing = git(root, &args)?;
    Ok(crate::util::sha256_hex(listing.as_bytes()).chars().take(32).collect())
}

/// `cargo xtask desktop-key`: prints the key of the desktop's inputs at `HEAD`, alone.
pub(crate) fn print_key(root: &Path) -> Result<()> {
    println!("{}", key(root, &inputs(root)?)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use super::{WorkspacePackage, crate_dirs, inputs, outermost, package_dirs, touches};

    fn repo_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
    }

    #[test]
    fn inputs_should_hold_what_the_app_builds_and_leave_out_what_it_never_reads() {
        let found = inputs(repo_root()).unwrap();
        let has = |path: &str| found.iter().any(|f| path.starts_with(f.as_str()));
        assert_eq!(
            [
                "crates/laminapdf-desktop-engine/src/main.rs",
                "crates/laminapdf-cos/src/lib.rs",
                "crates/laminapdf-wasm/src/lib.rs",
                "crates/laminapdf-render-wasm/src/lib.rs",
                "packages/viewer/src/index.ts",
                "packages/sdk/src/index.ts",
                "packages/wasm/package.json",
                "apps/desktop/src-tauri/src/lib.rs",
                "fixtures/encrypted/r3-rc4-128-user.pdf",
                "crates/laminapdf-cli/src/main.rs",
                "packages/cli/package.json",
                "packages/site-kit/package.json",
                "apps/web/src/app.tsx",
                "tools/xtask/src/doctor.rs",
            ]
            .map(has),
            [
                true, true, true, true, true, true, true, true, true, false, false, false, false, true
            ]
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
    fn crate_dirs_should_follow_path_dependencies_from_the_roots_only() {
        let metadata = serde_json::json!({ "packages": [
            { "name": "app", "manifest_path": "/r/crates/app/Cargo.toml",
              "dependencies": [{ "name": "core", "path": "/r/crates/core" }, { "name": "serde" }] },
            { "name": "core", "manifest_path": "/r/crates/core/Cargo.toml",
              "dependencies": [{ "name": "fixtures", "path": "/r/crates/fixtures", "kind": "dev" }] },
            { "name": "fixtures", "manifest_path": "/r/crates/fixtures/Cargo.toml", "dependencies": [] },
            { "name": "cli", "manifest_path": "/r/crates/cli/Cargo.toml",
              "dependencies": [{ "name": "core", "path": "/r/crates/core" }] },
        ]});
        let dirs = crate_dirs(&metadata, Path::new("/r"), &BTreeSet::from(["app"])).unwrap();
        assert_eq!(
            dirs.into_iter().collect::<Vec<_>>(),
            ["crates/app/", "crates/core/", "crates/fixtures/"]
        );
    }

    #[test]
    fn crate_dirs_should_refuse_a_root_cargo_metadata_doesnt_list() {
        let metadata = serde_json::json!({ "packages": [] });
        assert!(crate_dirs(&metadata, Path::new("/r"), &BTreeSet::from(["app"])).is_err());
    }

    #[test]
    fn package_dirs_should_follow_workspace_dependencies_from_the_app() {
        let pkg = |dir: &str, deps: &[&str]| WorkspacePackage {
            dir: dir.to_owned(),
            deps: deps.iter().map(|&d| d.to_owned()).collect(),
        };
        let packages = BTreeMap::from([
            ("desktop".to_owned(), pkg("apps/desktop/", &["ui"])),
            ("ui".to_owned(), pkg("packages/ui/", &["tokens"])),
            ("tokens".to_owned(), pkg("packages/tokens/", &[])),
            ("site-kit".to_owned(), pkg("packages/site-kit/", &["tokens"])),
        ]);
        assert_eq!(
            package_dirs(&packages, "apps/desktop/").into_iter().collect::<Vec<_>>(),
            ["apps/desktop/", "packages/tokens/", "packages/ui/"]
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
