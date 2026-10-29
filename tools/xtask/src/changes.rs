//! `cargo xtask changes`: does a branch touch code, or only docs? The full lane's first job
//! (`.github/workflows/full.yml`, ADR 0018 §2) runs it, and its suites run only for code.
//!
//! The filter is an allowlist of documentation paths ([`is_docs`]); every other path counts as
//! code. Inverting it this way means a root config file nobody thought to list
//! (`.cargo/config.toml`, `.gitattributes`, `rust-toolchain.toml`, a new workflow) always runs
//! the suites instead of slipping through.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

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

/// True when any path is outside the docs allowlist. No paths at all is docs-only.
pub(crate) fn touches_code(paths: &[&str]) -> bool {
    paths.iter().any(|p| !is_docs(p))
}

/// The docs allowlist: `docs/**`, Markdown anywhere (READMEs, `CLAUDE.md`, `CHANGELOG.md`),
/// `LICENSE*` at the root and the GitHub issue and pull request templates.
pub(crate) fn is_docs(path: &str) -> bool {
    let is_markdown = Path::new(path)
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("md"));
    path.starts_with("docs/")
        || is_markdown
        || (path.starts_with("LICENSE") && !path.contains('/'))
        || path.starts_with(".github/ISSUE_TEMPLATE/")
        || path == ".github/PULL_REQUEST_TEMPLATE.md"
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("git printed a path that is not UTF-8")
}

#[cfg(test)]
mod tests {
    use super::{is_docs, touches_code};

    /// One example per allowlist pattern.
    const DOCS: [&str; 10] = [
        "docs/todos/README.md",
        "docs/research/notes/figure.svg",
        "README.md",
        "CLAUDE.md",
        "CHANGELOG.md",
        "crates/lamina-cos/README.md",
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
        "crates/lamina-cos/src/lib.rs",
        "fixtures/corpus/hello.pdf",
        "tools/xtask/src/main.rs",
        "docs.toml",
        "brand-new-root-file",
    ];

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
        assert!(!is_docs("crates/lamina-cos/LICENSE-MIT"));
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
    fn an_empty_change_is_not_code() {
        assert!(!touches_code(&[]));
    }
}
