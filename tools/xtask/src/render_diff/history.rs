//! The history comparison: what a run measured, its basis-point record, the regressions against
//! the last comparable record and the issue they open.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Value, json};

use super::report::describe;
use super::rows::{Row, Tally, percent};
use super::{DPI, KEY, REGRESSION_BP};

/// What this run measured, as a history record's comparable part.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Sampling {
    pub(super) dpi: u32,
    pub(super) pages: Option<u32>,
    pub(super) sample: Option<usize>,
    pub(super) sources: Vec<String>,
}

impl Sampling {
    /// The sampling of a run; the sources are sorted and deduplicated, so `--source` order
    /// doesn't change which history records compare.
    pub(super) fn new(pages: Option<u32>, sample: Option<usize>, sources: &[String]) -> Self {
        let mut sources = sources.to_vec();
        sources.sort();
        sources.dedup();
        Self {
            dpi: DPI,
            pages,
            sample,
            sources,
        }
    }

    pub(super) fn json(&self) -> Value {
        json!({"dpi": self.dpi, "pages": self.pages, "sample": self.sample, "sources": self.sources})
    }
}

/// `render-diff/<source>` (and `render-diff/all`) → pass+minor share in basis points: the
/// history's `benches` entries (u64, like the benchmarks' nanoseconds).
pub(super) fn history_entries(tallies: &BTreeMap<String, Tally>) -> BTreeMap<String, u64> {
    tallies
        .iter()
        .map(|(source, t)| (format!("{KEY}/{source}"), t.rate_bp()))
        .collect()
}

/// A rate that dropped: key, previous and current basis points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Drop {
    pub(super) key: String,
    pub(super) before: u64,
    pub(super) now: u64,
}

/// The last render-diff record of `history` from `runner` with the same sampling, and every
/// rate that is more than [`REGRESSION_BP`] below it.
pub(super) fn regressions<'a>(
    history: &'a [Value],
    runner: &str,
    sampling: &Sampling,
    current: &BTreeMap<String, u64>,
) -> (Option<&'a Value>, Vec<Drop>) {
    let last = history.iter().rev().find(|r| {
        r.get("kind").and_then(Value::as_str) == Some(KEY)
            && r.get("runner").and_then(Value::as_str) == Some(runner)
            && r.get("sampling") == Some(&sampling.json())
    });
    let Some(last) = last else {
        return (None, Vec::new());
    };
    let drops = current
        .iter()
        .filter_map(|(key, &now)| {
            let before = last.get("benches")?.get(key)?.as_u64()?;
            (before > now + REGRESSION_BP).then(|| Drop {
                key: key.clone(),
                before,
                now,
            })
        })
        .collect();
    (Some(last), drops)
}

/// The regression issue's title.
pub(super) fn issue_title(reference: &str) -> String {
    format!("Nightly render-diff regression on {reference}")
}

/// The regression issue's body (the nightly lane puts the run's link above it).
pub(super) fn issue_body(drops: &[Drop], last: &Value, sampling: &Sampling, worst: &[&Row]) -> String {
    let mut md = String::new();
    let _ = writeln!(
        md,
        "The hayro-vs-PDFium pass rate (pass + minor share of all pages, ADR 0004 §6) dropped by more than \
         {} points against the last record (commit `{}`, {}). It does not fail the build.\n",
        REGRESSION_BP / 100,
        last.get("commit").and_then(Value::as_str).unwrap_or("unknown"),
        last.get("date").and_then(Value::as_str).unwrap_or("unknown date"),
    );
    let _ = writeln!(md, "| Key | Before | Now | Change |\n| --- | ---: | ---: | ---: |");
    for d in drops {
        let change = percent(d.before - d.now);
        let _ = writeln!(
            md,
            "| `{}` | {:.2} % | {:.2} % | −{change:.2} points |",
            d.key,
            percent(d.before),
            percent(d.now)
        );
    }
    let _ = writeln!(
        md,
        "\nSampling: {} DPI, {}, {}.",
        sampling.dpi,
        sampling
            .pages
            .map_or("every page".to_owned(), |n| format!("the first {n} pages of each file")),
        sampling
            .sample
            .map_or("every file".to_owned(), |n| format!("{n} files stratified per source"))
    );
    if !worst.is_empty() {
        let _ = writeln!(
            md,
            "\nThe worst pages now (diff images in the run's `render-diff` artefact):\n"
        );
        for row in worst.iter().take(10) {
            let _ = writeln!(md, "- {}", describe(row));
        }
    }
    md
}
