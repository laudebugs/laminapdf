//! `cargo xtask viewer-perf`: `@laminapdf/viewer`'s performance harness (todo C3c; ADR 0018
//! tier 11) in Chromium, Firefox and `WebKit`, after `cargo xtask wasm`. The harness
//! (`packages/viewer/perf/`) writes `target/viewer-perf/<browser>.json` and `summary.md` against
//! the web plan's §4 targets, which this task adds to the job summary. The targets are reported,
//! never asserted. `--history <dir> --runner <label>` compares the figures with
//! `<dir>/history.jsonl`, the nightly benchmarks' file and regression rule (more than 10 % worse
//! than the median of the last runs on that runner), and writes `regressions.md` with
//! `regression=true` in `$GITHUB_OUTPUT`; `--record` appends this run; `--dry-run` also prints the
//! issue a regression opens.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::nightly::{BASELINE_RUNS, append_record, history_record, read_history, regression_rows, regressions};
use crate::util::{github_output, summary};
use crate::workspace::Workspace;

const BROWSERS: [&str; 3] = ["chromium", "firefox", "webkit"];

const USAGE: &str = "usage: cargo xtask viewer-perf [chromium|firefox|webkit]… [--quick] \
                     [--history <dir> --runner <label> [--record] [--dry-run]]";

/// What `cargo xtask viewer-perf` was asked to do; the default runs every browser, recording nothing.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    browsers: Vec<String>,
    quick: bool,
    history: Option<String>,
    runner: Option<String>,
    record: bool,
    dry_run: bool,
}

impl Args {
    /// Every browser, recording nothing: the local nightly lane's run.
    pub(crate) fn every_browser(quick: bool) -> Self {
        Self {
            quick,
            ..Self::default()
        }
    }
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut parsed = Args::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--quick" => parsed.quick = true,
            "--record" => parsed.record = true,
            "--dry-run" => parsed.dry_run = true,
            "--history" => parsed.history = Some(rest.next().context(USAGE)?.clone()),
            "--runner" => parsed.runner = Some(rest.next().context(USAGE)?.clone()),
            browser if BROWSERS.contains(&browser) => {
                if !parsed.browsers.iter().any(|b| b == browser) {
                    parsed.browsers.push(browser.to_owned());
                }
            }
            other => bail!("unknown argument {other:?}; {USAGE}"),
        }
    }
    if parsed.history.is_some() != parsed.runner.is_some() {
        bail!("--history and --runner go together; {USAGE}");
    }
    if (parsed.record || parsed.dry_run) && parsed.history.is_none() {
        bail!("--record and --dry-run need --history; {USAGE}");
    }
    // A quick run covers 100 pages, not 1,000: its figures never become a baseline.
    if parsed.record && parsed.quick {
        bail!("--record with --quick: quick figures must not enter the history; {USAGE}");
    }
    Ok(parsed)
}

/// The history entries of the browsers' results: `viewer/<browser>/<figure>`, from the `history`
/// object the harness writes into each `<browser>.json` (nanoseconds, or bytes for memory).
fn entries(results: &[(String, Value)]) -> Result<BTreeMap<String, u64>> {
    let mut out = BTreeMap::new();
    for (browser, result) in results {
        let history = result
            .get("history")
            .and_then(Value::as_object)
            .with_context(|| format!("{browser}.json has no `history` object"))?;
        for (figure, value) in history {
            let value = value
                .as_u64()
                .with_context(|| format!("{browser}.json: history.{figure} is not a whole number"))?;
            out.insert(format!("viewer/{browser}/{figure}"), value);
        }
    }
    Ok(out)
}

/// The earlier records on `runner` that hold viewer figures: the runs a baseline comes from
/// (the shared history also holds the benchmarks', the SDK's and the render diff's).
fn viewer_runs(history: &[Value], runner: &str) -> usize {
    history
        .iter()
        .filter(|r| r.get("runner").and_then(Value::as_str) == Some(runner))
        .filter(|r| {
            r.get("benches")
                .and_then(Value::as_object)
                .is_some_and(|b| b.keys().any(|k| k.starts_with("viewer/")))
        })
        .count()
}

/// The Markdown table of the figures more than 10 % worse than their baseline ("" when none).
fn regression_report(found: &[(String, u64, u64)], runner: &str) -> String {
    if found.is_empty() {
        return String::new();
    }
    format!(
        "Viewer figures more than 10% worse than the median of their last {BASELINE_RUNS} runs on `{runner}` \
         (times in ns, memory in bytes):\n\n| Figure | Baseline | Now | Change |\n| --- | --- | --- | --- |\n{}",
        regression_rows(found)
    )
}

/// `cargo xtask viewer-perf [browser…] [--quick] [--history <dir> --runner <label> [--record] [--dry-run]]`:
/// needs `cargo xtask wasm` first; builds the viewer over it and installs the Playwright browsers
/// it runs (`playwright install`, a no-op when they are present).
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let browsers: Vec<&str> = if args.browsers.is_empty() {
        BROWSERS.to_vec()
    } else {
        args.browsers.iter().map(String::as_str).collect()
    };
    ws.require_node_modules()?;
    for dist in ["packages/wasm/dist/lamina_pdf_text.wasm", "packages/sdk/dist/index.js"] {
        if !ws.root.join(dist).is_file() {
            bail!("{dist} is missing: run `cargo xtask wasm` first");
        }
    }
    ws.pnpm(&["--filter", "@laminapdf/viewer", "build:wasm"])?;
    for browser in &browsers {
        ws.pnpm(&[
            "--filter",
            "@laminapdf/viewer",
            "exec",
            "playwright",
            "install",
            browser,
        ])?;
    }
    let out = ws.root.join("target/viewer-perf");
    let out_arg = out.to_string_lossy().into_owned();
    let list = browsers.join(",");
    let mut pnpm = vec![
        "--filter",
        "@laminapdf/viewer",
        "perf",
        "--out",
        &out_arg,
        "--browsers",
        &list,
    ];
    if args.quick {
        pnpm.push("--quick");
    }
    ws.pnpm(&pnpm)?;
    let mut table = fs::read_to_string(out.join("summary.md")).context("the harness wrote no summary.md")?;
    let (Some(history_dir), Some(runner)) = (&args.history, &args.runner) else {
        return summary(&table);
    };
    let mut results = Vec::new();
    for browser in &browsers {
        let path = out.join(format!("{browser}.json"));
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        results.push(((*browser).to_owned(), serde_json::from_str(&text)?));
    }
    let current = entries(&results)?;
    let history_path = Path::new(history_dir).join("history.jsonl");
    let history = read_history(&history_path)?;
    let found = regressions(&history, runner, args.quick, &current);
    let report = regression_report(&found, runner);
    fs::write(out.join("regressions.md"), &report)?;
    github_output("regression", if found.is_empty() { "false" } else { "true" })?;
    if found.is_empty() {
        let _ = writeln!(
            table,
            "\nNo figure is more than 10% worse than the median of its last {BASELINE_RUNS} runs on `{runner}` \
             ({} earlier viewer runs there).",
            viewer_runs(&history, runner)
        );
    } else {
        let _ = writeln!(table, "\n{report}");
        println!("::warning::{} viewer figures regressed by more than 10%", found.len());
        if args.dry_run {
            let reference = std::env::var("GITHUB_REF_NAME").unwrap_or_else(|_| "main".to_owned());
            println!(
                "--dry-run: the nightly would open or comment on \"Nightly viewer-perf regression on {reference}\":\n"
            );
            println!("{report}");
        }
    }
    if args.record {
        // Never quick: parse_args refuses --record with --quick.
        append_record(
            Path::new(history_dir),
            &history_record(&ws.root, runner, false, &current),
        )?;
    }
    summary(&table)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_args_should_default_to_every_browser_recording_nothing() {
        assert_eq!(parse_args(&[]).unwrap_or_default(), Args::default());
    }

    #[test]
    fn parse_args_should_keep_the_browsers_given_without_duplicates() {
        let parsed = parse_args(&args(&["webkit", "chromium", "webkit"])).unwrap_or_default();
        assert_eq!(parsed.browsers, ["webkit", "chromium"]);
    }

    #[test]
    fn parse_args_should_reject_an_unknown_browser() {
        assert!(parse_args(&args(&["safari"])).is_err());
    }

    #[test]
    fn parse_args_should_tie_the_history_flags_together() {
        assert!(parse_args(&args(&["--history", "h"])).is_err());
        assert!(parse_args(&args(&["--runner", "r"])).is_err());
        assert!(parse_args(&args(&["--record"])).is_err());
        assert!(parse_args(&args(&["--dry-run"])).is_err());
        assert!(parse_args(&args(&["--history", "h", "--runner", "r", "--record", "--dry-run"])).is_ok());
    }

    #[test]
    fn parse_args_should_never_record_a_quick_run() {
        assert!(parse_args(&args(&["--quick", "--history", "h", "--runner", "r", "--record"])).is_err());
        assert!(parse_args(&args(&["--quick", "--history", "h", "--runner", "r"])).is_ok());
    }

    #[test]
    fn entries_should_prefix_each_figure_with_its_browser() {
        let results = vec![
            (
                "chromium".to_owned(),
                json!({"history": {"to-sharp": 44_000_000, "memory-peak-bytes": 250}}),
            ),
            ("webkit".to_owned(), json!({"history": {"to-sharp": 61_000_000}})),
        ];
        let got = entries(&results).unwrap_or_default();
        assert_eq!(got.get("viewer/chromium/to-sharp"), Some(&44_000_000));
        assert_eq!(got.get("viewer/chromium/memory-peak-bytes"), Some(&250));
        assert_eq!(got.get("viewer/webkit/to-sharp"), Some(&61_000_000));
    }

    #[test]
    fn entries_should_fail_on_a_result_without_history() {
        assert!(entries(&[("firefox".to_owned(), json!({}))]).is_err());
        assert!(entries(&[("firefox".to_owned(), json!({"history": {"to-sharp": 1.5}}))]).is_err());
    }

    #[test]
    fn a_record_should_become_the_next_runs_baseline() {
        let current = BTreeMap::from([("viewer/firefox/to-sharp".to_owned(), 60_000_000)]);
        let record = history_record(Path::new("."), "mac", false, &current);
        assert_eq!(
            record.pointer("/benches/viewer~1firefox~1to-sharp"),
            Some(&json!(60_000_000))
        );
        let history = vec![record; 5];
        let slower = BTreeMap::from([("viewer/firefox/to-sharp".to_owned(), 67_000_000)]);
        assert_eq!(regressions(&history, "mac", false, &slower).len(), 1);
        let within = BTreeMap::from([("viewer/firefox/to-sharp".to_owned(), 66_000_000)]);
        assert_eq!(regressions(&history, "mac", false, &within), Vec::new());
        assert_eq!(viewer_runs(&history, "mac"), 5);
    }

    #[test]
    fn viewer_runs_should_count_only_this_runners_viewer_records() {
        let history = vec![
            json!({"runner": "mac", "benches": {"viewer/webkit/to-sharp": 1}}),
            json!({"runner": "mac", "benches": {"decode/flate": 1}}),
            json!({"runner": "other", "benches": {"viewer/webkit/to-sharp": 1}}),
        ];
        assert_eq!(viewer_runs(&history, "mac"), 1);
    }

    #[test]
    fn a_forced_regression_should_be_reported_with_its_change() {
        // The dry run's case: a baseline far below this run's figure.
        let history: Vec<Value> = (0..5)
            .map(|_| json!({"runner": "mac", "quick": false, "benches": {"viewer/chromium/to-sharp": 40_000_000}}))
            .collect();
        let current = BTreeMap::from([("viewer/chromium/to-sharp".to_owned(), 80_000_000)]);
        let found = regressions(&history, "mac", false, &current);
        let report = regression_report(&found, "mac");
        assert!(
            report.contains("| `viewer/chromium/to-sharp` | 40000000 | 80000000 | +100.0% |"),
            "{report}"
        );
        assert_eq!(
            regression_report(&regressions(&history, "other", false, &current), "other"),
            ""
        );
    }
}
