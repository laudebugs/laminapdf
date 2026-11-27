//! `cargo xtask desktop-reload [--gate] [--quick]` (todo E6f): the desktop's touched-page reload
//! after an edit, on files of about 1, 10 and 100 MB with the pinned native PDFium
//! (`crates/laminapdf-desktop-engine/benches/reload.rs`). It fetches PDFium, runs the bench, which
//! writes its medians to `target/desktop-reload/`, and prints the table into the job summary.
//! The files and the baselines are `packages/viewer/perf/reload.json`, shared with the web harness.
//!
//! `--gate` compares each median with its baseline (the CI runner's, recorded in that file): over
//! 20 %, and at least 10 ms, above it is a miss. A size with no baseline yet is only reported, and a
//! size the bench didn't measure fails. `--quick` skips the run: nothing in `cargo xtask nightly
//! --quick` waits minutes on a 100 MB file.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::util::summary;
use crate::workspace::Workspace;

const USAGE: &str = "usage: cargo xtask desktop-reload [--gate] [--quick]";

/// The files and baselines, as the web harness reads them.
const SHAPES: &str = include_str!("../../../packages/viewer/perf/reload.json");

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    gate: bool,
    quick: bool,
}

impl Args {
    /// The nightly's: gated, and short on a quick night.
    pub(crate) const fn nightly(quick: bool) -> Self {
        Self { gate: true, quick }
    }
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut parsed = Args::default();
    for arg in args {
        match arg.as_str() {
            "--gate" => parsed.gate = true,
            "--quick" => parsed.quick = true,
            other => bail!("unknown argument {other:?}; {USAGE}"),
        }
    }
    Ok(parsed)
}

/// One size's verdict.
#[derive(Debug, PartialEq)]
struct Row {
    megabytes: u64,
    median_ms: f64,
    baseline_ms: Option<f64>,
    miss: bool,
}

/// The most a gated figure may be over its baseline: 20 %, and at least 10 ms for the small ones.
fn limit_ms(baseline: f64) -> f64 {
    (baseline * 1.2).max(baseline + 10.0)
}

/// Each file of `shapes` against the bench's `measured` medians. A file the bench left out is an
/// error, never a pass; one with no baseline is reported and doesn't miss.
fn check(shapes: &Value, measured: &Value) -> Result<Vec<Row>> {
    let files = shapes["files"].as_array().context("reload.json has no files")?;
    let mut rows = Vec::new();
    for file in files {
        let megabytes = file["megabytes"].as_u64().context("a file has no megabytes")?;
        let key = megabytes.to_string();
        let median_ms = measured
            .pointer(&format!("/medianMs/{key}"))
            .and_then(Value::as_f64)
            .with_context(|| format!("the bench measured no figure for the {megabytes} MB file"))?;
        let baseline_ms = shapes
            .pointer(&format!("/baselinesMs/desktop/{key}"))
            .and_then(Value::as_f64);
        let miss = baseline_ms.is_some_and(|b| median_ms > limit_ms(b));
        rows.push(Row {
            megabytes,
            median_ms,
            baseline_ms,
            miss,
        });
    }
    Ok(rows)
}

fn table(rows: &[Row]) -> String {
    let mut out = String::from("| File | Median (ms) | Baseline (ms) | Result |\n| --- | --- | --- | --- |\n");
    for row in rows {
        let (baseline, result) = match row.baseline_ms {
            None => ("none yet".to_owned(), "reported"),
            Some(b) => (
                format!("{b:.0} (limit {:.0})", limit_ms(b)),
                if row.miss { "**miss**" } else { "pass" },
            ),
        };
        let _ = writeln!(
            out,
            "| {} MB | {:.0} | {baseline} | {result} |",
            row.megabytes, row.median_ms
        );
    }
    out
}

/// `cargo xtask desktop-reload [--gate] [--quick]`.
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    if args.quick {
        println!("desktop-reload: skipped with --quick (files of up to 100 MB)");
        return Ok(());
    }
    let pdfium = crate::pdfium::fetch(ws)?;
    let out = ws.root.join("target/desktop-reload");
    let out_arg = out.to_string_lossy().into_owned();
    crate::run_env(
        &ws.root,
        "cargo",
        &[
            "bench",
            "--locked",
            "-p",
            "laminapdf-desktop-engine",
            "--bench",
            "reload",
            "--",
            "--out",
            &out_arg,
        ],
        &[(crate::render::PDFIUM_DIR_ENV, &pdfium.to_string_lossy())],
    )?;
    let shapes: Value = serde_json::from_str(SHAPES)?;
    let measured: Value = serde_json::from_str(&fs::read_to_string(Path::new(&out).join("desktop-reload.json"))?)?;
    let rows = check(&shapes, &measured)?;
    let text = table(&rows);
    print!("{text}");
    summary(&format!("## Desktop: touched-page reload after an edit\n\n{text}"))?;
    if args.gate && rows.iter().any(|row| row.miss) {
        bail!("the desktop reload is over its baseline by more than 20 %");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn shapes(baseline: &Value) -> Value {
        json!({
            "files": [{ "megabytes": 1 }, { "megabytes": 100 }],
            "baselinesMs": { "desktop": { "1": baseline, "100": 100.0 } },
        })
    }

    #[test]
    fn check_should_fail_a_file_the_bench_did_not_measure() {
        let error = check(&shapes(&Value::Null), &json!({ "medianMs": { "1": 5.0 } })).unwrap_err();
        assert!(error.to_string().contains("100 MB"), "{error}");
    }

    #[test]
    fn check_should_only_report_a_file_with_no_baseline() {
        let rows = check(
            &shapes(&Value::Null),
            &json!({ "medianMs": { "1": 900.0, "100": 100.0 } }),
        )
        .unwrap();
        assert!(rows.iter().all(|row| !row.miss));
        assert_eq!(rows[0].baseline_ms, None);
    }

    #[test]
    fn check_should_miss_over_twenty_percent_and_ten_ms() {
        let measured = |one: f64, hundred: f64| json!({ "medianMs": { "1": one, "100": hundred } });
        // 2 ms baseline: 12 ms is the limit, not 2.4.
        let rows = check(&shapes(&json!(2.0)), &measured(11.0, 120.0)).unwrap();
        assert_eq!(rows.iter().map(|r| r.miss).collect::<Vec<_>>(), [false, false]);
        let rows = check(&shapes(&json!(2.0)), &measured(13.0, 121.0)).unwrap();
        assert_eq!(rows.iter().map(|r| r.miss).collect::<Vec<_>>(), [true, true]);
    }

    #[test]
    fn parse_args_should_take_the_flags_and_refuse_others() {
        assert_eq!(parse_args(&[]).unwrap(), Args::default());
        assert_eq!(
            parse_args(&["--gate".into(), "--quick".into()]).unwrap(),
            Args::nightly(true)
        );
        assert!(parse_args(&["--nope".into()]).is_err());
    }
}
