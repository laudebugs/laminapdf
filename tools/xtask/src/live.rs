//! `cargo xtask differential --live`: the nightly lane's differential (ADR 0018 tier 6) with the
//! tools themselves, not precomputed facts. lamina against pdf.js, PDFium, qpdf and pypdf over
//! the external corpus (`cargo xtask corpus fetch`) and the nightly set of `fixtures/corpus`
//! mutants, comparing the open/repair/fail outcome, the page count and the outline titles.
//!
//! Every tool writes the same JSON shape (`tools/differential/`, `examples/differential-facts.rs`
//! in laminapdf-model); this module compares them, writes `target/nightly/differential/report.md`
//! and `report.json`, and fails when lamina disagrees with *every* tool that compared a fact and
//! the disagreement is not excused by `fixtures/differential/exceptions.toml`, or when lamina
//! panics. Disagreements where at least one tool agrees with lamina are listed in the report
//! (they are differences between the tools as much as bugs) but don't fail the run.
//!
//! `--text` runs the text differential instead ([`text`], todo B9f): lamina's page text
//! against pdf.js and pdftotext over the external corpus. `--xmp` runs the XMP differential
//! ([`xmp`]): the XMP values lamina reads against exiftool's.

mod compare;
mod exceptions;
mod text;
mod xmp;
use compare::{Kind, RunContext, Tally, compare, rate};
use exceptions::{Class, Entry, Exceptions, Excuse, PENDING_ISSUE};

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::workspace::Workspace;
use crate::{corpus, runner, step, util};

/// The facts every tool reports (the order of the report's columns).
const FACTS: [&str; 3] = ["outcome", "page_count", "outline_titles"];

/// Mutants per `fixtures/corpus` file in the nightly set; the PR lane's 4 are its prefix.
const NIGHTLY_MUTANTS_PER_FILE: usize = 250;

/// Rows per table in report.md (the job summary has a 1 MiB limit); report.json has them all.
const MAX_ROWS: usize = 150;

/// An external reader in the differential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Tool {
    Pdfjs,
    Pdfium,
    Qpdf,
    Pypdf,
}

impl Tool {
    const ALL: [Self; 4] = [Self::Pdfjs, Self::Pdfium, Self::Qpdf, Self::Pypdf];

    fn name(self) -> &'static str {
        match self {
            Self::Pdfjs => "pdfjs",
            Self::Pdfium => "pdfium",
            Self::Qpdf => "qpdf",
            Self::Pypdf => "pypdf",
        }
    }

    fn parse(name: &str) -> Result<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name).with_context(|| {
            let known: Vec<&str> = Self::ALL.iter().map(|t| t.name()).collect();
            format!("unknown differential tool {name:?}; tools: {}", known.join(", "))
        })
    }
}

/// Which inputs a live run reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Inputs {
    External,
    Mutants,
    All,
}

impl Inputs {
    /// `--corpus`'s value, as the reports record it.
    fn name(self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Mutants => "mutants",
            Self::All => "all",
        }
    }
}

/// `differential --live [--corpus external|mutants|all] [--source <name>]… [--tools a,b] [--sample N]`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LiveArgs {
    pub(crate) inputs: Inputs,
    pub(crate) sources: Vec<String>,
    pub(crate) tools: Vec<Tool>,
    pub(crate) sample: Option<usize>,
    /// `--text`: the text differential (external corpus; pdf.js and pdftotext).
    pub(crate) text: bool,
    /// `--xmp`: the XMP differential (external corpus and committed PDFs; exiftool).
    pub(crate) xmp: bool,
}

impl Default for LiveArgs {
    fn default() -> Self {
        Self {
            inputs: Inputs::All,
            sources: Vec::new(),
            tools: Tool::ALL.to_vec(),
            sample: None,
            text: false,
            xmp: false,
        }
    }
}

const USAGE: &str = "usage: cargo xtask differential --live [--corpus external|mutants|all] \
                     [--source <name>]… [--tools pdfjs,pdfium,qpdf,pypdf] [--sample <N>]\n       \
                     cargo xtask differential --live --text [--source <name>]… [--sample <N>]\n       \
                     cargo xtask differential --live --xmp [--source <name>]… [--sample <N>]";

pub(crate) fn parse_args(args: &[String]) -> Result<LiveArgs> {
    let mut parsed = LiveArgs::default();
    let (mut corpus, mut tools) = (false, false);
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().with_context(|| format!("{arg} needs a value; {USAGE}"));
        match arg.as_str() {
            "--text" => parsed.text = true,
            "--xmp" => parsed.xmp = true,
            "--corpus" => {
                corpus = true;
                parsed.inputs = match value()?.as_str() {
                    "external" => Inputs::External,
                    "mutants" => Inputs::Mutants,
                    "all" => Inputs::All,
                    other => bail!("--corpus {other:?}; {USAGE}"),
                };
            }
            "--source" => parsed.sources.push(value()?.clone()),
            "--tools" => {
                tools = true;
                parsed.tools = value()?.split(',').map(Tool::parse).collect::<Result<_>>()?;
                parsed.tools.sort();
                parsed.tools.dedup();
            }
            "--sample" => {
                let n = value()?;
                let n: usize = n.parse().with_context(|| format!("--sample {n:?}: not a number"))?;
                if n == 0 {
                    bail!("--sample 0 would compare nothing; {USAGE}");
                }
                parsed.sample = Some(n);
            }
            other => bail!("unknown argument {other:?}; {USAGE}"),
        }
    }
    if parsed.text && (tools || (corpus && parsed.inputs != Inputs::External)) {
        bail!("--text reads the external corpus with pdf.js and pdftotext: no --tools or mutants; {USAGE}");
    }
    if parsed.xmp && (parsed.text || tools || corpus) {
        bail!(
            "--xmp reads the external corpus and committed PDFs with ExifTool: no --text, --tools or --corpus; {USAGE}"
        );
    }
    Ok(parsed)
}

/// The group a sampled input belongs to: its external source (`pdfjs/…` → `pdfjs`), or
/// `mutants` for the `fixtures/corpus` set.
pub(crate) fn group(name: &str) -> &str {
    name.split_once('/').map_or("mutants", |(source, _)| source)
}

/// At most `n` of the sorted `items`, stratified by [`group`]: each group gets a share
/// proportional to its size and at least one input (while `n` allows), spread evenly within it.
pub(crate) fn sample(items: Vec<(String, PathBuf)>, n: usize) -> Vec<(String, PathBuf)> {
    let len = items.len();
    if n >= len {
        return items;
    }
    let mut groups: BTreeMap<String, Vec<(String, PathBuf)>> = BTreeMap::new();
    for item in items {
        groups.entry(group(&item.0).to_owned()).or_default().push(item);
    }
    let mut quota: BTreeMap<String, usize> = groups
        .iter()
        .map(|(g, v)| {
            (
                g.clone(),
                (v.len() * n / len).max(usize::from(n >= groups.len())).min(v.len()),
            )
        })
        .collect();
    // Rounding down leaves some of `n` unused: hand it out to the largest groups first.
    let mut by_size: Vec<(&String, usize)> = groups.iter().map(|(g, v)| (g, v.len())).collect();
    by_size.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let mut left = n.saturating_sub(quota.values().sum());
    while left > 0 {
        let before = left;
        for (g, size) in &by_size {
            if left == 0 {
                break;
            }
            if let Some(q) = quota.get_mut(*g)
                && *q < *size
            {
                *q += 1;
                left -= 1;
            }
        }
        if left == before {
            break;
        }
    }
    let mut out = Vec::new();
    for (g, items) in groups {
        let take = quota.get(&g).copied().unwrap_or_default();
        let len = items.len();
        let keep: BTreeSet<usize> = (0..take).map(|i| i * len / take.max(1)).collect();
        out.extend(
            items
                .into_iter()
                .enumerate()
                .filter_map(|(i, item)| keep.contains(&i).then_some(item)),
        );
    }
    out.sort();
    out
}

/// The exceptions files the live differential applies: the fast set's (with the nightly
/// `[[class]]` and `[[known]]` entries) and the full lane's.
const EXCEPTIONS: [&str; 2] = [
    "fixtures/differential/exceptions.toml",
    "fixtures/differential/full-exceptions.toml",
];

/// Where `cargo build --release` puts the laminapdf-model example.
pub(crate) fn example_binary(ws: &Workspace, name: &str) -> PathBuf {
    ws.target_dir
        .join("release/examples")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

/// `cargo xtask differential --live …`.
pub(crate) fn run_live(ws: &Workspace, args: &LiveArgs) -> Result<()> {
    if args.text {
        return text::run(ws, args);
    }
    if args.xmp {
        return xmp::run(ws, args);
    }
    // Everything that can fail fast does so before hours of work.
    let mut exceptions = Exceptions::default();
    for file in EXCEPTIONS {
        exceptions.merge(Exceptions::read(&ws.root.join(file)).with_context(|| format!("reading {file}"))?);
    }
    if args.tools.contains(&Tool::Qpdf) {
        crate::doctor::require(ws, &crate::doctor::qpdf())?;
    }
    let pdfjs_modules = ws.root.join("tools/differential/node_modules/pdfjs-dist");
    if args.tools.contains(&Tool::Pdfjs) && !pdfjs_modules.is_dir() {
        bail!("pdf.js is not installed: run `pnpm --dir tools/differential install --frozen-lockfile`");
    }
    let out = ws.root.join("target/nightly/differential");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    fs::create_dir_all(&out)?;

    let mut inputs: Vec<(String, PathBuf)> = Vec::new();
    if matches!(args.inputs, Inputs::External | Inputs::All) {
        inputs.extend(corpus::verified_inputs(ws, &args.sources)?);
    }
    if matches!(args.inputs, Inputs::Mutants | Inputs::All) {
        inputs.extend(write_mutants(ws, &out.join("mutants"))?);
    }
    inputs.sort();
    if let Some(n) = args.sample {
        inputs = sample(inputs, n);
    }
    if inputs.is_empty() {
        bail!("no inputs: nothing would be compared (check --corpus, --source and --sample)");
    }
    step(&format!("differential: {} inputs", inputs.len()));

    ws.cargo(&[
        "build",
        "--release",
        "--locked",
        "-q",
        "-p",
        "laminapdf-model",
        "--example",
        "differential-facts",
    ])?;
    let mut readers = vec![runner::Reader {
        name: "lamina",
        program: example_binary(ws, "differential-facts").into_os_string(),
        args: Vec::new(),
        dir: ws.root.clone(),
        reports_repair: true,
    }];
    readers.extend(args.tools.iter().map(|tool| tool.reader(ws)));
    let (mut reports, broken) = run_readers(&readers, &inputs, &out)?;
    let lamina = reports.remove(0);
    let comparison = compare(&lamina, &reports, &exceptions);
    let context = RunContext {
        inputs: inputs.len(),
        external: inputs.iter().filter(|(n, _)| n.contains('/')).count(),
        commit: util::git_stdout(&ws.root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned()),
        sample: args.sample,
        corpus: args.inputs.name(),
        sources: args.sources.clone(),
        broken,
    };
    let markdown = comparison.markdown(&lamina, &reports, &exceptions, &context);
    fs::write(out.join("report.md"), &markdown)?;
    fs::write(
        out.join("report.json"),
        serde_json::to_string_pretty(&comparison.json(&lamina, &reports, &exceptions, &context))? + "\n",
    )?;
    util::summary(&markdown)?;
    println!("{}", comparison.summary_line());
    for pending in exceptions.pending() {
        println!("::warning::exceptions.toml: [[known]] {pending} has no tracking issue yet");
    }
    step(&format!("Report: {}", out.join("report.md").display()));
    if !context.broken.is_empty() {
        bail!("readers failed: {}; see the report", context.broken.join("; "));
    }
    if comparison.failed() {
        bail!(
            "lamina disagrees with every tool on {} facts, or failed to read {} files (panic, crash \
             or timeout), and fixtures/differential/exceptions.toml doesn't excuse them; see the report",
            comparison.failures().count(),
            comparison.lamina_errors.len()
        );
    }
    Ok(())
}

/// Runs every reader over `inputs` at once (each in its own processes, see runner.rs) and
/// writes their reports to `out/<name>.json`; returns the reports in `readers` order and the
/// readers that broke or reported the wrong files. Both lanes (facts and `--text`) use it.
fn run_readers(
    readers: &[runner::Reader],
    inputs: &[(String, PathBuf)],
    out: &Path,
) -> Result<(Vec<Report>, Vec<String>)> {
    // Every reader runs at once, each in its own processes (see runner.rs).
    let runs: Vec<Result<runner::Run>> = std::thread::scope(|scope| {
        let handles: Vec<_> = readers
            .iter()
            .map(|reader| scope.spawn(move || runner::run(reader, inputs, out, runner::Limits::default())))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("a reader thread panicked")))
            })
            .collect()
    });
    let mut broken = Vec::new();
    let mut reports = Vec::new();
    for (reader, run) in readers.iter().zip(runs) {
        let run = run.with_context(|| format!("running {}", reader.name))?;
        fs::write(
            out.join(format!("{}.json", reader.name)),
            serde_json::to_string_pretty(&run.report)? + "\n",
        )?;
        let (extra, missing) = runner::coverage(&run.report, inputs);
        if let Some(why) = run.broken {
            broken.push(format!("{}: {why}", reader.name));
        } else if !extra.is_empty() || !missing.is_empty() {
            broken.push(format!(
                "{}: {} inputs missing from its report, {} names that aren't inputs",
                reader.name,
                missing.len(),
                extra.len()
            ));
        }
        reports.push(Report::parse(&run.report)?);
    }
    Ok((reports, broken))
}

/// The nightly mutant set: every `fixtures/corpus` file, the first 250 mutants of the fast
/// set's series and the full lane's set (`differential-inputs --nightly`).
fn write_mutants(ws: &Workspace, dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    ws.cargo(&[
        "run",
        "--release",
        "--locked",
        "-q",
        "-p",
        "laminapdf-model",
        "--example",
        "differential-inputs",
        "--",
        &dir.to_string_lossy(),
        "--nightly",
        &NIGHTLY_MUTANTS_PER_FILE.to_string(),
    ])?;
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().context("file name")?.to_string_lossy().into_owned();
        out.push((name, path));
    }
    Ok(out)
}

impl Tool {
    /// How [`runner`] starts this tool.
    fn reader(self, ws: &Workspace) -> runner::Reader {
        let (program, args): (OsString, Vec<OsString>) = match self {
            Self::Pdfjs => (
                "node".into(),
                vec![ws.root.join("tools/differential/pdfjs.mjs").into_os_string()],
            ),
            Self::Pdfium | Self::Qpdf | Self::Pypdf => (
                crate::harness::python().into(),
                vec![
                    ws.root.join("tools/differential/facts.py").into_os_string(),
                    self.name().into(),
                ],
            ),
        };
        runner::Reader {
            name: self.name(),
            program,
            args,
            dir: ws.root.clone(),
            reports_repair: matches!(self, Self::Qpdf | Self::Pypdf),
        }
    }
}

/// One tool's facts: `{tool, version, reports_repair, facts: {name: {outcome, page_count, outline_titles}}}`.
#[derive(Debug)]
pub(crate) struct Report {
    tool: String,
    version: String,
    /// Whether the tool distinguishes "repaired" from "opened" (pdf.js and PDFium repair silently).
    reports_repair: bool,
    facts: BTreeMap<String, Value>,
}

impl Report {
    fn parse(value: &Value) -> Result<Self> {
        let text = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .with_context(|| format!("tool report without `{key}`"))
        };
        let facts = value
            .get("facts")
            .and_then(Value::as_object)
            .context("tool report without `facts`")?
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(Self {
            tool: text("tool")?,
            version: text("version")?,
            reports_repair: value.get("reports_repair").and_then(Value::as_bool).unwrap_or(false),
            facts,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::compare::{Comparison, cell};
    use super::exceptions::{Relation, glob};
    use super::*;

    fn report(tool: &str, reports_repair: bool, facts: &Value) -> Report {
        Report::parse(&json!({"tool": tool, "version": "1", "reports_repair": reports_repair, "facts": facts})).unwrap()
    }

    fn opened(pages: u64) -> Value {
        json!({"outcome": "opened", "page_count": pages, "outline_titles": []})
    }

    fn kinds(c: &Comparison) -> Vec<(&str, &str, Kind)> {
        c.disagreements
            .iter()
            .map(|d| (d.file.as_str(), d.fact, d.kind))
            .collect()
    }

    /// The version a `pypdf==` line or a `PINNED = "…"` line names.
    fn pypdf_pin(text: &str, prefix: &str) -> Option<String> {
        let rest = text.lines().find_map(|l| l.trim().strip_prefix(prefix))?;
        Some(rest.trim().trim_matches('"').to_owned())
    }

    /// The live differential installs tools/differential's pypdf, but its reader refuses any
    /// version but corpus-facts' `PINNED` (the PR lane's facts): a bump of one alone makes every
    /// pypdf file crash, as Dependabot's #115 did.
    #[test]
    fn live_differential_should_install_the_pypdf_its_reader_accepts() {
        let differential = pypdf_pin(include_str!("../../differential/requirements.txt"), "pypdf==");
        let corpus = pypdf_pin(include_str!("../../corpus-facts/requirements.txt"), "pypdf==");
        let reader = pypdf_pin(include_str!("../../corpus-facts/facts.py"), "PINNED = ");
        assert!(
            differential.is_some(),
            "tools/differential/requirements.txt pins no pypdf"
        );
        assert_eq!(
            differential, corpus,
            "tools/differential and tools/corpus-facts pin different pypdf"
        );
        assert_eq!(
            corpus, reader,
            "tools/corpus-facts/requirements.txt and facts.py's PINNED differ"
        );
    }

    #[test]
    fn compare_should_fail_when_every_tool_disagrees() {
        let lamina = report("lamina", true, &json!({"a.pdf": opened(2)}));
        let tools = [
            report("pdfjs", false, &json!({"a.pdf": opened(3)})),
            report("qpdf", true, &json!({"a.pdf": opened(3)})),
        ];
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert_eq!(kinds(&c), [("a.pdf", "page_count", Kind::Failure)]);
    }

    #[test]
    fn compare_should_only_report_a_split_when_one_tool_agrees() {
        let lamina = report("lamina", true, &json!({"a.pdf": opened(2)}));
        let tools = [
            report("pdfjs", false, &json!({"a.pdf": opened(2)})),
            report("qpdf", true, &json!({"a.pdf": opened(3)})),
        ];
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(!c.failed() && kinds(&c) == [("a.pdf", "page_count", Kind::Split)]);
    }

    #[test]
    fn compare_should_ignore_repairs_for_tools_that_repair_silently() {
        let repaired = json!({"outcome": "repaired", "page_count": 1, "outline_titles": []});
        let lamina = report("lamina", true, &json!({"a.pdf": repaired}));
        let tools = [report("pdfium", false, &json!({"a.pdf": opened(1)}))];
        assert!(
            compare(&lamina, &tools, &Exceptions::default())
                .disagreements
                .is_empty()
        );
    }

    #[test]
    fn compare_should_not_compare_pages_when_a_reader_fails() {
        let failed = json!({"outcome": "failed", "page_count": null, "outline_titles": null});
        let lamina = report("lamina", true, &json!({"a.pdf": opened(1)}));
        let tools = [report("pypdf", true, &json!({"a.pdf": failed}))];
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert_eq!(kinds(&c), [("a.pdf", "outcome", Kind::Failure)]);
    }

    #[test]
    fn compare_should_skip_tool_timeouts() {
        let timeout = json!({"outcome": "timeout", "page_count": null, "outline_titles": null});
        let lamina = report("lamina", true, &json!({"a.pdf": opened(1)}));
        let tools = [report("pdfjs", false, &json!({"a.pdf": timeout}))];
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(c.disagreements.is_empty() && c.tools["pdfjs"].timeouts == 1);
    }

    #[test]
    fn compare_should_fail_on_a_lamina_panic() {
        let panic = json!({"outcome": "panic", "page_count": null, "outline_titles": null});
        let lamina = report("lamina", true, &json!({"a.pdf": panic}));
        assert!(compare(&lamina, &[], &Exceptions::default()).failed());
    }

    #[test]
    fn exceptions_should_excuse_the_listed_tools_only() {
        let exceptions = Exceptions::parse(
            "[[exception]]\nfile = \"a.pdf\"\nfacts = [\"page_count\"]\ntools = [\"qpdf\"]\n\
             rule = \"ADR 0003 §1: Repair\"\nreason = \"r\"\n",
        )
        .unwrap();
        let lamina = report("lamina", true, &json!({"a.pdf": opened(2)}));
        let tools = [
            report("pdfjs", false, &json!({"a.pdf": opened(3)})),
            report("qpdf", true, &json!({"a.pdf": opened(3)})),
        ];
        let c = compare(&lamina, &tools, &exceptions);
        assert_eq!(kinds(&c), [("a.pdf", "page_count", Kind::Failure)]);
    }

    #[test]
    fn exceptions_should_turn_a_failure_into_an_excused_difference() {
        let exceptions = Exceptions::parse(
            "[[exception]]\nfile = \"a.pdf\"\nfacts = [\"page_count\"]\ntools = [\"qpdf\"]\n\
             rule = \"ADR 0003 §1: Repair\"\nreason = \"r\"\n",
        )
        .unwrap();
        let lamina = report("lamina", true, &json!({"a.pdf": opened(2)}));
        let tools = [report("qpdf", true, &json!({"a.pdf": opened(3)}))];
        assert_eq!(
            kinds(&compare(&lamina, &tools, &exceptions)),
            [("a.pdf", "page_count", Kind::Excused)]
        );
    }

    #[test]
    fn exceptions_without_tools_should_be_about_pypdf() {
        let exceptions = Exceptions::parse(
            "[[exception]]\nfile = \"a.pdf\"\nfacts = [\"outcome\"]\nrule = \"ADR 0003 §1: Repair\"\nreason = \"r\"\n",
        )
        .unwrap();
        let (ours, theirs) = (json!({"outcome": "repaired"}), json!("failed"));
        let pypdf = exceptions.excuse("a.pdf", "outcome", "pypdf", &ours, &theirs);
        let qpdf = exceptions.excuse("a.pdf", "outcome", "qpdf", &ours, &theirs);
        assert_eq!((pypdf, qpdf), (Some(Excuse::File), None));
    }

    #[test]
    fn exceptions_should_require_an_adr_0003_rule() {
        let text = "[[exception]]\nfile = \"a.pdf\"\nfacts = [\"outcome\"]\nrule = \"because\"\nreason = \"r\"\n";
        assert!(Exceptions::parse(text).is_err());
    }

    #[test]
    fn stale_exceptions_should_be_reported() {
        let exceptions = Exceptions::parse(
            "[[exception]]\nfile = \"a.pdf\"\nfacts = [\"page_count\"]\nrule = \"ADR 0003 §1: Repair\"\nreason = \"r\"\n",
        )
        .unwrap();
        let lamina = report("lamina", true, &json!({"a.pdf": opened(2)}));
        let tools = [report("pypdf", true, &json!({"a.pdf": opened(2)}))];
        assert_eq!(
            compare(&lamina, &tools, &exceptions).stale,
            ["a.pdf: page_count (pypdf)"]
        );
    }

    fn named(names: &[String]) -> Vec<(String, PathBuf)> {
        names.iter().map(|n| (n.clone(), PathBuf::from(n))).collect()
    }

    #[test]
    fn sample_should_keep_every_source() {
        let mut names: Vec<String> = (0..100).map(|i| format!("pdfjs/{i:03}.pdf")).collect();
        names.extend((0..100).map(|i| format!("m{i:03}.pdf")));
        names.push("qpdf/only.pdf".to_owned());
        let picked = sample(named(&names), 10);
        let groups: BTreeSet<&str> = picked.iter().map(|(n, _)| group(n)).collect();
        assert_eq!((picked.len(), groups.len()), (10, 3));
    }

    #[test]
    fn sample_should_spread_within_a_group() {
        let names: Vec<String> = (0..10).map(|i| format!("m{i}.pdf")).collect();
        let picked: Vec<String> = sample(named(&names), 3).into_iter().map(|(n, _)| n).collect();
        assert_eq!(picked, ["m0.pdf", "m3.pdf", "m6.pdf"]);
    }

    #[test]
    fn live_args_should_reject_a_zero_sample() {
        let args: Vec<String> = ["--sample", "0"].map(String::from).to_vec();
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn compare_should_fail_when_a_tool_report_lacks_a_file() {
        let lamina = report("lamina", true, &json!({"a.pdf": opened(1), "b.pdf": opened(1)}));
        let tools = [report("qpdf", true, &json!({"a.pdf": opened(1)}))];
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(c.failed() && c.missing == ["b.pdf (qpdf)"]);
    }

    #[test]
    fn compare_should_fail_on_a_lamina_crash_or_timeout() {
        let crash = json!({"outcome": "crash", "page_count": null, "outline_titles": null});
        let lamina = report("lamina", true, &json!({"a.pdf": crash}));
        assert!(compare(&lamina, &[], &Exceptions::default()).failed());
    }

    #[test]
    fn compare_should_skip_tool_crashes() {
        let crash = json!({"outcome": "crash", "page_count": null, "outline_titles": null});
        let lamina = report("lamina", true, &json!({"a.pdf": opened(1)}));
        let tools = [report("pdfium", false, &json!({"a.pdf": crash}))];
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(!c.failed() && c.tools["pdfium"].crashes == 1);
    }

    const RULE: &str = "rule = \"ADR 0003 §1: Repair\"\nreason = \"r\"\n";

    #[test]
    fn exceptions_should_reject_an_unknown_tool() {
        let text = format!("[[exception]]\nfile = \"a.pdf\"\nfacts = [\"outcome\"]\ntools = [\"mupdf\"]\n{RULE}");
        assert!(Exceptions::parse(&text).is_err());
    }

    #[test]
    fn exceptions_should_require_a_file() {
        let text = format!("[[exception]]\nfacts = [\"outcome\"]\n{RULE}");
        assert!(Exceptions::parse(&text).is_err());
    }

    #[test]
    fn exceptions_should_read_files_lists() {
        let text = format!("[[exception]]\nfiles = [\"a.pdf\", \"b.pdf\"]\nfacts = [\"outcome\"]\n{RULE}");
        let e = Exceptions::parse(&text).unwrap();
        let (ours, theirs) = (json!({"outcome": "repaired"}), json!("failed"));
        assert_eq!(
            e.excuse("b.pdf", "outcome", "pypdf", &ours, &theirs),
            Some(Excuse::File)
        );
    }

    #[test]
    fn known_entries_should_flag_a_pending_issue() {
        let text = "[[known]]\nfile = \"a.pdf\"\nfacts = [\"page_count\"]\nissue = \"PENDING\"\nreason = \"r\"\n";
        let e = Exceptions::parse(text).unwrap();
        assert_eq!(
            (e.pending(), e.describe(Excuse::Known(0))),
            (vec!["a.pdf"], "known: tracking issue pending".to_owned())
        );
    }

    #[test]
    fn cell_should_escape_pipes_after_truncating() {
        // 79 characters are kept (the opening quote and 78 x): the `|` after them is cut,
        // never half of its escape.
        let cut = cell(&json!(format!("{}|tail", "x".repeat(78))));
        assert!(cell(&json!("a|b")) == "\"a\\|b\"" && !cut.contains('|') && cut.ends_with('…'));
    }

    #[test]
    fn live_args_should_read_tools_and_sample() {
        let args: Vec<String> = ["--corpus", "external", "--tools", "qpdf,pdfjs", "--sample", "50"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            parse_args(&args).unwrap(),
            LiveArgs {
                inputs: Inputs::External,
                sources: Vec::new(),
                tools: vec![Tool::Pdfjs, Tool::Qpdf],
                sample: Some(50),
                text: false,
                xmp: false,
            }
        );
    }

    #[test]
    fn glob_should_match_runs_of_characters() {
        let cases = [
            ("*-m*.pdf", "hello-m12.pdf"),
            ("pdfjs/*", "pdfjs/test/pdfs/a.pdf"),
            ("*", "x"),
        ];
        assert!(cases.iter().all(|(p, t)| glob(p, t)) && !glob("*-m*.pdf", "pdfjs/a.pdf") && !glob("a", "ab"));
    }

    const CLASS: &str = "[[class]]\nfacts = [\"outcome\"]\nlamina = \"repaired\"\ntheirs = \"failed\"\n\
                         rule = \"ADR 0003 §1: Repair\"\nreason = \"r\"\n";

    #[test]
    fn classes_should_excuse_matching_values_for_every_tool() {
        let repaired = json!({"outcome": "repaired", "page_count": 1, "outline_titles": []});
        let failed = json!({"outcome": "failed", "page_count": null, "outline_titles": null});
        let lamina = report("lamina", true, &json!({"a.pdf": repaired}));
        let tools = [
            report("pdfjs", false, &json!({"a.pdf": &failed})),
            report("qpdf", true, &json!({"a.pdf": failed})),
        ];
        let c = compare(&lamina, &tools, &Exceptions::parse(CLASS).unwrap());
        assert_eq!(kinds(&c), [("a.pdf", "outcome", Kind::Excused)]);
    }

    #[test]
    fn classes_should_not_excuse_other_values() {
        let failed = json!({"outcome": "failed", "page_count": null, "outline_titles": null});
        let lamina = report("lamina", true, &json!({"a.pdf": failed}));
        let tools = [report("qpdf", true, &json!({"a.pdf": opened(1)}))];
        let c = compare(&lamina, &tools, &Exceptions::parse(CLASS).unwrap());
        assert_eq!(kinds(&c), [("a.pdf", "outcome", Kind::Failure)]);
    }

    #[test]
    fn classes_should_match_lamina_error_codes() {
        let class = "[[class]]\nfacts = [\"outcome\"]\nlamina_facts = { error = \"missing-root\" }\n\
                     rule = \"ADR 0003 §1: Rejected\"\nreason = \"r\"\n";
        let exceptions = Exceptions::parse(class).unwrap();
        let missing_root = json!({"outcome": "failed", "error": "missing-root"});
        let syntax = json!({"outcome": "failed", "error": "syntax"});
        let opened = json!("opened");
        assert_eq!(
            (
                exceptions.excuse("a.pdf", "outcome", "qpdf", &missing_root, &opened),
                exceptions.excuse("a.pdf", "outcome", "qpdf", &syntax, &opened)
            ),
            (Some(Excuse::Class(0)), None)
        );
    }

    #[test]
    fn lamina_skips_should_hold_for_fewer_pages_and_a_shorter_outline_subsequence() {
        let skips = Relation::LaminaPdfSkips;
        assert!(
            skips.holds(&json!(2), &json!(3))
                && skips.holds(&json!(["a", "c"]), &json!(["a", "b", "c"]))
                && !skips.holds(&json!(["c", "a"]), &json!(["a", "b", "c"]))
                && !skips.holds(&json!(3), &json!(3))
        );
    }

    #[test]
    fn known_entries_should_mark_a_failure_as_known() {
        let known = "[[known]]\nfile = \"a.pdf\"\nfacts = [\"page_count\"]\n\
                     issue = \"https://github.com/laudebugs/laminapdf/issues/1\"\nreason = \"r\"\n";
        let lamina = report("lamina", true, &json!({"a.pdf": opened(2)}));
        let tools = [report("qpdf", true, &json!({"a.pdf": opened(3)}))];
        let c = compare(&lamina, &tools, &Exceptions::parse(known).unwrap());
        assert!(!c.failed() && kinds(&c) == [("a.pdf", "page_count", Kind::Known)]);
    }

    #[test]
    fn known_entries_should_read_files_lists_one_entry_per_file() {
        let known = "[[known]]\nfiles = [\"a.pdf\", \"b.pdf\"]\nfacts = [\"page_count\"]\n\
                     issue = \"https://github.com/laudebugs/laminapdf/issues/1\"\nreason = \"r\"\n";
        let e = Exceptions::parse(known).unwrap();
        let (ours, theirs) = (json!({"page_count": 1}), json!(2));
        assert_eq!(
            (e.known.len(), e.excuse("b.pdf", "page_count", "qpdf", &ours, &theirs)),
            (2, Some(Excuse::Known(1)))
        );
    }

    #[test]
    fn known_entries_should_require_an_issue_url() {
        let known = "[[known]]\nfile = \"a.pdf\"\nfacts = [\"page_count\"]\nissue = \"#1\"\nreason = \"r\"\n";
        assert!(Exceptions::parse(known).is_err());
    }

    #[test]
    fn live_args_should_read_text() {
        let args: Vec<String> = ["--text", "--source", "pdfjs", "--sample", "50"]
            .map(String::from)
            .to_vec();
        let parsed = parse_args(&args).unwrap();
        assert!(parsed.text && parsed.sources == ["pdfjs"] && parsed.sample == Some(50));
    }

    #[test]
    fn live_args_should_reject_text_with_tools_or_mutants() {
        let with_tools: Vec<String> = ["--text", "--tools", "qpdf"].map(String::from).to_vec();
        let with_mutants: Vec<String> = ["--text", "--corpus", "mutants"].map(String::from).to_vec();
        assert!(parse_args(&with_tools).is_err() && parse_args(&with_mutants).is_err());
    }

    #[test]
    fn unmapped_should_hold_where_lamina_has_only_replacement_characters_more() {
        let unmapped = Relation::Unmapped;
        assert!(
            unmapped.holds(&json!("a\u{FFFD}b"), &json!("ab"))
                && unmapped.holds(&json!("\u{FFFD}"), &json!("\u{0}"))
                && !unmapped.holds(&json!("\u{FFFD}\u{FFFD}"), &json!("different"))
                && !unmapped.holds(&json!("ab"), &json!("ab"))
        );
    }

    #[test]
    fn rate_should_show_a_percentage() {
        let tally = Tally {
            agree: 3,
            disagree: 1,
            excused: 0,
        };
        assert_eq!(rate(&tally), "75.0% (3/4)");
    }
}
