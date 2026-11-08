//! `cargo xtask extract-bench`: Lamina PDF's extraction scored on a pinned public benchmark (todo L3f).
//!
//! `tools/bench/extract-datasets.toml` names each dataset, its revision, licence and subsets;
//! its `lock` holds every file's SHA-256. The command fetches what is missing into
//! `target/bench/datasets/<name>/<revision>/` (`curl`, from the dataset's `resolve/<revision>/`
//! URLs), checks every file against the lock, writes Lamina PDF's Markdown for each PDF with the
//! `extract-bench` example, scores it with the dataset's scorer (the benchmark's own tests, in
//! Python: `tools/bench/requirements.txt`, `$LAMINA_PDF_PYTHON` or `python3`) and writes
//! `target/bench/extract/<name>/report.json` and `report.md`. It never runs in the PR lanes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::runner;
use crate::step;
use crate::util::sha256_hex;
use crate::workspace::Workspace;

const MANIFEST: &str = "tools/bench/extract-datasets.toml";
const REQUIREMENTS: &str = "tools/bench/requirements.txt";

/// The manifest.
#[derive(Debug, Deserialize)]
struct Manifest {
    dataset: Vec<Dataset>,
}

/// One pinned dataset.
#[derive(Debug, Deserialize)]
struct Dataset {
    name: String,
    title: String,
    url: String,
    revision: String,
    licence: String,
    attribution: String,
    scorer: String,
    lock: String,
    subset: Vec<Subset>,
}

/// A subset scored, and what it measures.
#[derive(Debug, Deserialize)]
struct Subset {
    name: String,
    measures: String,
}

/// What `cargo xtask extract-bench` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    /// Only these subsets (all when empty).
    subsets: Vec<String>,
    /// The first N PDFs of each subset, in name order.
    sample: Option<usize>,
    /// Also look for tables without lines.
    stream: bool,
    /// Rewrite the lock from the files fetched.
    update_lock: bool,
}

/// Parses `[--subset <name>]… [--sample N] [--tables lattice-and-stream] [--update-lock]`.
pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut out = Args::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--subset" => out.subsets.push(rest.next().context("--subset needs a name")?.clone()),
            "--sample" => {
                let n = rest.next().context("--sample needs a number")?;
                out.sample = Some(n.parse().with_context(|| format!("--sample {n}: not a number"))?);
            }
            "--tables" => match rest.next().map(String::as_str) {
                Some("lattice-and-stream") => out.stream = true,
                Some("lattice") => out.stream = false,
                other => bail!("--tables takes lattice or lattice-and-stream, not {other:?}"),
            },
            "--update-lock" => out.update_lock = true,
            other => bail!("extract-bench: unknown argument {other}"),
        }
    }
    Ok(out)
}

/// `cargo xtask extract-bench`.
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let text = fs::read_to_string(ws.root.join(MANIFEST)).with_context(|| format!("reading {MANIFEST}"))?;
    let manifest: Manifest = toml::from_str(&text).with_context(|| format!("parsing {MANIFEST}"))?;
    for dataset in &manifest.dataset {
        run_dataset(ws, dataset, args)?;
    }
    Ok(())
}

fn run_dataset(ws: &Workspace, dataset: &Dataset, args: &Args) -> Result<()> {
    let subsets: Vec<&Subset> = dataset
        .subset
        .iter()
        .filter(|s| args.subsets.is_empty() || args.subsets.contains(&s.name))
        .collect();
    if subsets.is_empty() {
        bail!("{}: none of the subsets {:?}", dataset.name, args.subsets);
    }
    if args.update_lock && (!args.subsets.is_empty() || args.sample.is_some()) {
        bail!("--update-lock rewrites the whole lock: run it without --subset and --sample");
    }
    let cache = ws
        .root
        .join("target/bench/datasets")
        .join(&dataset.name)
        .join(&dataset.revision);
    let lock = read_lock(&ws.root.join(&dataset.lock));

    // The tests first: they name the PDFs.
    let mut wanted: BTreeSet<String> = subsets.iter().map(|s| format!("bench_data/{}.jsonl", s.name)).collect();
    fetch(dataset, &cache, &wanted)?;
    let mut tests = BTreeMap::new();
    for subset in &subsets {
        let rows = read_tests(&cache.join(format!("bench_data/{}.jsonl", subset.name)))?;
        let rows = sample(rows, args.sample);
        for pdf in rows.iter().map(|r| r.pdf.clone()) {
            wanted.insert(format!("bench_data/pdfs/{pdf}"));
        }
        tests.insert(subset.name.clone(), rows);
    }
    fetch(dataset, &cache, &wanted)?;
    verify(ws, dataset, &cache, &wanted, &lock, args.update_lock)?;

    let candidate = if args.stream { "laminapdf-stream" } else { "laminapdf" };
    let out = ws.root.join("target/bench/extract").join(&dataset.name);
    let md_dir = out.join(candidate);
    if md_dir.exists() {
        fs::remove_dir_all(&md_dir).with_context(|| format!("removing {}", md_dir.display()))?;
    }
    let bench = out.join("bench_data");
    fs::create_dir_all(&bench)?;
    let mut inputs = Vec::new();
    let mut pdf_counts = BTreeMap::new();
    for (subset, rows) in &tests {
        let mut jsonl = String::new();
        for row in rows {
            jsonl.push_str(&row.line);
            jsonl.push('\n');
        }
        fs::write(bench.join(format!("{subset}.jsonl")), jsonl)?;
        let pdfs: BTreeSet<&str> = rows.iter().map(|r| r.pdf.as_str()).collect();
        pdf_counts.insert(subset.clone(), pdfs.len());
        for pdf in pdfs {
            let stem = pdf.strip_suffix(".pdf").unwrap_or(pdf);
            let target = md_dir.join(format!("{stem}_pg1_repeat1.md"));
            inputs.push((
                target.to_string_lossy().into_owned(),
                cache.join("bench_data/pdfs").join(pdf),
            ));
        }
    }

    let extracted = extract(ws, args.stream, &inputs, &out, candidate)?;
    let report = out.join(format!("{candidate}-score.json"));
    score(ws, dataset, &bench, &md_dir, &report, tests.keys())?;
    let score_json: Value = serde_json::from_str(&fs::read_to_string(&report)?)?;
    let md = render_report(dataset, &subsets, candidate, args, &extracted, &score_json, &pdf_counts);
    fs::write(out.join(format!("{candidate}-report.md")), &md)?;
    step(&format!("extract-bench: {} ({candidate})", dataset.title));
    println!("{md}");
    Ok(())
}

/// Lamina PDF's Markdown for every input (`<out.md>`, `<pdf>`) with the `extract-bench`
/// example, under the differential runner: chunks of files under time limits, and a chunk that
/// crashes or hangs again file by file, so a bad PDF costs its own tests only. Returns the
/// report (`facts`: each file's outcome and time), also written next to the Markdown.
fn extract(
    ws: &Workspace,
    stream: bool,
    inputs: &[(String, std::path::PathBuf)],
    out: &Path,
    candidate: &str,
) -> Result<Value> {
    ws.cargo(&[
        "build",
        "--release",
        "--locked",
        "-q",
        "-p",
        "laminapdf",
        "--example",
        "extract-bench",
    ])?;
    let reader = runner::Reader {
        name: "laminapdf",
        program: crate::live::example_binary(ws, "extract-bench").into_os_string(),
        args: if stream {
            vec!["--tables".into(), "lattice-and-stream".into()]
        } else {
            Vec::new()
        },
        dir: ws.root.clone(),
        reports_repair: false,
    };
    step(&format!(
        "extract-bench: Markdown for {} PDFs ({candidate})",
        inputs.len()
    ));
    let run = runner::run(&reader, inputs, out, runner::Limits::default())?;
    if let Some(why) = run.broken {
        bail!("extract-bench: the example broke: {why}");
    }
    fs::write(
        out.join(format!("{candidate}-extract.json")),
        serde_json::to_string_pretty(&run.report)? + "\n",
    )?;
    Ok(run.report)
}

/// Runs the dataset's scorer over the Markdown in `md_dir` and the tests in `bench`, writing
/// `report`.
fn score<'a>(
    ws: &Workspace,
    dataset: &Dataset,
    bench: &Path,
    md_dir: &Path,
    report: &Path,
    subsets: impl Iterator<Item = &'a String>,
) -> Result<()> {
    let python = crate::harness::python();
    let mut args = vec![
        "-I".to_owned(),
        ws.root.join(&dataset.scorer).to_string_lossy().into_owned(),
        bench.to_string_lossy().into_owned(),
        md_dir.to_string_lossy().into_owned(),
        report.to_string_lossy().into_owned(),
    ];
    args.extend(subsets.cloned());
    crate::run_env(&ws.root, &python, &args, &[]).with_context(|| {
        format!("the scorer needs Python with {REQUIREMENTS}: `{python} -m pip install -r {REQUIREMENTS}`")
    })
}

/// One test of a subset: its PDF and its JSONL line, kept as written.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TestRow {
    pdf: String,
    line: String,
}

fn read_tests(path: &Path) -> Result<Vec<TestRow>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let value: Value =
                serde_json::from_str(line).with_context(|| format!("{}: a line isn't JSON", path.display()))?;
            let pdf = at(&value, "pdf")
                .as_str()
                .with_context(|| format!("{}: a test has no pdf", path.display()))?;
            if at(&value, "page").as_u64() != Some(1) {
                bail!(
                    "{}: a test on a page other than 1; extract-bench writes page 1 only",
                    path.display()
                );
            }
            Ok(TestRow {
                pdf: pdf.to_owned(),
                line: line.to_owned(),
            })
        })
        .collect()
}

/// The tests of the first `n` PDFs in name order (all of them without `n`).
fn sample(rows: Vec<TestRow>, n: Option<usize>) -> Vec<TestRow> {
    let Some(n) = n else { return rows };
    let keep: BTreeSet<String> = rows
        .iter()
        .map(|r| r.pdf.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(n)
        .collect();
    rows.into_iter().filter(|r| keep.contains(&r.pdf)).collect()
}

/// Downloads every file of `wanted` not yet in `cache`.
fn fetch(dataset: &Dataset, cache: &Path, wanted: &BTreeSet<String>) -> Result<()> {
    let missing: Vec<&String> = wanted.iter().filter(|p| !cache.join(p).is_file()).collect();
    if missing.is_empty() {
        return Ok(());
    }
    step(&format!(
        "extract-bench: fetching {} files of {}",
        missing.len(),
        dataset.title
    ));
    for path in missing {
        let target = cache.join(path);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        let url = format!("{}/resolve/{}/{path}", dataset.url, dataset.revision);
        let partial = target.with_extension("part");
        let status = std::process::Command::new("curl")
            .args(["-fsSL", "--retry", "3", "-o"])
            .arg(&partial)
            .arg(&url)
            .status()
            .context("failed to start curl")?;
        if !status.success() {
            bail!("downloading {url} failed with {status}");
        }
        fs::rename(&partial, &target)?;
    }
    Ok(())
}

/// The lock: path → SHA-256 (`<sha256>  <path>` lines, as `sha256sum` writes them).
fn read_lock(path: &Path) -> BTreeMap<String, String> {
    fs::read_to_string(path)
        .map(|text| parse_lock(&text))
        .unwrap_or_default()
}

fn parse_lock(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once("  "))
        .map(|(hash, path)| (path.to_owned(), hash.to_owned()))
        .collect()
}

/// Checks every fetched file against the lock, downloading again a cached file that doesn't
/// match (a partial or older download), or (`update`, a full run only) writes a new lock from
/// the files this run read.
fn verify(
    ws: &Workspace,
    dataset: &Dataset,
    cache: &Path,
    wanted: &BTreeSet<String>,
    lock: &BTreeMap<String, String>,
    update: bool,
) -> Result<()> {
    let hash_of = |path: &str| -> Result<String> {
        Ok(sha256_hex(
            &fs::read(cache.join(path)).with_context(|| format!("reading {path}"))?,
        ))
    };
    if update {
        let mut text = format!(
            "# SHA-256 of every {} file `cargo xtask extract-bench` reads, at revision {}.\n# Written by `cargo xtask extract-bench --update-lock`.\n",
            dataset.title, dataset.revision
        );
        for path in wanted {
            let _ = writeln!(text, "{}  {path}", hash_of(path)?);
        }
        fs::write(ws.root.join(&dataset.lock), text)?;
        return Ok(());
    }
    let mut wrong = Vec::new();
    for path in wanted {
        let Some(want) = lock.get(path) else {
            wrong.push(format!("{path}: not in the lock"));
            continue;
        };
        if hash_of(path)? == *want {
            continue;
        }
        fs::remove_file(cache.join(path))?;
        fetch(dataset, cache, &BTreeSet::from([path.clone()]))?;
        if hash_of(path)? != *want {
            wrong.push(format!("{path}: SHA-256 differs from the lock, downloaded again"));
        }
    }
    if !wrong.is_empty() {
        bail!(
            "{} files don't match {}, which pins revision {}:\n  {}\nIf the dataset's revision changed on purpose, run the full benchmark with --update-lock.",
            wrong.len(),
            dataset.lock,
            dataset.revision,
            wrong.join("\n  ")
        );
    }
    Ok(())
}

/// The Markdown table the run prints and writes.
fn render_report(
    dataset: &Dataset,
    subsets: &[&Subset],
    candidate: &str,
    args: &Args,
    extracted: &Value,
    scores: &Value,
    pdf_counts: &BTreeMap<String, usize>,
) -> String {
    let files = at(extracted, "facts").as_object();
    let count = |outcome: &str| files.map_or(0, |f| f.values().filter(|v| at(v, "outcome") == outcome).count());
    let millis: u64 = files.map_or(0, |f| f.values().filter_map(|v| at(v, "millis").as_u64()).sum());
    let mut md = format!(
        "{} ({}, revision `{}`), Lamina PDF {} ({candidate}{})\n\n| Subset | What it measures | PDFs | Tests | Passed | Score |\n| --- | --- | ---: | ---: | ---: | ---: |\n",
        dataset.title,
        dataset.licence,
        &dataset.revision[..dataset.revision.len().min(12)],
        at(extracted, "version").as_str().unwrap_or("?"),
        args.sample
            .map_or(String::new(), |n| format!(", first {n} PDFs per subset")),
    );
    for subset in subsets {
        let s = &at(at(scores, "subsets"), &subset.name);
        let pdfs = pdf_counts.get(&subset.name).copied().unwrap_or(0);
        let _ = writeln!(
            md,
            "| `{}` | {} | {pdfs} | {} | {} | {:.1} % |",
            subset.name,
            subset.measures,
            at(s, "tests"),
            at(s, "passed"),
            at(s, "score").as_f64().unwrap_or(0.0) * 100.0
        );
    }
    let baseline = at(at(scores, "subsets"), "baseline");
    let _ = writeln!(
        md,
        "| baseline | the page gave text, without long repeats (one test per PDF the benchmark adds) | | {} | {} | {:.1} % |",
        at(baseline, "tests"),
        at(baseline, "passed"),
        at(baseline, "score").as_f64().unwrap_or(0.0) * 100.0
    );
    let _ = write!(
        md,
        "\nOverall (the mean of the groups' scores, the baseline included, as the benchmark reports it): {:.1} %; \
         without the baseline: {:.1} %.\n\
         PDFs extracted: {}, failed: {}, panicked: {}, crashed or timed out: {}; extraction time {}.{} s in all.\n\nData: {}.\n",
        at(scores, "overall").as_f64().unwrap_or(0.0) * 100.0,
        at(scores, "overallWithoutBaseline").as_f64().unwrap_or(0.0) * 100.0,
        count("extracted"),
        count("failed"),
        count("panic"),
        count("crash") + count("timeout"),
        millis / 1000,
        millis % 1000 / 100,
        dataset.attribution
    );
    let _ = writeln!(md, "\nPages by text-layer status: {}.", status_counts(files));
    md
}

/// The pages of every extracted file counted by text-layer status, in the order the statuses
/// are decided: `native 120, mixed 4, …`.
fn status_counts(files: Option<&serde_json::Map<String, Value>>) -> String {
    const STATUSES: [&str; 5] = ["native", "mixed", "needs-ocr", "ocr-layer", "empty"];
    let mut counts = BTreeMap::new();
    for status in files
        .into_iter()
        .flat_map(|f| f.values())
        .filter_map(|v| at(v, "statuses").as_array())
        .flatten()
        .filter_map(Value::as_str)
    {
        *counts.entry(status).or_insert(0_usize) += 1;
    }
    STATUSES
        .iter()
        .map(|s| format!("{s} {}", counts.get(s).copied().unwrap_or(0)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `value[key]`, or `null` when `value` has no such key.
fn at<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "test code")]

    #[test]
    fn status_counts_should_add_up_every_files_pages_in_decision_order() {
        let facts = serde_json::json!({
            "a.md": {"outcome": "extracted", "statuses": ["native", "needs-ocr"]},
            "b.md": {"outcome": "extracted", "statuses": ["needs-ocr"]},
            "c.md": {"outcome": "failed"}
        });
        assert_eq!(
            status_counts(facts.as_object()),
            "native 1, mixed 0, needs-ocr 2, ocr-layer 0, empty 0"
        );
    }

    use super::*;

    fn row(pdf: &str) -> TestRow {
        TestRow {
            pdf: pdf.to_owned(),
            line: format!("{{\"pdf\":\"{pdf}\",\"page\":1}}"),
        }
    }

    #[test]
    fn parse_args_should_read_every_flag() {
        let args: Vec<String> = [
            "--subset",
            "a",
            "--sample",
            "3",
            "--tables",
            "lattice-and-stream",
            "--update-lock",
        ]
        .map(String::from)
        .into();
        let parsed = parse_args(&args).unwrap();
        assert_eq!(
            parsed,
            Args {
                subsets: vec!["a".into()],
                sample: Some(3),
                stream: true,
                update_lock: true
            }
        );
        assert!(parse_args(&["--tables".into(), "stream".into()]).is_err());
    }

    #[test]
    fn sample_should_keep_every_test_of_the_first_pdfs() {
        let rows = vec![row("b.pdf"), row("a.pdf"), row("c.pdf"), row("a.pdf")];
        let kept: Vec<String> = sample(rows, Some(2)).into_iter().map(|r| r.pdf).collect();
        assert_eq!(kept, ["b.pdf", "a.pdf", "a.pdf"]);
    }

    #[test]
    fn parse_lock_should_read_sha256sum_lines_and_skip_comments() {
        let lock = parse_lock("# header\nabc  bench_data/x.jsonl\ndef  bench_data/pdfs/y.pdf\n");
        assert_eq!(lock.get("bench_data/x.jsonl").map(String::as_str), Some("abc"));
        assert_eq!(lock.len(), 2);
    }

    #[test]
    fn the_manifest_should_parse_and_pin_a_full_revision() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = fs::read_to_string(root.join(MANIFEST)).unwrap();
        let manifest: Manifest = toml::from_str(&text).unwrap();
        for d in &manifest.dataset {
            assert_eq!(d.revision.len(), 40, "{}: pin a full commit", d.name);
            assert!(root.join(&d.scorer).is_file(), "{}: no scorer {}", d.name, d.scorer);
            assert!(!d.licence.is_empty() && !d.subset.is_empty());
        }
    }
}
