//! Small helpers the tasks share: GitHub Actions outputs and job summaries, appending to files,
//! reading one line from git, running the GitHub CLI, and waiting for a process with a time limit.

use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::program_path;

/// Appends `text` to `path`, creating it if needed.
pub(crate) fn append(path: &Path, text: &str) -> Result<()> {
    let mut file = fs::OpenOptions::new().append(true).create(true).open(path)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}

/// The last part of `path`, as text (lossy), or empty when it has none.
pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
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

/// The standard output of `program args…`, or `None` when it can't run or fails. On Windows the
/// program may be a `.cmd` shim, such as pnpm's ([`program_path`]).
pub(crate) fn stdout(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program_path(program)).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
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

/// `path` with `/` between its parts on every system, as messages and reports name repository
/// paths (Windows' `display()` would print `\`).
pub(crate) fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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

/// Copies the folder `from` into `to`, recursively.
pub(crate) fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).with_context(|| format!("copying to {}", target.display()))?;
        }
    }
    Ok(())
}

/// Waits up to `limit` for `child`: its exit status, or `None` when it ran longer, after stopping it
/// and every process it started (a browser suite's pnpm, Node, Playwright and browser).
pub(crate) fn wait_up_to(child: &mut Child, limit: Duration) -> Result<Option<ExitStatus>> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if start.elapsed() >= limit {
            stop_tree(child);
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Kills `child`'s descendants, then `child`, and reaps it. On Unix the descendants come from a
/// `pgrep -P` walk, read before any is killed; on Windows `taskkill /T` walks the tree. Not a
/// process group: Playwright starts each browser in a group of its own, which killing pnpm's group
/// would miss. Best effort: a process that exits during the walk leaves its children to init, one
/// started after it escapes, and a process may already have exited.
fn stop_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let mut tree = vec![child.id()];
        let mut next = 0;
        while let Some(&pid) = tree.get(next) {
            next += 1;
            if let Ok(out) = Command::new("pgrep").args(["-P", &pid.to_string()]).output() {
                let children = String::from_utf8_lossy(&out.stdout);
                tree.extend(children.lines().filter_map(|l| l.trim().parse::<u32>().ok()));
            }
        }
        let pids: Vec<String> = tree.iter().skip(1).map(u32::to_string).collect();
        if !pids.is_empty() {
            let _ = Command::new("kill").arg("-KILL").args(&pids).status();
        }
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use std::process::{Command, Stdio};
    use std::time::Duration;

    use anyhow::{Context, Result};

    use super::{md_cell, sha256_hex, slash_path, wait_up_to};

    // `true`, `sleep`, `sh` and `kill` are Unix commands.
    #[cfg(unix)]
    #[test]
    fn wait_up_to_should_tell_a_finished_process_from_one_that_hangs() -> Result<()> {
        let mut quick = Command::new("true").spawn()?;
        let quick = wait_up_to(&mut quick, Duration::from_secs(5))?.map(|s| s.success());
        let mut slow = Command::new("sleep").arg("30").spawn()?;
        let slow = wait_up_to(&mut slow, Duration::from_millis(200))?;
        assert_eq!((quick, slow), (Some(true), None));
        Ok(())
    }

    /// A hung suite's browser is pnpm's grandchild: stopping pnpm alone would leave it running.
    #[cfg(unix)]
    #[test]
    fn wait_up_to_should_stop_the_processes_its_child_started() -> Result<()> {
        let mut shell = Command::new("sh")
            .args(["-c", "sleep 30 & echo $!; wait"])
            .stdout(Stdio::piped())
            .spawn()?;
        let mut line = String::new();
        let out = shell.stdout.take().context("the shell's stdout")?;
        std::io::BufRead::read_line(&mut std::io::BufReader::new(out), &mut line)?;
        let sleep = line.trim().to_owned();
        let ended = wait_up_to(&mut shell, Duration::from_millis(500))?;
        // A killed process stays a zombie until init reaps it, so poll for it gone or a zombie.
        let gone = || -> Result<bool> {
            let ps = Command::new("ps").args(["-o", "stat=", "-p", &sleep]).output()?;
            let state = String::from_utf8_lossy(&ps.stdout);
            Ok(state.trim().is_empty() || state.trim().starts_with('Z'))
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !gone()? && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!((ended, gone()?), (None, true));
        Ok(())
    }

    #[test]
    fn sha256_hex_should_match_the_known_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn slash_path_should_name_windows_paths_with_forward_slashes() {
        assert_eq!(
            slash_path(Path::new("packages\\render\\package.json")),
            "packages/render/package.json"
        );
    }

    #[test]
    fn md_cell_should_escape_pipes_and_newlines() {
        assert_eq!(md_cell("a|b\nc"), "a\\|b c");
    }
}
