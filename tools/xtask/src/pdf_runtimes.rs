//! `cargo xtask pdf-runtimes`: @laminapdf/sdk's example suite (`packages/sdk/test/suite/`) in every
//! runtime the package supports (todo B6c; pdf-library §5): Node (whichever `node` is on PATH),
//! Chromium, Firefox and `WebKit` (Vitest browser mode with Playwright), Deno, Bun and workerd.
//! Each runner compares every example's bytes with its Rust golden and prints the performance
//! smoke's figures (`LAMINA_PDF_MEASUREMENT {json}` lines), which this task collects into
//! `target/xtask/pdf-runtimes/measurements.json` (`measurements-<label>.json` with `--label`)
//! and a job-summary table against the §4 targets.
//! The targets are reported, never asserted. `--history <dir> --runner <label> --record` appends
//! the figures to the nightly benchmark history (`<dir>/history.jsonl`, the record format of
//! `cargo xtask bench`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead as _, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::util::summary;
use crate::workspace::Workspace;
use crate::{program_path, step};

/// Every runtime, in the order a full run takes them.
const RUNTIMES: [&str; 7] = ["node", "chromium", "firefox", "webkit", "deno", "bun", "workerd"];

/// The prefix of a measurement line (`MEASUREMENT_PREFIX` in packages/sdk/test/suite/examples.ts).
const PREFIX: &str = "LAMINA_PDF_MEASUREMENT ";

/// pdf-library §4: hello after `init()`, the 100-page report, and peak memory in a Worker.
const HELLO_TARGET_MS: f64 = 5.0;
const REPORT_TARGET_MS: f64 = 500.0;
const WORKER_MEMORY_TARGET_BYTES: f64 = 64.0 * 1024.0 * 1024.0;

const USAGE: &str = "usage: cargo xtask pdf-runtimes [node|chromium|firefox|webkit|deno|bun|workerd]… [--label <name>] [--history <dir> --runner <label> --record]";

/// What `cargo xtask pdf-runtimes` was asked to do; the default runs every runtime, recording nothing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Args {
    runtimes: Vec<&'static str>,
    /// Names the measurements file (`measurements-<label>.json`), so several runs in one job
    /// (the fast lane's Node 22 and 26) each keep theirs.
    label: Option<String>,
    history: Option<String>,
    runner: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            runtimes: RUNTIMES.to_vec(),
            label: None,
            history: None,
            runner: None,
        }
    }
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut parsed = Args {
        runtimes: Vec::new(),
        ..Args::default()
    };
    let mut record = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--record" => record = true,
            "--history" => parsed.history = Some(rest.next().context(USAGE)?.clone()),
            "--runner" => parsed.runner = Some(rest.next().context(USAGE)?.clone()),
            "--label" => {
                let label = rest.next().context(USAGE)?;
                // Not starting with '-', so `--label --record` can't take the flag as the label.
                if label.is_empty()
                    || label.starts_with('-')
                    || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                {
                    bail!("--label takes letters, digits and '-' only, not {label:?}; {USAGE}");
                }
                parsed.label = Some(label.clone());
            }
            name => {
                let Some(&runtime) = RUNTIMES.iter().find(|r| **r == name) else {
                    bail!("unknown runtime or argument {name:?}; {USAGE}");
                };
                if !parsed.runtimes.contains(&runtime) {
                    parsed.runtimes.push(runtime);
                }
            }
        }
    }
    // Recording is the one use of the history, so the three flags go together.
    match (record, &parsed.history, &parsed.runner) {
        (false, None, None) | (true, Some(_), Some(_)) => {}
        _ => bail!("--record, --history and --runner go together; {USAGE}"),
    }
    if parsed.runtimes.is_empty() {
        parsed.runtimes = RUNTIMES.to_vec();
    }
    Ok(parsed)
}

/// One figure a runner printed (`Measurement` in test/suite/examples.ts).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Measurement {
    runtime: String,
    case: String,
    /// Milliseconds, or bytes for `wasm-memory`.
    value: f64,
}

/// The measurements in a runner's output: lines containing [`PREFIX`] followed by JSON (Vitest
/// and Deno print them as they are; a line that doesn't parse is an error, so a format change in
/// the suite can't silently drop figures).
fn parse_measurements(output: &str) -> Result<Vec<Measurement>> {
    output
        .lines()
        .filter_map(|line| line.split_once(PREFIX).map(|(_, json)| json.trim()))
        .map(|json| parse_measurement(json).with_context(|| format!("unreadable measurement {json:?}")))
        .collect()
}

fn parse_measurement(json: &str) -> Result<Measurement> {
    let value: Value = serde_json::from_str(json)?;
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    Ok(Measurement {
        runtime: text("runtime").context("no `runtime`")?,
        case: text("case").context("no `case`")?,
        value: value
            .get("value")
            .and_then(Value::as_f64)
            .context("no numeric `value`")?,
    })
}

impl Measurement {
    fn json(&self) -> Value {
        json!({ "runtime": self.runtime, "case": self.case, "value": self.value })
    }
}

/// Vitest over `config`, limited to `projects`, with every test's output.
fn vitest<'a>(config: &'a str, projects: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["vitest", "run", "-c", config, "--reporter", "verbose"];
    for project in projects {
        args.extend(["--project", project]);
    }
    args
}

/// The commands that run `runtime`'s suite, in order, as `pnpm --filter @laminapdf/sdk exec …`
/// arguments. `node` runs the `measure` project first (alone, so its timings don't compete with
/// the other test files), then the rest of the Node suite.
fn commands(runtime: &str) -> Result<Vec<Vec<&'static str>>> {
    Ok(match runtime {
        "node" => vec![
            vitest("vitest.node.config.ts", &["measure"]),
            vitest("vitest.node.config.ts", &["node"]),
        ],
        // Chromium alone first (its timings shouldn't share the machine with a Vite build), then
        // the `vite-build` project: a Vite production build run in a Chromium page.
        "chromium" => vec![
            vitest("vitest.browser.config.ts", &["chromium"]),
            vitest("vitest.browser.config.ts", &["vite-build"]),
        ],
        "firefox" => vec![vitest("vitest.browser.config.ts", &["firefox"])],
        "webkit" => vec![vitest("vitest.browser.config.ts", &["webkit"])],
        // --sloppy-imports: the suite imports `.ts` files by their `.js` paths (as tsc wants);
        // --no-check: tsc checks the types (the wasm job's typecheck).
        "deno" => vec![vec![
            "deno",
            "test",
            "--sloppy-imports",
            "--no-check",
            "--allow-read",
            "test/deno/",
        ]],
        "bun" => vec![vec!["bun", "test", "./test/bun/"]],
        "workerd" => vec![vitest("vitest.workers.config.ts", &[])],
        other => bail!("no command for runtime {other:?}"),
    })
}

/// The web render adapters' `RenderBackend` contract in `browser` (todo C2c): `@laminapdf/render`
/// built, then `@laminapdf/render-pdfium`, `@laminapdf/render-hayro` (over the `wasm` job's
/// `lamina_pdf_render_hayro.wasm`), each with the native render goldens, `@laminapdf/viewer`'s document worker and RPC
/// (C3a, built first), `@laminapdf/tokens`' high-contrast theme, whose system colours only a browser resolves
/// (U6a), `@laminapdf/fonts`' faces (U6b: what a page fetches, all from its own origin), `@laminapdf/icons`'
/// React `<Icon>` and gallery (U6c), `@laminapdf/ui`'s components (U6d), and `@laminapdf/site-kit`'s site
/// header (W1d: keyboard order, the current page, phone width, axe). They record no measurements.
fn render_suites(ws: &Workspace, browser: &str) -> Result<()> {
    crate::run(&ws.root, "pnpm", &["--filter", "@laminapdf/render", "run", "build"])?;
    // The viewer's document worker and RPC (C3a), over the built package.
    crate::run(
        &ws.root,
        "pnpm",
        &["--filter", "@laminapdf/viewer", "run", "build:wasm"],
    )?;
    // `@laminapdf/ui` imports the tokens and icons packages through their built `dist/` entries.
    crate::run(
        &ws.root,
        "pnpm",
        &[
            "--filter",
            "@laminapdf/tokens",
            "--filter",
            "@laminapdf/icons",
            "run",
            "build",
        ],
    )?;
    for package in [
        "@laminapdf/render-pdfium",
        "@laminapdf/render-hayro",
        "@laminapdf/viewer",
        "@laminapdf/tokens",
        "@laminapdf/fonts",
        "@laminapdf/icons",
        "@laminapdf/ui",
        "@laminapdf/site-kit",
    ] {
        let mut args = vec!["--filter", package, "exec"];
        args.extend(vitest("vitest.browser.config.ts", &[browser]));
        crate::run(&ws.root, "pnpm", &args)?;
    }
    Ok(())
}

/// Runs `pnpm --filter @laminapdf/sdk exec <args>`, echoing its stdout as it comes, and returns it.
fn run_captured(ws: &Workspace, args: &[&str]) -> Result<String> {
    let mut full = vec!["--filter", "@laminapdf/sdk", "exec"];
    full.extend_from_slice(args);
    step(&format!("pnpm {}", full.join(" ")));
    let mut child = Command::new(program_path("pnpm"))
        .args(&full)
        .current_dir(&ws.root)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to start pnpm; is it installed and on PATH?")?;
    let stdout = child.stdout.take().context("pnpm's stdout")?;
    let mut captured = String::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.context("reading pnpm's output")?;
        println!("{line}");
        captured.push_str(&line);
        captured.push('\n');
    }
    let status = child.wait().context("waiting for pnpm")?;
    if !status.success() {
        bail!("`pnpm {}` failed with {status}", full.join(" "));
    }
    Ok(captured)
}

/// `cargo xtask pdf-runtimes [runtime…] [--label <name>] [--history <dir> --runner <label> --record]`:
/// runs each runtime's suite in turn (all of them by default); every runtime runs even when an earlier
/// one failed, and the failures are listed at the end. Needs `cargo xtask wasm` first (it
/// builds `packages/wasm/dist/` and `packages/sdk/dist/`); installs the Playwright browsers it
/// needs (`playwright install`, a no-op when they are present).
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    ws.require_node_modules()?;
    for dist in [
        "packages/wasm/dist/lamina_pdf_text.wasm",
        "packages/wasm/dist/lamina_pdf_render_hayro.wasm",
        "packages/sdk/dist/index.js",
    ] {
        if !ws.root.join(dist).is_file() {
            bail!("{dist} is missing: run `cargo xtask wasm` first");
        }
    }
    let mut measurements = Vec::new();
    let mut failed = Vec::new();
    for &runtime in &args.runtimes {
        let result = (|| -> Result<Vec<Measurement>> {
            if matches!(runtime, "chromium" | "firefox" | "webkit") {
                crate::run(
                    &ws.root,
                    "pnpm",
                    &["--filter", "@laminapdf/sdk", "exec", "playwright", "install", runtime],
                )?;
            }
            let mut found = Vec::new();
            for command in commands(runtime)? {
                found.extend(parse_measurements(&run_captured(ws, &command)?)?);
            }
            if matches!(runtime, "chromium" | "firefox" | "webkit") {
                render_suites(ws, runtime)?;
            }
            Ok(found)
        })();
        match result {
            Ok(found) if found.is_empty() => {
                eprintln!("pdf-runtimes: {runtime} printed no measurements");
                failed.push(runtime);
            }
            Ok(found) => measurements.extend(found),
            Err(error) => {
                eprintln!("pdf-runtimes: {runtime} failed: {error:#}");
                failed.push(runtime);
            }
        }
    }
    let out_dir = ws.root.join("target/xtask/pdf-runtimes");
    fs::create_dir_all(&out_dir)?;
    fs::write(
        out_dir.join(measurements_file(args.label.as_deref())),
        serde_json::to_string_pretty(&measurements.iter().map(Measurement::json).collect::<Vec<_>>())? + "\n",
    )?;
    let table = table(&measurements);
    println!("\n{table}");
    summary(&table)?;
    // What the runtimes that passed measured is recorded even when others failed (the nightly
    // job pushes it before failing).
    if let (Some(history), Some(runner)) = (&args.history, &args.runner) {
        if measurements.is_empty() {
            eprintln!("pdf-runtimes: nothing to record: no runtime printed measurements");
        } else {
            record(ws, Path::new(history), runner, &measurements)?;
        }
    }
    if failed.is_empty() {
        step(&format!("pdf-runtimes: {} passed", args.runtimes.join(", ")));
        Ok(())
    } else {
        bail!("pdf-runtimes: {} failed", failed.join(", "))
    }
}

/// The measurements file's name: `measurements.json`, or `measurements-<label>.json`.
fn measurements_file(label: Option<&str>) -> String {
    label.map_or_else(
        || "measurements.json".to_owned(),
        |label| format!("measurements-{label}.json"),
    )
}

/// `value` against an upper `target`: a mark when it misses (reported, not a failure).
fn cell(value: Option<f64>, target: Option<f64>, unit: &str) -> String {
    let Some(value) = value else { return "–".to_owned() };
    let shown = if unit == "MB" {
        format!("{:.1} MB", value / (1024.0 * 1024.0))
    } else {
        format!("{value:.1} ms")
    };
    match target {
        Some(target) if value > target => format!("**{shown}** ⚠️"),
        _ => shown,
    }
}

/// The job-summary table: one row per runtime, the §4 targets in the header.
fn table(measurements: &[Measurement]) -> String {
    let mut by_runtime: BTreeMap<&str, BTreeMap<&str, f64>> = BTreeMap::new();
    for m in measurements {
        by_runtime.entry(&m.runtime).or_default().insert(&m.case, m.value);
    }
    let mut out = String::from(
        "## @laminapdf/sdk in every runtime\n\n\
         Recorded, not asserted (pdf-library §4: hello ≤ 5 ms after `init()`, a 100-page report ≤ 500 ms, \
         < 64 MB peak in a Worker); ⚠️ marks a miss. Hello is the median of 20 runs, the report of 3. \
         WebAssembly memory is both modules' linear memory after the report (it never shrinks, so it is \
         their peak); the JS heap is not included.\n\n\
         | Runtime | First hello | Hello | 100-page report | WebAssembly memory |\n| --- | --- | --- | --- | --- |\n",
    );
    for (runtime, cases) in &by_runtime {
        let memory_target = (*runtime == "workerd").then_some(WORKER_MEMORY_TARGET_BYTES);
        let _ = writeln!(
            out,
            "| {runtime} | {} | {} | {} | {} |",
            cell(cases.get("hello-first").copied(), None, "ms"),
            cell(cases.get("hello").copied(), Some(HELLO_TARGET_MS), "ms"),
            cell(cases.get("report").copied(), Some(REPORT_TARGET_MS), "ms"),
            cell(cases.get("wasm-memory").copied(), memory_target, "MB"),
        );
    }
    long_tasks(&by_runtime, &mut out);
    out
}

/// L2a's main-thread check: the longest long task while a 50 MB file is opened and fully
/// rewritten on the main thread and through `@laminapdf/sdk/worker` (Chromium's `longtask` entries;
/// 0 is none over 50 ms).
fn long_tasks(by_runtime: &BTreeMap<&str, BTreeMap<&str, f64>>, out: &mut String) {
    let rows: Vec<_> = by_runtime
        .iter()
        .filter_map(|(runtime, cases)| {
            let worker = cases.get("worker-rewrite-50mb-longest-task").copied()?;
            Some((*runtime, cases.get("sync-rewrite-50mb-longest-task").copied(), worker))
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    out.push_str(
        "\n### Main-thread long tasks, 50 MB open + full rewrite\n\n\
         | Runtime | Main thread | Worker client |\n| --- | --- | --- |\n",
    );
    for (runtime, sync, worker) in rows {
        let _ = writeln!(
            out,
            "| {runtime} | {} | {} |",
            cell(sync, None, "ms"),
            cell(Some(worker), None, "ms")
        );
    }
}

/// The history keys and values of `measurements`: `pdf/<runtime>/<case>` in nanoseconds
/// (`cargo xtask bench`'s unit), `pdf/<runtime>/wasm-memory-bytes` in bytes.
fn history_entries(measurements: &[Measurement]) -> BTreeMap<String, u64> {
    measurements
        .iter()
        .map(|m| {
            let (key, value) = if m.case == "wasm-memory" {
                (format!("pdf/{}/wasm-memory-bytes", m.runtime), m.value)
            } else {
                (format!("pdf/{}/{}", m.runtime, m.case), m.value * 1e6)
            };
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "non-negative timings and sizes far below 2^64, rounded"
            )]
            (key, value.max(0.0).round() as u64)
        })
        .collect()
}

/// Appends one record to `<history>/history.jsonl` (`cargo xtask bench`'s format; its
/// regression check only compares benchmarks a record has, so these never mix with criterion's).
fn record(ws: &Workspace, history: &Path, runner: &str, measurements: &[Measurement]) -> Result<()> {
    let record = crate::nightly::history_record(&ws.root, runner, false, &history_entries(measurements));
    crate::nightly::append_record(history, &record)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_args_should_default_to_every_runtime() {
        assert_eq!(parse_args(&[]).unwrap().runtimes, RUNTIMES.to_vec());
    }

    #[test]
    fn parse_args_should_keep_the_order_given_without_duplicates() {
        assert_eq!(
            parse_args(&args(&["deno", "node", "deno"])).unwrap().runtimes,
            ["deno", "node"]
        );
    }

    #[test]
    fn commands_should_cover_every_runtime() {
        for runtime in RUNTIMES {
            assert!(!commands(runtime).unwrap().is_empty(), "{runtime}");
        }
        assert!(commands("safari").is_err());
    }

    #[test]
    fn commands_should_run_the_measuring_project_alone_first() {
        let last = |runtime: &str| {
            commands(runtime)
                .unwrap()
                .iter()
                .map(|c| c.last().copied())
                .collect::<Vec<_>>()
        };
        assert_eq!(last("node"), [Some("measure"), Some("node")]);
        assert_eq!(last("chromium"), [Some("chromium"), Some("vite-build")]);
    }

    #[test]
    fn a_label_should_name_the_measurements_file() {
        let parsed = parse_args(&args(&["node", "--label", "node22"])).unwrap();
        assert_eq!(measurements_file(parsed.label.as_deref()), "measurements-node22.json");
    }

    #[test]
    fn no_label_should_keep_the_default_measurements_file() {
        assert_eq!(measurements_file(None), "measurements.json");
    }

    #[test]
    fn parse_args_should_reject_a_label_with_a_path_in_it() {
        assert!(parse_args(&args(&["--label", "../x"])).is_err());
    }

    #[test]
    fn parse_args_should_not_take_a_flag_as_the_label() {
        assert!(parse_args(&args(&["--label", "--record"])).is_err());
    }

    #[test]
    fn parse_args_should_reject_an_unknown_runtime() {
        assert!(parse_args(&args(&["safari"])).is_err());
    }

    #[test]
    fn parse_args_should_require_record_history_and_runner_together() {
        assert!(parse_args(&args(&["--record"])).is_err());
        assert!(parse_args(&args(&["--history", "h", "--runner", "r"])).is_err());
        assert!(parse_args(&args(&["--history", "h", "--record"])).is_err());
        assert!(parse_args(&args(&["--history", "h", "--runner", "r", "--record"])).is_ok());
    }

    #[test]
    fn table_should_list_the_worker_long_tasks_when_measured() {
        let measurements = [
            Measurement {
                runtime: "chromium".into(),
                case: "sync-rewrite-50mb-longest-task".into(),
                value: 207.0,
            },
            Measurement {
                runtime: "chromium".into(),
                case: "worker-rewrite-50mb-longest-task".into(),
                value: 0.0,
            },
        ];
        assert!(table(&measurements).contains("| chromium | 207.0 ms | 0.0 ms |"));
    }

    #[test]
    fn table_should_leave_out_the_long_tasks_when_not_measured() {
        let measurements = [Measurement {
            runtime: "node-24".into(),
            case: "hello".into(),
            value: 1.0,
        }];
        assert!(!table(&measurements).contains("long tasks"));
    }

    #[test]
    fn parse_measurements_should_read_prefixed_lines_anywhere_in_a_line() {
        let output = "noise\nstdout | x\nLAMINA_PDF_MEASUREMENT {\"runtime\":\"deno\",\"case\":\"hello\",\"value\":1.5}\n\
                      \u{1b}[0mLAMINA_PDF_MEASUREMENT {\"runtime\":\"bun\",\"case\":\"report\",\"value\":400}\n";
        let found = parse_measurements(output).unwrap();
        assert_eq!(
            found.iter().map(|m| (m.runtime.as_str(), m.value)).collect::<Vec<_>>(),
            [("deno", 1.5), ("bun", 400.0)]
        );
    }

    #[test]
    fn parse_measurements_should_fail_on_a_malformed_line() {
        assert!(parse_measurements("LAMINA_PDF_MEASUREMENT {not json}").is_err());
    }

    #[test]
    fn table_should_mark_misses_of_the_targets() {
        let m = |runtime: &str, case: &str, value: f64| Measurement {
            runtime: runtime.to_owned(),
            case: case.to_owned(),
            value,
        };
        let out = table(&[
            m("workerd", "report", 650.0),
            m("workerd", "hello", 1.0),
            m("workerd", "wasm-memory", 70.0 * 1024.0 * 1024.0),
            m("node-24", "wasm-memory", 70.0 * 1024.0 * 1024.0),
        ]);
        assert!(
            out.contains("| workerd | – | 1.0 ms | **650.0 ms** ⚠️ | **70.0 MB** ⚠️ |"),
            "{out}"
        );
        assert!(out.contains("| node-24 | – | – | – | 70.0 MB |"), "{out}");
    }

    #[test]
    fn history_entries_should_use_nanoseconds_and_bytes() {
        let m = |case: &str, value: f64| Measurement {
            runtime: "deno".to_owned(),
            case: case.to_owned(),
            value,
        };
        let entries = history_entries(&[m("report", 1.25), m("wasm-memory", 4096.0)]);
        assert_eq!(
            entries.into_iter().collect::<Vec<_>>(),
            [
                ("pdf/deno/report".to_owned(), 1_250_000),
                ("pdf/deno/wasm-memory-bytes".to_owned(), 4096)
            ]
        );
    }
}
