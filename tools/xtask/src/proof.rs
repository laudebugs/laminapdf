//! `cargo xtask proof [--from <dir>] [--jobs <json>] [--history <dir> --runner <label> [--record]]`:
//! gathers what the nightly lane measured into one versioned record ([`Record`], `kind: "proof"`):
//! how Lamina PDF read each corpus source, the differentials against the other readers, the
//! render diff, the fuzz runs, veraPDF, the round trip and the runtimes. It reads each task's
//! output where that task writes it, under `--from` (the workspace root by default), and writes
//! `target/nightly/proof/proof.json` and a job-summary table.
//!
//! A section whose input is missing (its job failed early, was skipped or was cancelled) is
//! `null`, and `jobs` (`toJSON(needs)` in the workflow) says why. With `--record` the record is
//! appended to `<history>/history.jsonl` beside the benchmark figures, but only when the night
//! ran its full budgets ([`is_full`], read from the reports themselves); otherwise it appends
//! nothing.

mod inputs;
mod record;
mod summary;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

pub(crate) use record::Record;
use record::{KIND, VERSION};

use crate::harness::FUZZ_TARGETS;
use crate::nightly::{FUZZ_SECONDS, RENDER_DIFF_PAGES};
use crate::workspace::Workspace;
use crate::{step, util};

/// Where the record is written, relative to the workspace root.
const OUT_DIR: &str = "target/nightly/proof";

const USAGE: &str =
    "usage: cargo xtask proof [--from <dir>] [--jobs <json>] [--history <dir> --runner <label> [--record]]";

/// What `cargo xtask proof` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    from: Option<PathBuf>,
    jobs: Option<String>,
    history: Option<PathBuf>,
    runner: Option<String>,
    record: bool,
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut parsed = Args::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().cloned().context(USAGE);
        match arg.as_str() {
            "--from" => parsed.from = Some(value()?.into()),
            "--jobs" => parsed.jobs = Some(value()?),
            "--history" => parsed.history = Some(value()?.into()),
            "--runner" => parsed.runner = Some(value()?),
            "--record" => parsed.record = true,
            other => bail!("unknown argument {other:?}; {USAGE}"),
        }
    }
    if parsed.record && parsed.history.is_none() {
        bail!("--record needs --history; {USAGE}");
    }
    Ok(parsed)
}

/// Each job's `result` in `toJSON(needs)`: `{"<job>": {"result": "success", "outputs": {…}}, …}`.
fn parse_jobs(json: &str) -> Result<BTreeMap<String, String>> {
    let needs: Value = serde_json::from_str(json).context("--jobs is not JSON")?;
    needs
        .as_object()
        .context("--jobs is not an object of jobs")?
        .iter()
        .map(|(job, v)| {
            let result = v
                .get("result")
                .and_then(Value::as_str)
                .with_context(|| format!("--jobs: `{job}` has no result"))?;
            Ok((job.clone(), result.to_owned()))
        })
        .collect()
}

/// A section's value, or `None` with a warning when its input is there but unreadable: a broken
/// report must not cost the night its other figures.
fn section<T>(name: &str, read: Result<Option<T>>) -> Option<T> {
    read.unwrap_or_else(|error| {
        println!("::warning::proof: the {name} section is null: {error:#}");
        None
    })
}

/// Whether the night ran its full budgets: the differential and the render diff unsampled, the
/// render diff at [`RENDER_DIFF_PAGES`] pages per file, and every fuzz target for at least
/// [`FUZZ_SECONDS`]. A missing report can't show its budget, so it makes the night not full.
pub(crate) fn is_full(record: &Record) -> bool {
    let differential = record.differential.as_ref().is_some_and(|d| d.sample.is_none());
    let render = record
        .render
        .as_ref()
        .is_some_and(|r| r.sample.is_none() && r.pages_per_file == Some(RENDER_DIFF_PAGES));
    let fuzz = record.fuzz.as_ref().is_some_and(|runs| {
        FUZZ_TARGETS
            .iter()
            .all(|t| runs.get(*t).is_some_and(|run| run.seconds >= FUZZ_SECONDS))
    });
    differential && render && fuzz
}

/// What identifies the run, gathered by [`run`]; the tests give fixed values.
pub(crate) struct Run {
    pub(crate) commit: String,
    pub(crate) date: String,
    pub(crate) ran: String,
    pub(crate) runner: String,
    pub(crate) lamina: String,
    pub(crate) jobs: BTreeMap<String, String>,
}

/// The record of the outputs under `root`.
pub(crate) fn record(root: &Path, run: Run) -> Record {
    let mut record = Record {
        kind: KIND.to_owned(),
        version: VERSION,
        commit: run.commit,
        date: run.date,
        ran: run.ran,
        runner: run.runner,
        quick: true,
        lamina: run.lamina,
        full: false,
        jobs: run.jobs,
        corpus: section("corpus", inputs::corpus(root)),
        differential: section("differential", inputs::differential(root)),
        text: section("text", inputs::text_differential(root)),
        render: section("render", inputs::render(root)),
        fuzz: section("fuzz", inputs::fuzz(root)),
        verapdf: section("verapdf", inputs::verapdf(root)),
        roundtrip: section("roundtrip", inputs::roundtrip(root)),
        runtimes: section("runtimes", inputs::runtimes(root)),
    };
    record.full = is_full(&record);
    record.quick = !record.full;
    record
}

/// Appends `record` to `<history>/history.jsonl` when it is full; otherwise says why not.
/// Returns whether it appended.
fn append(history: &Path, record: &Record) -> Result<bool> {
    if !record.full {
        println!(
            "::notice::proof: not a full-budget night (sampled, fewer pages or shorter fuzzing, or a \
             report missing): nothing appended to the history"
        );
        return Ok(false);
    }
    crate::nightly::append_record(history, &serde_json::to_value(record)?)?;
    Ok(true)
}

/// `cargo xtask proof …`.
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let root = args
        .from
        .as_ref()
        .map_or_else(|| ws.root.clone(), |from| ws.root.join(from));
    let git = |a: &[&str]| util::git_stdout(&ws.root, a).unwrap_or_else(|| "unknown".to_owned());
    let run = Run {
        commit: git(&["rev-parse", "HEAD"]),
        date: git(&["show", "-s", "--format=%cI", "HEAD"]),
        ran: crate::progress::today(),
        runner: args.runner.clone().unwrap_or_else(|| "local".to_owned()),
        lamina: crate::version::fixed_group_version(&ws.root)?.version,
        jobs: args.jobs.as_deref().map(parse_jobs).transpose()?.unwrap_or_default(),
    };
    let record = record(&root, run);
    let out = ws.root.join(OUT_DIR);
    fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
    let path = out.join("proof.json");
    fs::write(&path, serde_json::to_string_pretty(&record)? + "\n")
        .with_context(|| format!("writing {}", path.display()))?;
    let table = summary::table(&record);
    println!("{table}");
    util::summary(&table)?;
    if args.record
        && let Some(history) = &args.history
    {
        append(history, &record)?;
    }
    step(&format!("proof: {} (full: {})", path.display(), record.full));
    Ok(())
}

#[cfg(test)]
mod tests;
