//! Many inputs at once ([ADR 0031], amended by the batch rules): the inputs from the command
//! line and `--files-from`, run on a pool of threads, their results handed back in argument
//! order, and the batch's exit code.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;

use laminapdf::ErrorCode;
use schemars::JsonSchema;
use serde::Serialize;

use crate::exit::{CliError, ExitCode};
use crate::io::STDIO;

/// The most threads `--jobs` takes by default, to stay polite on shared machines.
pub const MAX_DEFAULT_JOBS: usize = 8;

/// The default `--jobs`: the machine's parallelism, at most [`MAX_DEFAULT_JOBS`].
#[must_use]
pub fn default_jobs() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().min(MAX_DEFAULT_JOBS))
}

/// The inputs a command names: those on its command line, then those `--files-from` lists
/// (one per line, or separated by NUL bytes with `-0`; empty entries skipped).
///
/// # Errors
///
/// `invalid-command-line` when there is no input at all, or `-` (standard input) is named
/// more than once, as an input or as the list itself.
pub fn inputs(named: &[String], files_from: Option<&[u8]>, nul: bool) -> Result<Vec<String>, CliError> {
    let mut all = named.to_vec();
    if let Some(list) = files_from {
        let separator = if nul { b'\0' } else { b'\n' };
        for entry in list.split(|&b| b == separator) {
            let entry = entry.strip_suffix(b"\r").filter(|_| !nul).unwrap_or(entry);
            if entry.is_empty() {
                continue;
            }
            let name = String::from_utf8(entry.to_vec()).map_err(|_| {
                CliError::new(ErrorCode::InvalidCommandLine, "a name in the --files-from list isn't UTF-8")
            })?;
            all.push(name);
        }
    }
    if all.is_empty() {
        return Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "name at least one input, or list them with --files-from",
        ));
    }
    if all.iter().filter(|i| *i == STDIO).count() > 1 {
        return Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "standard input can be read only once",
        ));
    }
    Ok(all)
}

/// How a batch runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchOptions {
    /// Threads at once (at least one).
    pub jobs: usize,
    /// Stop starting inputs after the first failure.
    pub fail_fast: bool,
}

/// Runs `work` on each input on up to `options.jobs` threads, and hands each result to `emit`
/// in argument order as soon as every input before it has been handed over. With
/// `fail_fast`, no input starts after a failure; those never started aren't handed over.
/// Returns how many inputs were skipped.
pub fn run<T, W, E>(inputs: &[String], options: BatchOptions, work: W, mut emit: E) -> usize
where
    T: Send,
    W: Fn(&str) -> Result<T, CliError> + Sync,
    E: FnMut(usize, &str, Result<T, CliError>),
{
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let (sender, receiver) = mpsc::channel();
    let mut emitted = 0;
    std::thread::scope(|scope| {
        for _ in 0..options.jobs.clamp(1, inputs.len().max(1)) {
            let sender = sender.clone();
            let (next, stop, work) = (&next, &stop, &work);
            scope.spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    let index = next.fetch_add(1, Ordering::AcqRel);
                    let Some(input) = inputs.get(index) else {
                        break;
                    };
                    let result = work(input);
                    if options.fail_fast && result.is_err() {
                        stop.store(true, Ordering::Release);
                    }
                    if sender.send((index, result)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        // Results that arrived before an earlier input's, by index.
        let mut waiting = BTreeMap::new();
        for (index, result) in receiver {
            waiting.insert(index, result);
            while let Some(result) = waiting.remove(&emitted) {
                let input = inputs.get(emitted).map_or("", String::as_str);
                emit(emitted, input, result);
                emitted += 1;
            }
        }
    });
    inputs.len().saturating_sub(emitted)
}

/// How one input ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// Its report, with no warnings.
    Ok,
    /// Its report, with warnings.
    Warnings,
    /// An error instead of a report.
    Error,
}

/// The counts a batch ends with: its last NDJSON line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// Always `summary`.
    #[serde(rename = "type")]
    #[schemars(schema_with = "summary_type")]
    pub kind: SummaryKind,
    /// Inputs whose report had no warnings.
    pub ok: usize,
    /// Inputs whose report had warnings.
    pub warnings: usize,
    /// Inputs that failed.
    pub failed: usize,
    /// Inputs `--fail-fast` never started.
    pub skipped: usize,
    /// How long the batch took, in milliseconds.
    pub elapsed_ms: u64,
    /// Whether every failure was a missing or wrong password.
    #[serde(skip)]
    pub only_password_failures: bool,
}

/// The `type` of a summary line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SummaryKind {
    /// `summary`.
    #[default]
    Summary,
}

fn summary_type(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({ "const": "summary" })
}

impl Summary {
    /// Counts one input's outcome: `Ok(has_warnings)`, or the error's exit code.
    pub fn count(&mut self, outcome: Result<bool, ExitCode>) {
        match outcome {
            Ok(false) => self.ok += 1,
            Ok(true) => self.warnings += 1,
            Err(exit) => {
                let first = self.failed == 0;
                self.failed += 1;
                self.only_password_failures = (first || self.only_password_failures) && exit == ExitCode::Password;
            }
        }
    }

    /// The batch's exit code: `1` when an input failed (`5` when every failure was a
    /// password's), else `3` for warnings unless `warnings_ok`, else `0`.
    #[must_use]
    pub const fn exit_code(&self, warnings_ok: bool) -> ExitCode {
        if self.failed > 0 {
            if self.only_password_failures {
                ExitCode::Password
            } else {
                ExitCode::Failure
            }
        } else if self.warnings > 0 && !warnings_ok {
            ExitCode::Warnings
        } else {
            ExitCode::Ok
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;

    fn names(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("in{i}.pdf")).collect()
    }

    #[test]
    fn results_should_come_back_in_argument_order_whatever_finishes_first() {
        let inputs = names(12);
        let mut seen = Vec::new();
        let skipped = run(
            &inputs,
            BatchOptions {
                jobs: 4,
                fail_fast: false,
            },
            |input| {
                // The first inputs take longest.
                let n: u64 = input.trim_start_matches("in").trim_end_matches(".pdf").parse().unwrap_or(0);
                std::thread::sleep(Duration::from_millis(12u64.saturating_sub(n) * 3));
                Ok(n)
            },
            |index, _, result| seen.push((index, result.ok())),
        );
        let expected: Vec<(usize, Option<u64>)> = (0..12).map(|i| (i, Some(i as u64))).collect();
        assert_eq!((seen, skipped), (expected, 0));
    }

    #[test]
    fn fail_fast_should_start_nothing_after_a_failure() {
        let inputs = names(50);
        let started = Mutex::new(0usize);
        let mut handed = 0;
        let skipped = run(
            &inputs,
            BatchOptions {
                jobs: 1,
                fail_fast: true,
            },
            |input| {
                *started.lock().unwrap_or_else(std::sync::PoisonError::into_inner) += 1;
                if input == "in2.pdf" {
                    Err(CliError::new(ErrorCode::InputUnreadable, "no such file"))
                } else {
                    Ok(())
                }
            },
            |_, _, _| handed += 1,
        );
        assert_eq!((handed, skipped), (3, 47));
        assert_eq!(*started.lock().unwrap_or_else(std::sync::PoisonError::into_inner), 3);
    }

    #[test]
    fn inputs_should_join_the_command_line_and_the_list() {
        let named = vec!["a.pdf".to_owned()];
        assert_eq!(
            inputs(&named, Some(b"b.pdf\r\n\nc d.pdf\n"), false).ok(),
            Some(vec!["a.pdf".to_owned(), "b.pdf".to_owned(), "c d.pdf".to_owned()])
        );
        assert_eq!(
            inputs(&[], Some(b"x\ny.pdf\0z.pdf\0"), true).ok(),
            Some(vec!["x\ny.pdf".to_owned(), "z.pdf".to_owned()])
        );
        let refused = |named: &[String]| inputs(named, None, false).map_err(|e| e.diagnostic().code.clone());
        assert_eq!(refused(&[]).err().as_deref(), Some("invalid-command-line"));
        assert_eq!(
            refused(&["-".to_owned(), "-".to_owned()]).err().as_deref(),
            Some("invalid-command-line")
        );
    }

    #[test]
    fn the_exit_code_should_follow_the_counts() {
        let summary = |outcomes: &[Result<bool, ExitCode>]| {
            let mut s = Summary::default();
            outcomes.iter().for_each(|o| s.count(*o));
            s.exit_code(false)
        };
        assert_eq!(summary(&[Ok(false), Ok(false)]), ExitCode::Ok);
        assert_eq!(summary(&[Ok(false), Ok(true)]), ExitCode::Warnings);
        assert_eq!(summary(&[Ok(true), Err(ExitCode::Failure)]), ExitCode::Failure);
        assert_eq!(
            summary(&[Ok(false), Err(ExitCode::Password), Err(ExitCode::Password)]),
            ExitCode::Password
        );
        assert_eq!(
            summary(&[Err(ExitCode::Password), Err(ExitCode::Failure)]),
            ExitCode::Failure
        );
    }
}
