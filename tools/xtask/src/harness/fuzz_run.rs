//! One fuzz target's run and what it leaves behind: its output in `target/xtask/fuzz/<target>.log`
//! and, for a run that fuzzed (whether it ended cleanly or crashed), a summary in
//! `target/xtask/fuzz/<target>.json` ([`FuzzRun`]) that `cargo xtask proof` reads. A run that
//! failed before fuzzing (a build or setup problem) leaves no summary, so it never reads as a
//! clean night.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::workspace::Workspace;

/// Where the logs and summaries go, relative to the workspace root.
pub(crate) const SUMMARY_DIR: &str = "target/xtask/fuzz";

/// The libFuzzer statistic the summary reads (printed with `-print_final_stats=1`).
const EXECUTED_UNITS: &str = "stat::number_of_executed_units:";

/// `target/xtask/fuzz/<target>.json`: what one target's run did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FuzzRun {
    /// The time budget the run was given (`-max_total_time`), in seconds. A run that crashes
    /// stops early, so this is what the night asked for, not how long it fuzzed.
    pub(crate) seconds: u64,
    /// How long it fuzzed, from libFuzzer's `Done <N> runs in <S> second(s)`; `None` when it
    /// printed none (a crash).
    pub(crate) fuzzed_seconds: Option<u64>,
    /// libFuzzer's `stat::number_of_executed_units`, or `None` when its final stats are missing.
    pub(crate) executions: Option<u64>,
    /// The files in the target's corpus folder after the run.
    pub(crate) corpus_files: u64,
    /// Whether the run saved a crash, timeout or OOM input in `fuzz/artifacts/<target>/`.
    pub(crate) crashed: bool,
}

/// How a run's output reaches the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Echo {
    /// As it comes (one target at a time).
    Live,
    /// Whole, every line prefixed `[target]`, when the run ends (several targets at once).
    Prefixed,
}

/// One `cargo fuzz run` of `target` over `corpus` with `args`, its output written to
/// `<root>/target/xtask/fuzz/<target>.log` and echoed per `echo`; then its summary. A failure
/// that left new files in `fuzz/artifacts/<target>/` is a crash; any other is a build or setup
/// failure.
pub(super) fn run_target(
    ws: &Workspace,
    target: &str,
    corpus: &Path,
    seconds: u64,
    args: &[String],
    echo: Echo,
) -> Result<()> {
    let dir = ws.root.join(SUMMARY_DIR);
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let summary_path = forget_last_run(&dir, target)?;
    let log = dir.join(format!("{target}.log"));
    let watch = ArtifactWatch::new(&ws.root, target);
    match echo {
        Echo::Live => crate::step(&format!("cargo {} (log: {})", args.join(" "), log.display())),
        Echo::Prefixed => println!("[{target}] started: cargo {}", args.join(" ")),
    }
    let status = spawn(&ws.root, args, &log, echo).map_err(|e| watch.failure(&format!("{e:#}")))?;
    let text = String::from_utf8_lossy(&fs::read(&log).unwrap_or_default()).into_owned();
    if echo == Echo::Prefixed {
        // A closed stdout must not hide the run's result, so a failed write is ignored.
        let _ = writeln!(
            std::io::stdout().lock(),
            "{}[{target}] finished: {status}",
            prefixed(target, &text)
        );
    }
    let crashed = !watch.new_artifacts().is_empty();
    if status.success() || crashed {
        let corpus_files = super::files_in(corpus).map_or(0, |files| files.len());
        let summary = FuzzRun {
            seconds,
            fuzzed_seconds: fuzzed_seconds(&text),
            executions: executed_units(&text),
            corpus_files: u64::try_from(corpus_files).unwrap_or(u64::MAX),
            crashed,
        };
        fs::write(&summary_path, serde_json::to_string_pretty(&summary)? + "\n")
            .with_context(|| format!("writing {}", summary_path.display()))?;
    }
    if status.success() {
        Ok(())
    } else {
        Err(watch.failure(&format!("cargo fuzz run exited with {status} (log: {})", log.display())))
    }
}

/// Runs `cargo args…` in `root` with stdout and stderr in `log`; [`Echo::Live`] copies both to
/// the terminal as they come.
fn spawn(root: &Path, args: &[String], log: &Path, echo: Echo) -> Result<ExitStatus> {
    let file = fs::File::create(log).with_context(|| format!("creating {}", log.display()))?;
    let mut command = Command::new(crate::program_path("cargo"));
    command.args(args).current_dir(root);
    if echo == Echo::Prefixed {
        let stderr = file.try_clone().with_context(|| format!("opening {}", log.display()))?;
        return command
            .stdout(Stdio::from(file))
            .stderr(Stdio::from(stderr))
            .status()
            .context("failed to start cargo");
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start cargo")?;
    let file = Mutex::new(file);
    let (stdout, stderr) = (child.stdout.take(), child.stderr.take());
    std::thread::scope(|scope| {
        if let Some(pipe) = stdout {
            scope.spawn(|| tee(pipe, &file, std::io::stdout()));
        }
        if let Some(pipe) = stderr {
            scope.spawn(|| tee(pipe, &file, std::io::stderr()));
        }
    });
    child.wait().context("waiting for cargo")
}

/// Copies `from` into `log` and `to` until it ends. A write that fails is dropped: the run's
/// result matters more than a complete echo.
fn tee(mut from: impl Read, log: &Mutex<fs::File>, mut to: impl Write) {
    let mut buffer = [0u8; 8192];
    while let Ok(n @ 1..) = from.read(&mut buffer) {
        let chunk = buffer.get(..n).unwrap_or_default();
        let _ = to.write_all(chunk);
        let _ = log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write_all(chunk);
    }
}

/// The last `stat::number_of_executed_units` libFuzzer printed in `log`.
pub(super) fn executed_units(log: &str) -> Option<u64> {
    log.lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix(EXECUTED_UNITS))
        .and_then(|n| n.trim().parse().ok())
}

/// How long libFuzzer says it fuzzed: the last `Done <N> runs in <S> second(s)` in `log`. A run
/// that crashed prints none.
pub(super) fn fuzzed_seconds(log: &str) -> Option<u64> {
    log.lines().rev().find_map(|line| {
        let rest = line.trim().strip_prefix("Done ")?;
        let (_, rest) = rest.split_once(" runs in ")?;
        rest.strip_suffix(" second(s)")?.trim().parse().ok()
    })
}

/// Removes `<dir>/<target>.json` from an earlier run, so a run that fails before fuzzing leaves
/// no summary; returns its path.
pub(super) fn forget_last_run(dir: &Path, target: &str) -> Result<PathBuf> {
    let path = dir.join(format!("{target}.json"));
    match fs::remove_file(&path) {
        Ok(()) => Ok(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}

/// `text` with `[target] ` before every line.
pub(super) fn prefixed(target: &str, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        out.push('[');
        out.push_str(target);
        out.push_str("] ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The crash inputs a run may leave in `fuzz/artifacts/<target>/`: the names there before
/// it, so a failure can be told apart as a crash (new files) or a build or setup problem.
pub(super) struct ArtifactWatch<'a> {
    target: &'a str,
    dir: PathBuf,
    before: std::collections::BTreeSet<String>,
}

impl<'a> ArtifactWatch<'a> {
    pub(super) fn new(root: &Path, target: &'a str) -> Self {
        let dir = root.join("fuzz/artifacts").join(target);
        let before = artifact_names(&dir);
        Self { target, dir, before }
    }

    /// The files that appeared in the folder since [`ArtifactWatch::new`].
    fn new_artifacts(&self) -> Vec<String> {
        artifact_names(&self.dir).difference(&self.before).cloned().collect()
    }

    /// The error for a failed run: [`describe_failure`] with the files that appeared since.
    pub(super) fn failure(&self, error: &str) -> anyhow::Error {
        anyhow::anyhow!(describe_failure(self.target, &self.new_artifacts(), error))
    }
}

/// How a failed fuzz run is reported: a crash with its saved inputs, or a run that failed
/// without saving any (the build, the toolchain or the corpus is the problem, not the code).
pub(super) fn describe_failure(target: &str, new_artifacts: &[String], error: &str) -> String {
    if new_artifacts.is_empty() {
        format!("{target}: failed without a crash input (build or setup problem): {error}")
    } else {
        format!(
            "{target}: crashed; inputs saved in fuzz/artifacts/{target}/: {}",
            new_artifacts.join(", ")
        )
    }
}

/// The file names in `dir` (empty when it doesn't exist).
fn artifact_names(dir: &Path) -> std::collections::BTreeSet<String> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The end of a libFuzzer run with `-print_final_stats=1`, as cargo fuzz passes it on.
    const TAIL: &str = "#1048576\tpulse  cov: 812 ft: 2210 corp: 301/18Kb exec/s: 3495 rss: 412Mb\n\
        Done 2097152 runs in 601 second(s)\n\
        stat::number_of_executed_units: 2097152\n\
        stat::average_exec_per_sec:     3489\n\
        stat::new_units_added:          57\n\
        stat::slowest_unit_time_sec:    0\n\
        stat::peak_rss_mb:              418\n";

    #[test]
    fn executed_units_should_read_libfuzzers_final_stats() {
        assert_eq!(executed_units(TAIL), Some(2_097_152));
    }

    #[test]
    fn fuzzed_seconds_should_read_the_done_line() {
        assert_eq!(fuzzed_seconds(TAIL), Some(601));
    }

    #[test]
    fn fuzzed_seconds_should_be_none_after_a_crash() {
        assert_eq!(fuzzed_seconds("==1== ERROR: libFuzzer: deadly signal\n"), None);
    }

    #[test]
    fn forget_last_run_should_remove_an_earlier_summary() {
        let dir = std::env::temp_dir().join(format!("xtask-fuzz-forget-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("lex.json"), "{}").unwrap();
        let path = forget_last_run(&dir, "lex").unwrap();
        let again = forget_last_run(&dir, "lex").is_ok();
        fs::remove_dir_all(&dir).unwrap();
        assert!(!path.exists() && again);
    }

    #[test]
    fn executed_units_should_be_none_without_final_stats() {
        assert_eq!(executed_units("Done 10 runs in 1 second(s)\n"), None);
    }

    #[test]
    fn a_summary_should_round_trip_through_its_json() {
        let run = FuzzRun {
            seconds: 600,
            fuzzed_seconds: Some(600),
            executions: Some(5),
            corpus_files: 3,
            crashed: false,
        };
        let text = serde_json::to_string(&run).unwrap();
        assert_eq!(serde_json::from_str::<FuzzRun>(&text).unwrap(), run);
    }
}
