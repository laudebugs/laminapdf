//! The proof record's job-summary table: one row per section, a `null` one marked as missing.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::record::{Corpus, CorpusSource, Differential, Record, Render, Runtimes, Text};
use crate::harness::{FUZZ_TARGETS, FuzzRun};
use crate::{roundtrip, verapdf};

pub(super) fn table(record: &Record) -> String {
    let mut out = format!(
        "## Proof record\n\n`{}` on `{}`, Lamina PDF {}; full budgets: **{}**.\n\n| Section | Figures |\n| --- | --- |\n",
        record.commit.get(..12).unwrap_or(&record.commit),
        record.runner,
        record.lamina,
        if record.full { "yes" } else { "no" }
    );
    let rows = [
        ("corpus", record.corpus.as_ref().map(corpus)),
        ("differential", record.differential.as_ref().map(differential)),
        ("text", record.text.as_ref().map(text)),
        ("render", record.render.as_ref().map(render)),
        ("fuzz", record.fuzz.as_ref().map(fuzz)),
        ("verapdf", record.verapdf.as_ref().map(verapdf)),
        ("roundtrip", record.roundtrip.as_ref().map(roundtrip)),
        ("runtimes", record.runtimes.as_ref().map(runtimes)),
    ];
    for (name, figures) in rows {
        let figures = figures.unwrap_or_else(|| "**missing**".to_owned());
        let _ = writeln!(out, "| {name} | {figures} |");
    }
    if !record.jobs.is_empty() {
        let jobs: Vec<String> = record.jobs.iter().map(|(j, r)| format!("{j} {r}")).collect();
        let _ = writeln!(out, "\nJobs: {}.", jobs.join(", "));
    }
    out
}

fn corpus(corpus: &Corpus) -> String {
    let sources = &corpus.sources;
    let total = |f: fn(&CorpusSource) -> u64| sources.values().map(f).sum::<u64>();
    let part = if corpus.complete {
        String::new()
    } else {
        let inputs = corpus
            .inputs
            .map_or_else(|| "an unknown number of".to_owned(), |n| n.to_string());
        format!(
            "**incomplete**: Lamina PDF's report covers {} of {inputs} inputs; ",
            total(|s| s.files)
        )
    };
    format!(
        "{part}{} files: {} opened, {} repaired, {} refused ({} where another reader opened them), \
         {} panics, crashes or timeouts",
        total(|s| s.files),
        total(|s| s.opened),
        total(|s| s.repaired),
        total(|s| s.refused),
        total(|s| s.refused_alone),
        total(|s| s.errors.panic + s.errors.crash + s.errors.timeout)
    )
}

fn differential(d: &Differential) -> String {
    format!(
        "{} inputs, {} tools; {} unexcused failures, {} lamina errors, {} missing from a tool's report, \
         {} broken readers",
        d.inputs,
        d.tools.len(),
        d.failures,
        d.lamina_errors,
        d.missing,
        d.broken_readers
    )
}

fn text(t: &Text) -> String {
    let (compared, agree) = t
        .sources
        .values()
        .fold((0, 0), |(c, a), s| (c + s.compared, a + s.agree));
    format!("{agree} of {compared} compared pages agree with a tool")
}

fn render(r: &Render) -> String {
    let all = r.sources.get("all");
    #[expect(clippy::cast_precision_loss, reason = "basis points are at most 10,000")]
    let rate = all.map_or_else(
        || "–".to_owned(),
        |s| format!("{:.2} %", s.pass_minor_bp as f64 / 100.0),
    );
    format!(
        "{} pages at {} dpi; pass + minor {rate}",
        all.map_or(0, |s| s.rows),
        r.dpi
    )
}

fn fuzz(runs: &BTreeMap<String, FuzzRun>) -> String {
    let crashed: Vec<&str> = runs
        .iter()
        .filter(|(_, r)| r.crashed)
        .map(|(t, _)| t.as_str())
        .collect();
    let executions: u64 = runs.values().filter_map(|r| r.executions).sum();
    format!(
        "{} of {} targets ran, {executions} executions; crashed: {}",
        runs.len(),
        FUZZ_TARGETS.len(),
        list(&crashed)
    )
}

fn verapdf(v: &verapdf::Report) -> String {
    let compliant = v.validations.iter().filter(|v| v.compliant).count();
    format!(
        "{compliant} of {} validations compliant (veraPDF {})",
        v.validations.len(),
        v.version
    )
}

fn roundtrip(r: &roundtrip::Summary) -> String {
    let passed: Vec<&str> = r
        .tools
        .iter()
        .filter(|(_, t)| t.passed)
        .map(|(n, _)| n.as_str())
        .collect();
    format!(
        "{} cases: {} saved, {} refused; every output passed in: {}",
        r.cases,
        r.saved,
        r.refused,
        list(&passed)
    )
}

fn runtimes(r: &Runtimes) -> String {
    format!(
        "passed: {}; failed: {}; no result: {}",
        list(&r.passed),
        list(&r.failed),
        list(&r.missing)
    )
}

/// `items` joined with commas, or `none`.
fn list(items: &[impl AsRef<str>]) -> String {
    if items.is_empty() {
        return "none".to_owned();
    }
    items.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(", ")
}
