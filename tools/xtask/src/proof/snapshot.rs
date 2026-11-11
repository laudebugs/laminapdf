//! `cargo xtask proof snapshot [--offline]`: writes `apps/site/src/data/proof.json`, the figures
//! the website's How we test page shows (T1b). It reads `history.jsonl` from `origin/bench-data`,
//! takes the newest full proof record ([`Record`], `full: true`) cut to the fields the page shows,
//! counts how many of the last [`NIGHTS`] nights passed, and sums the fuzzing time over every full
//! record. A night is a day: its runs (a re-run of failed jobs, a dispatched run) count once, and
//! each job shows its worst result of the day. The production deploy runs it before the build, as it runs `progress`.
//!
//! With `--offline`, without the branch, or with no full record, it keeps the committed file and
//! warns (`::warning::` under GitHub Actions); it also warns when the newest full record is more
//! than [`STALE_DAYS`] days old.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::record::{KIND, VERSION};
use super::{Record, record};
use crate::changes::at_revision;
use crate::progress::days_since_epoch;

/// The snapshot, relative to the workspace root.
pub(crate) const OUTPUT: &str = "apps/site/src/data/proof.json";
/// Where the nightly appends its records.
const HISTORY: (&str, &str) = ("origin/bench-data", "history.jsonl");
/// The snapshot format's version; a change `proof.ts` can't read bumps it.
const SNAPSHOT_VERSION: u32 = 1;
/// How many of the newest full nights "N of the last 30 nights passed" counts.
pub(crate) const NIGHTS: usize = 30;
/// A newest full night older than this many days is stale: the page still shows it, dated.
const STALE_DAYS: u64 = 3;

/// `apps/site/src/data/proof.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    pub(crate) version: u32,
    /// The day the snapshot was written, `YYYY-MM-DD`.
    pub(crate) date: String,
    /// The newest full night.
    pub(crate) night: Night,
    pub(crate) nights: Nights,
    /// Fuzzing time summed over every full run, in whole hours, by libFuzzer's own count.
    pub(crate) fuzz_hours: u64,
}

/// How the last [`NIGHTS`] full nights went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Nights {
    /// Nights (days with a full run) counted: at most [`NIGHTS`].
    pub(crate) recorded: u64,
    /// Of those, the nights where every job of every run succeeded.
    pub(crate) passed: u64,
    /// The day the first of them ran, `YYYY-MM-DD`.
    pub(crate) since: String,
}

/// One full night, cut to what the page shows. A section is `null` when the record's is, so a
/// failed job never reads as a zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Night {
    /// The commit the night tested.
    pub(crate) commit: String,
    /// The day it ran, `YYYY-MM-DD`.
    pub(crate) ran: String,
    /// The fixed group's version.
    pub(crate) lamina: String,
    /// Whether every job of every run that day succeeded.
    pub(crate) passed: bool,
    /// Each job's worst result that day (`success`, `failure`, `cancelled`, `skipped`).
    pub(crate) jobs: BTreeMap<String, String>,
    pub(crate) corpus: Option<Corpus>,
    pub(crate) differential: Option<Differential>,
    pub(crate) text: Option<Text>,
    pub(crate) fuzz: Option<Fuzz>,
    pub(crate) verapdf: Option<Verapdf>,
    pub(crate) roundtrip: Option<Roundtrip>,
    pub(crate) runtimes: Option<Runtimes>,
    pub(crate) render: Option<Render>,
}

/// How Lamina PDF read each source's files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Corpus {
    /// Whether the counts cover every file of the run.
    pub(crate) complete: bool,
    pub(crate) sources: BTreeMap<String, CorpusSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorpusSource {
    pub(crate) files: u64,
    pub(crate) opened: u64,
    pub(crate) repaired: u64,
    pub(crate) refused: u64,
    pub(crate) refused_alone: u64,
    /// Panics, crashes and timeouts: files Lamina PDF failed to read at all.
    pub(crate) errors: u64,
}

/// The structural differential: per tool, its version and how often it agreed per fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Differential {
    pub(crate) inputs: u64,
    /// Disagreements with every tool that no exceptions file excuses.
    pub(crate) failures: u64,
    pub(crate) tools: BTreeMap<String, DifferentialTool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DifferentialTool {
    pub(crate) version: String,
    /// Per fact: `outcome`, `pageCount`, `outlineTitles`.
    pub(crate) facts: BTreeMap<String, Tally>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Tally {
    pub(crate) agree: u64,
    pub(crate) disagree: u64,
    pub(crate) excused: u64,
}

/// Page text against pdf.js and pdftotext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Text {
    /// Each tool's version, by name.
    pub(crate) tools: BTreeMap<String, String>,
    pub(crate) sources: BTreeMap<String, TextSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TextSource {
    pub(crate) pages: u64,
    pub(crate) compared: u64,
    pub(crate) agree: u64,
}

/// The night's fuzzing, over every target that ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Fuzz {
    pub(crate) targets: u64,
    /// Each target's budget, in seconds (the longest when they differ).
    pub(crate) seconds: u64,
    /// Inputs run, summed; `None` when a target's count is missing.
    pub(crate) executions: Option<u64>,
    /// Targets that saved a crash, timeout or out-of-memory input.
    pub(crate) crashed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Verapdf {
    pub(crate) version: String,
    pub(crate) validations: Vec<Validation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Validation {
    /// The example's output file.
    pub(crate) file: String,
    /// `2b`, `2u`, `3b` or `ua1`.
    pub(crate) profile: String,
    pub(crate) compliant: bool,
}

/// Saves read back by other readers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Roundtrip {
    pub(crate) cases: u64,
    pub(crate) saved: u64,
    pub(crate) refused: u64,
    /// Per checker: whether every saved file passed.
    pub(crate) tools: BTreeMap<String, bool>,
}

/// The runtimes whose example suite gave the reference bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Runtimes {
    pub(crate) passed: Vec<String>,
    pub(crate) failed: Vec<String>,
    pub(crate) missing: Vec<String>,
}

/// hayro against PDFium.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Render {
    pub(crate) dpi: u32,
    pub(crate) pages_per_file: Option<u32>,
    /// Per source, and `all`.
    pub(crate) sources: BTreeMap<String, RenderSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenderSource {
    pub(crate) pages: u64,
    /// Pages that look the same or nearly, in basis points (10,000 = 100 %).
    pub(crate) pass_minor_bp: u64,
}

/// Says why the committed snapshot stays, where the deploy's run shows it.
fn keep(why: &str) {
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        println!("::warning title=proof snapshot::{why}: kept the committed {OUTPUT}");
    }
    eprintln!("warning: {why}: kept the committed {OUTPUT}");
}

/// Writes the snapshot to [`OUTPUT`], or keeps the committed one.
pub(crate) fn run(root: &Path, offline: bool) -> Result<()> {
    if offline {
        keep("--offline");
        return Ok(());
    }
    let (rev, path) = HISTORY;
    let Ok(history) = at_revision(root, rev, path) else {
        keep(&format!("couldn't read {path} on {rev}"));
        return Ok(());
    };
    let today = crate::progress::today();
    let Some(snapshot) = snapshot(&history, &today)? else {
        keep(&format!("no full proof record in {path} on {rev}"));
        return Ok(());
    };
    if let Some(age) = days_between(&snapshot.night.ran, &today)
        && age > STALE_DAYS
    {
        let why = format!(
            "the newest full night ran {age} days ago ({}); is the nightly recording?",
            snapshot.night.ran
        );
        if std::env::var_os("GITHUB_ACTIONS").is_some() {
            println!("::warning title=proof snapshot::{why}");
        }
        eprintln!("warning: {why}");
    }
    let output = root.join(OUTPUT);
    let json = serde_json::to_string_pretty(&snapshot).context("serialising the snapshot")? + "\n";
    fs::write(&output, json).with_context(|| format!("writing {OUTPUT}"))?;
    println!(
        "proof snapshot: wrote {OUTPUT}: the night of {} ({}), {} of the last {} nights passed",
        snapshot.night.ran, snapshot.nights.since, snapshot.nights.passed, snapshot.nights.recorded
    );
    Ok(())
}

/// The full proof records of `history` (one JSON object per line), oldest first. Lines of other
/// kinds, short nights and records of another version are left out; a proof line of this version
/// that doesn't parse fails, since the page would otherwise miss a night without saying so.
pub(crate) fn full_records(history: &str) -> Result<Vec<Record>> {
    let mut records = Vec::new();
    for (i, line) in history.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line).with_context(|| format!("history line {}", i + 1))?;
        if value.get("kind").and_then(Value::as_str) != Some(KIND) {
            continue;
        }
        if value.get("version").and_then(Value::as_u64) != Some(u64::from(VERSION)) {
            eprintln!(
                "warning: history line {}: a proof record of another version, left out",
                i + 1
            );
            continue;
        }
        let record: Record =
            serde_json::from_value(value).with_context(|| format!("history line {}: a proof record", i + 1))?;
        if record.full {
            records.push(record);
        }
    }
    Ok(records)
}

/// The full records grouped by the day they ran, in the order the days first appear. A day with
/// several runs (a re-run of failed jobs, a dispatched run) is one night.
fn days(records: &[Record]) -> Vec<Vec<&Record>> {
    let mut days: Vec<Vec<&Record>> = Vec::new();
    for record in records {
        match days
            .iter_mut()
            .find(|day| day.first().is_some_and(|r| r.ran == record.ran))
        {
            Some(day) => day.push(record),
            None => days.push(vec![record]),
        }
    }
    days
}

/// How bad a job's result is: a night shows each job's worst run.
fn severity(result: &str) -> u8 {
    match result {
        "success" => 0,
        "skipped" => 1,
        "cancelled" => 2,
        _ => 3,
    }
}

/// Each job's worst result over a day's runs: a failure a re-run later fixed still counts, so a
/// night passes only when every run of it passed.
fn day_jobs(day: &[&Record]) -> BTreeMap<String, String> {
    let mut jobs: BTreeMap<String, String> = BTreeMap::new();
    for (job, result) in day.iter().flat_map(|r| &r.jobs) {
        let worse = jobs.get(job).is_none_or(|seen| severity(result) > severity(seen));
        if worse {
            jobs.insert(job.clone(), result.clone());
        }
    }
    jobs
}

/// The snapshot of `history` written on `today`, or `None` when it holds no full proof record.
pub(crate) fn snapshot(history: &str, today: &str) -> Result<Option<Snapshot>> {
    let records = full_records(history)?;
    let days = days(&records);
    let Some((newest_day, newest)) = days.last().and_then(|day| Some((day, *day.last()?))) else {
        return Ok(None);
    };
    // The newest NIGHTS days, newest first.
    let last: Vec<&Vec<&Record>> = days.iter().rev().take(NIGHTS).collect();
    let nights = Nights {
        recorded: count(last.len()),
        passed: count(last.iter().filter(|day| passed(&day_jobs(day))).count()),
        since: last
            .last()
            .and_then(|day| day.first())
            .map_or_else(String::new, |r| r.ran.clone()),
    };
    let fuzz_seconds: u64 = records.iter().map(fuzzed_seconds).sum();
    Ok(Some(Snapshot {
        version: SNAPSHOT_VERSION,
        date: today.to_owned(),
        night: night(newest, day_jobs(newest_day)),
        nights,
        fuzz_hours: fuzz_seconds / 3600,
    }))
}

/// A count as the snapshot writes it.
fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Whether every job succeeded. A night with no jobs (a local run) never passes.
fn passed(jobs: &BTreeMap<String, String>) -> bool {
    !jobs.is_empty() && jobs.values().all(|result| result == "success")
}

/// How long the night fuzzed, over every target, by libFuzzer's own count. A run that printed none
/// (a crash stops it early) adds nothing, so the total never claims time that wasn't fuzzed.
fn fuzzed_seconds(record: &Record) -> u64 {
    record
        .fuzz
        .iter()
        .flat_map(BTreeMap::values)
        .filter_map(|run| run.fuzzed_seconds)
        .sum()
}

/// The night of `record`, the day's newest run, with `jobs` its day's worst results.
fn night(record: &Record, jobs: BTreeMap<String, String>) -> Night {
    Night {
        commit: record.commit.clone(),
        ran: record.ran.clone(),
        lamina: record.lamina.clone(),
        passed: passed(&jobs),
        jobs,
        corpus: record.corpus.as_ref().map(corpus),
        differential: record.differential.as_ref().map(differential),
        text: record.text.as_ref().map(|t| Text {
            tools: t.tools.clone(),
            sources: t
                .sources
                .iter()
                .map(|(name, s)| {
                    let source = TextSource {
                        pages: s.pages,
                        compared: s.compared,
                        agree: s.agree,
                    };
                    (name.clone(), source)
                })
                .collect(),
        }),
        fuzz: record.fuzz.as_ref().map(|runs| Fuzz {
            targets: count(runs.len()),
            seconds: runs.values().map(|run| run.seconds).max().unwrap_or(0),
            executions: runs.values().map(|run| run.executions).sum(),
            crashed: count(runs.values().filter(|run| run.crashed).count()),
        }),
        verapdf: record.verapdf.as_ref().map(|v| Verapdf {
            version: v.version.clone(),
            validations: v
                .validations
                .iter()
                .map(|v| Validation {
                    file: v.file.clone(),
                    profile: v.profile.clone(),
                    compliant: v.compliant,
                })
                .collect(),
        }),
        roundtrip: record.roundtrip.as_ref().map(|r| Roundtrip {
            cases: r.cases,
            saved: r.saved,
            refused: r.refused,
            tools: r.tools.iter().map(|(name, tool)| (name.clone(), tool.passed)).collect(),
        }),
        runtimes: record.runtimes.as_ref().map(|r| Runtimes {
            passed: r.passed.clone(),
            failed: r.failed.clone(),
            missing: r.missing.clone(),
        }),
        render: record.render.as_ref().map(|r| Render {
            dpi: r.dpi,
            pages_per_file: r.pages_per_file,
            sources: r
                .sources
                .iter()
                .map(|(name, s)| {
                    (
                        name.clone(),
                        RenderSource {
                            pages: s.rows,
                            pass_minor_bp: s.pass_minor_bp,
                        },
                    )
                })
                .collect(),
        }),
    }
}

fn corpus(corpus: &record::Corpus) -> Corpus {
    Corpus {
        complete: corpus.complete,
        sources: corpus
            .sources
            .iter()
            .map(|(name, s)| {
                let source = CorpusSource {
                    files: s.files,
                    opened: s.opened,
                    repaired: s.repaired,
                    refused: s.refused,
                    refused_alone: s.refused_alone,
                    errors: s.errors.panic + s.errors.crash + s.errors.timeout,
                };
                (name.clone(), source)
            })
            .collect(),
    }
}

/// The fact names as the page reads them (`page_count` → `pageCount`).
fn camel(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper = false;
    for c in name.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn differential(differential: &record::Differential) -> Differential {
    Differential {
        inputs: differential.inputs,
        failures: differential.failures,
        tools: differential
            .tools
            .iter()
            .map(|(name, tool)| {
                let facts = tool
                    .facts
                    .iter()
                    .map(|(fact, t)| {
                        let tally = Tally {
                            agree: t.agree,
                            disagree: t.disagree,
                            excused: t.excused,
                        };
                        (camel(fact), tally)
                    })
                    .collect();
                (
                    name.clone(),
                    DifferentialTool {
                        version: tool.version.clone(),
                        facts,
                    },
                )
            })
            .collect(),
    }
}

/// Days from `from` to `to` (both `YYYY-MM-DD`), or `None` when either doesn't parse or `to` is
/// earlier.
fn days_between(from: &str, to: &str) -> Option<u64> {
    days_since_epoch(to)?.checked_sub(days_since_epoch(from)?)
}

#[cfg(test)]
mod tests;
