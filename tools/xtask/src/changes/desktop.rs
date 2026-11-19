//! What the desktop app's jobs read (the full lane's `desktop` and `desktop-e2e`; todos X10a,
//! X10b): the crates and packages it builds, worked out from `cargo metadata` and the
//! `package.json` files rather than listed by hand, and a key that names those inputs' contents,
//! so a leg that passed on the same inputs needn't run again.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;

use super::graph::{Graph, outermost};
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
const FIXED: [&str; 22] = [
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
    ".github/actions/apt-fail-fast/",
    ".github/actions/apt-install/",
    ".github/actions/install-tools/",
    ".github/actions/linux-webkit/",
    ".github/actions/node-pnpm/",
];

/// Every path prefix the desktop jobs read, sorted, none inside another.
pub(crate) fn inputs(graph: &Graph) -> Result<Vec<String>> {
    let crates = graph.crate_dirs(root_crates())?;
    let packages = graph.package_dirs(ROOT_PACKAGE);
    Ok(outermost(
        FIXED.iter().map(|&p| p.to_owned()).chain(crates).chain(packages),
    ))
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
    println!("{}", key(root, &inputs(&Graph::load(root)?)?)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::graph::Graph;
    use super::inputs;

    fn repo_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
    }

    #[test]
    fn inputs_should_hold_what_the_app_builds_and_leave_out_what_it_never_reads() {
        let found = inputs(&Graph::load(repo_root()).unwrap()).unwrap();
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
}
