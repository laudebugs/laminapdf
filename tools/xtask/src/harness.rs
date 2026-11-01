//! The test-harness tasks of ADR 0018: goldens (tier 3), corpus facts (tier 4), the pypdf
//! differential (tier 6) and the fuzz targets (tier 9).
//!
//! Python is needed only to regenerate facts (`corpus facts`, `differential --update`): it is
//! `$LAMINA_PYTHON` or `python3`, with `tools/corpus-facts/requirements.txt` installed. The
//! fast lane only reads the precomputed files.

use std::ffi::OsStr;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;
use crate::{run, step};

/// The fuzz targets in `fuzz/fuzz_targets/` (ADR 0018 tier 9).
pub(crate) const FUZZ_TARGETS: [&str; 9] = [
    "lex",
    "parse_object",
    "flate_decode",
    "open",
    "page_tree",
    "content",
    "ops_json",
    "save_roundtrip",
    "extract_text",
];

/// Default `cargo xtask fuzz` run time in seconds (the scheduled workflow uses 600).
const DEFAULT_FUZZ_SECONDS: u64 = 60;

pub(crate) fn python() -> String {
    std::env::var("LAMINA_PYTHON").unwrap_or_else(|_| "python3".to_owned())
}

/// A clean scratch directory under `target/xtask/`.
fn scratch(ws: &Workspace, name: &str) -> Result<PathBuf> {
    let dir = ws.root.join("target/xtask").join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir)
}

fn files_in(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// The shared `page.text()` snapshot (todo B9c): `crates/laminapdf/tests/page_text_snapshot.rs`
/// writes it (its ignored `write_snapshot` test, to a scratch file); with `update`, into
/// `fixtures/text/page-text.json`, else compared. `@laminapdf/sdk`'s text test reads the same file.
fn page_text_snapshot(ws: &Workspace, update: bool) -> Result<Vec<String>> {
    let out = scratch(ws, "page-text")?.join("page-text.json");
    let out_arg = out.to_string_lossy().into_owned();
    crate::run_env(
        &ws.root,
        "cargo",
        &[
            "test",
            "--locked",
            "-q",
            "-p",
            "laminapdf",
            "--test",
            "page_text_snapshot",
            "--",
            "--ignored",
            "--exact",
            "write_snapshot",
        ],
        &[("LAMINA_SNAPSHOT_OUT", out_arg.as_str())],
    )?;
    let target = ws.root.join("fixtures/text/page-text.json");
    let bytes = fs::read(&out)?;
    if bytes == fs::read(&target).unwrap_or_default() {
        return Ok(Vec::new());
    }
    if update {
        fs::write(&target, &bytes).with_context(|| format!("writing {}", target.display()))?;
        return Ok(Vec::new());
    }
    Ok(vec!["text/page-text.json".to_owned()])
}

/// The reading-order corpus (todo U2b): `laminapdf-build`'s `reading_order` example writes it.
/// With `update`, into `fixtures/reading-order/`, removing the files no layout writes any
/// more (they would still be scored); without, it compares and returns the files that
/// differ or have no generator. `threshold.json` is hand-kept.
fn reading_order(ws: &Workspace, update: bool) -> Result<Vec<String>> {
    let dir = ws.root.join("fixtures/reading-order");
    let out = scratch(ws, "reading-order")?;
    ws.cargo(&[
        "run",
        "--locked",
        "-q",
        "-p",
        "laminapdf-build",
        "--example",
        "reading_order",
        "--",
        &out.to_string_lossy(),
    ])?;
    let generated = files_in(&out)?;
    let mut stale = Vec::new();
    for path in &generated {
        let name = path.file_name().context("file name")?;
        let target = dir.join(name);
        let bytes = fs::read(path)?;
        if bytes != fs::read(&target).unwrap_or_default() {
            if update {
                fs::write(&target, &bytes).with_context(|| format!("writing {}", target.display()))?;
            } else {
                stale.push(format!("reading-order/{}", name.to_string_lossy()));
            }
        }
    }
    for path in files_in(&dir)? {
        let name = path.file_name().context("file name")?;
        if name == "threshold.json" || generated.iter().any(|g| g.file_name() == Some(name)) {
            continue;
        }
        if update {
            fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
            step(&format!("removed {} (no layout writes it)", path.display()));
        } else {
            stale.push(format!("reading-order/{} (no generator)", name.to_string_lossy()));
        }
    }
    Ok(stale)
}

/// The crates whose `golden` example writes goldens into `fixtures/golden/`.
const GOLDEN_CRATES: [&str; 3] = ["laminapdf-build", "laminapdf-save", "laminapdf-text"];

/// `cargo xtask golden [--update]`: regenerates every golden in `fixtures/golden/` from its
/// source (`--update` writes them in place and prints the list), the `lamina` examples' into
/// `fixtures/golden/examples/` and the reading-order corpus into `fixtures/reading-order/`; without `--update`, checks that regenerating them changes
/// nothing.
pub(crate) fn golden(ws: &Workspace, update: bool) -> Result<()> {
    let golden_dir = ws.root.join("fixtures/golden");
    let out = if update {
        golden_dir.clone()
    } else {
        scratch(ws, "golden")?
    };
    let out_arg = out.to_string_lossy().into_owned();
    // Every crate with goldens has a `golden` example that writes its files into a directory.
    for krate in GOLDEN_CRATES {
        ws.cargo(&[
            "run",
            "--locked",
            "-q",
            "-p",
            krate,
            "--example",
            "golden",
            "--",
            &out_arg,
        ])?;
    }
    // The façade's examples are goldens too, one directory down (todo B5c).
    let examples = crate::examples::run_all(ws, &out.join("examples"))?;
    let mut reading_stale = reading_order(ws, update)?;
    reading_stale.extend(page_text_snapshot(ws, update)?);
    if update {
        // A golden whose example is gone would fail `crates/laminapdf/tests/examples.rs`.
        for path in files_in(&out.join("examples"))? {
            if !examples.contains(&path) {
                fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
                step(&format!("removed {} (no example)", path.display()));
            }
        }
        shaping_goldens(ws)?;
        step("Goldens regenerated; review the diff (`git diff fixtures/golden`) like code");
        return Ok(());
    }
    let mut stale = reading_stale;
    let generated = files_in(&out)?;
    for path in &generated {
        let name = path.file_name().context("file name")?;
        let committed = fs::read(golden_dir.join(name)).unwrap_or_default();
        if fs::read(path)? != committed {
            stale.push(name.to_string_lossy().into_owned());
        }
    }
    for path in files_in(&golden_dir)? {
        let name = path.file_name().context("file name")?;
        if !generated.iter().any(|g| g.file_name() == Some(name)) {
            stale.push(format!("{} (no generator)", name.to_string_lossy()));
        }
    }
    let examples_dir = golden_dir.join("examples");
    for path in &examples {
        let name = path.file_name().context("file name")?;
        if fs::read(path)? != fs::read(examples_dir.join(name)).unwrap_or_default() {
            stale.push(format!("examples/{}", name.to_string_lossy()));
        }
    }
    for path in files_in(&examples_dir)? {
        let name = path.file_name().context("file name")?;
        if !examples.iter().any(|g| g.file_name() == Some(name)) {
            stale.push(format!("examples/{} (no example)", name.to_string_lossy()));
        }
    }
    if !stale.is_empty() {
        bail!(
            "goldens differ from their sources: {}; run `cargo xtask golden --update`",
            stale.join(", ")
        );
    }
    step("Goldens match their sources");
    Ok(())
}

/// Rewrites the shaping goldens `fixtures/shaping/<script>.json` with HarfBuzz's `hb-shape`
/// (`tools/shaping-goldens/goldens.py`, todo B4b) when `hb-shape` is installed at the pinned
/// version; otherwise says so and leaves them (they are HarfBuzz's output, not lamina's, so
/// `golden` without `--update` never needs them: `crates/laminapdf-text/tests/shaping_goldens.rs`
/// compares).
fn shaping_goldens(ws: &Workspace) -> Result<()> {
    match crate::doctor::hb_shape().probe(ws) {
        crate::doctor::Probe::Missing => {
            step("hb-shape not found: fixtures/shaping/ left as they are (install HarfBuzz to rewrite them)");
            Ok(())
        }
        crate::doctor::Probe::OffPin(found) => {
            step(&format!(
                "hb-shape is {found}, the goldens are pinned to HarfBuzz {}: fixtures/shaping/ left as they are",
                crate::doctor::HB_SHAPE_VERSION
            ));
            Ok(())
        }
        crate::doctor::Probe::Ok(_) => {
            let script = ws.root.join("tools/shaping-goldens/goldens.py");
            run(&ws.root, &python(), &[script.as_os_str()])
        }
    }
}

/// `cargo xtask corpus facts`: rewrites `fixtures/corpus/expected.json` with pypdf.
pub(crate) fn corpus_facts(ws: &Workspace) -> Result<()> {
    let script = ws.root.join("tools/corpus-facts/facts.py");
    run(&ws.root, &python(), &[script.as_os_str(), OsStr::new("corpus")])
}

/// What `cargo xtask differential` was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Differential {
    /// The full lane's set (`--full`) instead of the fast one.
    pub(crate) full: bool,
    /// Regenerate the expectations with pypdf first.
    pub(crate) update: bool,
    /// The text differential (`--text`, todo B9c): lamina's page text against pdf.js and
    /// pdftotext, from `fixtures/differential/text.json`.
    pub(crate) text: bool,
}

/// Parses `differential [--full] [--update]` (flags in any order, each at most once).
pub(crate) fn parse_differential_args(args: &[String]) -> Result<Differential> {
    let mut task = Differential {
        full: false,
        update: false,
        text: false,
    };
    for arg in args {
        match arg.as_str() {
            "--update" if !task.update => task.update = true,
            "--full" if !task.full && !task.text => task.full = true,
            "--text" if !task.text && !task.full => task.text = true,
            _ => bail!("usage: cargo xtask differential [--full | --text] [--update]"),
        }
    }
    Ok(task)
}

/// The test that checks the full set, by its exact name.
const FULL_SET_TEST: &str = "full_set_agrees_with_pypdf";

/// `cargo nextest` arguments for the full set: its ignored test only, under the `ci` profile
/// (no retries, the per-test hang timeout), failing when the filter matches no test (a
/// renamed test must not pass silently).
fn full_set_test_args() -> Vec<String> {
    [
        "nextest",
        "run",
        "--locked",
        "--profile",
        "ci",
        "-p",
        "laminapdf-model",
        "--test",
        "differential",
        "--run-ignored",
        "only",
        "--no-tests",
        "fail",
        "-E",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([format!("test(={FULL_SET_TEST})")])
    .collect()
}

/// `cargo xtask differential [--full] [--update]`: compares lamina with pypdf's precomputed
/// facts (no Python); `--update` regenerates them with pypdf first. Without `--full`, the fast
/// set (`fixtures/differential/expected.json`, run with the other tests); with it, the full
/// lane's set (`full.json`, exceptions in `full-exceptions.toml`), run through nextest.
pub(crate) fn differential(ws: &Workspace, task: &Differential) -> Result<()> {
    if task.text {
        return text_differential(ws, task.update);
    }
    if task.update {
        let inputs = scratch(ws, "differential")?;
        let inputs_arg = inputs.to_string_lossy().into_owned();
        let mut args = vec![
            "run",
            "--locked",
            "-q",
            "-p",
            "laminapdf-model",
            "--example",
            "differential-inputs",
            "--",
            &inputs_arg,
        ];
        if task.full {
            args.push("--full");
        }
        ws.cargo(&args)?;
        let script = ws.root.join("tools/corpus-facts/facts.py");
        let expected =
            ws.root
                .join("fixtures/differential")
                .join(if task.full { "full.json" } else { "expected.json" });
        run(
            &ws.root,
            &python(),
            &[
                script.as_os_str(),
                OsStr::new("dir"),
                inputs.as_os_str(),
                expected.as_os_str(),
            ],
        )?;
    }
    if task.full {
        crate::doctor::require(ws, &crate::doctor::cargo_nextest())?;
        let args = full_set_test_args();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        ws.cargo(&args)
    } else {
        cargo_test(ws, "laminapdf-model", "differential")
    }
}

/// The inputs of the text differential: the corpus and the text goldens.
fn text_inputs(ws: &Workspace) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for dir in ["fixtures/corpus", "fixtures/golden"] {
        for path in files_in(&ws.root.join(dir))? {
            let name = path.file_name().context("file name")?.to_string_lossy().into_owned();
            let wanted = path.extension().is_some_and(|e| e == "pdf")
                && (dir == "fixtures/corpus" || name.starts_with("text-") || name.starts_with("build-text-"));
            if wanted {
                out.push(format!("{dir}/{name}"));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// `differential --text [--update]` (todo B9c): with `--update`, `tools/differential/text.mjs`
/// rewrites `fixtures/differential/text.json` with pdf.js and pdftotext; then lamina's test
/// compares against it.
fn text_differential(ws: &Workspace, update: bool) -> Result<()> {
    if update {
        crate::doctor::require(ws, &crate::doctor::pdftotext())?;
        ws.pnpm(&["--dir", "tools/differential", "install", "--frozen-lockfile"])?;
        let mut args = vec![
            "tools/differential/text.mjs".to_owned(),
            "fixtures/differential/text.json".to_owned(),
        ];
        args.extend(text_inputs(ws)?);
        run(&ws.root, "node", &args)?;
    }
    cargo_test(ws, "laminapdf", "text_differential")
}

/// The quad fixtures: the text goldens and the examples with text.
const QUAD_FILES: [&str; 10] = [
    "fixtures/golden/build-text-latin.pdf",
    "fixtures/golden/build-text-multiscript.pdf",
    "fixtures/golden/examples/hello.pdf",
    "fixtures/golden/examples/invoice.pdf",
    "fixtures/golden/examples/multilingual.pdf",
    "fixtures/golden/text-cff.pdf",
    "fixtures/golden/text-latin.pdf",
    "fixtures/golden/text-shaped.pdf",
    "fixtures/golden/text-std14.pdf",
    "fixtures/golden/text-variable-collection.pdf",
];

/// `cargo xtask quads [--update]` (todo B9c): with `--update`, `tools/render/charbox.py`
/// (pypdfium2, `tools/render/requirements.txt`) rewrites `fixtures/text/charboxes.json`; then
/// lamina's `quads` test compares its glyph quads with PDFium's character boxes.
pub(crate) fn quads(ws: &Workspace, args: &[String]) -> Result<()> {
    let update = match args {
        [] => false,
        [flag] if flag == "--update" => true,
        _ => bail!("usage: cargo xtask quads [--update]"),
    };
    if update {
        let mut script = vec![
            "tools/render/charbox.py".to_owned(),
            "fixtures/text/charboxes.json".to_owned(),
        ];
        script.extend(QUAD_FILES.iter().map(|f| (*f).to_owned()));
        run(&ws.root, &python(), &script)?;
    }
    cargo_test(ws, "laminapdf", "quads")
}

/// `cargo test --locked -q -p <package> --test <test>`.
fn cargo_test(ws: &Workspace, package: &str, test: &str) -> Result<()> {
    ws.cargo(&["test", "--locked", "-q", "-p", package, "--test", test])
}

/// What `cargo xtask fuzz` was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FuzzTask<'a> {
    /// Run one target for `seconds`.
    Run { target: &'a str, seconds: u64 },
    /// Run every target, each for `seconds`, `jobs` of them at once (`--all [--jobs N]`).
    RunAll { seconds: u64, jobs: usize },
    /// Only build the seed corpora (all targets, or one).
    Seeds(Option<&'a str>),
    /// Minimise the corpora in place (`cargo fuzz cmin`; all targets, or one).
    Cmin(Option<&'a str>),
}

const FUZZ_USAGE: &str = "usage: cargo xtask fuzz <target> [--time <seconds>] | fuzz --all [--time <seconds>] \
     [--jobs <n>] | fuzz seeds [<target>] | fuzz cmin [<target>]";

/// `target` if it is one of [`FUZZ_TARGETS`], else an error listing them.
pub(crate) fn known_fuzz_target(target: &str) -> Result<&str> {
    if FUZZ_TARGETS.contains(&target) {
        Ok(target)
    } else {
        bail!("unknown fuzz target {target:?}; targets: {}", FUZZ_TARGETS.join(", "))
    }
}

/// Parses `fuzz <target> [--time N]`, `fuzz --all [--time N] [--jobs N]` (flags in any order),
/// `fuzz seeds [<target>]` and `fuzz cmin [<target>]`.
pub(crate) fn parse_fuzz_args(args: &[String]) -> Result<FuzzTask<'_>> {
    let known = |t: &str| known_fuzz_target(t).map(drop);
    let seconds = |s: &str| -> Result<u64> { s.parse().with_context(|| format!("--time {s:?}: not a number")) };
    match args {
        [all, flags @ ..] if all == "--all" => parse_run_all(flags),
        [cmd] if cmd == "seeds" => Ok(FuzzTask::Seeds(None)),
        [cmd, target] if cmd == "seeds" => {
            known(target)?;
            Ok(FuzzTask::Seeds(Some(target)))
        }
        [cmd] if cmd == "cmin" => Ok(FuzzTask::Cmin(None)),
        [cmd, target] if cmd == "cmin" => {
            known(target)?;
            Ok(FuzzTask::Cmin(Some(target)))
        }
        [target] => {
            known(target)?;
            Ok(FuzzTask::Run {
                target,
                seconds: DEFAULT_FUZZ_SECONDS,
            })
        }
        [target, flag, time] if flag == "--time" => {
            known(target)?;
            Ok(FuzzTask::Run {
                target,
                seconds: seconds(time)?,
            })
        }
        _ => bail!("{FUZZ_USAGE}"),
    }
}

/// `--time N` and `--jobs N` after `--all`, in any order, each at most once.
fn parse_run_all(flags: &[String]) -> Result<FuzzTask<'static>> {
    let mut seconds = None;
    let mut jobs = None;
    for pair in flags.chunks(2) {
        let [flag, value] = pair else { bail!("{FUZZ_USAGE}") };
        match flag.as_str() {
            "--time" if seconds.is_none() => {
                seconds = Some(
                    value
                        .parse()
                        .with_context(|| format!("--time {value:?}: not a number"))?,
                );
            }
            "--jobs" if jobs.is_none() => {
                let n: usize = value
                    .parse()
                    .with_context(|| format!("--jobs {value:?}: not a number"))?;
                if !(1..=MAX_FUZZ_JOBS).contains(&n) {
                    bail!("--jobs must be between 1 and {MAX_FUZZ_JOBS}");
                }
                jobs = Some(n);
            }
            _ => bail!("{FUZZ_USAGE}"),
        }
    }
    Ok(FuzzTask::RunAll {
        seconds: seconds.unwrap_or(DEFAULT_FUZZ_SECONDS),
        jobs: jobs.unwrap_or(1),
    })
}

/// `cargo xtask fuzz …`.
pub(crate) fn fuzz(ws: &Workspace, task: &FuzzTask<'_>) -> Result<()> {
    match *task {
        FuzzTask::Seeds(target) => targets(target).into_iter().try_for_each(|t| seeds(ws, t).map(drop)),
        FuzzTask::Cmin(target) => {
            let host = fuzz_setup(ws)?;
            targets(target).into_iter().try_for_each(|t| cmin(ws, &host, t))
        }
        FuzzTask::Run { target, seconds } => {
            let host = fuzz_setup(ws)?;
            fuzz_one(ws, &host, target, seconds)
        }
        FuzzTask::RunAll { seconds, jobs } => {
            let host = fuzz_setup(ws)?;
            // Every target runs even after one fails, so one run reports every crash.
            let failed: Vec<String> = if jobs == 1 {
                FUZZ_TARGETS
                    .into_iter()
                    .filter_map(|target| fuzz_one(ws, &host, target, seconds).err())
                    .map(|e| format!("{e:#}"))
                    .collect()
            } else {
                fuzz_parallel(ws, &host, seconds, jobs)?
            };
            if !failed.is_empty() {
                bail!("{} fuzz target(s) failed:\n  {}", failed.len(), failed.join("\n  "));
            }
            step(&format!(
                "All {} fuzz targets ran {seconds} s without a crash",
                FUZZ_TARGETS.len()
            ));
            Ok(())
        }
    }
}

/// Runs every target for `seconds`, `jobs` cargo-fuzz processes at once, and returns one
/// message per failed target (in target order). Seed corpora are written and the targets
/// built first, so the concurrent runs only fuzz; a target whose seeds or build fail is
/// reported by name and the others still run. Each run streams its output to
/// `target/xtask/fuzz/<target>.log` (kept if the job is killed; the full lane uploads it) and
/// prints it whole, every line prefixed `[target]`, when the run ends.
fn fuzz_parallel(ws: &Workspace, host: &str, seconds: u64, jobs: usize) -> Result<Vec<String>> {
    let mut failures: Vec<(&str, String)> = Vec::new();
    let mut ready = Vec::new();
    for target in FUZZ_TARGETS {
        match seeds(ws, target) {
            Ok(corpus) => ready.push((target, corpus)),
            Err(e) => failures.push((target, format!("{target}: setup failed (seed corpus): {e:#}"))),
        }
    }
    // One build for every target, with `cargo fuzz run`'s default options so the runs find it
    // up to date. If it fails, each target is built on its own to name the broken ones.
    if let Err(e) = run(&ws.root, "cargo", &fuzz_build_args(host, None)) {
        eprintln!("note: building every fuzz target at once failed ({e:#}); building each on its own");
        ready.retain(
            |&(target, _)| match run(&ws.root, "cargo", &fuzz_build_args(host, Some(target))) {
                Ok(()) => true,
                Err(e) => {
                    failures.push((target, format!("{target}: build failed (setup problem): {e:#}")));
                    false
                }
            },
        );
    }
    let logs = scratch(ws, "fuzz")?;
    step(&format!(
        "Fuzzing {} targets for {seconds} s each, {jobs} at once (logs in {})",
        ready.len(),
        logs.display()
    ));
    let next = AtomicUsize::new(0);
    let run_failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.min(ready.len()) {
            scope.spawn(|| {
                while let Some((target, corpus)) = ready.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let log = logs.join(format!("{target}.log"));
                    if let Err(e) = fuzz_logged(ws, host, target, corpus, seconds, jobs, &log) {
                        run_failures
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .push((*target, format!("{e:#}")));
                    }
                }
            });
        }
    });
    failures.extend(
        run_failures
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    // Report in target order, whatever order the runs ended in.
    failures.sort_by_key(|(target, _)| FUZZ_TARGETS.iter().position(|t| t == target));
    Ok(failures.into_iter().map(|(_, message)| message).collect())
}

/// One target's run for [`fuzz_parallel`]: stdout and stderr stream into `log`, which is
/// printed with `[target]` prefixes when the run ends.
fn fuzz_logged(
    ws: &Workspace,
    host: &str,
    target: &str,
    corpus: &Path,
    seconds: u64,
    jobs: usize,
    log: &Path,
) -> Result<()> {
    let watch = ArtifactWatch::new(&ws.root, target);
    let args = fuzz_run_args(host, target, &corpus.to_string_lossy(), seconds, jobs);
    let file = fs::File::create(log).with_context(|| format!("{target}: creating {}", log.display()))?;
    let stderr = file
        .try_clone()
        .with_context(|| format!("{target}: opening {}", log.display()))?;
    println!("[{target}] started: cargo {}", args.join(" "));
    let status = Command::new(crate::program_path("cargo"))
        .args(&args)
        .current_dir(&ws.root)
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(stderr))
        .status()
        .map_err(|e| watch.failure(&format!("failed to start cargo: {e}")))?;
    let text = fs::read(log).unwrap_or_default();
    // A closed stdout must not hide the run's result, so a failed write is ignored.
    let _ = writeln!(
        std::io::stdout().lock(),
        "{}[{target}] finished: {status}",
        prefixed(target, &String::from_utf8_lossy(&text))
    );
    if status.success() {
        Ok(())
    } else {
        Err(watch.failure(&format!("cargo fuzz run exited with {status} (log: {})", log.display())))
    }
}

/// `text` with `[target] ` before every line.
fn prefixed(target: &str, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        out.push('[');
        out.push_str(target);
        out.push_str("] ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// `cargo fuzz build` for every target (`None`) or one, with `cargo fuzz run`'s defaults.
fn fuzz_build_args<'a>(host: &'a str, target: Option<&'a str>) -> Vec<&'a str> {
    let mut args = vec!["+nightly", "fuzz", "build", "--target", host];
    args.extend(target);
    args
}

/// The most cargo-fuzz processes `--jobs` allows at once: each may use up to
/// `-rss_limit_mb=2048`, so 8 bounds a run at 16 GB.
const MAX_FUZZ_JOBS: usize = 8;

/// libFuzzer's limits for a run with `jobs` processes at once. With one, exactly
/// [`crate::nightly::LIBFUZZER_LIMITS`] (which `tmin` uses). With more, the per-input timeout
/// grows with `jobs` (10 s each), so a slow input on a busy, shared machine isn't reported as
/// a timeout crash; the memory limit stays 2048 MB per process (lowering it would turn inputs
/// that are fine alone into OOMs), and [`MAX_FUZZ_JOBS`] bounds the total.
fn libfuzzer_limits(jobs: usize) -> Vec<String> {
    if jobs <= 1 {
        return crate::nightly::LIBFUZZER_LIMITS.map(str::to_owned).to_vec();
    }
    let timeout = 10 * jobs;
    vec![format!("-timeout={timeout}"), "-rss_limit_mb=2048".to_owned()]
}

/// `cargo` arguments that fuzz `target` over `corpus` for `seconds` under
/// [`libfuzzer_limits`] for `jobs` runs at once.
fn fuzz_run_args(host: &str, target: &str, corpus: &str, seconds: u64, jobs: usize) -> Vec<String> {
    // fuzz/ is its own nightly workspace (ADR 0018); `+nightly` overrides the root pin.
    let max_time = format!("-max_total_time={seconds}");
    [
        "+nightly", "fuzz", "run", "--target", host, target, corpus, "--", &max_time,
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(libfuzzer_limits(jobs))
    .collect()
}

/// The crash inputs a run may leave in `fuzz/artifacts/<target>/`: the names there before
/// it, so a failure can be told apart as a crash (new files) or a build or setup problem.
struct ArtifactWatch<'a> {
    target: &'a str,
    dir: PathBuf,
    before: std::collections::BTreeSet<String>,
}

impl<'a> ArtifactWatch<'a> {
    fn new(root: &Path, target: &'a str) -> Self {
        let dir = root.join("fuzz/artifacts").join(target);
        let before = artifact_names(&dir);
        Self { target, dir, before }
    }

    /// The error for a failed run: [`describe_failure`] with the files that appeared since.
    fn failure(&self, error: &str) -> anyhow::Error {
        let new: Vec<String> = artifact_names(&self.dir).difference(&self.before).cloned().collect();
        anyhow::anyhow!(describe_failure(self.target, &new, error))
    }
}

/// `[target]`, or every target when none was named.
fn targets(target: Option<&str>) -> Vec<&str> {
    target.map_or_else(|| FUZZ_TARGETS.to_vec(), |t| vec![t])
}

/// Checks cargo-fuzz once and returns the nightly toolchain's host triple. cargo-fuzz
/// defaults to the triple it was built for, which for prebuilt binaries is musl (no
/// sanitizer support), so every run names the host explicitly.
fn fuzz_setup(ws: &Workspace) -> Result<String> {
    crate::doctor::require(ws, &crate::doctor::cargo_fuzz())?;
    nightly_host()
}

/// Builds `target`'s seed corpus and fuzzes it for `seconds`. A failure that left new files
/// in `fuzz/artifacts/<target>/` is a crash; any other is a build or setup failure.
fn fuzz_one(ws: &Workspace, host: &str, target: &str, seconds: u64) -> Result<()> {
    let corpus = seeds(ws, target)?;
    let watch = ArtifactWatch::new(&ws.root, target);
    run(
        &ws.root,
        "cargo",
        &fuzz_run_args(host, target, &corpus.to_string_lossy(), seconds, 1),
    )
    .map_err(|e| watch.failure(&format!("{e:#}")))
}

/// How a failed fuzz run is reported: a crash with its saved inputs, or a run that failed
/// without saving any (the build, the toolchain or the corpus is the problem, not the code).
fn describe_failure(target: &str, new_artifacts: &[String], error: &str) -> String {
    if new_artifacts.is_empty() {
        format!("{target}: failed without a crash input (build or setup problem): {error}")
    } else {
        format!(
            "{target}: crashed; inputs saved in fuzz/artifacts/{target}/: {}",
            new_artifacts.join(", ")
        )
    }
}

/// The file names in `dir` (empty when it doesn't exist).
fn artifact_names(dir: &Path) -> std::collections::BTreeSet<String> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// `cargo fuzz cmin`: shrinks `fuzz/corpus/<target>/` (seeds included) to the inputs that
/// keep its coverage, so a cached corpus stays bounded.
fn cmin(ws: &Workspace, host: &str, target: &str) -> Result<()> {
    let corpus = seeds(ws, target)?;
    let corpus = corpus.to_string_lossy().into_owned();
    run(
        &ws.root,
        "cargo",
        &["+nightly", "fuzz", "cmin", "--target", host, target, &corpus],
    )
}

/// The host triple of the nightly toolchain (`rustc +nightly -vV`).
pub(crate) fn nightly_host() -> Result<String> {
    let output = Command::new("rustc")
        .args(["+nightly", "-vV"])
        .output()
        .context("running `rustc +nightly -vV`; is the nightly toolchain installed?")?;
    if !output.status.success() {
        bail!("`rustc +nightly -vV` failed: run `rustup toolchain install nightly --profile minimal`");
    }
    host_triple(&String::from_utf8_lossy(&output.stdout)).context("no `host:` line in `rustc +nightly -vV`")
}

fn host_triple(version: &str) -> Option<String> {
    version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(|h| h.trim().to_owned())
}

/// The parameter bytes appended to every Flate stream seed: the `flate_decode` target reads
/// its predictor parameters from the input's last four bytes (predictor 1: none).
/// Must match `FLATE_SEED_PARAMS` in `crates/laminapdf-model/tests/common/fuzz_entry.rs`.
const FLATE_SEED_PARAMS: [u8; 4] = [1, 1, 8, 1];

/// The control bytes every `save_roundtrip` seed starts with, before a fixture PDF.
/// Must match `SAVE_SEED_CONTROL` in `crates/laminapdf-save/tests/common/fuzz_entry.rs`.
const SAVE_SEED_CONTROL: [u8; 8] = [0, 0x14, 0x41, 0x02, 0x03, 0x24, 0x34, 0x04];

/// Builds `fuzz/corpus/<target>/` (gitignored) from the shared fixture PDFs (`corpus/`,
/// `golden/` and `content/`), the target's
/// saved crash inputs in `fixtures/regressions/<target>/` and everything under
/// `fuzz/seeds/<target>/`. Fixture PDFs are split into their streams for `flate_decode`,
/// follow [`SAVE_SEED_CONTROL`] for `save_roundtrip` (which also gets `fixtures/signed/`), and
/// `ops_json` starts from the op samples in `fixtures/ops/v1/` instead;
/// crash inputs and hand-written seeds are copied verbatim (they already are what the target
/// reads). Seed names carry their source path, so files from different folders never
/// collide. Returns the corpus directory.
fn seeds(ws: &Workspace, target: &str) -> Result<PathBuf> {
    let corpus = ws.root.join("fuzz/corpus").join(target);
    fs::create_dir_all(&corpus).with_context(|| format!("creating {}", corpus.display()))?;
    let mut fixtures: Vec<PathBuf> = Vec::new();
    if target == "ops_json" {
        // Op logs, not PDFs: the frozen samples of the op format.
        fixtures.extend(files_in(&ws.root.join("fixtures/ops/v1"))?);
    } else {
        for dir in ["fixtures/corpus", "fixtures/golden", "fixtures/content"] {
            fixtures.extend(files_in(&ws.root.join(dir))?.into_iter().filter(|p| is_pdf(p)));
        }
    }
    if target == "save_roundtrip" {
        let signed = files_in(&ws.root.join("fixtures/signed"))?;
        fixtures.extend(signed.into_iter().filter(|p| is_pdf(p)));
    }
    let mut verbatim: Vec<PathBuf> = Vec::new();
    for dir in [
        ws.root.join("fixtures/regressions").join(target),
        ws.root.join("fuzz/seeds").join(target),
    ] {
        if dir.is_dir() {
            collect_tree(&dir, &mut verbatim)?;
        }
    }
    let mut written = 0usize;
    let mut write = |source: &Path, i: usize, seed: &[u8]| -> Result<()> {
        fs::write(corpus.join(seed_name(&ws.root, source, i)), seed)?;
        written += 1;
        Ok(())
    };
    for source in &fixtures {
        let bytes = fs::read(source).with_context(|| format!("reading {}", source.display()))?;
        if target == "flate_decode" {
            for (i, mut stream) in stream_bodies(&bytes).into_iter().enumerate() {
                stream.extend(FLATE_SEED_PARAMS);
                write(source, i, &stream)?;
            }
        } else if target == "save_roundtrip" {
            write(source, 0, &[&SAVE_SEED_CONTROL[..], &bytes].concat())?;
        } else {
            write(source, 0, &bytes)?;
        }
    }
    for source in &verbatim {
        let bytes = fs::read(source).with_context(|| format!("reading {}", source.display()))?;
        write(source, 0, &bytes)?;
    }
    step(&format!("{target}: {written} seeds in {}", corpus.display()));
    Ok(corpus)
}

/// `seed-<path from the repository root, / as _>-<i>`.
fn seed_name(root: &Path, source: &Path, i: usize) -> String {
    let relative = source.strip_prefix(root).unwrap_or(source);
    let flat: String = relative
        .to_string_lossy()
        .chars()
        .map(|c| if c == '/' || c == '\\' { '_' } else { c })
        .collect();
    format!("seed-{flat}-{i}")
}

fn is_pdf(path: &Path) -> bool {
    path.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf"))
}

/// Every file under `dir` (recursively) except `passwords.txt`, which the `open` target
/// compiles in rather than reads as a seed.
fn collect_tree(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_tree(&path, out)?;
        } else if path.file_name().is_some_and(|n| n != "passwords.txt") {
            out.push(path);
        }
    }
    Ok(())
}

/// The raw data of every stream in `pdf` (between `stream` + EOL and `endstream`): Flate
/// seeds for `flate_decode`. Files without streams are used whole.
fn stream_bodies(pdf: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut rest = pdf;
    while let Some(after) = find(rest, b"stream").and_then(|start| rest.get(start + b"stream".len()..)) {
        let body = after
            .strip_prefix(b"\r\n")
            .or_else(|| after.strip_prefix(b"\n"))
            .unwrap_or(after);
        let Some(end) = find(body, b"endstream") else { break };
        out.extend(body.get(..end).map(<[u8]>::to_vec));
        rest = body.get(end + b"endstream".len()..).unwrap_or_default();
    }
    if out.is_empty() {
        out.push(pdf.to_vec());
    }
    out
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The root manifest must exclude `fuzz/` (its own nightly workspace, ADR 0015/0018), and
/// `fuzz/Cargo.toml` must declare its own `[workspace]`.
pub(crate) fn check_fuzz_excluded(ws: &Workspace) -> Result<()> {
    let root: toml::Table = fs::read_to_string(ws.root.join("Cargo.toml"))?.parse()?;
    let excluded = root
        .get("workspace")
        .and_then(|w| w.get("exclude"))
        .and_then(toml::Value::as_array)
        .is_some_and(|ex| ex.iter().any(|e| e.as_str() == Some("fuzz")));
    if !excluded {
        bail!("Cargo.toml: [workspace] exclude must list \"fuzz\" (fuzz/ is its own nightly workspace)");
    }
    let fuzz = ws.root.join("fuzz/Cargo.toml");
    if fuzz.exists() {
        let manifest: toml::Table = fs::read_to_string(&fuzz)?.parse()?;
        if !manifest.contains_key("workspace") {
            bail!("fuzz/Cargo.toml must declare its own [workspace]");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The array literal after `const <name>` in `source`, as numbers.
    fn const_bytes(source: &str, name: &str) -> Vec<u8> {
        let after = source.split(&format!("const {name}")).nth(1).unwrap();
        let list = after.split('=').nth(1).unwrap();
        let list = list.split(']').next().unwrap().trim().trim_start_matches('[');
        list.split(',')
            .map(|n| {
                let n = n.trim();
                n.strip_prefix("0x")
                    .map_or_else(|| n.parse().unwrap(), |h| u8::from_str_radix(h, 16).unwrap())
            })
            .collect()
    }

    #[test]
    fn the_save_seed_control_bytes_match_the_fuzz_entry() {
        let entry = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/laminapdf-save/tests/common/fuzz_entry.rs"),
        )
        .unwrap();
        assert_eq!(const_bytes(&entry, "SAVE_SEED_CONTROL"), SAVE_SEED_CONTROL);
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn fuzz_args_default_to_one_minute() {
        let a = args(&["lex"]);
        assert_eq!(
            parse_fuzz_args(&a).unwrap(),
            FuzzTask::Run {
                target: "lex",
                seconds: 60
            }
        );
    }

    #[test]
    fn fuzz_args_read_the_time() {
        let a = args(&["open", "--time", "600"]);
        assert_eq!(
            parse_fuzz_args(&a).unwrap(),
            FuzzTask::Run {
                target: "open",
                seconds: 600
            }
        );
    }

    #[test]
    fn fuzz_all_reads_the_time() {
        let a = args(&["--all", "--time", "120"]);
        assert_eq!(parse_fuzz_args(&a).unwrap(), FuzzTask::RunAll { seconds: 120, jobs: 1 });
    }

    #[test]
    fn fuzz_all_reads_jobs_in_any_order() {
        let a = args(&["--all", "--jobs", "4", "--time", "30"]);
        assert_eq!(parse_fuzz_args(&a).unwrap(), FuzzTask::RunAll { seconds: 30, jobs: 4 });
    }

    #[test]
    fn fuzz_all_rejects_zero_jobs() {
        assert!(parse_fuzz_args(&args(&["--all", "--jobs", "0"])).is_err());
    }

    #[test]
    fn fuzz_all_rejects_more_jobs_than_the_memory_bound() {
        assert!(parse_fuzz_args(&args(&["--all", "--jobs", "9"])).is_err());
    }

    #[test]
    fn one_job_uses_the_tmin_limits() {
        assert_eq!(libfuzzer_limits(1), crate::nightly::LIBFUZZER_LIMITS);
    }

    #[test]
    fn parallel_jobs_scale_the_timeout_not_the_memory_limit() {
        assert_eq!(libfuzzer_limits(4), ["-timeout=40", "-rss_limit_mb=2048"]);
    }

    #[test]
    fn fuzz_run_args_end_with_the_limits() {
        let a = fuzz_run_args("aarch64-apple-darwin", "lex", "fuzz/corpus/lex", 30, 4);
        assert_eq!(
            a[7..],
            ["--", "-max_total_time=30", "-timeout=40", "-rss_limit_mb=2048"]
        );
    }

    #[test]
    fn fuzz_build_args_name_one_target_or_none() {
        assert_eq!(
            fuzz_build_args("h", None),
            ["+nightly", "fuzz", "build", "--target", "h"]
        );
        assert_eq!(fuzz_build_args("h", Some("lex")).last(), Some(&"lex"));
    }

    #[test]
    fn prefixed_marks_every_line() {
        assert_eq!(prefixed("lex", "a\nb\n"), "[lex] a\n[lex] b\n");
    }

    fn artifact_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("xtask-fuzz-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("fuzz/artifacts/lex")).unwrap();
        root
    }

    #[test]
    fn a_failure_with_new_inputs_is_a_crash() {
        let root = artifact_dir("crash");
        fs::write(root.join("fuzz/artifacts/lex/old"), b"").unwrap();
        let watch = ArtifactWatch::new(&root, "lex");
        fs::write(root.join("fuzz/artifacts/lex/crash-1"), b"").unwrap();
        let message = watch.failure("exit 1").to_string();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(message, "lex: crashed; inputs saved in fuzz/artifacts/lex/: crash-1");
    }

    #[test]
    fn a_failure_without_new_inputs_is_a_setup_problem() {
        let root = artifact_dir("setup");
        fs::write(root.join("fuzz/artifacts/lex/old"), b"").unwrap();
        let watch = ArtifactWatch::new(&root, "lex");
        let message = watch.failure("exit 1").to_string();
        fs::remove_dir_all(&root).unwrap();
        assert!(message.contains("build or setup problem"), "{message}");
    }

    #[test]
    fn fuzz_all_rejects_a_repeated_flag() {
        assert!(parse_fuzz_args(&args(&["--all", "--time", "1", "--time", "2"])).is_err());
    }

    #[test]
    fn fuzz_jobs_needs_all() {
        assert!(parse_fuzz_args(&args(&["lex", "--jobs", "2"])).is_err());
    }

    #[test]
    fn differential_args_default_to_the_fast_set() {
        assert_eq!(
            parse_differential_args(&[]).unwrap(),
            Differential {
                full: false,
                update: false,
                text: false
            }
        );
    }

    #[test]
    fn differential_args_take_flags_in_any_order() {
        let a = args(&["--update", "--full"]);
        assert_eq!(
            parse_differential_args(&a).unwrap(),
            Differential {
                full: true,
                update: true,
                text: false
            }
        );
    }

    #[test]
    fn differential_args_take_text_but_not_with_full() {
        let a = args(&["--text", "--update"]);
        assert_eq!(
            parse_differential_args(&a).unwrap(),
            Differential {
                full: false,
                update: true,
                text: true
            }
        );
        assert!(parse_differential_args(&args(&["--text", "--full"])).is_err());
    }

    #[test]
    fn differential_args_reject_a_count() {
        assert!(parse_differential_args(&args(&["--mutants", "500"])).is_err());
    }

    #[test]
    fn the_full_set_runs_under_nextest_and_fails_when_no_test_matches() {
        let a = full_set_test_args();
        let has = |pair: [&str; 2]| a.windows(2).any(|w| w == pair);
        assert!(
            a.first().map(String::as_str) == Some("nextest")
                && has(["--no-tests", "fail"])
                && has(["--profile", "ci"])
                && has(["--run-ignored", "only"]),
            "{a:?}"
        );
    }

    #[test]
    fn the_full_set_filter_names_the_test_exactly() {
        assert_eq!(
            full_set_test_args().last().map(String::as_str),
            Some("test(=full_set_agrees_with_pypdf)")
        );
    }

    #[test]
    fn differential_args_reject_repeated_flags() {
        assert!(parse_differential_args(&args(&["--update", "--update"])).is_err());
    }

    #[test]
    fn fuzz_cmin_takes_an_optional_target() {
        assert_eq!(parse_fuzz_args(&args(&["cmin"])).unwrap(), FuzzTask::Cmin(None));
    }

    #[test]
    fn a_failure_with_new_artifacts_is_a_crash() {
        let message = describe_failure("lex", &["crash-1a2b".to_owned()], "exit 77");
        assert!(
            message.contains("crashed") && message.contains("crash-1a2b"),
            "{message}"
        );
    }

    #[test]
    fn a_failure_without_artifacts_is_not_called_a_crash() {
        let message = describe_failure("lex", &[], "could not compile");
        assert!(
            !message.contains("crashed") && message.contains("could not compile"),
            "{message}"
        );
    }

    #[test]
    fn known_fuzz_target_should_reject_subcommand_names() {
        assert!(known_fuzz_target("seeds").is_err() && known_fuzz_target("open").is_ok());
    }

    #[test]
    fn fuzz_args_reject_unknown_targets() {
        assert!(parse_fuzz_args(&args(&["nope"])).is_err());
    }

    #[test]
    fn fuzz_seeds_take_an_optional_target() {
        let a = args(&["seeds", "page_tree"]);
        assert_eq!(parse_fuzz_args(&a).unwrap(), FuzzTask::Seeds(Some("page_tree")));
    }

    #[test]
    fn host_triple_reads_the_host_line() {
        let out = "rustc 1.99.0-nightly\nbinary: rustc\nhost: x86_64-unknown-linux-gnu\nrelease: 1.99.0\n";
        assert_eq!(host_triple(out).as_deref(), Some("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn seed_names_keep_their_source_folder() {
        let root = Path::new("/r");
        let a = seed_name(root, Path::new("/r/fixtures/corpus/x.pdf"), 0);
        let b = seed_name(root, Path::new("/r/fuzz/seeds/open/x.pdf"), 0);
        assert_eq!(
            (a.as_str(), b.as_str()),
            ("seed-fixtures_corpus_x.pdf-0", "seed-fuzz_seeds_open_x.pdf-0")
        );
    }

    #[test]
    fn stream_bodies_extract_every_stream() {
        let pdf = b"1 0 obj << >> stream\nabc\nendstream endobj 2 0 obj << >> stream\r\nxyz endstream";
        assert_eq!(stream_bodies(pdf), [b"abc\n".to_vec(), b"xyz ".to_vec()]);
    }

    #[test]
    fn stream_bodies_fall_back_to_the_whole_file() {
        assert_eq!(stream_bodies(b"no streams"), [b"no streams".to_vec()]);
    }
}
