//! `render-diff --reuse`: the nightly renders the corpus again only when something that decides
//! the result changed. The result is a function of the renderer's code (`tools/render` and every
//! workspace crate it builds from, its lockfile, the Rust toolchain), the corpus (whose manifest
//! pins every file's SHA-256), xtask's own comparison and runner code, the run's arguments and
//! the machine; [`inputs_key`] hashes all of them. The nightly keeps the last report in the
//! Actions cache with its key ([`STAMP`]), and a run whose key matches reuses that report.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use serde_json::Value;

use super::Args;
use crate::step;
use crate::util::{git_stdout, github_output, sha256_hex, summary};

/// The key of the inputs the report in the output folder was made from.
pub(super) const STAMP: &str = "inputs.sha256";

/// Paths besides the renderer's own crates that decide the result: its lockfile and toolchain,
/// the corpus manifest, and xtask's comparison, sampling, runner and PDFium pin.
const ALSO: [&str; 9] = [
    "tools/render/Cargo.lock",
    "rust-toolchain.toml",
    "fixtures/corpus-manifest.toml",
    "tools/xtask/src/render_diff.rs",
    "tools/xtask/src/render_diff",
    "tools/xtask/src/runner.rs",
    "tools/xtask/src/corpus.rs",
    "tools/xtask/src/live.rs",
    "tools/xtask/src/pdfium.rs",
];

/// The key of everything that decides this run's result, or `None` when it can't be known:
/// a source with uncommitted changes (git's hashes wouldn't cover them), or git or cargo failing.
pub(super) fn inputs_key(root: &Path, args: &Args) -> Option<String> {
    let mut paths = renderer_crates(root)?;
    paths.extend(ALSO.iter().map(PathBuf::from));
    let names: Vec<String> = paths.iter().map(|p| p.to_string_lossy().replace('\\', "/")).collect();
    let mut status = vec!["status", "--porcelain", "--"];
    status.extend(names.iter().map(String::as_str));
    if !git_stdout(root, &status)?.trim().is_empty() {
        return None;
    }
    let mut key = format!(
        "os {} {}\nrunner {}\nsources {:?}\npages {:?}\nsample {:?}\ntop {}\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        args.runner,
        args.sources,
        args.pages,
        args.sample,
        args.top
    );
    for name in &names {
        let object = git_stdout(root, &["rev-parse", &format!("HEAD:{name}")])?;
        let _ = writeln!(key, "{name} {}", object.trim());
    }
    Some(sha256_hex(key.as_bytes()))
}

/// The folders, relative to `root`, of every local package `tools/render` builds from.
fn renderer_crates(root: &Path) -> Option<Vec<PathBuf>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--locked", "--manifest-path"])
        .arg(root.join(crate::render::MANIFEST))
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let metadata: Value = serde_json::from_slice(&output.stdout).ok()?;
    let root = root.canonicalize().ok()?;
    let mut dirs: Vec<PathBuf> = metadata
        .get("packages")?
        .as_array()?
        .iter()
        .filter(|p| p.get("source").is_some_and(Value::is_null))
        .filter_map(|p| {
            let manifest = PathBuf::from(p.get("manifest_path")?.as_str()?);
            let dir = manifest.parent()?.canonicalize().ok()?;
            dir.strip_prefix(&root).ok().map(Path::to_path_buf)
        })
        .collect();
    dirs.sort();
    dirs.dedup();
    (!dirs.is_empty()).then_some(dirs)
}

/// Whether `out` holds a report made from `key`'s inputs.
fn matches(out: &Path, key: &str) -> bool {
    out.join("report.json").is_file() && fs::read_to_string(out.join(STAMP)).is_ok_and(|stamp| stamp.trim() == key)
}

/// When `out` holds a report made from `key`'s inputs, reports it again (job summary, no
/// regression: there is no new point to compare) and returns `true`.
pub(super) fn reused(out: &Path, key: &str) -> Result<bool> {
    if !matches(out, key) {
        return Ok(false);
    }
    step("render-diff: nothing that decides the result changed since the last report; reusing it");
    github_output("regression", "false")?;
    let markdown = fs::read_to_string(out.join("summary.md")).unwrap_or_default();
    summary(&format!(
        "{markdown}\n_Reused: the renderer's code, the corpus and the settings are the same as when this report was made._\n"
    ))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_should_need_the_report_and_the_same_key() -> Result<()> {
        let out = std::env::temp_dir().join(format!("xtask-render-diff-reuse-{}", std::process::id()));
        fs::create_dir_all(&out)?;
        fs::write(out.join(STAMP), "abc\n")?;
        assert!(!matches(&out, "abc"), "no report.json");
        fs::write(out.join("report.json"), "{}")?;
        assert!(matches(&out, "abc"));
        assert!(!matches(&out, "abd"));
        fs::remove_dir_all(&out)?;
        Ok(())
    }

    #[test]
    fn inputs_key_should_cover_the_renderer_crates_and_change_with_the_arguments() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        // A checkout with local edits gives no key, which is right too: nothing to assert then.
        let Some(key) = inputs_key(&root, &Args::default()) else {
            return;
        };
        let crates = renderer_crates(&root).unwrap_or_default();
        assert!(crates.iter().any(|c| c.ends_with("laminapdf-render")), "{crates:?}");
        let other = Args {
            pages: Some(20),
            ..Args::default()
        };
        assert_ne!(Some(key), inputs_key(&root, &other));
    }
}
