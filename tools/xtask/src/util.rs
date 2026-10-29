//! Small helpers the nightly tasks share: GitHub Actions outputs and job summaries, appending
//! to files, and reading one line from git.

use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

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

/// `text` safe inside a Markdown table cell (a `|` would end the cell, even in a code span).
pub(crate) fn md_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::md_cell;

    #[test]
    fn md_cell_should_escape_pipes_and_newlines() {
        assert_eq!(md_cell("a|b\nc"), "a\\|b c");
    }
}
