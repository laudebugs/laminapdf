//! `cargo xtask differential --live`: the nightly lane's differential (ADR 0018 tier 6) with the
//! tools themselves, not precomputed facts. lamina against pdf.js, PDFium, qpdf and pypdf over
//! the external corpus (`cargo xtask corpus fetch`) and the nightly set of `fixtures/corpus`
//! mutants, comparing the open/repair/fail outcome, the page count and the outline titles.
//!
//! Every tool writes the same JSON shape (`tools/differential/`, `examples/differential-facts.rs`
//! in lamina-model); this module compares them, writes `target/nightly/differential/report.md`
//! and `report.json`, and fails when lamina disagrees with *every* tool that compared a fact and
//! the disagreement is not excused by `fixtures/differential/exceptions.toml`, or when lamina
//! panics. Disagreements where at least one tool agrees with lamina are listed in the report
//! (they are differences between the tools as much as bugs) but don't fail the run.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

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

/// `differential --live [--corpus external|mutants|all] [--source <name>]… [--tools a,b] [--sample N]`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LiveArgs {
    pub(crate) inputs: Inputs,
    pub(crate) sources: Vec<String>,
    pub(crate) tools: Vec<Tool>,
    pub(crate) sample: Option<usize>,
}

impl Default for LiveArgs {
    fn default() -> Self {
        Self {
            inputs: Inputs::All,
            sources: Vec::new(),
            tools: Tool::ALL.to_vec(),
            sample: None,
        }
    }
}

const USAGE: &str = "usage: cargo xtask differential --live [--corpus external|mutants|all] \
                     [--source <name>]… [--tools pdfjs,pdfium,qpdf,pypdf] [--sample <N>]";

pub(crate) fn parse_args(args: &[String]) -> Result<LiveArgs> {
    let mut parsed = LiveArgs::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().with_context(|| format!("{arg} needs a value; {USAGE}"));
        match arg.as_str() {
            "--corpus" => {
                parsed.inputs = match value()?.as_str() {
                    "external" => Inputs::External,
                    "mutants" => Inputs::Mutants,
                    "all" => Inputs::All,
                    other => bail!("--corpus {other:?}; {USAGE}"),
                };
            }
            "--source" => parsed.sources.push(value()?.clone()),
            "--tools" => {
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
    Ok(parsed)
}

/// The group a sampled input belongs to: its external source (`pdfjs/…` → `pdfjs`), or
/// `mutants` for the `fixtures/corpus` set.
fn group(name: &str) -> &str {
    name.split_once('/').map_or("mutants", |(source, _)| source)
}

/// At most `n` of the sorted `items`, stratified by [`group`]: each group gets a share
/// proportional to its size and at least one input (while `n` allows), spread evenly within it.
fn sample(items: Vec<(String, PathBuf)>, n: usize) -> Vec<(String, PathBuf)> {
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

/// Where `cargo build --release` puts the lamina-model example.
fn example_binary(ws: &Workspace, name: &str) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| ws.root.join("target"), PathBuf::from);
    target
        .join("release/examples")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

/// `cargo xtask differential --live …`.
pub(crate) fn run_live(ws: &Workspace, args: &LiveArgs) -> Result<()> {
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
        "lamina-model",
        "--example",
        "differential-facts",
    ])?;
    let (mut reports, broken) = run_readers(ws, args, &inputs, &out)?;
    let lamina = reports.remove(0);
    let comparison = compare(&lamina, &reports, &exceptions);
    let context = RunContext {
        inputs: inputs.len(),
        external: inputs.iter().filter(|(n, _)| n.contains('/')).count(),
        commit: util::git_stdout(&ws.root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned()),
        sample: args.sample,
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

/// Runs lamina and every tool over `inputs` at once (each in its own processes, see
/// runner.rs) and writes their reports to `out`; returns the reports, lamina's first, and the
/// readers that broke or reported the wrong files.
fn run_readers(
    ws: &Workspace,
    args: &LiveArgs,
    inputs: &[(String, PathBuf)],
    out: &Path,
) -> Result<(Vec<Report>, Vec<String>)> {
    let mut readers = vec![runner::Reader {
        name: "lamina",
        program: example_binary(ws, "differential-facts").into_os_string(),
        args: Vec::new(),
        dir: ws.root.clone(),
        reports_repair: true,
    }];
    readers.extend(args.tools.iter().map(|tool| tool.reader(ws)));
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
        "lamina-model",
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

/// How an exception relates lamina's value to a tool's (`relation` in a `[[class]]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Relation {
    /// lamina skipped what it could not read: a lower page count, or outline titles that are
    /// a shorter subsequence of the tool's.
    LaminaSkips,
}

impl Relation {
    fn holds(self, ours: &Value, theirs: &Value) -> bool {
        match self {
            Self::LaminaSkips => match (ours, theirs) {
                (Value::Number(a), Value::Number(b)) => a.as_u64().zip(b.as_u64()).is_some_and(|(a, b)| a < b),
                (Value::Array(a), Value::Array(b)) => {
                    let mut rest = b.iter();
                    a.len() < b.len() && a.iter().all(|x| rest.any(|y| y == x))
                }
                _ => false,
            },
        }
    }
}

/// A `[[class]]` of deliberate differences: every disagreement on `facts` with `tools` (all
/// when empty) in a file matching `file` (a glob, `*` for any run of characters), where lamina's
/// value is `lamina` and the tool's `theirs` (either may be omitted), `relation` holds, and
/// lamina's other facts (including its `error` code) match `lamina_facts`.
#[derive(Debug)]
struct Class {
    file: String,
    facts: Vec<String>,
    tools: Vec<String>,
    lamina: Option<Value>,
    lamina_facts: BTreeMap<String, Value>,
    theirs: Option<Value>,
    relation: Option<Relation>,
    rule: String,
}

impl Class {
    fn matches(&self, file: &str, fact: &str, tool: &str, lamina: &Value, theirs: &Value) -> bool {
        let ours = lamina.get(fact).unwrap_or(&Value::Null);
        glob(&self.file, file)
            && self.lamina_facts.iter().all(|(k, v)| lamina.get(k) == Some(v))
            && self.facts.iter().any(|f| f == fact)
            && (self.tools.is_empty() || self.tools.iter().any(|t| t == tool))
            && self.lamina.as_ref().is_none_or(|v| v == ours)
            && self.theirs.as_ref().is_none_or(|v| v == theirs)
            && self.relation.is_none_or(|r| r.holds(ours, theirs))
    }
}

/// A `[[known]]` disagreement: not deliberate, tracked by an issue until lamina is fixed (like
/// an ignored flaky test, ADR 0018 §4).
#[derive(Debug)]
struct Known {
    file: String,
    facts: Vec<String>,
    tools: Vec<String>,
    /// The tracking issue, or `None` while it is [`PENDING_ISSUE`] (reported as a warning).
    issue: Option<String>,
}

/// The `issue` of a `[[known]]` entry whose tracking issue doesn't exist yet. Accepted, but every
/// run warns about it and the report lists it, so it can't pass for tracking.
const PENDING_ISSUE: &str = "PENDING";

/// Which entry excused a disagreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Excuse {
    /// An `[[exception]]` for that file (both lanes).
    File,
    /// The `[[class]]` at this index.
    Class(usize),
    /// The `[[known]]` entry at this index.
    Known(usize),
}

/// `fixtures/differential/exceptions.toml` and `full-exceptions.toml`. `[[exception]]` entries
/// (shared with the PR lanes' test) name a `file` or `files`; without `tools` they are about
/// pypdf, the PR lanes' only tool.
/// `[[class]]` and `[[known]]` entries are read by the nightly lane only. Every `[[exception]]`
/// and `[[class]]` names the ADR 0003 rule behind lamina's answer; a `[[known]]` names its issue.
#[derive(Debug, Default)]
pub(crate) struct Exceptions {
    files: BTreeSet<(String, String, String)>,
    classes: Vec<Class>,
    known: Vec<Known>,
}

fn toml_to_json(value: &toml::Value) -> Value {
    match value {
        toml::Value::String(s) => json!(s),
        toml::Value::Integer(n) => json!(n),
        toml::Value::Float(f) => json!(f),
        toml::Value::Boolean(b) => json!(b),
        toml::Value::Array(a) => Value::Array(a.iter().map(toml_to_json).collect()),
        _ => Value::Null,
    }
}

/// `*` matches any run of characters (including `/`); everything else matches itself.
fn glob(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(at) => rest = rest.get(at + part.len()..).unwrap_or_default(),
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

impl Exceptions {
    fn read(path: &Path) -> Result<Self> {
        Self::parse(&fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?)
    }

    /// Adds `other`'s entries (class and known indexes continue after this file's).
    fn merge(&mut self, other: Self) {
        self.files.extend(other.files);
        self.classes.extend(other.classes);
        self.known.extend(other.known);
    }

    /// `[[known]]` files whose tracking issue is still [`PENDING_ISSUE`].
    fn pending(&self) -> Vec<&str> {
        self.known
            .iter()
            .filter(|k| k.issue.is_none())
            .map(|k| k.file.as_str())
            .collect()
    }

    fn parse(text: &str) -> Result<Self> {
        let table: toml::Table = text.parse().context("parsing exceptions.toml")?;
        let entries = |key: &str| -> Vec<toml::Value> {
            table
                .get(key)
                .and_then(toml::Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let mut out = Self::default();
        for entry in entries("exception") {
            let e = Entry::read(&entry, "exception")?;
            e.require_rule()?;
            if e.files.is_empty() {
                bail!("every [[exception]] names its `file` or `files`: {entry:?}");
            }
            let tools = if e.tools.is_empty() {
                vec![Tool::Pypdf.name().to_owned()]
            } else {
                e.tools.clone()
            };
            for file in &e.files {
                for fact in &e.facts {
                    for tool in &tools {
                        out.files.insert((file.clone(), fact.clone(), tool.clone()));
                    }
                }
            }
        }
        for entry in entries("class") {
            let e = Entry::read(&entry, "class")?;
            e.require_rule()?;
            let relation = match entry.get("relation").and_then(toml::Value::as_str) {
                None => None,
                Some("lamina-skips") => Some(Relation::LaminaSkips),
                Some(other) => bail!("[[class]] relation {other:?}: only \"lamina-skips\""),
            };
            let file = match e.files.as_slice() {
                [] => "*".to_owned(),
                [one] => one.clone(),
                _ => bail!("a [[class]] takes one `file` glob: {entry:?}"),
            };
            out.classes.push(Class {
                file,
                facts: e.facts,
                tools: e.tools,
                lamina: entry.get("lamina").map(toml_to_json),
                lamina_facts: entry
                    .get("lamina_facts")
                    .and_then(toml::Value::as_table)
                    .map(|t| t.iter().map(|(k, v)| (k.clone(), toml_to_json(v))).collect())
                    .unwrap_or_default(),
                theirs: entry.get("theirs").map(toml_to_json),
                relation,
                rule: e.rule,
            });
        }
        for entry in entries("known") {
            let e = Entry::read(&entry, "known")?;
            let issue = entry.get("issue").and_then(toml::Value::as_str).unwrap_or_default();
            let issue = match issue {
                PENDING_ISSUE => None,
                url if url.starts_with("https://") => Some(url.to_owned()),
                _ => bail!("a [[known]] `issue` is a URL or \"{PENDING_ISSUE}\": {entry:?}"),
            };
            if e.reason.is_empty() {
                bail!("every [[known]] gives a `reason`: {entry:?}");
            }
            let [file] = e.files.as_slice() else {
                bail!("a [[known]] names exactly one `file`: {entry:?}");
            };
            out.known.push(Known {
                file: file.clone(),
                facts: e.facts,
                tools: e.tools,
                issue,
            });
        }
        Ok(out)
    }

    /// A short label for the entry behind `excuse`, for the report.
    fn describe(&self, excuse: Excuse) -> String {
        match excuse {
            Excuse::File => "exception".to_owned(),
            Excuse::Class(i) => self
                .classes
                .get(i)
                .map_or_else(String::new, |c| format!("class {}: {}", i + 1, c.rule)),
            Excuse::Known(i) => self.known.get(i).map_or_else(String::new, |k| match &k.issue {
                Some(url) => format!("known: {url}"),
                None => "known: tracking issue pending".to_owned(),
            }),
        }
    }

    /// The entry that excuses lamina's `fact` (from all of lamina's facts about `file`) against
    /// `tool`'s value `theirs`, if any.
    fn excuse(&self, file: &str, fact: &str, tool: &str, lamina: &Value, theirs: &Value) -> Option<Excuse> {
        if self
            .files
            .contains(&(file.to_owned(), fact.to_owned(), tool.to_owned()))
        {
            return Some(Excuse::File);
        }
        if let Some(i) = self
            .classes
            .iter()
            .position(|c| c.matches(file, fact, tool, lamina, theirs))
        {
            return Some(Excuse::Class(i));
        }
        self.known
            .iter()
            .position(|k| {
                k.file == file
                    && k.facts.iter().any(|f| f == fact)
                    && (k.tools.is_empty() || k.tools.iter().any(|t| t == tool))
            })
            .map(Excuse::Known)
    }
}

/// The fields every kind of entry shares.
struct Entry {
    /// `file` or `files` (exactly one of them, or neither).
    files: Vec<String>,
    facts: Vec<String>,
    /// Validated tool names (`pdfjs`, `pdfium`, `qpdf`, `pypdf`).
    tools: Vec<String>,
    rule: String,
    reason: String,
}

impl Entry {
    fn read(entry: &toml::Value, kind: &str) -> Result<Self> {
        let text = |key: &str| {
            entry
                .get(key)
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let strings = |key: &str| -> Result<Vec<String>> {
            let Some(value) = entry.get(key) else {
                return Ok(Vec::new());
            };
            let list = value
                .as_array()
                .with_context(|| format!("[[{kind}]] `{key}` must be an array: {entry:?}"))?;
            let out: Vec<String> = list.iter().filter_map(toml::Value::as_str).map(str::to_owned).collect();
            if out.is_empty() || out.len() != list.len() {
                bail!("[[{kind}]] `{key}` must be a non-empty list of strings: {entry:?}");
            }
            Ok(out)
        };
        let facts = strings("facts")?;
        if facts.is_empty() || facts.iter().any(|f| !FACTS.contains(&f.as_str())) {
            bail!("[[{kind}]] needs `facts` from {FACTS:?}: {entry:?}");
        }
        let tools = strings("tools")?;
        for tool in &tools {
            Tool::parse(tool).with_context(|| format!("[[{kind}]] {entry:?}"))?;
        }
        let files = match (entry.get("file"), entry.get("files")) {
            (None, None) => Vec::new(),
            (Some(file), None) => match file.as_str() {
                Some(f) if !f.is_empty() => vec![f.to_owned()],
                _ => bail!("[[{kind}]] `file` must be a non-empty string: {entry:?}"),
            },
            (None, Some(_)) => strings("files")?,
            (Some(_), Some(_)) => bail!("[[{kind}]] has both `file` and `files`: {entry:?}"),
        };
        Ok(Self {
            files,
            facts,
            tools,
            rule: text("rule"),
            reason: text("reason"),
        })
    }

    fn require_rule(&self) -> Result<()> {
        if !self.rule.starts_with("ADR 0003") || self.reason.is_empty() {
            bail!(
                "every exception and class names its ADR 0003 `rule` and a `reason` ({:?})",
                self.files
            );
        }
        Ok(())
    }
}

/// How lamina's answer on one fact of one file relates to the tools'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Every tool that compared the fact disagrees with lamina, and not all of them are excused.
    Failure,
    /// At least one tool agrees with lamina; the others disagree.
    Split,
    /// Every disagreeing tool is excused by an `[[exception]]` or a `[[class]]`.
    Excused,
    /// Every disagreeing tool is excused, at least one by a `[[known]]` issue.
    Known,
}

#[derive(Debug)]
struct Disagreement {
    file: String,
    fact: &'static str,
    kind: Kind,
    lamina: Value,
    /// `(tool, its value, excused)` for each disagreeing tool.
    tools: Vec<(String, Value, Option<Excuse>)>,
    agreeing: Vec<String>,
}

#[derive(Debug, Default, Clone)]
struct Tally {
    agree: usize,
    disagree: usize,
    excused: usize,
}

#[derive(Debug, Default)]
struct ToolSummary {
    files: usize,
    timeouts: usize,
    /// Files whose single-file run crashed the tool (skipped, like timeouts).
    crashes: usize,
    /// Per fact, in [`FACTS`] order.
    tallies: [Tally; 3],
}

#[derive(Debug, Default)]
struct Comparison {
    tools: BTreeMap<String, ToolSummary>,
    disagreements: Vec<Disagreement>,
    /// `file: panic|crash|timeout` where lamina itself failed to read a file: always a failure.
    lamina_errors: Vec<String>,
    /// `file (tool)` for inputs lamina read that a tool's report lacks: always a failure.
    missing: Vec<String>,
    /// `file: fact (tool)` for `[[exception]]` and `[[known]]` entries the tool now agrees on.
    stale: Vec<String>,
    /// How many disagreements each `[[class]]` and `[[known]]` entry excused.
    class_hits: BTreeMap<usize, usize>,
    known_hits: BTreeMap<usize, usize>,
}

struct RunContext {
    /// Readers that broke or reported the wrong files (the run fails).
    broken: Vec<String>,
    inputs: usize,
    external: usize,
    commit: String,
    sample: Option<usize>,
}

fn readable(facts: &Value) -> bool {
    matches!(
        facts.get("outcome").and_then(Value::as_str),
        Some("opened" | "repaired")
    )
}

/// The value compared for `fact`; outcomes lose the repaired/opened distinction for tools
/// that repair silently.
fn comparable(facts: &Value, fact: &str, reports_repair: bool) -> Value {
    let value = facts.get(fact).cloned().unwrap_or(Value::Null);
    if fact == "outcome" && !reports_repair && value == "repaired" {
        return json!("opened");
    }
    value
}

fn compare(lamina: &Report, tools: &[Report], exceptions: &Exceptions) -> Comparison {
    let mut out = Comparison::default();
    for tool in tools {
        out.tools.entry(tool.tool.clone()).or_default();
    }
    for (file, ours) in &lamina.facts {
        if let Some(error @ ("panic" | "crash" | "timeout")) = ours.get("outcome").and_then(Value::as_str) {
            out.lamina_errors.push(format!("{file}: {error}"));
            continue;
        }
        for tool in tools.iter().filter(|t| !t.facts.contains_key(file)) {
            out.missing.push(format!("{file} ({})", tool.tool));
        }
        let mut seen = BTreeSet::new();
        for (index, fact) in FACTS.into_iter().enumerate() {
            let mut agreeing = Vec::new();
            let mut disagreeing = Vec::new();
            for tool in tools {
                let Some(theirs) = tool.facts.get(file) else { continue };
                let summary = out.tools.entry(tool.tool.clone()).or_default();
                if seen.insert(tool.tool.clone()) {
                    summary.files += 1;
                    match theirs.get("outcome").and_then(Value::as_str) {
                        Some("timeout") => summary.timeouts += 1,
                        Some("crash") => summary.crashes += 1,
                        _ => {}
                    }
                }
                if matches!(theirs.get("outcome").and_then(Value::as_str), Some("timeout" | "crash"))
                    || (fact != "outcome" && !(readable(ours) && readable(theirs)))
                {
                    continue;
                }
                let Some(tally) = summary.tallies.get_mut(index) else {
                    continue;
                };
                let theirs = comparable(theirs, fact, tool.reports_repair);
                let mine = comparable(ours, fact, tool.reports_repair);
                // Exceptions match lamina's own values (a silent-repair tool still sees "repaired").
                let excuse = exceptions.excuse(file, fact, &tool.tool, ours, &theirs);
                if mine == theirs {
                    tally.agree += 1;
                    agreeing.push(tool.tool.clone());
                    if matches!(excuse, Some(Excuse::File | Excuse::Known(_))) {
                        out.stale.push(format!("{file}: {fact} ({})", tool.tool));
                    }
                } else {
                    tally.disagree += 1;
                    tally.excused += usize::from(excuse.is_some());
                    match excuse {
                        Some(Excuse::Class(i)) => *out.class_hits.entry(i).or_default() += 1,
                        Some(Excuse::Known(i)) => *out.known_hits.entry(i).or_default() += 1,
                        _ => {}
                    }
                    disagreeing.push((tool.tool.clone(), theirs, excuse));
                }
            }
            if disagreeing.is_empty() {
                continue;
            }
            let kind = if disagreeing.iter().all(|(_, _, excuse)| excuse.is_some()) {
                if disagreeing.iter().any(|(_, _, e)| matches!(e, Some(Excuse::Known(_)))) {
                    Kind::Known
                } else {
                    Kind::Excused
                }
            } else if agreeing.is_empty() {
                Kind::Failure
            } else {
                Kind::Split
            };
            out.disagreements.push(Disagreement {
                file: file.clone(),
                fact,
                kind,
                lamina: ours.get(fact).cloned().unwrap_or(Value::Null),
                tools: disagreeing,
                agreeing,
            });
        }
    }
    out
}

/// `agree / compared` as a percentage with one decimal, or `-` when nothing was compared.
fn rate(tally: &Tally) -> String {
    let compared = tally.agree + tally.disagree;
    if compared == 0 {
        return "-".to_owned();
    }
    #[expect(clippy::cast_precision_loss, reason = "file counts are far below 2^52")]
    let percent = 100.0 * tally.agree as f64 / compared as f64;
    format!("{percent:.1}% ({}/{compared})", tally.agree)
}

/// A JSON value for a Markdown table cell: compact, `|` escaped, at most 80 characters.
fn cell(value: &Value) -> String {
    let text = value.to_string();
    let text = if text.chars().count() > 80 {
        format!("{}…", text.chars().take(79).collect::<String>())
    } else {
        text
    };
    util::md_cell(&text)
}

impl Comparison {
    fn failures(&self) -> impl Iterator<Item = &Disagreement> {
        self.disagreements.iter().filter(|d| d.kind == Kind::Failure)
    }

    fn failed(&self) -> bool {
        self.failures().next().is_some() || !self.lamina_errors.is_empty() || !self.missing.is_empty()
    }

    fn count(&self, kind: Kind) -> usize {
        self.disagreements.iter().filter(|d| d.kind == kind).count()
    }

    fn summary_line(&self) -> String {
        format!(
            "differential: {} failures, {} lamina errors, {} missing, {} split, {} excused, {} known, {} stale exceptions",
            self.count(Kind::Failure),
            self.lamina_errors.len(),
            self.missing.len(),
            self.count(Kind::Split),
            self.count(Kind::Excused),
            self.count(Kind::Known),
            self.stale.len()
        )
    }

    fn markdown(&self, lamina: &Report, tools: &[Report], exceptions: &Exceptions, context: &RunContext) -> String {
        let mut md = String::new();
        let _ = writeln!(md, "## Nightly differential\n");
        let _ = writeln!(
            md,
            "{} inputs ({} external, {} mutants{}) at `{}`. {}\n",
            context.inputs,
            context.external,
            context.inputs - context.external,
            context.sample.map_or(String::new(), |n| format!(", sampled to {n}")),
            context.commit,
            self.summary_line()
        );
        let _ = writeln!(
            md,
            "Agreement is lamina's: page count and outline are compared only when both readers \
             open the file; pdf.js and PDFium don't report repairs, so their outcome is \
             readable/failed. A failure is a fact where every tool disagrees with lamina and the \
             difference is not in `fixtures/differential/exceptions.toml`.\n"
        );
        let _ = writeln!(
            md,
            "| Tool | Version | Files | Timeouts / crashes | Outcome | Page count | Outline titles |"
        );
        let _ = writeln!(md, "| --- | --- | --- | --- | --- | --- | --- |");
        let _ = writeln!(
            md,
            "| lamina | {} (`{}`) | {} | {} | - | - | - |",
            util::md_cell(&lamina.version),
            context.commit.get(..12).unwrap_or(&context.commit),
            lamina.facts.len(),
            self.lamina_errors.len()
        );
        for tool in tools {
            let Some(summary) = self.tools.get(&tool.tool) else {
                continue;
            };
            let rates: Vec<String> = summary.tallies.iter().map(rate).collect();
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} / {} | {} |",
                util::md_cell(&tool.tool),
                util::md_cell(&tool.version),
                summary.files,
                summary.timeouts,
                summary.crashes,
                rates.join(" | ")
            );
        }
        if !context.broken.is_empty() {
            let _ = writeln!(md, "\n### Readers that failed (the run fails)\n");
            for why in &context.broken {
                let _ = writeln!(md, "- {}", util::md_cell(why));
            }
        }
        self.markdown_details(&mut md, exceptions);
        let pending = exceptions.pending();
        if !pending.is_empty() {
            let _ = writeln!(
                md,
                "\n### `[[known]]` entries without a tracking issue ({})\n\nOpen the issue and put its URL in \
                 `issue`.\n",
                pending.len()
            );
            for file in pending {
                let _ = writeln!(md, "- `{}`", util::md_cell(file));
            }
        }
        if !self.stale.is_empty() {
            let _ = writeln!(
                md,
                "\n### Exceptions the tool now agrees with ({})\n\nRemove them from exceptions.toml \
                 unless another lane still needs them.\n",
                self.stale.len()
            );
            for entry in self.stale.iter().take(MAX_ROWS) {
                let _ = writeln!(md, "- `{}`", util::md_cell(entry));
            }
        }
        md
    }

    /// Panics, the disagreement tables and the class and known-issue counts.
    fn markdown_details(&self, md: &mut String, exceptions: &Exceptions) {
        for (title, list) in [
            ("lamina failed to read (panic, crash or timeout)", &self.lamina_errors),
            ("Missing from a tool's report", &self.missing),
        ] {
            if !list.is_empty() {
                let _ = writeln!(md, "\n### {title} ({})\n", list.len());
                for entry in list.iter().take(MAX_ROWS) {
                    let _ = writeln!(md, "- `{}`", util::md_cell(entry));
                }
            }
        }
        for (kind, title) in [
            (Kind::Failure, "Failures: every tool disagrees with lamina"),
            (Kind::Split, "Split: some tools agree with lamina, some don't"),
            (Kind::Known, "Known: tracked by an issue"),
            (Kind::Excused, "Excused by exceptions.toml"),
        ] {
            let rows: Vec<&Disagreement> = self.disagreements.iter().filter(|d| d.kind == kind).collect();
            if rows.is_empty() {
                continue;
            }
            let _ = writeln!(md, "\n### {title} ({})\n", rows.len());
            if rows.len() > MAX_ROWS {
                let _ = writeln!(md, "First {MAX_ROWS}; report.json has every row.\n");
            }
            let _ = writeln!(md, "| File | Fact | lamina | Disagreeing tools | Agreeing |");
            let _ = writeln!(md, "| --- | --- | --- | --- | --- |");
            for d in rows.iter().take(MAX_ROWS) {
                let theirs: Vec<String> = d
                    .tools
                    .iter()
                    .map(|(t, v, excuse)| match excuse {
                        Some(e) => format!("{t}: {} ({})", cell(v), util::md_cell(&exceptions.describe(*e))),
                        None => format!("{t}: {}", cell(v)),
                    })
                    .collect();
                let _ = writeln!(
                    md,
                    "| `{}` | {} | {} | {} | {} |",
                    util::md_cell(&d.file),
                    d.fact,
                    cell(&d.lamina),
                    theirs.join("<br>"),
                    d.agreeing.join(", ")
                );
            }
        }
        if !exceptions.classes.is_empty() || !exceptions.known.is_empty() {
            let _ = writeln!(md, "\n### Classes and known issues\n");
            let _ = writeln!(md, "| Entry | Disagreements excused |\n| --- | --- |");
            for i in 0..exceptions.classes.len() {
                let hits = self.class_hits.get(&i).copied().unwrap_or_default();
                let _ = writeln!(
                    md,
                    "| {} | {hits} |",
                    util::md_cell(&exceptions.describe(Excuse::Class(i)))
                );
            }
            for i in 0..exceptions.known.len() {
                let hits = self.known_hits.get(&i).copied().unwrap_or_default();
                let file = exceptions.known.get(i).map_or("", |k| k.file.as_str());
                let _ = writeln!(
                    md,
                    "| `{}` ({}) | {hits} |",
                    util::md_cell(file),
                    util::md_cell(&exceptions.describe(Excuse::Known(i)))
                );
            }
        }
    }

    fn json(&self, lamina: &Report, tools: &[Report], exceptions: &Exceptions, context: &RunContext) -> Value {
        let tool_rows: Vec<Value> = tools
            .iter()
            .map(|tool| {
                let summary = self.tools.get(&tool.tool);
                let facts: serde_json::Map<String, Value> = FACTS
                    .iter()
                    .zip(summary.map(|s| s.tallies.clone()).unwrap_or_default())
                    .map(|(fact, t)| {
                        (
                            (*fact).to_owned(),
                            json!({"agree": t.agree, "disagree": t.disagree, "excused": t.excused}),
                        )
                    })
                    .collect();
                json!({
                    "tool": tool.tool,
                    "version": tool.version,
                    "reports_repair": tool.reports_repair,
                    "files": summary.map_or(0, |s| s.files),
                    "timeouts": summary.map_or(0, |s| s.timeouts),
                    "crashes": summary.map_or(0, |s| s.crashes),
                    "facts": facts,
                })
            })
            .collect();
        let disagreements: Vec<Value> = self
            .disagreements
            .iter()
            .map(|d| {
                json!({
                    "file": d.file,
                    "fact": d.fact,
                    "kind": format!("{:?}", d.kind).to_lowercase(),
                    "lamina": d.lamina,
                    "disagreeing": d.tools.iter().map(|(t, v, e)| json!({"tool": t, "value": v, "excused": e.map(|e| exceptions.describe(e))})).collect::<Vec<_>>(),
                    "agreeing": d.agreeing,
                })
            })
            .collect();
        json!({
            "commit": context.commit,
            "inputs": context.inputs,
            "external": context.external,
            "sample": context.sample,
            "lamina": {"version": lamina.version, "files": lamina.facts.len()},
            "tools": tool_rows,
            "lamina_errors": self.lamina_errors,
            "missing": self.missing,
            "broken_readers": context.broken,
            "disagreements": disagreements,
            "stale_exceptions": self.stale,
            "class_hits": (0..exceptions.classes.len()).map(|i| self.class_hits.get(&i).copied().unwrap_or_default()).collect::<Vec<_>>(),
            "known_hits": (0..exceptions.known.len()).map(|i| self.known_hits.get(&i).copied().unwrap_or_default()).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
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
        let skips = Relation::LaminaSkips;
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
                     issue = \"https://github.com/laudebugs/lamina/issues/1\"\nreason = \"r\"\n";
        let lamina = report("lamina", true, &json!({"a.pdf": opened(2)}));
        let tools = [report("qpdf", true, &json!({"a.pdf": opened(3)}))];
        let c = compare(&lamina, &tools, &Exceptions::parse(known).unwrap());
        assert!(!c.failed() && kinds(&c) == [("a.pdf", "page_count", Kind::Known)]);
    }

    #[test]
    fn known_entries_should_require_an_issue_url() {
        let known = "[[known]]\nfile = \"a.pdf\"\nfacts = [\"page_count\"]\nissue = \"#1\"\nreason = \"r\"\n";
        assert!(Exceptions::parse(known).is_err());
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
