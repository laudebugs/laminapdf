//! The test-harness tasks of ADR 0018: goldens (tier 3), corpus facts (tier 4), the pypdf
//! differential (tier 6) and the fuzz targets (tier 9).
//!
//! Python is needed only to regenerate facts (`corpus facts`, `differential --update`): it is
//! `$LAMINA_PYTHON` or `python3`, with `tools/corpus-facts/requirements.txt` installed. The
//! fast lane only reads the precomputed files.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;
use crate::{run, step};

/// The fuzz targets in `fuzz/fuzz_targets/` (ADR 0018 tier 9).
pub(crate) const FUZZ_TARGETS: [&str; 5] = ["lex", "parse_object", "flate_decode", "open", "page_tree"];

/// Default `cargo xtask fuzz` run time in seconds (the scheduled workflow uses 600).
const DEFAULT_FUZZ_SECONDS: u64 = 60;

fn python() -> String {
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

/// `cargo xtask golden [--update]`: regenerates every golden in `fixtures/golden/` from its
/// source (`--update` writes them in place and prints the list); without `--update`, checks
/// that regenerating them changes nothing.
pub(crate) fn golden(ws: &Workspace, update: bool) -> Result<()> {
    let golden_dir = ws.root.join("fixtures/golden");
    let out = if update {
        golden_dir.clone()
    } else {
        scratch(ws, "golden")?
    };
    let out_arg = out.to_string_lossy().into_owned();
    ws.cargo(&[
        "run",
        "--locked",
        "-q",
        "-p",
        "lamina-cos",
        "--example",
        "golden",
        "--",
        &out_arg,
    ])?;
    if update {
        step("Goldens regenerated; review the diff (`git diff fixtures/golden`) like code");
        return Ok(());
    }
    let mut stale = Vec::new();
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
    if !stale.is_empty() {
        bail!(
            "goldens differ from their sources: {}; run `cargo xtask golden --update`",
            stale.join(", ")
        );
    }
    step("Goldens match their sources");
    Ok(())
}

/// `cargo xtask corpus facts`: rewrites `fixtures/corpus/expected.json` with pypdf.
pub(crate) fn corpus_facts(ws: &Workspace) -> Result<()> {
    let script = ws.root.join("tools/corpus-facts/facts.py");
    run(&ws.root, &python(), &[script.as_os_str(), OsStr::new("corpus")])
}

/// `cargo xtask differential [--update]`: compares lamina with pypdf's precomputed facts
/// (no Python); `--update` regenerates `fixtures/differential/expected.json` with pypdf.
pub(crate) fn differential(ws: &Workspace, update: bool) -> Result<()> {
    if update {
        let inputs = scratch(ws, "differential")?;
        let inputs_arg = inputs.to_string_lossy().into_owned();
        ws.cargo(&[
            "run",
            "--locked",
            "-q",
            "-p",
            "lamina-model",
            "--example",
            "differential-inputs",
            "--",
            &inputs_arg,
        ])?;
        let script = ws.root.join("tools/corpus-facts/facts.py");
        let expected = ws.root.join("fixtures/differential/expected.json");
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
    ws.cargo(&["test", "--locked", "-q", "-p", "lamina-model", "--test", "differential"])
}

/// What `cargo xtask fuzz` was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FuzzTask<'a> {
    /// Run one target for `seconds`.
    Run { target: &'a str, seconds: u64 },
    /// Only build the seed corpora (all targets, or one).
    Seeds(Option<&'a str>),
}

/// Parses `fuzz <target> [--time N]` and `fuzz seeds [<target>]`.
pub(crate) fn parse_fuzz_args(args: &[String]) -> Result<FuzzTask<'_>> {
    let known = |t: &str| {
        if FUZZ_TARGETS.contains(&t) {
            Ok(())
        } else {
            bail!("unknown fuzz target {t:?}; targets: {}", FUZZ_TARGETS.join(", "))
        }
    };
    match args {
        [cmd] if cmd == "seeds" => Ok(FuzzTask::Seeds(None)),
        [cmd, target] if cmd == "seeds" => {
            known(target)?;
            Ok(FuzzTask::Seeds(Some(target)))
        }
        [target] => {
            known(target)?;
            Ok(FuzzTask::Run {
                target,
                seconds: DEFAULT_FUZZ_SECONDS,
            })
        }
        [target, flag, seconds] if flag == "--time" => {
            known(target)?;
            let seconds = seconds
                .parse()
                .with_context(|| format!("--time {seconds:?}: not a number"))?;
            Ok(FuzzTask::Run { target, seconds })
        }
        _ => bail!("usage: cargo xtask fuzz <target> [--time <seconds>] | fuzz seeds [<target>]"),
    }
}

/// `cargo xtask fuzz …`.
pub(crate) fn fuzz(ws: &Workspace, task: &FuzzTask<'_>) -> Result<()> {
    match *task {
        FuzzTask::Seeds(Some(target)) => seeds(ws, target).map(drop),
        FuzzTask::Seeds(None) => FUZZ_TARGETS.iter().try_for_each(|t| seeds(ws, t).map(drop)),
        FuzzTask::Run { target, seconds } => {
            crate::doctor::require(ws, &crate::doctor::cargo_fuzz())?;
            let corpus = seeds(ws, target)?;
            let corpus = corpus.to_string_lossy().into_owned();
            let max_time = format!("-max_total_time={seconds}");
            // fuzz/ is its own nightly workspace (ADR 0018); `+nightly` overrides the root pin.
            run(
                &ws.root,
                "cargo",
                &[
                    "+nightly",
                    "fuzz",
                    "run",
                    target,
                    &corpus,
                    "--",
                    &max_time,
                    "-timeout=10",
                    "-rss_limit_mb=2048",
                ],
            )
        }
    }
}

/// Builds `fuzz/corpus/<target>/` (gitignored) from the shared fixtures, the target's
/// regressions in `fixtures/regressions/`, and everything under `fuzz/seeds/<target>/`.
/// Returns the corpus directory.
fn seeds(ws: &Workspace, target: &str) -> Result<PathBuf> {
    let corpus = ws.root.join("fuzz/corpus").join(target);
    fs::create_dir_all(&corpus).with_context(|| format!("creating {}", corpus.display()))?;
    let mut sources: Vec<PathBuf> = Vec::new();
    for dir in ["fixtures/corpus", "fixtures/golden"] {
        sources.extend(files_in(&ws.root.join(dir))?.into_iter().filter(|p| is_pdf(p)));
    }
    let regressions = ws.root.join("fixtures/regressions");
    if regressions.is_dir() {
        let prefix = format!("{target}-");
        sources.extend(
            files_in(&regressions)?
                .into_iter()
                .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&prefix))),
        );
    }
    let own = ws.root.join("fuzz/seeds").join(target);
    if own.is_dir() {
        collect_tree(&own, &mut sources)?;
    }
    let mut written = 0usize;
    for source in &sources {
        let bytes = fs::read(source).with_context(|| format!("reading {}", source.display()))?;
        let seeds = if target == "flate_decode" {
            stream_bodies(&bytes)
        } else {
            vec![bytes]
        };
        for (i, seed) in seeds.into_iter().enumerate() {
            let stem = source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            fs::write(corpus.join(format!("seed-{stem}-{i}")), seed)?;
            written += 1;
        }
    }
    step(&format!("{target}: {written} seeds in {}", corpus.display()));
    Ok(corpus)
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
    fn fuzz_args_reject_unknown_targets() {
        assert!(parse_fuzz_args(&args(&["nope"])).is_err());
    }

    #[test]
    fn fuzz_seeds_take_an_optional_target() {
        let a = args(&["seeds", "page_tree"]);
        assert_eq!(parse_fuzz_args(&a).unwrap(), FuzzTask::Seeds(Some("page_tree")));
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
