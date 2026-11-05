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
    check_lock(ws, dataset, &cache, &wanted, &lock, args.update_lock)?;

    let candidate = if args.stream { "laminapdf-stream" } else { "laminapdf" };
    let out = ws.root.join("target/bench/extract").join(&dataset.name);
    let md_dir = out.join(candidate);
    if md_dir.exists() {
        fs::remove_dir_all(&md_dir).with_context(|| format!("removing {}", md_dir.display()))?;
    }
    let bench = out.join("bench_data");
    fs::create_dir_all(&bench)?;
    let mut list = String::new();
    for (subset, rows) in &tests {
        let mut jsonl = String::new();
        for row in rows {
            jsonl.push_str(&row.line);
            jsonl.push('\n');
        }
        fs::write(bench.join(format!("{subset}.jsonl")), jsonl)?;
        let pdfs: BTreeSet<&str> = rows.iter().map(|r| r.pdf.as_str()).collect();
        for pdf in pdfs {
            let stem = pdf.strip_suffix(".pdf").unwrap_or(pdf);
            let source = cache.join("bench_data/pdfs").join(pdf);
            let target = md_dir.join(format!("{stem}_pg1_repeat1.md"));
            let _ = writeln!(list, "{}\t{}", source.display(), target.display());
        }
    }
    let list_path = out.join(format!("{candidate}.tsv"));
    fs::write(&list_path, list)?;
    let summary = out.join(format!("{candidate}-extract.json"));
    let list_arg = list_path.to_string_lossy().into_owned();
    let summary_arg = summary.to_string_lossy().into_owned();
    let mut cargo = vec![
        "run",
        "--locked",
        "--release",
        "-q",
        "-p",
        "laminapdf",
        "--example",
        "extract-bench",
        "--",
        &list_arg,
        &summary_arg,
    ];
    if args.stream {
        cargo.extend(["--tables", "lattice-and-stream"]);
    }
    crate::run_env(&ws.root, "cargo", &cargo, &[])?;

    let report = out.join(format!("{candidate}-score.json"));
    let python = crate::harness::python();
    let scorer = ws.root.join(&dataset.scorer).to_string_lossy().into_owned();
    let mut score_args = vec![
        "-I".to_owned(),
        scorer,
        bench.to_string_lossy().into_owned(),
        md_dir.to_string_lossy().into_owned(),
        report.to_string_lossy().into_owned(),
    ];
    score_args.extend(tests.keys().cloned());
    crate::run_env(&ws.root, &python, &score_args, &[]).with_context(|| {
        format!("the scorer needs Python with {REQUIREMENTS}: `{python} -m pip install -r {REQUIREMENTS}`")
    })?;

    let extracted: Value = serde_json::from_str(&fs::read_to_string(&summary)?)?;
    let score_json: Value = serde_json::from_str(&fs::read_to_string(&report)?)?;
    let md = render_report(dataset, &subsets, candidate, args, &extracted, &score_json);
    fs::write(out.join(format!("{candidate}-report.md")), &md)?;
    step(&format!("extract-bench: {} ({candidate})", dataset.title));
    println!("{md}");
    Ok(())
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

/// Checks every fetched file against the lock, or (`update`) records the hashes in it.
fn check_lock(
    ws: &Workspace,
    dataset: &Dataset,
    cache: &Path,
    wanted: &BTreeSet<String>,
    lock: &BTreeMap<String, String>,
    update: bool,
) -> Result<()> {
    let mut hashes = lock.clone();
    let mut wrong = Vec::new();
    for path in wanted {
        let bytes = fs::read(cache.join(path)).with_context(|| format!("reading {path}"))?;
        let hash = sha256_hex(&bytes);
        match lock.get(path) {
            Some(want) if *want == hash => {}
            _ if update => {
                hashes.insert(path.clone(), hash);
            }
            Some(_) => wrong.push(format!("{path}: SHA-256 differs from the lock")),
            None => wrong.push(format!("{path}: not in the lock")),
        }
    }
    if update {
        let mut text = format!(
            "# SHA-256 of every {} file `cargo xtask extract-bench` reads, at revision {}.\n# Written by `cargo xtask extract-bench --update-lock`.\n",
            dataset.title, dataset.revision
        );
        for (path, hash) in &hashes {
            let _ = writeln!(text, "{hash}  {path}");
        }
        fs::write(ws.root.join(&dataset.lock), text)?;
        return Ok(());
    }
    if !wrong.is_empty() {
        bail!(
            "{} files don't match {} (run with --update-lock after a deliberate change):\n  {}",
            wrong.len(),
            dataset.lock,
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
) -> String {
    let files = at(extracted, "files").as_object();
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
        let pdfs = files.map_or(0, |f| {
            f.keys()
                .filter(|k| k.contains(&format!("/{}/", pdf_dir(&subset.name))))
                .count()
        });
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
    let _ = write!(
        md,
        "\nOverall (the mean of the subsets' scores, as the benchmark reports it): {:.1} %.\n\
         PDFs extracted: {}, failed: {}, panicked: {}; extraction time {}.{} s in all.\n\nData: {}.\n",
        at(scores, "overall").as_f64().unwrap_or(0.0) * 100.0,
        count("extracted"),
        count("failed"),
        count("panic"),
        millis / 1000,
        millis % 1000 / 100,
        dataset.attribution
    );
    md
}

/// `value[key]`, or `null` when `value` has no such key.
fn at<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

/// The folder of a subset's PDFs (`table_tests` keeps its PDFs in `tables/`).
fn pdf_dir(subset: &str) -> &str {
    if subset == "table_tests" { "tables" } else { subset }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "test code")]

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
