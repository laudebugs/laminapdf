//! `<candidate>-results.json`: one run of `cargo xtask extract-bench` as data, for the docs
//! site's benchmarks page (`cargo xtask benchmarks` copies it): the dataset, each subset's
//! tests, passes and score, the overall scores, how many pages were extracted, and how fast.
//!
//! Every benchmark file is one page, so a file is a page here. Speed is on one thread, from the
//! `extract-bench` example's own clock around each file: open, extract and write the Markdown.

use serde_json::{Value, json};

use super::{Scored, at};

/// The pages a run read, by outcome, and how long the extracted ones took.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Speed {
    pub(super) extracted: u64,
    pub(super) failed: u64,
    pub(super) panicked: u64,
    pub(super) crashed_or_timed_out: u64,
    /// The extracted pages' milliseconds, summed.
    pub(super) total_millis: u64,
    /// The extracted pages' median milliseconds (the mean of the middle two for an even count).
    pub(super) median_millis: f64,
    /// Extracted pages over their summed seconds.
    pub(super) pages_per_second: f64,
}

/// The speed figures of the `extract-bench` example's report (`facts`: per file an `outcome`
/// and `millis`).
pub(super) fn speed(extracted: &Value) -> Speed {
    let facts: Vec<&Value> = at(extracted, "facts")
        .as_object()
        .map_or_else(Vec::new, |f| f.values().collect());
    let count =
        |outcome: &str| u64::try_from(facts.iter().filter(|v| at(v, "outcome") == outcome).count()).unwrap_or(u64::MAX);
    let mut millis: Vec<u64> = facts
        .iter()
        .filter(|v| at(v, "outcome") == "extracted")
        .filter_map(|v| at(v, "millis").as_u64())
        .collect();
    millis.sort_unstable();
    let total_millis: u64 = millis.iter().sum();
    let pages = u64::try_from(millis.len()).unwrap_or(u64::MAX);
    #[expect(
        clippy::cast_precision_loss,
        reason = "page counts and milliseconds are far below 2^52"
    )]
    let pages_per_second = if total_millis == 0 {
        0.0
    } else {
        pages as f64 * 1000.0 / total_millis as f64
    };
    Speed {
        extracted: count("extracted"),
        failed: count("failed"),
        panicked: count("panic"),
        crashed_or_timed_out: count("crash") + count("timeout"),
        total_millis,
        median_millis: median(&millis),
        pages_per_second,
    }
}

/// The median of sorted `values`, 0 for none.
fn median(sorted: &[u64]) -> f64 {
    let mid = sorted.len() / 2;
    #[expect(clippy::cast_precision_loss, reason = "milliseconds are far below 2^52")]
    match (
        sorted.len() % 2,
        sorted.get(mid),
        mid.checked_sub(1).and_then(|m| sorted.get(m)),
    ) {
        (1, Some(&m), _) => m as f64,
        (0, Some(&hi), Some(&lo)) => f64::midpoint(lo as f64, hi as f64),
        _ => 0.0,
    }
}

/// What `run` scored and how fast, with its `stamp` (`machine::stamp`, taken as it started): the
/// file `cargo xtask benchmarks` reads.
pub(super) fn results(run: &Scored<'_>, stamp: &Value) -> Value {
    let Scored {
        dataset,
        subsets,
        candidate,
        args,
        extracted,
        scores,
        pdf_counts,
    } = run;
    let per_subset: Vec<Value> = subsets
        .iter()
        .map(|subset| {
            let s = at(at(scores, "subsets"), &subset.name);
            json!({
                "name": subset.name,
                "measures": subset.measures,
                "pdfs": pdf_counts.get(&subset.name).copied().unwrap_or(0),
                "tests": at(s, "tests"),
                "passed": at(s, "passed"),
                "score": at(s, "score"),
            })
        })
        .collect();
    let baseline = at(at(scores, "subsets"), "baseline");
    let speed = speed(extracted);
    json!({
        "dataset": {
            "name": dataset.name,
            "title": dataset.title,
            "url": dataset.url,
            "revision": dataset.revision,
            "licence": dataset.licence,
            "attribution": dataset.attribution,
        },
        "candidate": candidate,
        "tables": if args.stream { "lattice-and-stream" } else { "lattice" },
        "sample": args.sample,
        "run": stamp,
        "subsets": per_subset,
        "baseline": { "tests": at(baseline, "tests"), "passed": at(baseline, "passed"), "score": at(baseline, "score") },
        "overall": at(scores, "overall"),
        "overallWithoutBaseline": at(scores, "overallWithoutBaseline"),
        "pages": {
            "extracted": speed.extracted,
            "failed": speed.failed,
            "panicked": speed.panicked,
            "crashedOrTimedOut": speed.crashed_or_timed_out,
        },
        "speed": {
            "threads": 1,
            "totalMillis": speed.total_millis,
            "medianMillisPerPage": speed.median_millis,
            "pagesPerSecond": speed.pages_per_second,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Value {
        json!({ "facts": {
            "a.md": { "outcome": "extracted", "millis": 4 },
            "b.md": { "outcome": "extracted", "millis": 1 },
            "c.md": { "outcome": "extracted", "millis": 10 },
            "d.md": { "outcome": "extracted", "millis": 5 },
            "e.md": { "outcome": "failed", "millis": 2 },
            "f.md": { "outcome": "panic", "millis": 0 },
            "g.md": { "outcome": "timeout" },
        }})
    }

    #[test]
    fn speed_should_sum_the_extracted_pages_and_take_their_median() {
        let s = speed(&facts());
        assert_eq!((s.extracted, s.total_millis, s.median_millis), (4, 20, 4.5));
    }

    #[test]
    fn pages_per_second_should_be_the_extracted_pages_over_their_summed_seconds() {
        // 4 pages in 20 ms.
        assert!((speed(&facts()).pages_per_second - 200.0).abs() < 1e-9);
    }

    #[test]
    fn speed_should_count_every_other_outcome_apart() {
        let s = speed(&facts());
        assert_eq!((s.failed, s.panicked, s.crashed_or_timed_out), (1, 1, 1));
    }

    #[test]
    fn speed_should_be_zero_without_extracted_pages() {
        let s = speed(&json!({ "facts": { "a.md": { "outcome": "failed", "millis": 3 } } }));
        assert_eq!((s.total_millis, s.median_millis, s.pages_per_second), (0, 0.0, 0.0));
    }

    #[test]
    fn median_should_take_the_middle_of_an_odd_count() {
        assert!((median(&[1, 4, 10]) - 4.0).abs() < 1e-9);
    }
}
