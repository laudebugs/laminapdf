//! Small helpers the tasks share: GitHub Actions outputs and job summaries, appending to files,
//! reading one line from git, and running the GitHub CLI.

use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::program_path;

/// Appends `text` to `path`, creating it if needed.
pub(crate) fn append(path: &Path, text: &str) -> Result<()> {
    let mut file = fs::OpenOptions::new().append(true).create(true).open(path)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}

/// Writes `key=value` to `$GITHUB_OUTPUT` when running in Actions.
pub(crate) fn github_output(key: &str, value: &str) -> Result<()> {
    if let Ok(path) = std::env::var("GITHUB_OUTPUT") {
        append(Path::new(&path), &format!("{key}={value}\n"))?;
    }
    Ok(())
}

/// Appends Markdown to the job summary (`$GITHUB_STEP_SUMMARY`) when running in Actions.
pub(crate) fn summary(markdown: &str) -> Result<()> {
    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        append(Path::new(&path), markdown)?;
    }
    Ok(())
}

/// The trimmed stdout of `git args…` in `dir`, or `None` when git fails.
pub(crate) fn git_stdout(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).current_dir(dir).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The stdout of `gh args…` run in `dir`, so `{owner}/{repo}` in a `gh api` path names the
/// repository `dir` belongs to; fails when gh is missing or exits unsuccessfully.
pub(crate) fn gh(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new(program_path("gh"))
        .args(args)
        .current_dir(dir)
        .output()
        .context("failed to run gh; is the GitHub CLI installed (and GH_TOKEN set in Actions)?")?;
    if !output.status.success() {
        bail!(
            "`gh {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("gh printed non-UTF-8 output")
}

/// The SHA-256 of `bytes` as lowercase hex.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// True when `path`'s extension is one of `extensions`, ignoring case.
pub(crate) fn has_extension(path: &str, extensions: &[&str]) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extensions.iter().any(|wanted| extension.eq_ignore_ascii_case(wanted)))
}

/// `text` safe inside a Markdown table cell (a `|` would end the cell, even in a code span).
pub(crate) fn md_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::{md_cell, sha256_hex};

    #[test]
    fn sha256_hex_should_match_the_known_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn md_cell_should_escape_pipes_and_newlines() {
        assert_eq!(md_cell("a|b\nc"), "a\\|b c");
    }
}
