//! hayro-parity spike (Round C1a): render every page of the pdf.js and PDFBox
//! test corpora with hayro and with PDFium, compare, and report.
//!
//! ```text
//! cargo run --release -- [--dpi 96] [--jobs N] [--timeout 30] [--corpus pdfjs|pdfbox]
//!                        [--limit N] [--sample K] [--engine both|hayro|pdfium]
//!                        [--out out] [--files a.pdf b.pdf …]
//! ```
//!
//! Each document runs in a child process (`--one`, see `child.rs`) so a crash or
//! a hang is recorded as a row instead of ending the run.

mod child;
mod compare;
mod driver;
mod render;
mod summary;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use driver::{Engines, Job, Options};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str| -> Option<&str> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
    };
    let flag = |name: &str| args.iter().any(|a| a == name);
    let dpi: f32 = value("--dpi").unwrap_or("96").parse().context("--dpi")?;
    let pdfium_lib = value("--pdfium-lib").map_or_else(|| Path::new(ROOT).join("pdfium/lib"), PathBuf::from);

    if let Some(file) = value("--one") {
        return child::run(&child::ChildArgs {
            file: file.into(),
            dpi,
            start: value("--start").unwrap_or("1").parse().context("--start")?,
            hayro: !flag("--no-hayro"),
            pdfium: !flag("--no-pdfium"),
            pdfium_lib,
            fail_dir: value("--fail-dir").map(PathBuf::from),
        });
    }

    let engines = match value("--engine").unwrap_or("both") {
        "both" => Engines::Both,
        "hayro" => Engines::Hayro,
        "pdfium" => Engines::Pdfium,
        other => bail!("--engine {other}: expected both, hayro or pdfium"),
    };
    let default_out = match engines {
        Engines::Both => format!("out/dpi{dpi}"),
        Engines::Hayro => format!("out/dpi{dpi}-hayro-only"),
        Engines::Pdfium => format!("out/dpi{dpi}-pdfium-only"),
    };
    let out = Path::new(ROOT).join(value("--out").unwrap_or(&default_out));
    let jobs_n = value("--jobs")
        .map(str::parse)
        .transpose()
        .context("--jobs")?
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let timeout = Duration::from_secs(value("--timeout").unwrap_or("30").parse().context("--timeout")?);

    let jobs = if flag("--files") {
        let start = args.iter().position(|a| a == "--files").map_or(0, |i| i + 1);
        args[start..]
            .iter()
            .take_while(|a| !a.starts_with("--"))
            .map(|f| Job {
                corpus: "files".into(),
                path: f.into(),
                name: f.clone(),
            })
            .collect()
    } else {
        corpus_jobs(value("--corpus"), value("--limit"), value("--sample"))?
    };
    if jobs.is_empty() {
        bail!("no input files: run scripts/fetch-corpora.sh first, or pass --files");
    }
    if engines != Engines::Hayro && !pdfium_lib.exists() {
        bail!("{} missing: run scripts/fetch-pdfium.sh first", pdfium_lib.display());
    }
    clear_out_dir(&out)?;

    let n = jobs.len();
    eprintln!("{n} documents, {jobs_n} parallel jobs, {dpi} DPI → {}", out.display());
    let started = Instant::now();
    let opts = Options {
        dpi,
        jobs: jobs_n,
        timeout,
        engines,
        pdfium_lib,
        out: out.clone(),
    };
    let (rows, procs) = driver::run_all(jobs, &opts)?;
    let wall = started.elapsed();

    driver::write_csv(&out.join("results.csv"), &rows)?;
    driver::write_rss_csv(&out.join("rss.csv"), &procs)?;
    let header = format!(
        "{n} documents, {} rows, {dpi} DPI, engines: {}, {jobs_n} parallel jobs, per-page timeout {} s, wall time {:.0} s.",
        rows.len(),
        value("--engine").unwrap_or("both"),
        timeout.as_secs(),
        wall.as_secs_f64()
    );
    let md = summary::render(&rows, &procs, &header);
    std::fs::write(out.join("summary.md"), &md)?;
    println!("{md}");
    Ok(())
}

/// Marks a directory as written by this harness, so a later run may delete it.
const OUT_MARKER: &str = ".hayro-parity-out";

/// Empties `out` for a new run. Deletes it only when it carries the marker this
/// harness writes (so `--out src` or `--out results` can't wipe tracked files);
/// a non-empty directory without the marker is an error.
fn clear_out_dir(out: &Path) -> Result<()> {
    if out.exists() {
        if out.join(OUT_MARKER).is_file() {
            std::fs::remove_dir_all(out)?;
        } else if std::fs::read_dir(out)?.next().is_some() {
            bail!(
                "{} exists and was not written by this harness (no {OUT_MARKER}); choose another --out",
                out.display()
            );
        }
    }
    std::fs::create_dir_all(out)?;
    std::fs::write(out.join(OUT_MARKER), b"")?;
    Ok(())
}

/// Lists the corpus files from `corpora.manifest.toml` (written by the fetch script).
fn corpus_jobs(only: Option<&str>, limit: Option<&str>, sample: Option<&str>) -> Result<Vec<Job>> {
    let manifest = std::fs::read_to_string(Path::new(ROOT).join("corpora.manifest.toml"))
        .context("corpora.manifest.toml: run scripts/fetch-corpora.sh first")?;
    let limit: usize = limit
        .map(str::parse)
        .transpose()
        .context("--limit")?
        .unwrap_or(usize::MAX);
    let sample: usize = sample
        .map(str::parse)
        .transpose()
        .context("--sample")?
        .unwrap_or(1)
        .max(1);
    let mut jobs = Vec::new();
    let mut corpus = String::new();
    let mut taken = 0;
    let mut seen = 0;
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            corpus = name.to_owned();
            (taken, seen) = (0, 0);
            continue;
        }
        let Some(path) = line
            .strip_prefix("{ path = \"")
            .and_then(|l| l.split_once('"'))
            .map(|(p, _)| p)
        else {
            continue;
        };
        if only.is_some_and(|o| o != corpus) {
            continue;
        }
        seen += 1;
        if (seen - 1) % sample != 0 || taken >= limit {
            continue;
        }
        taken += 1;
        let full = Path::new(ROOT).join("corpora").join(&corpus).join(path);
        jobs.push(Job {
            corpus: corpus.clone(),
            path: full,
            name: path.to_owned(),
        });
    }
    Ok(jobs)
}
