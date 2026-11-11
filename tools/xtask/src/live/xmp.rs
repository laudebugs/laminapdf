//! `cargo xtask differential --live --xmp` (ADR 0018 tier 6): the XMP values
//! `laminapdf_model::xmp::Packet` reads against exiftool's, every night.
//!
//! The inputs are the external corpus and the committed PDFs in [`LOCAL_DIRS`] (which hold the
//! XMP-bearing goldens: `metadata.pdf`, the PDF/A and PDF/UA examples). lamina's side is the
//! `xmp-facts` example (`crates/laminapdf-model/examples/`) under [`runner`] like the other
//! live readers; exiftool (pinned in `doctor.rs`, an external process only, ADR 0019) reads
//! every file in one `exiftool -j -G1 -n -XMP:all` run, whose `XMP-<group>:<Tag>` keys
//! [`FIELDS`] maps to the getters' names. [`compare_file`] compares them field by field after
//! [`normalise`]: text trimmed, `dc:creator` as a list, dates as instants written one way (exiftool
//! prints `2026:10:28 14:30:00+01:00`, lamina `2026-10-28T14:30:00+01:00`; fractions of a second
//! dropped, missing parts at their XMP defaults), the PDF/A and PDF/UA parts as numbers.
//!
//! A field where the two differ fails the run unless a `[[known]]` entry of [`EXCEPTIONS_FILE`]
//! excuses it (`file`, a glob; `fields`, default all; `issue`, its URL or `PENDING`; `reason`);
//! entries that excused nothing are listed as stale. Files lamina fails to open (the structural
//! differential reports those) and files exiftool reports an error for are counted and left
//! out; a lamina panic, crash or hang always fails. The report goes to
//! `target/nightly/differential-xmp/`, its summary to the job summary.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::exceptions::glob;
use super::{LiveArgs, MAX_ROWS, PENDING_ISSUE, Report, example_binary, run_readers};
use crate::doctor::Probe;
use crate::workspace::Workspace;
use crate::{corpus, runner, step, util};

/// The committed folders whose PDFs join the external corpus (not their subfolders).
const LOCAL_DIRS: [&str; 3] = ["fixtures/corpus", "fixtures/golden", "fixtures/golden/examples"];

/// The `[[known]]` disagreements; a missing file excuses nothing.
const EXCEPTIONS_FILE: &str = "fixtures/differential/xmp-exceptions.toml";

/// How a field's values are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    /// An ordered list of texts (`dc:creator`); exiftool prints a single item as a string.
    List,
    Date,
    /// A small number (`pdfaid:part`, `pdfuaid:part`).
    Part,
}

/// lamina's field name (`xmp-facts`' keys, the getters' names), exiftool's `-G1` key, and how
/// they compare.
const FIELDS: [(&str, &str, Kind); 14] = [
    ("title", "XMP-dc:Title", Kind::Text),
    ("authors", "XMP-dc:Creator", Kind::List),
    ("subject", "XMP-dc:Description", Kind::Text),
    ("keywords", "XMP-pdf:Keywords", Kind::Text),
    ("creator_tool", "XMP-xmp:CreatorTool", Kind::Text),
    ("producer", "XMP-pdf:Producer", Kind::Text),
    ("create_date", "XMP-xmp:CreateDate", Kind::Date),
    ("modify_date", "XMP-xmp:ModifyDate", Kind::Date),
    ("metadata_date", "XMP-xmp:MetadataDate", Kind::Date),
    ("document_id", "XMP-xmpMM:DocumentID", Kind::Text),
    ("instance_id", "XMP-xmpMM:InstanceID", Kind::Text),
    ("pdfa_part", "XMP-pdfaid:Part", Kind::Part),
    ("pdfa_conformance", "XMP-pdfaid:Conformance", Kind::Text),
    ("pdfua_part", "XMP-pdfuaid:Part", Kind::Part),
];

/// exiftool's key for the error it reports on a file it can't read.
const EXIFTOOL_ERROR: &str = "ExifTool:Error";

/// A field lamina and exiftool read differently: the values as each wrote them (`null`: none).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Mismatch {
    pub(super) file: String,
    pub(super) field: &'static str,
    pub(super) lamina: Value,
    pub(super) exiftool: Value,
}

/// A value's comparable form: `None` when the field is absent or empty.
fn normalise(kind: Kind, value: &Value) -> Option<Value> {
    match kind {
        Kind::Text => text(value).filter(|s| !s.is_empty()).map(Value::String),
        Kind::List => {
            let items: Vec<Value> = match value {
                Value::Array(items) => items.iter().filter_map(text).map(Value::String).collect(),
                other => text(other).into_iter().map(Value::String).collect(),
            };
            (!items.is_empty()).then_some(Value::Array(items))
        }
        Kind::Date => {
            let raw = text(value).filter(|s| !s.is_empty())?;
            Some(Value::String(canonical_date(&raw).unwrap_or(raw)))
        }
        Kind::Part => {
            let raw = text(value)?;
            Some(raw.parse::<u64>().map_or(Value::String(raw), Value::from))
        }
    }
}

/// A scalar as trimmed text: exiftool prints values that look like numbers as JSON numbers.
fn text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.trim().to_owned()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

/// An XMP date (ISO 8601's profile: `YYYY[-MM[-DD[THH:MM[:SS[.s]][TZD]]]]`) or exiftool's
/// printing of one (`:` between the date's parts, a space before the time) as
/// `YYYY-MM-DDTHH:MM:SS` plus `Z` or `±HH:MM` when it has an offset: missing parts take their
/// defaults (month and day 1, time 0), fractions of a second are dropped. `None` when it is
/// not such a date.
fn canonical_date(text: &str) -> Option<String> {
    let (date, time) = match text.split_once(['T', ' ']) {
        Some((date, time)) => (date, Some(time)),
        None => (text, None),
    };
    let parts: Vec<u32> = date.split(['-', ':']).map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let (&year, rest) = parts.split_first()?;
    if rest.len() > 2 {
        return None;
    }
    let month = rest.first().copied().unwrap_or(1);
    let day = rest.get(1).copied().unwrap_or(1);
    let (mut clock, mut zone) = ([0_u32; 3], String::new());
    if let Some(time) = time {
        let (hms, tz) = time.find(['Z', '+', '-']).map_or((time, ""), |at| time.split_at(at));
        let hms = hms.split_once('.').map_or(hms, |(whole, _)| whole);
        let fields: Vec<u32> = hms.split(':').map(|p| p.parse().ok()).collect::<Option<_>>()?;
        if !(2..=3).contains(&fields.len()) {
            return None;
        }
        for (slot, value) in clock.iter_mut().zip(fields) {
            *slot = value;
        }
        zone = canonical_zone(tz)?;
    }
    let [hour, minute, second] = clock;
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{zone}"
    ))
}

/// `Z`, `+HH:MM`, `+HHMM` or `+HH` (and `-`) as `Z` for no offset, else `±HH:MM`; empty stays empty.
fn canonical_zone(tz: &str) -> Option<String> {
    let Some(sign) = tz.chars().next() else {
        return Some(String::new());
    };
    if tz == "Z" {
        return Some("Z".to_owned());
    }
    let digits: String = tz.get(1..)?.chars().filter(|c| *c != ':').collect();
    let (hours, minutes) = match digits.len() {
        2 => (digits.parse::<u32>().ok()?, 0),
        4 => (
            digits.get(..2)?.parse::<u32>().ok()?,
            digits.get(2..)?.parse::<u32>().ok()?,
        ),
        _ => return None,
    };
    match (sign, hours, minutes) {
        (_, 0, 0) => Some("Z".to_owned()),
        ('+' | '-', _, _) => Some(format!("{sign}{hours:02}:{minutes:02}")),
        _ => None,
    }
}

/// exiftool's value for `key`; for a language alternative (`XMP-dc:Title`) with no
/// `x-default` entry, its one other language (`XMP-dc:Title-de`), as lamina reads it.
fn exiftool_value<'a>(entry: &'a Value, key: &str) -> &'a Value {
    if let Some(value) = entry.get(key) {
        return value;
    }
    let prefix = format!("{key}-");
    let mut others = entry
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(k, _)| k.starts_with(&prefix));
    match (others.next(), others.next()) {
        (Some((_, value)), None) => value,
        _ => &Value::Null,
    }
}

/// The fields of one file that lamina (an `xmp-facts` entry) and exiftool (one object of its
/// `-j` array) read differently, and how many fields either side has (the compared ones).
pub(super) fn compare_file(file: &str, lamina: &Value, exiftool: &Value) -> (Vec<Mismatch>, usize) {
    let fields = lamina.get("fields").unwrap_or(&Value::Null);
    let mut mismatches = Vec::new();
    let mut compared = 0;
    for (name, key, kind) in FIELDS {
        let ours = fields.get(name).unwrap_or(&Value::Null);
        let theirs = exiftool_value(exiftool, key);
        let (a, b) = (normalise(kind, ours), normalise(kind, theirs));
        if a.is_none() && b.is_none() {
            continue;
        }
        compared += 1;
        if a != b {
            mismatches.push(Mismatch {
                file: file.to_owned(),
                field: name,
                lamina: ours.clone(),
                exiftool: theirs.clone(),
            });
        }
    }
    (mismatches, compared)
}

/// A `[[known]]` entry of [`EXCEPTIONS_FILE`].
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Known {
    file: String,
    /// Empty: every field.
    fields: Vec<String>,
    issue: String,
}

impl Known {
    fn excuses(&self, m: &Mismatch) -> bool {
        glob(&self.file, &m.file) && (self.fields.is_empty() || self.fields.iter().any(|f| f == m.field))
    }

    fn label(&self) -> String {
        format!("{} [{}] ({})", self.file, self.fields.join(", "), self.issue)
    }
}

/// The `[[known]]` entries of an exceptions file's text; anything else in it is an error.
pub(super) fn parse_exceptions(text: &str) -> Result<Vec<Known>> {
    let table: toml::Table = toml::from_str(text)?;
    if let Some(other) = table.keys().find(|k| *k != "known") {
        bail!("unknown table {other:?}: only [[known]] entries");
    }
    let Some(entries) = table.get("known") else {
        return Ok(Vec::new());
    };
    let entries = entries.as_array().context("`known` must be [[known]] entries")?;
    entries
        .iter()
        .map(|entry| {
            let entry = entry.as_table().context("a [[known]] entry must be a table")?;
            if let Some(other) = entry
                .keys()
                .find(|k| !["file", "fields", "issue", "reason"].contains(&k.as_str()))
            {
                bail!("[[known]]: unknown key {other:?}");
            }
            let text = |key: &str| {
                entry
                    .get(key)
                    .and_then(toml::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_owned)
                    .with_context(|| format!("[[known]] without `{key}`: {entry:?}"))
            };
            let fields: Vec<String> = match entry.get("fields") {
                None => Vec::new(),
                Some(v) => v
                    .as_array()
                    .context("`fields` must be an array")?
                    .iter()
                    .map(|f| f.as_str().map(str::to_owned).context("`fields` holds strings"))
                    .collect::<Result<_>>()?,
            };
            if let Some(bad) = fields.iter().find(|f| !FIELDS.iter().any(|(n, _, _)| n == f)) {
                bail!("[[known]]: unknown field {bad:?}");
            }
            text("reason")?;
            Ok(Known {
                file: text("file")?,
                fields,
                issue: text("issue")?,
            })
        })
        .collect()
}

/// The comparison of a whole run.
#[derive(Debug, Default)]
pub(super) struct Comparison {
    /// Mismatches with the index of the `[[known]]` entry that excused each, if one did.
    pub(super) mismatches: Vec<(Mismatch, Option<usize>)>,
    /// Files both read and compared, and those where either side has a field.
    pub(super) files: usize,
    pub(super) with_xmp: usize,
    /// Fields compared and fields that agreed.
    pub(super) fields: usize,
    /// Files lamina failed to open, by name with its error code.
    pub(super) lamina_failed: BTreeMap<String, String>,
    /// Files lamina panicked, crashed or hung on (always failures), with the outcome.
    pub(super) lamina_errors: BTreeMap<String, String>,
    /// Files exiftool reported an error for, or left out of its output.
    pub(super) exiftool_errors: BTreeMap<String, String>,
    /// `[[known]]` entries that excused nothing.
    pub(super) stale: Vec<String>,
}

impl Comparison {
    fn unexcused(&self) -> impl Iterator<Item = &Mismatch> {
        self.mismatches.iter().filter(|(_, e)| e.is_none()).map(|(m, _)| m)
    }

    pub(super) fn failed(&self) -> bool {
        self.unexcused().next().is_some() || !self.lamina_errors.is_empty()
    }
}

/// Compares lamina's facts (by input name) with exiftool's entries (by input name).
pub(super) fn compare(
    lamina: &BTreeMap<String, Value>,
    exiftool: &BTreeMap<String, Value>,
    known: &[Known],
) -> Comparison {
    let mut c = Comparison::default();
    let mut used = vec![false; known.len()];
    for (name, ours) in lamina {
        match ours.get("outcome").and_then(Value::as_str) {
            Some("opened") => {}
            Some("failed") => {
                let code = ours.get("error").and_then(Value::as_str).unwrap_or("unknown");
                c.lamina_failed.insert(name.clone(), code.to_owned());
                continue;
            }
            other => {
                c.lamina_errors
                    .insert(name.clone(), other.unwrap_or("no outcome").to_owned());
                continue;
            }
        }
        let Some(theirs) = exiftool.get(name) else {
            c.exiftool_errors
                .insert(name.clone(), "not in ExifTool's output".to_owned());
            continue;
        };
        if let Some(error) = theirs.get(EXIFTOOL_ERROR) {
            c.exiftool_errors.insert(name.clone(), text(error).unwrap_or_default());
            continue;
        }
        let (mismatches, compared) = compare_file(name, ours, theirs);
        c.files += 1;
        c.with_xmp += usize::from(compared > 0);
        c.fields += compared;
        for m in mismatches {
            let excuse = known.iter().position(|k| k.excuses(&m));
            if let Some(slot) = excuse.and_then(|i| used.get_mut(i)) {
                *slot = true;
            }
            c.mismatches.push((m, excuse));
        }
    }
    c.stale = known
        .iter()
        .zip(&used)
        .filter(|(_, u)| !**u)
        .map(|(k, _)| k.label())
        .collect();
    c
}

/// What the report says about the run besides the comparison.
struct RunInfo {
    inputs: usize,
    commit: String,
    sample: Option<usize>,
    lamina_version: String,
    exiftool_version: String,
    broken: Vec<String>,
}

impl Comparison {
    fn summary_line(&self) -> String {
        let unexcused = self.unexcused().count();
        format!(
            "XMP differential: {} files compared ({} with XMP), {} fields, {} agree, {} differ ({} unexcused); \
             skipped {} lamina couldn't open and {} ExifTool couldn't read; {} lamina crashes",
            self.files,
            self.with_xmp,
            self.fields,
            self.fields - self.mismatches.len(),
            self.mismatches.len(),
            unexcused,
            self.lamina_failed.len(),
            self.exiftool_errors.len(),
            self.lamina_errors.len()
        )
    }

    fn markdown(&self, known: &[Known], info: &RunInfo) -> String {
        let mut md = String::from("## XMP differential (lamina vs ExifTool)\n\n");
        let _ = writeln!(
            md,
            "{} inputs{} at `{}`; lamina {}, ExifTool {}.\n\n{}\n",
            info.inputs,
            info.sample.map(|n| format!(" (sample of {n})")).unwrap_or_default(),
            info.commit,
            info.lamina_version,
            info.exiftool_version,
            self.summary_line()
        );
        for broken in &info.broken {
            let _ = writeln!(md, "- **Reader broken:** {broken}");
        }
        for (name, outcome) in &self.lamina_errors {
            let _ = writeln!(md, "- **lamina {outcome}:** `{name}`");
        }
        let rows: Vec<_> = self.mismatches.iter().take(MAX_ROWS).collect();
        if !rows.is_empty() {
            // The values are text from the files: they stay in report.json, out of the job summary,
            // which people and agents read (the text differential keeps page text out the same way).
            md.push_str(
                "\nThe values each reader found are in report.json, not here: they are text from the files.\n\n\
                 | File | Field | Excused by |\n| --- | --- | --- |\n",
            );
            for (m, excuse) in rows {
                let by = excuse
                    .and_then(|i| known.get(i))
                    .map_or_else(|| "**unexcused**".to_owned(), Known::label);
                let _ = writeln!(md, "| `{}` | {} | {by} |", util::md_cell(&m.file), m.field);
            }
            if self.mismatches.len() > MAX_ROWS {
                let _ = writeln!(md, "\n{} more in report.json.", self.mismatches.len() - MAX_ROWS);
            }
        }
        for stale in &self.stale {
            let _ = writeln!(md, "- Stale `[[known]]` entry (excused nothing): {stale}");
        }
        md
    }

    fn json(&self, known: &[Known], info: &RunInfo) -> Value {
        json!({
            "inputs": info.inputs,
            "commit": info.commit,
            "sample": info.sample,
            "lamina_version": info.lamina_version,
            "exiftool_version": info.exiftool_version,
            "broken_readers": info.broken,
            "files": self.files,
            "with_xmp": self.with_xmp,
            "fields": self.fields,
            "lamina_failed": self.lamina_failed,
            "lamina_errors": self.lamina_errors,
            "exiftool_errors": self.exiftool_errors,
            "stale": self.stale,
            "mismatches": self.mismatches.iter().map(|(m, excuse)| json!({
                "file": m.file,
                "field": m.field,
                "lamina": m.lamina,
                "exiftool": m.exiftool,
                "excused_by": excuse.and_then(|i| known.get(i)).map(Known::label),
            })).collect::<Vec<_>>(),
        })
    }
}

/// The PDFs directly in [`LOCAL_DIRS`], named by their path from the repository root.
fn local_inputs(root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut out = Vec::new();
    for dir in LOCAL_DIRS {
        for entry in fs::read_dir(root.join(dir)).with_context(|| format!("reading {dir}"))? {
            let path = entry?.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "pdf") {
                let file = path.file_name().context("file name")?.to_string_lossy().into_owned();
                out.push((format!("{dir}/{file}"), path));
            }
        }
    }
    Ok(out)
}

/// exiftool's entry per input name, from one `exiftool -j` run over every input (its output
/// and messages kept in `out/`). A file exiftool can't read still gets an entry, with its
/// error, and makes exiftool exit 1; only output that isn't its JSON array fails.
fn run_exiftool(inputs: &[(String, PathBuf)], out: &Path) -> Result<BTreeMap<String, Value>> {
    let names: BTreeMap<String, &str> = inputs
        .iter()
        .map(|(name, path)| (path.to_string_lossy().into_owned(), name.as_str()))
        .collect();
    let list = out.join("exiftool-files.txt");
    fs::write(&list, names.keys().fold(String::new(), |s, p| s + p + "\n"))?;
    let output = Command::new(crate::program_path("exiftool"))
        .args([
            "-j",
            "-G1",
            "-n",
            "-XMP:all",
            "-ExifTool:Error",
            "-charset",
            "filename=utf8",
            "-@",
        ])
        .arg(&list)
        .output()
        .context("running exiftool")?;
    fs::write(out.join("exiftool.json"), &output.stdout)?;
    fs::write(out.join("exiftool.log"), &output.stderr)?;
    let entries: Vec<Value> = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("exiftool ({}) printed no JSON array; see exiftool.log", output.status))?;
    let mut by_name = BTreeMap::new();
    for entry in entries {
        let source = entry.get("SourceFile").and_then(Value::as_str).unwrap_or_default();
        if let Some(name) = names.get(source) {
            by_name.insert((*name).to_owned(), entry);
        }
    }
    Ok(by_name)
}

/// `cargo xtask differential --live --xmp …`.
pub(super) fn run(ws: &Workspace, args: &LiveArgs) -> Result<()> {
    let path = ws.root.join(EXCEPTIONS_FILE);
    let known = match fs::read_to_string(&path) {
        Ok(text) => parse_exceptions(&text).with_context(|| format!("reading {EXCEPTIONS_FILE}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e).with_context(|| format!("reading {EXCEPTIONS_FILE}")),
    };
    let tool = crate::doctor::exiftool();
    let exiftool_version = match tool.probe(ws) {
        Probe::Ok(version) => version,
        probe => bail!("{}", tool.problem(&probe)),
    };
    let out = ws.root.join("target/nightly/differential-xmp");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    fs::create_dir_all(&out)?;
    let mut inputs = corpus::verified_inputs(ws, &args.sources)?;
    if args.sources.is_empty() {
        inputs.extend(local_inputs(&ws.root)?);
    }
    inputs.sort();
    if let Some(n) = args.sample {
        inputs = super::sample(inputs, n);
    }
    if inputs.is_empty() {
        bail!("no inputs: nothing would be compared (check --source and --sample)");
    }
    step(&format!("XMP differential: {} inputs", inputs.len()));
    ws.cargo(&[
        "build",
        "--release",
        "--locked",
        "-q",
        "-p",
        "laminapdf-model",
        "--example",
        "xmp-facts",
    ])?;
    let readers = [runner::Reader {
        name: "lamina",
        program: example_binary(ws, "xmp-facts").into_os_string(),
        args: Vec::new(),
        dir: ws.root.clone(),
        reports_repair: false,
    }];
    let (reports, broken) = run_readers(&readers, &inputs, &out)?;
    let lamina: Report = reports.into_iter().next().context("no lamina report")?;
    let exiftool = run_exiftool(&inputs, &out)?;
    let comparison = compare(&lamina.facts, &exiftool, &known);
    let info = RunInfo {
        inputs: inputs.len(),
        commit: util::git_stdout(&ws.root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned()),
        sample: args.sample,
        lamina_version: lamina.version,
        exiftool_version,
        broken,
    };
    let markdown = comparison.markdown(&known, &info);
    fs::write(out.join("report.md"), &markdown)?;
    fs::write(
        out.join("report.json"),
        serde_json::to_string_pretty(&comparison.json(&known, &info))? + "\n",
    )?;
    util::summary(&markdown)?;
    println!("{}", comparison.summary_line());
    for k in known.iter().filter(|k| k.issue == PENDING_ISSUE) {
        println!(
            "::warning::{EXCEPTIONS_FILE}: [[known]] {} has no tracking issue yet",
            k.label()
        );
    }
    step(&format!("Report: {}", out.join("report.md").display()));
    if !info.broken.is_empty() {
        bail!("readers failed: {}; see the report", info.broken.join("; "));
    }
    if comparison.failed() {
        bail!(
            "lamina's XMP disagrees with ExifTool on {} fields, or lamina failed to read {} files (panic, \
             crash or timeout), and {EXCEPTIONS_FILE} doesn't excuse them; see the report",
            comparison.unexcused().count(),
            comparison.lamina_errors.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-written, not captured: exiftool's output in the shape `exiftool -j -G1 -n -XMP:all`
    /// prints, and the report `xmp-facts` writes for the same files.
    const EXIFTOOL: &str = include_str!("xmp/exiftool-sample.json");
    const LAMINA: &str = include_str!("xmp/lamina-sample.json");

    fn samples() -> (BTreeMap<String, Value>, BTreeMap<String, Value>) {
        let entries: Vec<Value> = serde_json::from_str(EXIFTOOL).unwrap();
        let exiftool = entries
            .into_iter()
            .map(|e| (e["SourceFile"].as_str().unwrap().to_owned(), e))
            .collect();
        let lamina: Value = serde_json::from_str(LAMINA).unwrap();
        let lamina = lamina["facts"].as_object().unwrap().clone().into_iter().collect();
        (lamina, exiftool)
    }

    fn mismatched(c: &Comparison) -> Vec<(&str, &str)> {
        c.mismatches.iter().map(|(m, _)| (m.file.as_str(), m.field)).collect()
    }

    #[test]
    fn samples_should_differ_only_where_written_to() {
        let (lamina, exiftool) = samples();
        let c = compare(&lamina, &exiftool, &[]);
        assert_eq!(
            (mismatched(&c), c.files, c.with_xmp, c.fields),
            (vec![("differs.pdf", "title"), ("differs.pdf", "modify_date")], 4, 2, 14)
        );
    }

    #[test]
    fn samples_should_count_what_was_left_out() {
        let (lamina, exiftool) = samples();
        let c = compare(&lamina, &exiftool, &[]);
        let keys = |m: &BTreeMap<String, String>| m.keys().cloned().collect::<Vec<_>>();
        assert_eq!(
            (keys(&c.lamina_failed), keys(&c.lamina_errors), keys(&c.exiftool_errors)),
            (
                vec!["broken.pdf".to_owned()],
                vec!["panics.pdf".to_owned()],
                vec!["unreadable.pdf".to_owned()]
            )
        );
    }

    #[test]
    fn known_entry_should_excuse_its_field_and_report_the_rest() {
        let (lamina, exiftool) = samples();
        let known = parse_exceptions(
            "[[known]]\nfile = \"differs*\"\nfields = [\"title\"]\nissue = \"PENDING\"\nreason = \"r\"\n\
             [[known]]\nfile = \"nowhere.pdf\"\nissue = \"PENDING\"\nreason = \"r\"\n",
        )
        .unwrap();
        let c = compare(&lamina, &exiftool, &known);
        let unexcused: Vec<&str> = c.unexcused().map(|m| m.field).collect();
        assert_eq!((unexcused, c.stale.len()), (vec!["modify_date"], 1));
    }

    #[test]
    fn exceptions_should_refuse_unknown_fields() {
        let error = parse_exceptions("[[known]]\nfile = \"a\"\nfields = [\"tilte\"]\nissue = \"x\"\nreason = \"r\"\n");
        assert!(error.unwrap_err().to_string().contains("tilte"));
    }

    #[test]
    fn dates_should_agree_across_both_printings() {
        let pairs = [
            ("2026:10:28 14:30:00+01:00", "2026-10-28T14:30:00+01:00"),
            ("2026:10:28 14:30:00.25Z", "2026-10-28T14:30:00Z"),
            ("2026:10:28 14:30+00:00", "2026-10-28T14:30:00Z"),
            ("2026:10:28", "2026-10-28T00:00:00"),
            ("2026:10", "2026-10-01T00:00:00"),
            ("2026:10:28 14:30:00-0530", "2026-10-28T14:30:00-05:30"),
        ];
        let differ: Vec<_> = pairs
            .iter()
            .filter(|(a, b)| canonical_date(a) != canonical_date(b) || canonical_date(a).is_none())
            .collect();
        assert!(differ.is_empty(), "{differ:?}");
    }

    #[test]
    fn dates_should_keep_offsets_apart() {
        assert_ne!(
            canonical_date("2026:10:28 14:30:00+01:00"),
            canonical_date("2026:10:28 14:30:00")
        );
    }

    #[test]
    fn values_should_compare_as_their_kind() {
        let cases = [
            (Kind::Text, json!(2024), json!("2024")),
            (Kind::List, json!("Ada"), json!(["Ada"])),
            (Kind::Part, json!("2"), json!(2)),
            (Kind::Text, json!(" B "), json!("B")),
        ];
        assert!(cases.iter().all(|(k, a, b)| normalise(*k, a) == normalise(*k, b)));
    }

    #[test]
    fn a_title_in_one_other_language_should_stand_for_the_default() {
        let entry = json!({"XMP-dc:Title-de": "Hallo"});
        assert_eq!(exiftool_value(&entry, "XMP-dc:Title"), &json!("Hallo"));
    }

    /// The workflow installs the exiftool doctor.rs pins.
    #[test]
    fn nightly_should_install_the_pinned_exiftool() {
        let workflow = include_str!("../../../../.github/workflows/nightly.yml");
        let install = format!("libimage-exiftool-perl={}", crate::doctor::EXIFTOOL_APT_VERSION);
        assert!(
            workflow.contains(&install)
                && crate::doctor::EXIFTOOL_APT_VERSION.starts_with(&format!("{}+", crate::doctor::EXIFTOOL_VERSION)),
            "nightly.yml should install {install}"
        );
    }
}
