//! A command over several inputs (ADR 0031's batch rules): each input runs through
//! [`laminapdf_cli::batch::run`], and its outcome is printed in argument order, as an NDJSON
//! `result` line under `--json` or as text under a `==> input <==` header, then a summary.

use std::io::{IsTerminal as _, Write as _};
use std::time::Instant;

use clap::Args;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use laminapdf::ErrorCode;
use laminapdf_cli::batch::{self, BatchOptions, BatchSummary, Status};
use laminapdf_cli::exit::{CliError, ExitCode};
use laminapdf_cli::io::STDIO;
use serde_json::{Map, Value};

use crate::render::{self, Streams};
use crate::{GlobalArgs, Outcome};

/// How a command runs over several inputs.
#[derive(Debug, Args)]
pub(crate) struct BatchArgs {
    /// Read more inputs from FILE (or `-`), one per line.
    #[arg(long, value_name = "FILE")]
    pub(crate) files_from: Option<String>,
    /// With --files-from: the names are separated by NUL bytes (`find -print0`, `fd -0`).
    #[arg(short = '0', long = "null", requires = "files_from")]
    pub(crate) null: bool,
    /// Inputs read at once; the machine's parallelism, at most 8, by default.
    #[arg(short, long, value_name = "N", value_parser = clap::value_parser!(u16).range(1..))]
    pub(crate) jobs: Option<u16>,
    /// Start no input after the first failure; the summary counts those skipped.
    #[arg(long)]
    pub(crate) fail_fast: bool,
}

/// A command's own arguments, after its inputs and the batch flags.
#[derive(Debug, Args)]
pub(crate) struct Many<A: Args> {
    /// The PDFs to read, or `-` for standard input.
    #[arg(value_name = "INPUT")]
    pub(crate) inputs: Vec<String>,
    #[command(flatten)]
    pub(crate) batch: BatchArgs,
    #[command(flatten)]
    pub(crate) args: A,
}

impl<A: Args> Many<A> {
    /// Every input: the command line's, then `--files-from`'s.
    pub(crate) fn expand(&self, io: &mut dyn laminapdf_cli::io::Io) -> Result<Vec<String>, CliError> {
        let list = match &self.batch.files_from {
            Some(file) => Some(io.read(file)?),
            None => None,
        };
        let inputs = batch::inputs(&self.inputs, list.as_deref(), self.batch.null)?;
        // The list took standard input, so no input can.
        if self.batch.files_from.as_deref() == Some(STDIO) && inputs.iter().any(|i| i == STDIO) {
            return Err(CliError::new(
                ErrorCode::InvalidCommandLine,
                "standard input can be read only once",
            ));
        }
        Ok(inputs)
    }
}

/// What every result line shares with the report it carries.
fn result_line(index: usize, input: &str, schema: &str, outcome: &Result<Outcome, CliError>) -> Value {
    let mut line = Map::new();
    line.insert("type".to_owned(), "result".into());
    line.insert("schema".to_owned(), schema.into());
    line.insert("index".to_owned(), index.into());
    line.insert("input".to_owned(), input.into());
    match outcome {
        Ok(outcome) => {
            let status = if outcome.warnings.is_empty() {
                Status::Ok
            } else {
                Status::Warnings
            };
            line.insert("status".to_owned(), serde_json::to_value(status).unwrap_or_default());
            if let Value::Object(report) = &outcome.json {
                for (key, value) in report {
                    line.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
        }
        Err(error) => {
            line.insert(
                "status".to_owned(),
                serde_json::to_value(Status::Error).unwrap_or_default(),
            );
            let error = error.to_line(Some(input)).get("error").cloned().unwrap_or_default();
            line.insert("error".to_owned(), error);
        }
    }
    Value::Object(line)
}

/// A progress bar on standard error, when it is a terminal and `--quiet` isn't set.
fn progress(global: &GlobalArgs, total: usize) -> ProgressBar {
    if global.quiet || !std::io::stderr().is_terminal() {
        return ProgressBar::hidden();
    }
    let bar = ProgressBar::with_draw_target(
        Some(u64::try_from(total).unwrap_or(u64::MAX)),
        ProgressDrawTarget::stderr(),
    );
    if let Ok(style) = ProgressStyle::with_template("{bar:30} {pos}/{len} {elapsed}") {
        bar.set_style(style);
    }
    bar
}

/// Runs `work` over `inputs` and prints each outcome; the batch's exit code.
pub(crate) fn run<W>(
    streams: &mut Streams,
    global: &GlobalArgs,
    batch: &BatchArgs,
    inputs: &[String],
    schema: &str,
    work: W,
) -> ExitCode
where
    W: Fn(&str) -> Result<Outcome, CliError> + Sync,
{
    let started = Instant::now();
    let options = BatchOptions {
        jobs: batch.jobs.map_or_else(batch::default_jobs, usize::from),
        fail_fast: batch.fail_fast,
    };
    let bar = progress(global, inputs.len());
    let mut summary = BatchSummary::default();
    let mut unwritable: Option<std::io::Error> = None;
    let terminal = streams.out.is_terminal();
    let skipped = batch::run(inputs, options, work, |index, input, outcome| {
        summary.count(
            outcome
                .as_ref()
                .map(|o| !o.warnings.is_empty())
                .map_err(CliError::exit_code),
        );
        bar.suspend(|| {
            let written = if global.json {
                let line = result_line(index, input, schema, &outcome);
                writeln!(streams.out, "{line}")
            } else {
                match &outcome {
                    Ok(outcome) => {
                        if !global.quiet {
                            render::warnings(streams, &outcome.warnings);
                        }
                        let text = if terminal {
                            render::for_terminal(&outcome.text)
                        } else {
                            outcome.text.clone()
                        };
                        let header = if terminal {
                            render::for_terminal(input)
                        } else {
                            input.to_owned()
                        };
                        let gap = if index > 0 { "\n" } else { "" };
                        write!(streams.out, "{gap}==> {header} <==\n{text}")
                    }
                    Err(error) => {
                        render::error(streams, error, Some(input), false);
                        Ok(())
                    }
                }
            };
            if let Err(error) = written {
                unwritable.get_or_insert(error);
            }
        });
        bar.inc(1);
    });
    bar.finish_and_clear();
    summary.skipped = skipped;
    summary.elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let written = if global.json {
        let line = serde_json::to_value(&summary).unwrap_or_default();
        writeln!(streams.out, "{line}").and_then(|()| streams.out.flush())
    } else {
        if !global.quiet {
            let _ = writeln!(
                streams.err,
                "{} ok, {} with warnings, {} failed, {} skipped",
                summary.ok, summary.warnings, summary.failed, summary.skipped
            );
        }
        streams.out.flush()
    };
    if let Some(error) = unwritable.or(written.err()) {
        let error = CliError::new(ErrorCode::OutputUnwritable, "couldn't write the output")
            .with("output", STDIO)
            .with("reason", format!("{:?}", error.kind()));
        render::error(streams, &error, None, global.json);
        return error.exit_code();
    }
    summary.exit_code(global.warnings_ok)
}
