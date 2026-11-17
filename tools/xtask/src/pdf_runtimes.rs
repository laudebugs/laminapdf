//! `cargo xtask pdf-runtimes`: @laminapdf/sdk's example suite (`packages/sdk/test/suite/`) in every
//! runtime the package supports (todo B6c; pdf-library §5): Node (whichever `node` is on PATH),
//! Chromium, Firefox and `WebKit` (Vitest browser mode with Playwright), Deno, Bun and workerd.
//! Each runner compares every example's bytes with its Rust golden and prints the performance
//! smoke's figures (`LAMINA_PDF_MEASUREMENT {json}` lines), which this task collects into
//! `target/xtask/pdf-runtimes/measurements.json` (`measurements-<label>.json` with `--label`)
//! and a job-summary table against the §4 targets, and the runtimes whose example suite passed and
//! failed into `runtimes.json` (`runtimes-<label>.json`; [`RuntimeResults`], which `cargo xtask
//! proof` reads).
//! The targets are reported, never asserted. `--history <dir> --runner <label> --record` appends
//! the figures to the nightly benchmark history (`<dir>/history.jsonl`, the record format of
//! `cargo xtask bench`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead as _, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::util::{github_output, summary, wait_up_to};
use crate::workspace::Workspace;
use crate::{program_path, step};

/// Every runtime, in the order a full run takes them.
pub(crate) const RUNTIMES: [&str; 7] = ["node", "chromium", "firefox", "webkit", "deno", "bun", "workerd"];

/// Where the task writes, relative to the workspace root.
pub(crate) const OUT_DIR: &str = "target/xtask/pdf-runtimes";

/// `runtimes.json`: which runtimes' example suite passed, byte for byte against the Rust goldens.
/// A browser's render suites run after its example suite; their failure fails the task and drops
/// its measurements, but leaves the runtime under `passed`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RuntimeResults {
    pub(crate) passed: Vec<String>,
    pub(crate) failed: Vec<String>,
}

/// The prefix of a measurement line (`MEASUREMENT_PREFIX` in packages/sdk/test/suite/examples.ts).
const PREFIX: &str = "LAMINA_PDF_MEASUREMENT ";

/// pdf-library §4: hello after `init()`, the 100-page report, and peak memory in a Worker.
const HELLO_TARGET_MS: f64 = 5.0;
const REPORT_TARGET_MS: f64 = 500.0;
const WORKER_MEMORY_TARGET_BYTES: f64 = 64.0 * 1024.0 * 1024.0;

const USAGE: &str = "usage: cargo xtask pdf-runtimes [node|chromium|firefox|webkit|deno|bun|workerd]… [--suites <name,…>] [--label <name>] [--history <dir> --runner <label> --record]";

/// What `cargo xtask pdf-runtimes` was asked to do; the default runs every runtime, recording nothing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Args {
    runtimes: Vec<&'static str>,
    /// Names the measurements file (`measurements-<label>.json`), so several runs in one job
    /// (the fast lane's Node 22 and 26) each keep theirs.
    label: Option<String>,
    history: Option<String>,
    runner: Option<String>,
    /// The suites to run (`changes::suites::SUITES`' names, in that order); every suite when
    /// `--suites` isn't given, as the nightly and release lanes run them (todo X12).
    suites: Vec<&'static str>,
}

impl Args {
    fn runs(&self, suite: &str) -> bool {
        self.suites.contains(&suite)
    }
}

/// Every suite's name, in the order a browser leg runs them.
fn all_suites() -> Vec<&'static str> {
    crate::changes::suites::SUITES.iter().map(|s| s.name).collect()
}

/// The suites `list` names (comma-separated), in [`all_suites`]' order.
fn parse_suites(list: &str) -> Result<Vec<&'static str>> {
    let mut asked = Vec::new();
    for name in list.split(',').filter(|n| !n.is_empty()) {
        asked.push(crate::changes::suites::by_name(name)?.name);
    }
    if asked.is_empty() {
        bail!("--suites names no suite; {USAGE}");
    }
    Ok(all_suites().into_iter().filter(|s| asked.contains(s)).collect())
}

impl Default for Args {
    fn default() -> Self {
        Self {
            runtimes: RUNTIMES.to_vec(),
            label: None,
            history: None,
            runner: None,
            suites: all_suites(),
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
            "--suites" => parsed.suites = parse_suites(rest.next().context(USAGE)?)?,
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

/// The SDK's browser config, whose reporters print every test and report a pass on retry as flaky
/// (`@laminapdf/test-utils/flaky`).
const BROWSER_CONFIG: &str = "vitest.browser.config.ts";

/// Vitest over `config`, limited to `projects`, with every test's output. The browser config lists
/// its own reporters, which a `--reporter` would replace; the others get `--reporter verbose`.
fn vitest<'a>(config: &'a str, projects: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["vitest", "run", "-c", config];
    if config != BROWSER_CONFIG {
        args.extend(["--reporter", "verbose"]);
    }
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
            vitest(BROWSER_CONFIG, &["chromium"]),
            vitest(BROWSER_CONFIG, &["vite-build"]),
        ],
        "firefox" => vec![vitest(BROWSER_CONFIG, &["firefox"])],
        "webkit" => vec![vitest(BROWSER_CONFIG, &["webkit"])],
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
/// React `<Icon>` and gallery (U6c), `@laminapdf/ui`'s components (U6d), `@laminapdf/site-kit`'s site
/// header (W1d: keyboard order, the current page, phone width, axe), the docs' diagrams (contrast in the
/// light, dark and high-contrast themes and with forced colours, the 12 px label floor), and the reader app (`apps/web`,
/// C4b): typechecked and built over the packages above, then its e2e suite against `wrangler dev`
/// (headers, local files opened four ways, CSP violations and other origins), and the docs site's
/// "Edit and run" (B7e: built, then its e2e suite), and the website (built, then its e2e suite: the
/// cursor check over its pages, and the download page's choice of installer). They record no
/// measurements. Only the suites in
/// `args.suites` run (`--suites`, todo X12), each its part of this list.
fn render_suites(ws: &Workspace, browser: &str, args: &Args) -> Result<()> {
    if args.suites.iter().all(|&s| s == "examples") {
        return Ok(());
    }
    crate::web::packages(ws)?;
    for suite in &crate::changes::suites::SUITES {
        if !args.runs(suite.name) {
            continue;
        }
        let e2e = || {
            run_suite(
                ws,
                &["--filter", suite.package, "run", "e2e"],
                &[("LAMINA_PDF_E2E_BROWSER", browser)],
                E2E_LIMIT,
                false,
            )
            .map(drop)
        };
        let build = || crate::run(&ws.root, "pnpm", &["--filter", suite.package, "run", "build"]);
        match suite.name {
            "examples" => {}
            "web" => {
                // The reader imports the built viewer, so its typecheck runs here, after `wasm`,
                // not in `check ts`.
                crate::run(&ws.root, "pnpm", &["--filter", suite.package, "run", "typecheck:wasm"])?;
                build()?;
                // The app shell's startup JavaScript against its budget, on the build just made
                // (the PR lane's size gate, todo C7c): bytes, not time, so it may fail a pull
                // request (ADR 0018 §2).
                crate::web::size(ws)?;
                e2e()?;
            }
            "docs" => {
                // The docs' diagrams in the browser, then "Edit and run" (B7e): its build makes
                // the example runner from the same WebAssembly, then its e2e suite runs the six
                // examples against `wrangler dev`, with the docs' cursor check and states (U7e).
                vitest_browser(ws, suite.package, browser)?;
                build()?;
                e2e()?;
            }
            // The website's cursor check and states over its built pages, against `wrangler dev`,
            // and its download page, built from the e2e fixture's release lists rather than
            // the snapshot (apps/site/src/data/downloads.ts; the path is the site's own, where its
            // build runs).
            "site" => {
                crate::run_env(
                    &ws.root,
                    "pnpm",
                    &["--filter", suite.package, "run", "build"],
                    &[("LAMINA_PDF_DOWNLOADS", "e2e/downloads.fixture.json")],
                )?;
                e2e()?;
            }
            _ => vitest_browser(ws, suite.package, browser)?,
        }
    }
    Ok(())
}

/// A package's browser tests (its [`BROWSER_CONFIG`]) in `browser`.
fn vitest_browser(ws: &Workspace, package: &str, browser: &str) -> Result<()> {
    let mut args = vec!["--filter", package, "exec"];
    args.extend(vitest(BROWSER_CONFIG, &[browser]));
    run_suite(ws, &args, &[], PACKAGE_LIMIT, false).map(drop)
}

/// Runs `pnpm --filter @laminapdf/sdk exec <args>`, a runtime's example suite, under
/// [`PACKAGE_LIMIT`], and returns its stdout, which holds the measurements.
fn example_suite(ws: &Workspace, args: &[&str]) -> Result<String> {
    let mut full = vec!["--filter", "@laminapdf/sdk", "exec"];
    full.extend_from_slice(args);
    run_suite(ws, &full, &[], PACKAGE_LIMIT, true)
}

/// How long a package's browser tests, or a runtime's example suite, may run: the slowest took
/// 110 s (the viewer's browser tests, in `WebKit`).
const PACKAGE_LIMIT: Duration = Duration::from_mins(10);

/// How long an e2e suite may run: the slowest took 12.5 minutes (the reader's, in `WebKit`).
const E2E_LIMIT: Duration = Duration::from_mins(20);

/// Runs `pnpm <args>` as a suite (a runtime's examples, a package's browser tests, an e2e suite),
/// stopped with every process it started, a browser included, once it runs past `limit`; returns
/// its stdout when `capture` is set (echoed as it comes either way). A page that freezes the browser stops Vitest's own test timeouts with it: one `WebKit` suite once
/// printed nothing for 44 minutes, until the nightly job's limit cancelled the job and every
/// suite after it. A suite stopped that way runs once more; when that run passes, it counts as
/// flaky, like a test that passes on its retry (ADR 0018 §4). Two stopped runs of one suite cost
/// twice its limit, which a 60-minute job can't always spare: the job's own limit then ends it.
fn run_suite(ws: &Workspace, args: &[&str], envs: &[(&str, &str)], limit: Duration, capture: bool) -> Result<String> {
    let shown = crate::shown_command("pnpm", args, envs);
    let minutes = limit.as_secs() / 60;
    let mut ended = attempt(ws, args, envs, limit, capture, &shown)?;
    let retried = ended.is_none();
    if retried {
        println!(
            "::warning title=Suite stopped::`{shown}` ran past {minutes} minutes and was stopped; \
             running it once more"
        );
        ended = attempt(ws, args, envs, limit, capture, &shown)?;
    }
    let after = if retried {
        format!(" on its second run, after the first ran past {minutes} minutes")
    } else {
        String::new()
    };
    let Some((status, out)) = ended else {
        bail!("`{shown}` ran past {minutes} minutes twice");
    };
    if !status.success() {
        bail!("`{shown}` failed with {status}{after}");
    }
    if retried {
        println!("::warning title=Flaky suite (ADR 0018 §4)::`{shown}` passed{after}");
        github_output("flaky", "true")?;
    }
    Ok(out)
}

/// One run of `pnpm <args>` for [`run_suite`]: its status and (with `capture`) its stdout, or
/// `None` when it ran past `limit`. Captured lines are read on a thread so the limit still holds
/// while the suite prints nothing; a stopped run's thread is left to end when the pipe closes.
fn attempt(
    ws: &Workspace,
    args: &[&str],
    envs: &[(&str, &str)],
    limit: Duration,
    capture: bool,
    shown: &str,
) -> Result<Option<(std::process::ExitStatus, String)>> {
    step(shown);
    let mut command = Command::new(program_path("pnpm"));
    command.args(args).envs(envs.iter().copied()).current_dir(&ws.root);
    if capture {
        command.stdout(Stdio::piped());
    }
    let mut child = command
        .spawn()
        .context("failed to start pnpm; is it installed and on PATH?")?;
    let reader = child.stdout.take().map(|stdout| {
        std::thread::spawn(move || -> Result<String> {
            // Bytes, decoded lossily: a line that isn't UTF-8 must not stop the draining, or
            // pnpm would block on a full pipe and read as hung.
            let mut captured = String::new();
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            while reader.read_until(b'\n', &mut line).context("reading pnpm's output")? > 0 {
                let text = String::from_utf8_lossy(&line);
                print!("{text}");
                captured.push_str(&text);
                line.clear();
            }
            Ok(captured)
        })
    });
    let Some(status) = wait_up_to(&mut child, limit)? else {
        return Ok(None);
    };
    let out = match reader {
        Some(thread) => thread
            .join()
            .map_err(|_| anyhow::anyhow!("the thread reading pnpm's output panicked"))??,
        None => String::new(),
    };
    Ok(Some((status, out)))
}

/// `cargo xtask pdf-runtimes [runtime…] [--label <name>] [--history <dir> --runner <label> --record]`:
/// runs each runtime's suite in turn (all of them by default); every runtime runs even when an earlier
/// one failed, and the failures are listed at the end. Needs `cargo xtask wasm` first (it
/// builds `packages/wasm/dist/` and `packages/sdk/dist/`); installs the Playwright browsers it
/// needs (`playwright install`, a no-op when they are present).
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let out_dir = ws.root.join(OUT_DIR);
    // An earlier run's results must not stand in for this one if it stops before writing them.
    forget_results(&out_dir, args.label.as_deref())?;
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
    let mut results = RuntimeResults::default();
    for &runtime in &args.runtimes {
        let mut example_passed = false;
        let examples = args.runs("examples");
        let browser = matches!(runtime, "chromium" | "firefox" | "webkit");
        let result = (|| -> Result<Vec<Measurement>> {
            if !examples && !browser {
                // The server runtimes run the examples alone.
                step(&format!("pdf-runtimes: {runtime} has no suite among {:?}", args.suites));
                return Ok(Vec::new());
            }
            if browser {
                crate::run(
                    &ws.root,
                    "pnpm",
                    &["--filter", "@laminapdf/sdk", "exec", "playwright", "install", runtime],
                )?;
            }
            let mut found = Vec::new();
            if examples {
                if runtime == "node" {
                    // The Node suite's compare-pages test draws pages with
                    // @laminapdf/render-hayro, which it imports through its built `dist/` (the
                    // `wasm` job uploads only the modules and the SDK's).
                    crate::run(&ws.root, "pnpm", &["--filter", "@laminapdf/render", "run", "build"])?;
                    crate::run(
                        &ws.root,
                        "pnpm",
                        &["--filter", "@laminapdf/render-hayro", "run", "build:wasm"],
                    )?;
                }
                for command in commands(runtime)? {
                    found.extend(parse_measurements(&example_suite(ws, &command)?)?);
                }
                // The example suite passed; a browser's suites below can still fail the runtime.
                example_passed = !found.is_empty();
            }
            if browser {
                render_suites(ws, runtime, args)?;
            }
            Ok(found)
        })();
        // Without the examples, a runtime passes when its suites do.
        if example_passed || (!examples && result.is_ok()) {
            results.passed.push(runtime.to_owned());
        } else {
            results.failed.push(runtime.to_owned());
        }
        match result {
            Ok(found) if found.is_empty() && examples => {
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
    fs::create_dir_all(&out_dir)?;
    fs::write(
        out_dir.join(results_file(args.label.as_deref())),
        serde_json::to_string_pretty(&results)? + "\n",
    )?;
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

/// The results file's name: `runtimes.json`, or `runtimes-<label>.json`.
pub(crate) fn results_file(label: Option<&str>) -> String {
    label.map_or_else(|| "runtimes.json".to_owned(), |label| format!("runtimes-{label}.json"))
}

/// Removes `dir`'s results file for `label` from an earlier run.
fn forget_results(dir: &Path, label: Option<&str>) -> Result<()> {
    let path = dir.join(results_file(label));
    match fs::remove_file(&path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(e).with_context(|| format!("removing {}", path.display()))
        }
        _ => Ok(()),
    }
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
         < 64 MB peak in a Worker); ⚠️ marks a miss. Hello and the one-page invoice are the median of 20 runs, \
         the report of 3. \
         WebAssembly memory is both modules' linear memory after the report (it never shrinks, so it is \
         their peak); the JS heap is not included.\n\n\
         | Runtime | First hello | Hello | Invoice | 100-page report | WebAssembly memory |\n\
         | --- | --- | --- | --- | --- | --- |\n",
    );
    for (runtime, cases) in &by_runtime {
        let memory_target = (*runtime == "workerd").then_some(WORKER_MEMORY_TARGET_BYTES);
        let _ = writeln!(
            out,
            "| {runtime} | {} | {} | {} | {} | {} |",
            cell(cases.get("hello-first").copied(), None, "ms"),
            cell(cases.get("hello").copied(), Some(HELLO_TARGET_MS), "ms"),
            cell(cases.get("invoice").copied(), None, "ms"),
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
    fn browser_suites_should_keep_their_config_reporters_and_the_others_print_every_test() {
        // A --reporter replaces the config's list, which reports a pass on retry as flaky.
        assert!(!vitest(BROWSER_CONFIG, &["webkit"]).contains(&"--reporter"));
        assert!(vitest("vitest.node.config.ts", &["node"]).ends_with(&["--reporter", "verbose", "--project", "node"]));
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
    fn forget_results_should_remove_only_its_labels_file() {
        let dir = std::env::temp_dir().join(format!("xtask-runtimes-forget-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("runtimes-linux.json"), "{}").unwrap();
        fs::write(dir.join("runtimes-webkit.json"), "{}").unwrap();
        forget_results(&dir, Some("linux")).unwrap();
        let kept = (
            dir.join("runtimes-linux.json").exists(),
            dir.join("runtimes-webkit.json").exists(),
        );
        let missing_is_fine = forget_results(&dir, None).is_ok();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!((kept, missing_is_fine), ((false, true), true));
    }

    #[test]
    fn a_label_should_name_the_results_file_too() {
        assert_eq!(
            [results_file(None), results_file(Some("webkit"))],
            ["runtimes.json", "runtimes-webkit.json"]
        );
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
    fn parse_args_should_run_every_suite_without_suites() {
        assert_eq!(parse_args(&[]).unwrap().suites, all_suites());
    }

    #[test]
    fn parse_args_should_keep_the_suites_given_in_the_order_a_leg_runs_them() {
        assert_eq!(
            parse_args(&args(&["chromium", "--suites", "site,examples,ui"]))
                .unwrap()
                .suites,
            ["examples", "ui", "site"]
        );
    }

    #[test]
    fn parse_args_should_reject_an_unknown_or_empty_suite_list() {
        assert!(parse_args(&args(&["--suites", "gallery"])).is_err());
        assert!(parse_args(&args(&["--suites", ","])).is_err());
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
            m("workerd", "invoice", 2.5),
            m("workerd", "wasm-memory", 70.0 * 1024.0 * 1024.0),
            m("node-24", "wasm-memory", 70.0 * 1024.0 * 1024.0),
        ]);
        assert!(
            out.contains("| workerd | – | 1.0 ms | 2.5 ms | **650.0 ms** ⚠️ | **70.0 MB** ⚠️ |"),
            "{out}"
        );
        assert!(out.contains("| node-24 | – | – | – | – | 70.0 MB |"), "{out}");
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
