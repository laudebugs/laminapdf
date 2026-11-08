//! One reader per input of the proof record, each over the file its task writes, under a root
//! (`--from`). Each returns `Ok(None)` when its file is missing and an error when the file is
//! there but unreadable; the caller turns both into a `null` section.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::record::{
    Corpus, CorpusSource, Differential, DifferentialTool, Render, RenderSource, Runtimes, Tally, Text, TextSource,
};
use crate::harness::{FUZZ_SUMMARY_DIR, FUZZ_TARGETS, FuzzRun};
use crate::live;
use crate::pdf_runtimes::{self, RuntimeResults};

/// The length of `report`'s array `key` (0 when it has none).
fn count(report: &Value, key: &str) -> Result<u64> {
    let n = report.get(key).and_then(Value::as_array).map_or(0, Vec::len);
    Ok(u64::try_from(n)?)
}

/// A report's `--source` list under `key`: `None` when the report doesn't say.
fn source_filter(value: &Value, key: &str) -> Option<Vec<String>> {
    let list = value.get(key)?.as_array()?;
    list.iter().map(|s| s.as_str().map(str::to_owned)).collect()
}
use crate::roundtrip;
use crate::verapdf;

/// The structural differential's folder, as `cargo xtask differential --live` writes it.
pub(super) const DIFFERENTIAL: &str = "target/nightly/differential";
/// The text differential's report.
pub(super) const TEXT: &str = "target/nightly/differential-text/report.json";
/// The render diff's report.
pub(super) const RENDER: &str = "target/nightly/render-diff/report.json";

/// The tools whose reports say whether another reader opened a file Lamina PDF refused.
const TOOLS: [&str; 4] = ["pdfjs", "pdfium", "qpdf", "pypdf"];

/// `path` as JSON, `None` when it doesn't exist.
fn json(path: &Path) -> Result<Option<Value>> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .with_context(|| format!("parsing {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// `path` as a `T`, `None` when it doesn't exist.
fn typed<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    json(path)?
        .map(|v| serde_json::from_value(v).with_context(|| format!("reading {}", path.display())))
        .transpose()
}

fn uint(value: &Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .with_context(|| format!("no count `{key}`"))
}

fn text(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("no string `{key}`"))
}

/// A report's `facts` object, by file name.
fn facts(report: &Value) -> Result<&serde_json::Map<String, Value>> {
    report
        .get("facts")
        .and_then(Value::as_object)
        .context("a reader report without `facts`")
}

/// The `outcome` of one file's facts.
fn outcome(facts: &Value) -> &str {
    facts.get("outcome").and_then(Value::as_str).unwrap_or_default()
}

/// Lamina PDF's outcomes per source, from `lamina.json` and the tools' reports beside it.
pub(super) fn corpus(root: &Path) -> Result<Option<Corpus>> {
    let dir = root.join(DIFFERENTIAL);
    let Some(lamina) = json(&dir.join("lamina.json"))? else {
        return Ok(None);
    };
    let mut tools = Vec::new();
    for tool in TOOLS {
        if let Some(report) = json(&dir.join(format!("{tool}.json")))? {
            tools.push(report);
        }
    }
    let opened_by_a_tool = |file: &str| -> Result<bool> {
        for report in &tools {
            if facts(report)?
                .get(file)
                .is_some_and(|f| matches!(outcome(f), "opened" | "repaired"))
            {
                return Ok(true);
            }
        }
        Ok(false)
    };
    let mut sources: BTreeMap<String, CorpusSource> = BTreeMap::new();
    for (file, fact) in facts(&lamina)? {
        let source = sources.entry(live::group(file).to_owned()).or_default();
        source.files += 1;
        match outcome(fact) {
            "opened" => source.opened += 1,
            "repaired" => source.repaired += 1,
            "failed" => {
                source.refused += 1;
                source.refused_alone += u64::from(opened_by_a_tool(file)?);
            }
            "panic" => source.errors.panic += 1,
            "crash" => source.errors.crash += 1,
            "timeout" => source.errors.timeout += 1,
            other => bail!("lamina.json: unknown outcome {other:?}"),
        }
    }
    let report = json(&dir.join("report.json"))?;
    let inputs = report.as_ref().and_then(|r| r.get("inputs")).and_then(Value::as_u64);
    let lamina_broke = report
        .as_ref()
        .and_then(|r| r.get("broken_readers"))
        .and_then(Value::as_array)
        .is_some_and(|b| b.iter().any(|e| e.as_str().is_some_and(|e| e.starts_with("lamina:"))));
    let read: u64 = sources.values().map(|s| s.files).sum();
    Ok(Some(Corpus {
        complete: inputs == Some(read) && !lamina_broke,
        inputs,
        sources,
    }))
}

/// The differential's `report.json`: each tool's tallies and the run's failures.
pub(super) fn differential(root: &Path) -> Result<Option<Differential>> {
    let Some(report) = json(&root.join(DIFFERENTIAL).join("report.json"))? else {
        return Ok(None);
    };
    let mut tools = BTreeMap::new();
    for tool in report.get("tools").and_then(Value::as_array).context("no `tools`")? {
        let mut facts = BTreeMap::new();
        for (fact, t) in tool.get("facts").and_then(Value::as_object).context("no `facts`")? {
            let tally = Tally {
                agree: uint(t, "agree")?,
                disagree: uint(t, "disagree")?,
                excused: uint(t, "excused")?,
            };
            facts.insert(fact.clone(), tally);
        }
        let row = DifferentialTool {
            version: text(tool, "version")?,
            files: uint(tool, "files")?,
            timeouts: uint(tool, "timeouts")?,
            crashes: uint(tool, "crashes")?,
            facts,
        };
        tools.insert(text(tool, "tool")?, row);
    }
    let disagreements = report.get("disagreements").and_then(Value::as_array);
    let failures = disagreements.map_or(0, |d| {
        d.iter()
            .filter(|d| d.get("kind").and_then(Value::as_str) == Some("failure"))
            .count()
    });
    Ok(Some(Differential {
        inputs: uint(&report, "inputs")?,
        sample: report.get("sample").and_then(Value::as_u64),
        corpus: report.get("corpus").and_then(Value::as_str).map(str::to_owned),
        source_filter: source_filter(&report, "source_filter"),
        tools,
        failures: u64::try_from(failures)?,
        lamina_errors: count(&report, "lamina_errors")?,
        missing: count(&report, "missing")?,
        broken_readers: count(&report, "broken_readers")?,
    }))
}

/// The text differential's per-source pages and the tools' versions.
pub(super) fn text_differential(root: &Path) -> Result<Option<Text>> {
    let Some(report) = json(&root.join(TEXT))? else {
        return Ok(None);
    };
    let mut tools = BTreeMap::new();
    for tool in report.get("tools").and_then(Value::as_array).context("no `tools`")? {
        tools.insert(text(tool, "tool")?, text(tool, "version")?);
    }
    let mut sources = BTreeMap::new();
    for (name, s) in report
        .get("sources")
        .and_then(Value::as_object)
        .context("no `sources`")?
    {
        let source = TextSource {
            files: uint(s, "files")?,
            pages: uint(s, "pages")?,
            compared: uint(s, "compared")?,
            agree: uint(s, "agree")?,
        };
        sources.insert(name.clone(), source);
    }
    Ok(Some(Text {
        sample: report.get("sample").and_then(Value::as_u64),
        source_filter: source_filter(&report, "source_filter"),
        tools,
        sources,
    }))
}

/// The render diff's sampling and per-source classes; the rate is the one its history records.
pub(super) fn render(root: &Path) -> Result<Option<Render>> {
    let Some(report) = json(&root.join(RENDER))? else {
        return Ok(None);
    };
    let sampling = report.get("sampling").context("no `sampling`")?;
    let rates = report.pointer("/record/benches").context("no `record.benches`")?;
    let mut sources = BTreeMap::new();
    for (name, s) in report
        .get("sources")
        .and_then(Value::as_object)
        .context("no `sources`")?
    {
        let classes = s
            .get("classes")
            .and_then(Value::as_object)
            .context("no `classes`")?
            .iter()
            .map(|(class, n)| Ok((class.clone(), n.as_u64().context("a class count that isn't one")?)))
            .collect::<Result<_>>()?;
        let source = RenderSource {
            rows: uint(s, "rows")?,
            classes,
            pass_minor_bp: uint(rates, &format!("render-diff/{name}"))?,
        };
        sources.insert(name.clone(), source);
    }
    Ok(Some(Render {
        dpi: u32::try_from(uint(sampling, "dpi")?)?,
        pages_per_file: sampling
            .get("pages")
            .and_then(Value::as_u64)
            .map(u32::try_from)
            .transpose()?,
        sample: sampling.get("sample").and_then(Value::as_u64),
        source_filter: source_filter(sampling, "sources").context("no `sampling.sources`")?,
        sources,
    }))
}

/// Each fuzz target's summary that exists; `None` when there is none at all.
pub(super) fn fuzz(root: &Path) -> Result<Option<BTreeMap<String, FuzzRun>>> {
    let dir = root.join(FUZZ_SUMMARY_DIR);
    let mut runs = BTreeMap::new();
    for target in FUZZ_TARGETS {
        if let Some(run) = typed::<FuzzRun>(&dir.join(format!("{target}.json")))? {
            runs.insert(target.to_owned(), run);
        }
    }
    Ok((!runs.is_empty()).then_some(runs))
}

pub(super) fn verapdf(root: &Path) -> Result<Option<verapdf::Report>> {
    typed(&root.join(verapdf::REPORT_DIR).join("report.json"))
}

pub(super) fn roundtrip(root: &Path) -> Result<Option<roundtrip::Summary>> {
    typed(&root.join(roundtrip::OUT_DIR).join("summary.json"))
}

/// Every `runtimes.json` and `runtimes-<label>.json` (one per nightly job of the matrix),
/// merged; `None` when there is none.
pub(super) fn runtimes(root: &Path) -> Result<Option<Runtimes>> {
    let dir = root.join(pdf_runtimes::OUT_DIR);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(None);
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            n == &pdf_runtimes::results_file(None)
                || (n.starts_with("runtimes-") && Path::new(n).extension().is_some_and(|e| e == "json"))
        })
        .collect();
    names.sort();
    let mut merged: Option<Runtimes> = None;
    for name in names {
        let Some(results) = typed::<RuntimeResults>(&dir.join(&name))? else {
            continue;
        };
        let all = merged.get_or_insert_with(Runtimes::default);
        all.passed.extend(results.passed);
        all.failed.extend(results.failed);
    }
    if let Some(all) = &mut merged {
        all.missing = pdf_runtimes::RUNTIMES
            .iter()
            .filter(|r| !all.passed.iter().chain(&all.failed).any(|n| n == *r))
            .map(|r| (*r).to_owned())
            .collect();
    }
    Ok(merged)
}
