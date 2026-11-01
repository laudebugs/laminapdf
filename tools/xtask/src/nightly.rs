//! The nightly lane's local entry points (ADR 0018 §2, tiers 9 and 11; todo X3a), each also a
//! job of `.github/workflows/nightly.yml`: release-mode scaling tests with one retry that only
//! reports flakiness (§4), benchmarks with history and a 10% regression check that never fails
//! the build, fuzz crash minimisation, and `nightly`, which runs the whole lane in sequence (the
//! external link check is `docs_links::external`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::util::{append, github_output, summary};
use crate::workspace::Workspace;
use crate::{corpus, docs_links, harness, live, pdf_runtimes, render_diff, step};

/// A benchmark regressed when it is this much slower than its baseline (ADR 0018 tier 11).
const REGRESSION_RATIO: f64 = 1.10;

/// The criterion benchmark targets `cargo xtask bench` runs: `(package, bench)`. `laminapdf`'s
/// `report` is the 100-page text report of pdf-library §4 (B5d).
const BENCHES: [(&str, &str); 3] = [
    ("laminapdf-cos", "read"),
    ("laminapdf-save", "save"),
    ("lamina", "report"),
];

/// The pages per file the nightly render diff renders (`--pages`; nightly.yml passes the same).
const RENDER_DIFF_PAGES: u32 = 20;

/// The baseline is the median of this many previous records from the same runner.
const BASELINE_RUNS: usize = 5;

/// The release-mode `#[ignore]`d tests: scaling ratios and anything else too slow or
/// timing-sensitive for the PR lanes.
const SCALING_ARGS: [&str; 11] = [
    "test",
    "--release",
    "--locked",
    "--workspace",
    // Its ignored tests need native PDFium and hold only on the macOS arm64 runner:
    // `cargo xtask render-golden` runs them.
    "--exclude",
    "laminapdf-render-pdfium",
    "--all-features",
    "--tests",
    "--",
    "--ignored",
    "--test-threads=1",
];

/// `cargo xtask scaling`: the ignored release-mode tests (incl. `reader_cache`'s
/// `pages_should_scale_linearly_with_shared_resources`), one thread so they don't compete for
/// the CPU. A failure is retried once; a pass on retry succeeds but is reported as flaky
/// (`flaky=true` in `$GITHUB_OUTPUT`, which opens a nightly issue), never hidden (ADR 0018 §4).
pub(crate) fn scaling(ws: &Workspace) -> Result<()> {
    if ws.cargo(&SCALING_ARGS).is_ok() {
        return github_output("flaky", "false");
    }
    step("scaling: failed; retrying once to tell a flaky test from a real failure");
    ws.cargo(&SCALING_ARGS)
        .context("scaling tests failed twice: a real failure, not flakiness")?;
    println!("::warning::scaling tests failed once and passed on retry: flaky, see ADR 0018 §4");
    github_output("flaky", "true")?;
    summary("## Scaling\n\nThe scaling tests failed once and passed on retry: **flaky** (ADR 0018 §4).\n")
}

/// What `cargo xtask bench` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct BenchArgs {
    quick: bool,
    history: Option<String>,
    runner: Option<String>,
    record: bool,
}

const BENCH_USAGE: &str = "usage: cargo xtask bench [--quick] [--history <dir> [--runner <label>] [--record]]";

pub(crate) fn parse_bench_args(args: &[String]) -> Result<BenchArgs> {
    let mut parsed = BenchArgs::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--quick" => parsed.quick = true,
            "--record" => parsed.record = true,
            "--history" => parsed.history = Some(rest.next().context(BENCH_USAGE)?.clone()),
            "--runner" => parsed.runner = Some(rest.next().context(BENCH_USAGE)?.clone()),
            other => bail!("unknown argument {other:?}; {BENCH_USAGE}"),
        }
    }
    if parsed.record && parsed.history.is_none() {
        bail!("--record needs --history; {BENCH_USAGE}");
    }
    // A quick run's timings aren't comparable with full ones; it never becomes a baseline.
    if parsed.record && parsed.quick {
        bail!("--record with --quick: quick timings must not enter the history; {BENCH_USAGE}");
    }
    Ok(parsed)
}

/// `test <name> ... bench: <ns> ns/iter (+/- <ns>)` lines (criterion's `--output-format bencher`).
fn parse_bencher(output: &str) -> BTreeMap<String, u64> {
    output
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("test ")?;
            let (name, rest) = rest.split_once(" ... bench:")?;
            let ns = rest.split_whitespace().next()?.replace(',', "");
            Some((name.trim().to_owned(), ns.parse().ok()?))
        })
        .collect()
}

fn median(mut values: Vec<u64>) -> Option<u64> {
    values.sort_unstable();
    values.get(values.len() / 2).copied()
}

/// `(name, baseline ns, current ns)` for every benchmark slower than its baseline by more
/// than [`REGRESSION_RATIO`]; the baseline is the median of the last [`BASELINE_RUNS`] records
/// from the same runner, of the same kind (quick or full), that have the benchmark.
fn regressions(
    history: &[Value],
    runner: &str,
    quick: bool,
    current: &BTreeMap<String, u64>,
) -> Vec<(String, u64, u64)> {
    let same_runner: Vec<&Value> = history
        .iter()
        .filter(|r| r.get("runner").and_then(Value::as_str) == Some(runner))
        .filter(|r| r.get("quick").and_then(Value::as_bool).unwrap_or(false) == quick)
        .collect();
    let mut out = Vec::new();
    for (name, &now) in current {
        let previous: Vec<u64> = same_runner
            .iter()
            .rev()
            .filter_map(|r| r.pointer(&format!("/benches/{}", name.replace('~', "~0").replace('/', "~1"))))
            .filter_map(Value::as_u64)
            .take(BASELINE_RUNS)
            .collect();
        let Some(baseline) = median(previous) else { continue };
        #[expect(clippy::cast_precision_loss, reason = "nanosecond timings are far below 2^52")]
        let slower = now as f64 > baseline as f64 * REGRESSION_RATIO;
        if slower {
            out.push((name.clone(), baseline, now));
        }
    }
    out
}

/// `cargo xtask bench [--quick] [--history <dir> [--runner <label>] [--record]]`: runs the
/// criterion benches in [`BENCHES`] and writes `target/nightly/bench/results.json`.
/// With `--history`, compares them with `<dir>/history.jsonl` (one record per run) and writes
/// `regressions.md` (`regression=true` in `$GITHUB_OUTPUT`); `--record` appends this run to the
/// history. A regression is reported, never a failure.
pub(crate) fn bench(ws: &Workspace, args: &BenchArgs) -> Result<()> {
    let out_dir = ws.root.join("target/nightly/bench");
    fs::create_dir_all(&out_dir)?;
    // History lives in history.jsonl, not criterion's own baselines. A partial target/criterion
    // restored by the CI cache made criterion fail ("Failed to access … base/sample.json") on
    // the second branch run, so every run starts without one. Criterion writes it under cargo's
    // target directory, which persists between jobs on the self-hosted runners.
    let criterion_dir = ws.target_dir.join("criterion");
    if criterion_dir.exists() {
        fs::remove_dir_all(&criterion_dir).with_context(|| format!("removing {}", criterion_dir.display()))?;
    }
    // Every benchmark target of the nightly lane, run by one `cargo bench`.
    let mut cargo_args = vec!["bench", "--locked"];
    for (package, bench) in BENCHES {
        cargo_args.extend(["-p", package, "--bench", bench]);
    }
    cargo_args.extend(["--", "--output-format", "bencher"]);
    if args.quick {
        cargo_args.extend(["--warm-up-time", "1", "--measurement-time", "2", "--sample-size", "20"]);
    }
    step(&format!("cargo {}", cargo_args.join(" ")));
    let output = Command::new("cargo")
        .args(&cargo_args)
        .current_dir(&ws.root)
        .stderr(Stdio::inherit())
        .output()
        .context("running cargo bench")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    if !output.status.success() {
        bail!("cargo bench failed with {}", output.status);
    }
    fs::write(out_dir.join("bencher.txt"), stdout.as_bytes())?;
    let benches = parse_bencher(&stdout);
    if benches.is_empty() {
        bail!("cargo bench printed no `test … bench:` lines");
    }
    let runner = args.runner.clone().unwrap_or_else(|| "local".to_owned());
    let record = json!({
        "commit": git(ws, &["rev-parse", "HEAD"]),
        "date": git(ws, &["show", "-s", "--format=%cI", "HEAD"]),
        "runner": runner,
        "quick": args.quick,
        "benches": benches,
    });
    fs::write(
        out_dir.join("results.json"),
        serde_json::to_string_pretty(&record)? + "\n",
    )?;
    let mut table = String::from("## Benchmarks\n\n| Benchmark | ns/iter |\n| --- | --- |\n");
    for (name, ns) in &benches {
        let _ = writeln!(table, "| `{name}` | {ns} |");
    }
    let Some(history_dir) = &args.history else {
        return summary(&table);
    };
    let history_path = Path::new(history_dir).join("history.jsonl");
    let history = read_history(&history_path)?;
    let found = regressions(&history, &runner, args.quick, &benches);
    let mut report = String::new();
    if found.is_empty() {
        let _ = writeln!(
            table,
            "\nNo benchmark is more than 10% slower than the median of its last {BASELINE_RUNS} runs on `{runner}` \
             ({} earlier records).",
            history.len()
        );
    } else {
        let _ = writeln!(
            report,
            "Benchmarks more than 10% slower than the median of their last {BASELINE_RUNS} runs on `{runner}`:\n\n\
             | Benchmark | Baseline ns/iter | Now ns/iter | Change |\n| --- | --- | --- | --- |"
        );
        for (name, baseline, now) in &found {
            #[expect(clippy::cast_precision_loss, reason = "nanosecond timings are far below 2^52")]
            let change = 100.0 * (*now as f64 / (*baseline).max(1) as f64 - 1.0);
            let _ = writeln!(report, "| `{name}` | {baseline} | {now} | +{change:.1}% |");
        }
        let _ = writeln!(table, "\n{report}");
        println!("::warning::{} benchmarks regressed by more than 10%", found.len());
    }
    fs::write(out_dir.join("regressions.md"), &report)?;
    github_output("regression", if found.is_empty() { "false" } else { "true" })?;
    if args.record {
        fs::create_dir_all(history_dir)?;
        append(&history_path, &(serde_json::to_string(&record)? + "\n"))?;
        step(&format!("Appended this run to {}", history_path.display()));
    }
    summary(&table)
}

/// The records in `history.jsonl`: none when the file doesn't exist yet (the first run);
/// any other read or parse error fails, so a broken history never hides a regression.
pub(crate) fn read_history(path: &Path) -> Result<Vec<Value>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .with_context(|| format!("parsing {}", path.display()))
}

fn git(ws: &Workspace, args: &[&str]) -> String {
    crate::util::git_stdout(&ws.root, args).unwrap_or_else(|| "unknown".to_owned())
}

/// The libFuzzer limits of every run (`cargo xtask fuzz`), so `tmin` minimises a timeout or an
/// OOM under the limits that found it.
pub(crate) const LIBFUZZER_LIMITS: [&str; 2] = ["-timeout=10", "-rss_limit_mb=2048"];

/// The artefacts `tmin` minimises: crashes, timeouts and OOMs. `slow-unit-*` files are slow
/// inputs libFuzzer reports, not failures.
fn is_failure_artefact(name: &str) -> bool {
    ["crash-", "timeout-", "oom-"].iter().any(|p| name.starts_with(p))
}

/// `cargo xtask fuzz tmin <target>`: minimises every crash, timeout and OOM input in
/// `fuzz/artifacts/<target>/` with `cargo fuzz tmin`, leaving `minimized-from-*` files next to
/// them for the nightly issue and `fixtures/regressions/` (fuzz/README.md).
pub(crate) fn fuzz_tmin(ws: &Workspace, target: &str) -> Result<()> {
    crate::doctor::require(ws, &crate::doctor::cargo_fuzz())?;
    let dir = ws.root.join("fuzz/artifacts").join(target);
    let mut inputs = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if is_failure_artefact(&name) {
                inputs.push(path);
            }
        }
    }
    inputs.sort();
    if inputs.is_empty() {
        step(&format!("fuzz tmin: no crash inputs in {}", dir.display()));
        return Ok(());
    }
    let host = harness::nightly_host()?;
    for input in &inputs {
        let input = input.to_string_lossy().into_owned();
        // A minimisation that fails (e.g. a timeout that doesn't reproduce) keeps the original.
        if let Err(error) = crate::run(&ws.root, "cargo", &tmin_args(&host, target, &input)) {
            eprintln!("warning: could not minimise {input}: {error:#}");
        }
    }
    Ok(())
}

fn tmin_args<'a>(host: &'a str, target: &'a str, input: &'a str) -> Vec<&'a str> {
    let mut args = vec![
        "+nightly", "fuzz", "tmin", "--target", host, "--runs", "1000", target, input, "--",
    ];
    args.extend(LIBFUZZER_LIMITS);
    args
}

/// What `cargo xtask nightly` was asked to do.
pub(crate) fn parse_nightly_args(args: &[String]) -> Result<bool> {
    match args {
        [] => Ok(false),
        [flag] if flag == "--quick" => Ok(true),
        _ => bail!("usage: cargo xtask nightly [--quick]"),
    }
}

/// `cargo xtask nightly [--quick]`: the whole nightly lane locally, in the workflow's order;
/// every part runs even when an earlier one failed, and the failures are listed at the end.
/// `--quick` shortens it (a 300-file differential sample, 30 s per fuzz target, quick
/// benchmarks, a 100-file render diff); the full lane takes hours.
pub(crate) fn nightly(ws: &Workspace, quick: bool) -> Result<()> {
    let mut failed: Vec<String> = Vec::new();
    let mut part = |name: &str, result: Result<()>| {
        if let Err(error) = result {
            eprintln!("nightly: {name} failed: {error:#}");
            failed.push(name.to_owned());
        }
    };
    part("corpus fetch", corpus::fetch(ws, &corpus::FetchArgs::default()));
    let mut live_args = live::LiveArgs::default();
    if quick {
        live_args.sample = Some(300);
    }
    part("differential --live", live::run_live(ws, &live_args));
    let text_args = live::LiveArgs {
        inputs: live::Inputs::External,
        text: true,
        ..live_args
    };
    part("differential --live --text", live::run_live(ws, &text_args));
    let seconds = if quick { 30 } else { 600 };
    for target in harness::FUZZ_TARGETS {
        let task = harness::FuzzTask::Run { target, seconds };
        part(&format!("fuzz {target}"), harness::fuzz(ws, &task));
    }
    part("scaling", scaling(ws));
    let bench_args = BenchArgs {
        quick,
        ..BenchArgs::default()
    };
    part("bench", bench(ws, &bench_args));
    part(
        "pdf-runtimes",
        crate::wasm(ws).and_then(|()| pdf_runtimes::run(ws, &pdf_runtimes::Args::default())),
    );
    let render_args = render_diff::Args {
        pages: Some(RENDER_DIFF_PAGES),
        sample: quick.then_some(100),
        ..render_diff::Args::default()
    };
    part("render-diff", render_diff::run(ws, &render_args));
    part("verapdf", crate::verapdf::run(ws, None));
    part("docs-links --external", docs_links::external::run(&ws.root, false));
    if failed.is_empty() {
        step("Nightly lane passed");
        Ok(())
    } else {
        bail!("nightly lane: {} failed", failed.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bencher_should_read_names_and_nanoseconds() {
        let out = "\ntest open_walk/fixtures_corpus ... bench:   1,409,451 ns/iter (+/- 1,424,439)\nnoise\n";
        let parsed = parse_bencher(out);
        assert_eq!(parsed.get("open_walk/fixtures_corpus"), Some(&1_409_451));
    }

    fn record(runner: &str, ns: u64) -> Value {
        json!({"runner": runner, "benches": {"decode/flate": ns}})
    }

    #[test]
    fn regressions_should_flag_more_than_ten_percent() {
        let history = [record("ubuntu", 100), record("ubuntu", 100)];
        let now = BTreeMap::from([("decode/flate".to_owned(), 111)]);
        assert_eq!(
            regressions(&history, "ubuntu", false, &now),
            [("decode/flate".to_owned(), 100, 111)]
        );
    }

    #[test]
    fn regressions_should_tolerate_ten_percent() {
        let history = [record("ubuntu", 100)];
        let now = BTreeMap::from([("decode/flate".to_owned(), 110)]);
        assert_eq!(regressions(&history, "ubuntu", false, &now), []);
    }

    #[test]
    fn regressions_should_only_compare_the_same_runner() {
        let history = [record("macos", 10)];
        let now = BTreeMap::from([("decode/flate".to_owned(), 100)]);
        assert_eq!(regressions(&history, "ubuntu", false, &now), []);
    }

    #[test]
    fn regressions_should_use_the_median_of_recent_runs() {
        let history = [100, 100, 300, 100, 100].map(|ns| record("ubuntu", ns));
        let now = BTreeMap::from([("decode/flate".to_owned(), 105)]);
        assert_eq!(regressions(&history, "ubuntu", false, &now), []);
    }

    #[test]
    fn regressions_should_ignore_quick_records_in_a_full_run() {
        let mut quick = record("ubuntu", 10);
        quick["quick"] = json!(true);
        let now = BTreeMap::from([("decode/flate".to_owned(), 100)]);
        assert_eq!(regressions(&[quick], "ubuntu", false, &now), []);
    }

    #[test]
    fn bench_args_should_reject_record_with_quick() {
        let args: Vec<String> = ["--quick", "--history", "h", "--record"].map(String::from).to_vec();
        assert!(parse_bench_args(&args).is_err());
    }

    #[test]
    fn read_history_should_treat_a_missing_file_as_empty() {
        let missing = std::env::temp_dir().join("xtask-no-such-history.jsonl");
        assert_eq!(read_history(&missing).unwrap(), Vec::<Value>::new());
    }

    #[test]
    fn read_history_should_fail_when_the_file_cannot_be_read() {
        // Bytes that aren't UTF-8: a read error other than "missing" (the same on every OS),
        // so not an empty history.
        let path = std::env::temp_dir().join(format!("xtask-bad-history-{}.jsonl", std::process::id()));
        fs::write(&path, [0xff, 0xfe, b'\n']).unwrap();
        let result = read_history(&path);
        fs::remove_file(&path).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn tmin_should_skip_slow_units_and_keep_the_run_limits() {
        let args = tmin_args("x86_64-unknown-linux-gnu", "open", "crash-1");
        assert!(
            !is_failure_artefact("slow-unit-1")
                && is_failure_artefact("timeout-1")
                && args.ends_with(&["--", "-timeout=10", "-rss_limit_mb=2048"])
        );
    }

    #[test]
    fn bench_args_should_require_history_for_record() {
        assert!(parse_bench_args(&["--record".to_owned()]).is_err());
    }
}
