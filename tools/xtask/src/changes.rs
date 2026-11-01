//! `cargo xtask changes`: does a branch touch code, or only docs? The full lane's first job
//! (`.github/workflows/full.yml`, ADR 0018 §2) runs it, and its suites run only for code.
//!
//! The filter is an allowlist of documentation paths ([`is_docs`], where it is defined); every
//! other path counts as code. Inverting it this way means a root config file nobody thought to list
//! (`.cargo/config.toml`, `.gitattributes`, `rust-toolchain.toml`, a new workflow) always runs
//! the suites instead of slipping through.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, Result};

use crate::tracked::git;

/// `cargo xtask changes [--base <rev>]`: lists the paths changed since the merge base of
/// `base` and `HEAD`, prints whether any of them is code, and appends `code=true|false` to
/// `$GITHUB_OUTPUT` when it is set (in GitHub Actions).
pub(crate) fn run(root: &Path, base: &str) -> Result<()> {
    let merge_base = git(root, &["merge-base", base, "HEAD"])
        .with_context(|| format!("no merge base of {base} and HEAD; fetch {base} with full history"))?;
    let merge_base = merge_base.trim();
    // --no-renames: a rename out of code (crates/x.rs → docs/x.rs) lists both of its paths.
    let diff = git(root, &["diff", "--name-only", "--no-renames", "-z", merge_base, "HEAD"])?;
    let paths: Vec<&str> = diff.split('\0').filter(|p| !p.is_empty()).collect();
    let code = touches_code(&paths);
    println!("{} paths changed since {merge_base} ({base}):", paths.len());
    for path in &paths {
        let kind = if is_docs(path) { "docs" } else { "code" };
        println!("  {kind}  {path}");
    }
    println!("code={code}");
    if let Some(out) = std::env::var_os("GITHUB_OUTPUT") {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&out)
            .with_context(|| format!("opening $GITHUB_OUTPUT ({})", Path::new(&out).display()))?;
        writeln!(file, "code={code}").context("writing $GITHUB_OUTPUT")?;
    }
    Ok(())
}

/// Old path → new path for each file renamed between `merge_base` and the working tree.
pub(crate) fn renames(root: &Path, merge_base: &str) -> Result<BTreeMap<String, String>> {
    let diff = git(
        root,
        &[
            "diff",
            "--name-status",
            "-M",
            "-l0",
            "--diff-filter=R",
            "-z",
            merge_base,
        ],
    )?;
    Ok(parse_renames(&diff))
}

/// The renames in `git diff --name-status -z` output: `R<score>`, old path, new path, each
/// ended by a NUL.
fn parse_renames(name_status: &str) -> BTreeMap<String, String> {
    let mut fields = name_status.split('\0').filter(|field| !field.is_empty());
    let mut renames = BTreeMap::new();
    while let Some(status) = fields.next() {
        let paths = if status.starts_with(['R', 'C']) { 2 } else { 1 };
        let (Some(old), new) = (fields.next(), if paths == 2 { fields.next() } else { None }) else {
            break;
        };
        if let (true, Some(new)) = (status.starts_with('R'), new) {
            renames.insert(old.to_owned(), new.to_owned());
        }
    }
    renames
}

/// True when any path is outside the docs allowlist. No paths at all is docs-only.
pub(crate) fn touches_code(paths: &[&str]) -> bool {
    paths.iter().any(|p| !is_docs(p))
}

/// The docs allowlist, the one place it is defined (ADR 0018 §2 implementation note):
///
/// - anything under `docs/`;
/// - Markdown files at the repository root (`README.md`, `CLAUDE.md`, `CHANGELOG.md`);
/// - `LICENSE*` files at the root;
/// - the GitHub issue templates (`.github/ISSUE_TEMPLATE/`) and the pull request template
///   (`.github/PULL_REQUEST_TEMPLATE.md`).
///
/// Markdown anywhere else is code: a crate README can be its rustdoc and doc tests
/// (`#![doc = include_str!("../README.md")]`), and a docs site's pages are its build input.
pub(crate) fn is_docs(path: &str) -> bool {
    let at_root = !path.contains('/');
    let is_markdown = Path::new(path)
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("md"));
    path.starts_with("docs/")
        || (at_root && (is_markdown || path.starts_with("LICENSE")))
        || path.starts_with(".github/ISSUE_TEMPLATE/")
        || path == ".github/PULL_REQUEST_TEMPLATE.md"
}

/// The default base when `--base` isn't given: `origin/main` when that ref exists (a normal
/// clone, CI), else `main` (this repository's bare clone with worktrees has no `origin/*`).
pub(crate) fn default_base(root: &Path) -> &'static str {
    pick_base(git(root, &["rev-parse", "--verify", "--quiet", "origin/main"]).is_ok())
}

fn pick_base(has_origin_main: bool) -> &'static str {
    if has_origin_main { "origin/main" } else { "main" }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_renames_should_map_each_old_path_to_its_new_one() {
        let renames = super::parse_renames("R100\0crates/a/x.rs\0crates/b/x.rs\0M\0README.md\0R087\0a.md\0b.md\0");
        assert_eq!(
            renames.into_iter().collect::<Vec<_>>(),
            [
                ("a.md".to_owned(), "b.md".to_owned()),
                ("crates/a/x.rs".to_owned(), "crates/b/x.rs".to_owned())
            ]
        );
    }

    use super::{is_docs, pick_base, touches_code};

    /// One example per allowlist pattern.
    const DOCS: [&str; 9] = [
        "docs/todos/README.md",
        "docs/research/notes/figure.svg",
        "README.md",
        "CLAUDE.md",
        "CHANGELOG.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        ".github/ISSUE_TEMPLATE/bug.yml",
        ".github/PULL_REQUEST_TEMPLATE.md",
    ];

    /// Root config the suites depend on, code and an unknown new root file.
    const CODE: [&str; 21] = [
        "rust-toolchain.toml",
        ".github/workflows/ci.yml",
        ".github/workflows/full.yml",
        ".github/actions/isolate-rust-home/action.yml",
        ".github/dependabot.yml",
        ".cargo/config.toml",
        ".config/nextest.toml",
        ".gitattributes",
        "biome.json",
        "tsconfig.base.json",
        ".nvmrc",
        ".npmrc",
        "Cargo.toml",
        "Cargo.lock",
        "pnpm-lock.yaml",
        "deny.toml",
        "crates/laminapdf-cos/src/lib.rs",
        "fixtures/corpus/hello.pdf",
        "tools/xtask/src/main.rs",
        "docs.toml",
        "brand-new-root-file",
    ];

    /// Markdown that can be a build input (`#![doc = include_str!("../README.md")]`, doc
    /// tests, a docs site's content) or sits outside the allowlisted folders.
    const CODE_MARKDOWN: [&str; 7] = [
        "crates/laminapdf-cos/README.md",
        "packages/sdk/README.md",
        "apps/docs/src/content/docs/index.md",
        "tools/corpus-facts/README.md",
        "fuzz/README.md",
        "fixtures/README.md",
        ".github/SECURITY.md",
    ];

    #[test]
    fn markdown_outside_the_docs_folders_is_code() {
        let docs: Vec<&str> = CODE_MARKDOWN.into_iter().filter(|p| is_docs(p)).collect();
        assert!(docs.is_empty(), "classified as docs: {docs:?}");
    }

    #[test]
    fn every_allowlist_pattern_is_docs() {
        let code: Vec<&str> = DOCS.into_iter().filter(|p| !is_docs(p)).collect();
        assert!(code.is_empty(), "classified as code: {code:?}");
    }

    #[test]
    fn root_config_and_unknown_files_are_code() {
        let docs: Vec<&str> = CODE.into_iter().filter(|p| is_docs(p)).collect();
        assert!(docs.is_empty(), "classified as docs: {docs:?}");
    }

    #[test]
    fn license_counts_only_at_the_root() {
        assert!(!is_docs("crates/laminapdf-cos/LICENSE-MIT"));
    }

    #[test]
    fn a_docs_only_change_is_not_code() {
        assert!(!touches_code(&["docs/a.md", "README.md", "LICENSE-MIT"]));
    }

    #[test]
    fn one_code_path_makes_a_change_code() {
        assert!(touches_code(&["docs/a.md", ".nvmrc"]));
    }

    #[test]
    fn the_default_base_falls_back_to_main_without_origin() {
        assert_eq!((pick_base(true), pick_base(false)), ("origin/main", "main"));
    }

    #[test]
    fn an_empty_change_is_not_code() {
        assert!(!touches_code(&[]));
    }
}
