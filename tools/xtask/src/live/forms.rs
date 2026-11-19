//! `cargo xtask differential --live --forms` (ADR 0018 tier 6): the form fields
//! `laminapdf::Document::form` reads against pypdf's and pdf.js's, every night.
//!
//! The inputs are the external corpus and the committed forms in [`LOCAL_DIRS`]. lamina's side is
//! the `form-facts` example (`crates/laminapdf/examples/differential/`); pypdf's is
//! `tools/differential/facts.py pypdf-forms` (`get_fields`) and pdf.js's
//! `tools/differential/pdfjs.mjs --forms` (each page's widget annotations), all under [`runner`].
//! Each writes every terminal field's fully qualified name, kind and value; [`compare_file`]
//! pairs the fields by name and compares the kind and, after [`normalise`], the value: a text
//! (empty is none), an on-state name (`Off` is none; pypdf writes `/Yes`, the others `Yes`), or
//! a choice's export values as a list (pypdf writes a single one as a string). A push button's
//! and a signature's values aren't compared.
//!
//! A difference fails the run unless [`EXCEPTIONS_FILE`] excuses it: a `[[class]]` names one of
//! [`Class`]'s rules, each a way pdf.js reads a form by design (it knows the fields only
//! through the pages' widgets), and gives its `reason`; a `[[known]]` entry (`file`, a glob;
//! `facts`, of [`ASPECTS`], default all; `issue`; `reason`) excuses what is left in its files.
//! Entries that excused nothing are listed as stale. Files lamina fails to open (the structural
//! differential reports those) and files a tool fails on are counted and left out; a lamina
//! panic, crash or hang always fails. The report goes to `target/nightly/differential-forms/`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::known::{Known, known_entries, warn_pending};
use super::{LiveArgs, MAX_ROWS, Report, example_binary, local_pdfs, run_readers};
use crate::workspace::Workspace;
use crate::{corpus, runner, step, util};

/// The committed folders whose PDFs join the external corpus (not their subfolders).
const LOCAL_DIRS: [&str; 1] = ["fixtures/forms"];

/// The `[[class]]` and `[[known]]` entries; it must exist.
pub(super) const EXCEPTIONS_FILE: &str = "fixtures/differential/forms-exceptions.toml";

/// What a difference is about: a field only one side lists, its kind, or its value.
const ASPECTS: [&str; 3] = ["name", "kind", "value"];

/// The tools, by the name their reports carry here.
const TOOLS: [&str; 2] = ["pypdf", "pdfjs"];

/// A rule by which a tool reads a form differently from lamina by design; a `[[class]]` entry
/// turns it on. Most are pdf.js's: it builds its fields from the widgets each page's `/Annots`
/// lists and reads each widget as a field, where lamina walks `/AcroForm /Fields` down `/Kids`
/// (ISO 32000-2 §12.7.4) and reads values from the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    /// A field none of whose widgets a page shows: lamina lists it, pdf.js can't see it.
    Widgetless,
    /// A widget no field of the tree reaches: pdf.js lists the field its `/Parent` names, in a
    /// file where lamina reports such widgets.
    OrphanWidget,
    /// In a file whose tree has a node whose `/Parent` names another node, a name only one side
    /// lists: pdf.js and pypdf name a field by climbing `/Parent`, lamina by walking the tree.
    FollowsParent,
    /// A text field whose value is a text stream (§12.7.5.3): pdf.js reads it as empty.
    TextStream,
    /// A button group with `/Opt`: lamina gives the on-state name (an index into `/Opt`),
    /// pdf.js the export value at that index (§12.7.5.2.4).
    OptExport,
    /// A choice with `/I`: pdf.js gives the options its indices select, lamina `/V`, which wins
    /// where they differ (§12.7.5.4, Table 233).
    ChoiceIndices,
    /// A check box or radio group whose `/V` no widget's `/AS` shows: pdf.js gives a state a
    /// widget shows, lamina `/V` (§12.7.5.2.3).
    AppearanceState,
    /// A widget with a `/V` of its own, which only a field holds (§12.7.4.1): pdf.js gives it,
    /// lamina the field's.
    WidgetValue,
    /// pypdf names a field that has a mapping name (`/TM`, Table 226) by it rather than by its
    /// fully qualified name: in a file where it does, a name only one of the two lists.
    MappingName,
    /// pypdf skips every field below a node without `/T`, which §12.7.4.2 allows.
    UnnamedNode,
    /// pypdf reads a text string with a UTF-8 byte order mark (§7.9.2.2, new in PDF 2.0) as
    /// PDFDocEncoding: its value is lamina's behind `ï»¿`.
    Utf8Bom,
}

impl Class {
    const ALL: [Self; 11] = [
        Self::Widgetless,
        Self::OrphanWidget,
        Self::FollowsParent,
        Self::TextStream,
        Self::OptExport,
        Self::ChoiceIndices,
        Self::AppearanceState,
        Self::WidgetValue,
        Self::MappingName,
        Self::UnnamedNode,
        Self::Utf8Bom,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Widgetless => "widgetless",
            Self::OrphanWidget => "orphan-widget",
            Self::FollowsParent => "follows-parent",
            Self::TextStream => "text-stream",
            Self::OptExport => "opt-export",
            Self::ChoiceIndices => "choice-indices",
            Self::AppearanceState => "appearance-state",
            Self::WidgetValue => "widget-value",
            Self::MappingName => "mapping-name",
            Self::UnnamedNode => "unnamed-node",
            Self::Utf8Bom => "utf8-bom",
        }
    }

    /// The tools whose differences it excuses.
    fn tools(self) -> &'static [&'static str] {
        match self {
            Self::FollowsParent => &TOOLS,
            Self::MappingName | Self::UnnamedNode | Self::Utf8Bom => &["pypdf"],
            _ => &["pdfjs"],
        }
    }

    /// Whether it excuses `d`, given lamina's and the tool's facts about the file.
    fn excuses(self, d: &Difference, file: &Value, tool_file: &Value) -> bool {
        if !self.tools().contains(&d.tool) {
            return false;
        }
        let count = |key: &str| file.get(key).and_then(Value::as_u64).unwrap_or(0);
        let ours = d.lamina.as_ref().unwrap_or(&Value::Null);
        let theirs = d.theirs.as_ref().map_or(&Value::Null, |t| &t["value"]);
        let flag = |key: &str| ours.get(key).and_then(Value::as_bool) == Some(true);
        let kind = ours["kind"].as_str().unwrap_or_default();
        // Whether the tool's value is one of lamina's `key` list, compared as `kind`'s values.
        let among = |key: &str| {
            let theirs = normalise(kind, theirs);
            let list = ours.get(key).and_then(Value::as_array).into_iter().flatten();
            theirs.is_some() && list.map(|v| normalise(kind, v)).any(|v| v == theirs)
        };
        match (self, d.aspect) {
            (Self::Widgetless, "name") => d.theirs.is_none() && flag("widgetless"),
            (Self::OrphanWidget, "name") => d.lamina.is_none() && count("orphan_widgets") > 0,
            (Self::FollowsParent, "name") => count("parent_mismatches") > 0,
            (Self::TextStream, "value") => flag("stream") && normalise("text", theirs).is_none(),
            (Self::OptExport, "value") => {
                let index = ours["value"].as_str().and_then(|v| v.parse::<usize>().ok());
                let export = index.and_then(|i| ours.get("options")?.get(i));
                export.is_some_and(|e| e == theirs)
            }
            (Self::ChoiceIndices, "value") => ours
                .get("indices")
                .is_some_and(|i| normalise(kind, i) == normalise(kind, theirs)),
            (Self::AppearanceState, "value") => among("shown"),
            (Self::WidgetValue, "value") => among("widget_values"),
            (Self::MappingName, "name") => by_name(tool_file).values().any(|f| f["mapping_name"] == true),
            (Self::UnnamedNode, "name") => d.theirs.is_none() && flag("unnamed_ancestor"),
            (Self::Utf8Bom, "value") => {
                let ours = ours["value"].as_str().unwrap_or_default();
                theirs.as_str() == Some(&format!("\u{ef}\u{bb}\u{bf}{ours}"))
            }
            _ => false,
        }
    }
}

/// A field lamina and a tool read differently: each side's record (`null`: it doesn't list it).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Difference {
    pub(super) file: String,
    pub(super) field: String,
    pub(super) aspect: &'static str,
    pub(super) tool: &'static str,
    pub(super) lamina: Option<Value>,
    pub(super) theirs: Option<Value>,
}

/// What excused a difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Excuse {
    Class(Class),
    Known(usize),
}

/// A value's comparable form for a field of `kind`; `None` when it has none or isn't compared.
pub(super) fn normalise(kind: &str, value: &Value) -> Option<Value> {
    let text = |v: &Value| v.as_str().map(|s| s.strip_prefix('/').unwrap_or(s).to_owned());
    match kind {
        "text" => value.as_str().filter(|s| !s.is_empty()).map(|s| json!(s)),
        "checkBox" | "radioGroup" => text(value).filter(|s| !s.is_empty() && s != "Off").map(Value::String),
        "comboBox" | "listBox" => {
            let items: Vec<&str> = match value {
                Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
                other => other.as_str().into_iter().collect(),
            };
            // An empty export value selects nothing: pdf.js writes no selection as `[""]`.
            let items: Vec<Value> = items.into_iter().filter(|s| !s.is_empty()).map(|s| json!(s)).collect();
            (!items.is_empty()).then_some(Value::Array(items))
        }
        _ => None,
    }
}

/// A file's fields by name (the first record of a name wins).
fn by_name(file: &Value) -> BTreeMap<&str, &Value> {
    let mut out = BTreeMap::new();
    for field in file.get("fields").and_then(Value::as_array).into_iter().flatten() {
        if let Some(name) = field.get("name").and_then(Value::as_str) {
            out.entry(name).or_insert(field);
        }
    }
    out
}

/// How lamina's facts about `file` and `tool`'s differ, and how many fields both list.
pub(super) fn compare_file(file: &str, lamina: &Value, tool: &'static str, theirs: &Value) -> (Vec<Difference>, usize) {
    let (ours, theirs) = (by_name(lamina), by_name(theirs));
    let mut out = Vec::new();
    let mut both = 0;
    let names = ours.keys().chain(theirs.keys().filter(|n| !ours.contains_key(*n)));
    for &name in names {
        let (a, b) = (ours.get(name).copied(), theirs.get(name).copied());
        let difference = |aspect| Difference {
            file: file.to_owned(),
            field: name.to_owned(),
            aspect,
            tool,
            lamina: a.cloned(),
            theirs: b.cloned(),
        };
        let (Some(a), Some(b)) = (a, b) else {
            out.push(difference("name"));
            continue;
        };
        both += 1;
        let kind = a["kind"].as_str().unwrap_or_default();
        if a["kind"] != b["kind"] {
            out.push(difference("kind"));
        } else if normalise(kind, &a["value"]) != normalise(kind, &b["value"]) {
            out.push(difference("value"));
        }
    }
    (out, both)
}

/// The exceptions file's `[[class]]` rules turned on and its `[[known]]` entries.
pub(super) fn parse_exceptions(text: &str) -> Result<(Vec<Class>, Vec<Known>)> {
    let mut table: toml::Table = toml::from_str(text)?;
    let mut classes = Vec::new();
    if let Some(entries) = table.remove("class") {
        for entry in entries.as_array().context("`class` must be [[class]] entries")? {
            let entry = entry.as_table().context("a [[class]] entry must be a table")?;
            if let Some(other) = entry.keys().find(|k| !["name", "reason"].contains(&k.as_str())) {
                bail!("[[class]]: unknown key {other:?}");
            }
            if entry
                .get("reason")
                .and_then(toml::Value::as_str)
                .is_none_or(|r| r.trim().is_empty())
            {
                bail!("[[class]] without `reason`: {entry:?}");
            }
            let name = entry.get("name").and_then(toml::Value::as_str).unwrap_or_default();
            let class = Class::ALL.into_iter().find(|c| c.name() == name).with_context(|| {
                format!(
                    "[[class]] name {name:?}: one of {}",
                    Class::ALL.map(Class::name).join(", ")
                )
            })?;
            classes.push(class);
        }
    }
    Ok((classes, known_entries(&table, "facts", &ASPECTS)?))
}

/// The comparison of a whole run.
#[derive(Debug, Default)]
pub(super) struct Comparison {
    pub(super) differences: Vec<(Difference, Option<Excuse>)>,
    /// Files lamina and at least one tool read, and those where lamina finds fields.
    pub(super) files: usize,
    pub(super) with_fields: usize,
    /// Fields lamina and a tool both list, summed over the tools.
    pub(super) fields: usize,
    pub(super) lamina_failed: BTreeMap<String, String>,
    /// Files lamina panicked, crashed or hung on (always failures), with the outcome.
    pub(super) lamina_errors: BTreeMap<String, String>,
    /// Per tool, the files it failed on.
    pub(super) tool_errors: BTreeMap<&'static str, usize>,
    pub(super) stale: Vec<String>,
}

impl Comparison {
    fn unexcused(&self) -> impl Iterator<Item = &Difference> {
        self.differences.iter().filter(|(_, e)| e.is_none()).map(|(d, _)| d)
    }

    pub(super) fn failed(&self) -> bool {
        self.unexcused().next().is_some() || !self.lamina_errors.is_empty()
    }
}

/// Compares lamina's facts with each tool's (all by input name).
pub(super) fn compare(
    lamina: &BTreeMap<String, Value>,
    tools: &[(&'static str, &BTreeMap<String, Value>)],
    classes: &[Class],
    known: &[Known],
) -> Comparison {
    let mut c = Comparison::default();
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
        let mut read = false;
        for &(tool, facts) in tools {
            let Some(theirs) = facts.get(name).filter(|t| t["outcome"] == "opened") else {
                *c.tool_errors.entry(tool).or_default() += 1;
                continue;
            };
            read = true;
            let (differences, both) = compare_file(name, ours, tool, theirs);
            c.fields += both;
            for d in differences {
                let excuse = match classes.iter().find(|k| k.excuses(&d, ours, theirs)) {
                    Some(&class) => Some(Excuse::Class(class)),
                    None => known
                        .iter()
                        .position(|k| k.excuses(&d.file, d.aspect))
                        .map(Excuse::Known),
                };
                c.differences.push((d, excuse));
            }
        }
        c.files += usize::from(read);
        c.with_fields += usize::from(read && !by_name(ours).is_empty());
    }
    let used = |excuse: Excuse| c.differences.iter().any(|(_, e)| *e == Some(excuse));
    let stale_classes = classes.iter().filter(|&&k| !used(Excuse::Class(k)));
    let mut stale: Vec<String> = stale_classes.map(|k| format!("[[class]] {}", k.name())).collect();
    stale.extend(
        (0..known.len())
            .filter(|&i| !used(Excuse::Known(i)))
            .filter_map(|i| known.get(i).map(Known::label)),
    );
    c.stale = stale;
    c
}

/// What the report says about the run besides the comparison.
struct RunInfo {
    inputs: usize,
    commit: String,
    sample: Option<usize>,
    versions: Vec<String>,
    broken: Vec<String>,
}

impl Comparison {
    fn summary_line(&self) -> String {
        format!(
            "Form differential: {} files compared ({} with fields), {} fields both sides list, {} differ ({} \
             unexcused); skipped {} lamina couldn't open, {}; {} lamina crashes",
            self.files,
            self.with_fields,
            self.fields,
            self.differences.len(),
            self.unexcused().count(),
            self.lamina_failed.len(),
            TOOLS
                .map(|t| format!("{} {t} couldn't read", self.tool_errors.get(t).copied().unwrap_or(0)))
                .join(", "),
            self.lamina_errors.len()
        )
    }

    fn excuse_label(excuse: Option<Excuse>, known: &[Known]) -> String {
        match excuse {
            Some(Excuse::Class(class)) => format!("[[class]] {}", class.name()),
            Some(Excuse::Known(i)) => known.get(i).map_or_else(String::new, Known::label),
            None => "**unexcused**".to_owned(),
        }
    }

    fn markdown(&self, known: &[Known], info: &RunInfo) -> String {
        let mut md = String::from("## Form differential (lamina vs pypdf and pdf.js)\n\n");
        let _ = writeln!(
            md,
            "{} inputs{} at `{}`; {}.\n\n{}\n",
            info.inputs,
            info.sample.map(|n| format!(" (sample of {n})")).unwrap_or_default(),
            info.commit,
            info.versions.join(", "),
            self.summary_line()
        );
        for broken in &info.broken {
            let _ = writeln!(md, "- **Reader broken:** {broken}");
        }
        for (name, outcome) in &self.lamina_errors {
            let _ = writeln!(md, "- **lamina {outcome}:** `{name}`");
        }
        let rows: Vec<_> = self.differences.iter().take(MAX_ROWS).collect();
        if !rows.is_empty() {
            // Field names and values are text from the files: they stay in report.json, out of
            // the job summary, as the XMP and text differentials keep theirs.
            md.push_str(
                "\nThe fields' names and values are in report.json, not here: they are text from the files.\n\n\
                 | File | Tool | Aspect | Excused by |\n| --- | --- | --- | --- |\n",
            );
            for (d, excuse) in rows {
                let by = Self::excuse_label(*excuse, known);
                let _ = writeln!(
                    md,
                    "| `{}` | {} | {} | {by} |",
                    util::md_cell(&d.file),
                    d.tool,
                    d.aspect
                );
            }
            if self.differences.len() > MAX_ROWS {
                let _ = writeln!(md, "\n{} more in report.json.", self.differences.len() - MAX_ROWS);
            }
        }
        for stale in &self.stale {
            let _ = writeln!(md, "- Stale entry (excused nothing): {stale}");
        }
        md
    }

    fn json(&self, known: &[Known], info: &RunInfo) -> Value {
        json!({
            "inputs": info.inputs,
            "commit": info.commit,
            "sample": info.sample,
            "versions": info.versions,
            "broken_readers": info.broken,
            "files": self.files,
            "with_fields": self.with_fields,
            "fields": self.fields,
            "lamina_failed": self.lamina_failed,
            "lamina_errors": self.lamina_errors,
            "tool_errors": self.tool_errors,
            "stale": self.stale,
            "differences": self.differences.iter().map(|(d, excuse)| json!({
                "file": d.file,
                "field": d.field,
                "aspect": d.aspect,
                "tool": d.tool,
                "lamina": d.lamina,
                "theirs": d.theirs,
                "excused_by": excuse.map(|e| Self::excuse_label(Some(e), known)),
            })).collect::<Vec<_>>(),
        })
    }
}

/// `cargo xtask differential --live --forms …`.
pub(super) fn run(ws: &Workspace, args: &LiveArgs) -> Result<()> {
    let text =
        fs::read_to_string(ws.root.join(EXCEPTIONS_FILE)).with_context(|| format!("reading {EXCEPTIONS_FILE}"))?;
    let (classes, known) = parse_exceptions(&text).with_context(|| format!("reading {EXCEPTIONS_FILE}"))?;
    if !ws.root.join("tools/differential/node_modules/pdfjs-dist").is_dir() {
        bail!("pdf.js is not installed: run `pnpm --dir tools/differential install --frozen-lockfile`");
    }
    let out = ws.root.join("target/nightly/differential-forms");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    fs::create_dir_all(&out)?;
    let mut inputs = corpus::verified_inputs(ws, &args.sources)?;
    if args.sources.is_empty() {
        inputs.extend(local_pdfs(&ws.root, &LOCAL_DIRS)?);
    }
    inputs.sort();
    if let Some(n) = args.sample {
        inputs = super::sample(inputs, n);
    }
    if inputs.is_empty() {
        bail!("no inputs: nothing would be compared (check --source and --sample)");
    }
    step(&format!("form differential: {} inputs", inputs.len()));
    ws.cargo(&[
        "build",
        "--release",
        "--locked",
        "-q",
        "-p",
        "laminapdf",
        "--example",
        "form-facts",
    ])?;
    let reader = |name, program, args: Vec<std::ffi::OsString>| runner::Reader {
        name,
        program,
        args,
        dir: ws.root.clone(),
        reports_repair: false,
    };
    let script = |path: &str| ws.root.join(path).into_os_string();
    let readers = [
        reader("lamina", example_binary(ws, "form-facts").into_os_string(), Vec::new()),
        reader(
            "pypdf",
            crate::harness::python().into(),
            vec![script("tools/differential/facts.py"), "pypdf-forms".into()],
        ),
        reader(
            "pdfjs",
            "node".into(),
            vec![script("tools/differential/pdfjs.mjs"), "--forms".into()],
        ),
    ];
    let (reports, broken) = run_readers(&readers, &inputs, &out)?;
    let [lamina, pypdf, pdfjs]: [Report; 3] = reports
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected three reports"))?;
    let comparison = compare(
        &lamina.facts,
        &[("pypdf", &pypdf.facts), ("pdfjs", &pdfjs.facts)],
        &classes,
        &known,
    );
    let info = RunInfo {
        inputs: inputs.len(),
        commit: util::git_stdout(&ws.root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned()),
        sample: args.sample,
        versions: [&lamina, &pypdf, &pdfjs]
            .map(|r| format!("{} {}", r.tool, r.version))
            .to_vec(),
        broken,
    };
    write_report(&out, &comparison, &known, &info)?;
    warn_pending(&known, EXCEPTIONS_FILE);
    if !info.broken.is_empty() {
        bail!("readers failed: {}; see the report", info.broken.join("; "));
    }
    if comparison.failed() {
        bail!(
            "lamina's form fields differ from pypdf's or pdf.js's in {} places, or lamina failed to read {} \
             files (panic, crash or timeout), and {EXCEPTIONS_FILE} doesn't excuse them; see the report",
            comparison.unexcused().count(),
            comparison.lamina_errors.len()
        );
    }
    Ok(())
}

fn write_report(out: &Path, comparison: &Comparison, known: &[Known], info: &RunInfo) -> Result<()> {
    let markdown = comparison.markdown(known, info);
    fs::write(out.join("report.md"), &markdown)?;
    fs::write(
        out.join("report.json"),
        serde_json::to_string_pretty(&comparison.json(known, info))? + "\n",
    )?;
    util::summary(&markdown)?;
    println!("{}", comparison.summary_line());
    step(&format!("Report: {}", out.join("report.md").display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(entries: &[(&str, Value)]) -> BTreeMap<String, Value> {
        entries.iter().map(|(n, v)| ((*n).to_owned(), v.clone())).collect()
    }

    fn opened(fields: &Value) -> Value {
        json!({"outcome": "opened", "fields": fields})
    }

    /// The aspects and tools of `c`'s unexcused differences.
    fn unexcused(c: &Comparison) -> Vec<(&str, &str)> {
        c.unexcused().map(|d| (d.aspect, d.tool)).collect()
    }

    #[test]
    fn values_should_compare_as_each_reader_writes_them() {
        let same = [
            ("checkBox", json!("/Yes"), json!("Yes")),
            ("radioGroup", json!("Off"), Value::Null),
            ("comboBox", json!("US"), json!(["US"])),
            ("listBox", json!([]), Value::Null),
            ("text", json!(""), Value::Null),
            ("pushButton", json!("x"), json!("y")),
            ("signature", json!({}), Value::Null),
        ];
        let differ: Vec<_> = same
            .iter()
            .filter(|(kind, a, b)| normalise(kind, a) != normalise(kind, b))
            .collect();
        assert!(differ.is_empty(), "{differ:?}");
    }

    #[test]
    fn values_should_keep_different_texts_apart() {
        assert_ne!(normalise("text", &json!("a")), normalise("text", &json!("a ")));
    }

    #[test]
    fn compare_should_find_names_kinds_and_values_per_tool() {
        let lamina = facts(&[(
            "a.pdf",
            opened(&json!([
                {"name": "x", "kind": "text", "value": "1"},
                {"name": "y", "kind": "checkBox", "value": "Yes"},
                {"name": "z", "kind": "text", "value": null},
            ])),
        )]);
        let pypdf = facts(&[(
            "a.pdf",
            opened(&json!([
                {"name": "x", "kind": "text", "value": "2"},
                {"name": "y", "kind": "radioGroup", "value": "/Yes"},
            ])),
        )]);
        let pdfjs = facts(&[("a.pdf", json!({"outcome": "failed", "fields": null}))]);
        let c = compare(&lamina, &[("pypdf", &pypdf), ("pdfjs", &pdfjs)], &[], &[]);
        assert_eq!(
            (unexcused(&c), c.fields, c.tool_errors.get("pdfjs").copied()),
            (
                vec![("value", "pypdf"), ("kind", "pypdf"), ("name", "pypdf")],
                2,
                Some(1)
            )
        );
    }

    #[test]
    fn classes_should_excuse_pdfjs_by_design_and_nothing_else() {
        let lamina = facts(&[(
            "a.pdf",
            json!({"outcome": "opened", "orphan_widgets": 1, "parent_mismatches": 0, "fields": [
                {"name": "hidden", "kind": "text", "value": "h", "widgetless": true},
                {"name": "notes", "kind": "text", "value": "long", "stream": true},
                {"name": "size", "kind": "radioGroup", "value": "1", "options": ["Small", "Large"]},
                {"name": "plain", "kind": "text", "value": "p"},
            ]}),
        )]);
        let pdfjs = facts(&[(
            "a.pdf",
            opened(&json!([
                {"name": "notes", "kind": "text", "value": ""},
                {"name": "size", "kind": "radioGroup", "value": "Large"},
                {"name": "plain", "kind": "text", "value": ""},
                {"name": "orphan", "kind": "text", "value": null},
            ])),
        )]);
        let pypdf = facts(&[(
            "a.pdf",
            opened(&json!([{"name": "notes", "kind": "text", "value": ""}])),
        )]);
        let c = compare(&lamina, &[("pdfjs", &pdfjs), ("pypdf", &pypdf)], &Class::ALL, &[]);
        assert_eq!(
            unexcused(&c),
            [
                ("value", "pdfjs"),
                ("name", "pypdf"),
                ("value", "pypdf"),
                ("name", "pypdf"),
                ("name", "pypdf")
            ]
        );
    }

    #[test]
    fn classes_should_excuse_values_read_from_elsewhere_only_when_they_match() {
        let lamina = facts(&[(
            "a.pdf",
            opened(&json!([
                {"name": "list", "kind": "listBox", "value": ["a"], "indices": ["b"]},
                {"name": "check", "kind": "checkBox", "value": null, "shown": ["Yes"]},
                {"name": "total", "kind": "text", "value": "9", "widget_values": ["3"]},
                {"name": "other", "kind": "text", "value": "9", "widget_values": ["3"]},
                {"name": "leaf", "kind": "text", "value": null, "unnamed_ancestor": true},
                {"name": "bom", "kind": "text", "value": "abc"},
            ])),
        )]);
        let pdfjs = facts(&[(
            "a.pdf",
            opened(&json!([
                {"name": "list", "kind": "listBox", "value": ["b"]},
                {"name": "check", "kind": "checkBox", "value": "Yes"},
                {"name": "total", "kind": "text", "value": "3"},
                {"name": "other", "kind": "text", "value": "4"},
                {"name": "leaf", "kind": "text", "value": null},
                {"name": "bom", "kind": "text", "value": "abc"},
            ])),
        )]);
        let pypdf = facts(&[(
            "a.pdf",
            opened(&json!([
                {"name": "list", "kind": "listBox", "value": "a"},
                {"name": "check", "kind": "checkBox", "value": "/Off"},
                {"name": "total", "kind": "text", "value": "9"},
                {"name": "other", "kind": "text", "value": "9"},
                {"name": "bom", "kind": "text", "value": "\u{ef}\u{bb}\u{bf}abc"},
            ])),
        )]);
        let c = compare(&lamina, &[("pdfjs", &pdfjs), ("pypdf", &pypdf)], &Class::ALL, &[]);
        assert_eq!(unexcused(&c), [("value", "pdfjs")]);
    }

    #[test]
    fn known_entries_should_excuse_their_aspects_in_their_files() {
        let (classes, known) = parse_exceptions(
            "[[class]]\nname = \"widgetless\"\nreason = \"r\"\n\
             [[known]]\nfile = \"a*\"\nfacts = [\"value\"]\nissue = \"PENDING\"\nreason = \"r\"\n",
        )
        .unwrap();
        let lamina = facts(&[(
            "a.pdf",
            opened(&json!([{"name": "x", "kind": "text", "value": "1"}, {"name": "y", "kind": "text", "value": "a"}])),
        )]);
        let tool = facts(&[(
            "a.pdf",
            opened(
                &json!([{"name": "x", "kind": "checkBox", "value": "2"}, {"name": "y", "kind": "text", "value": "b"}]),
            ),
        )]);
        let c = compare(&lamina, &[("pypdf", &tool)], &classes, &known);
        assert_eq!(
            (unexcused(&c), &c.stale[..]),
            (vec![("kind", "pypdf")], &["[[class]] widgetless".to_owned()][..])
        );
    }

    #[test]
    fn exceptions_should_refuse_unknown_classes_and_classes_without_reasons() {
        assert!(parse_exceptions("[[class]]\nname = \"nope\"\nreason = \"r\"\n").is_err());
        assert!(parse_exceptions("[[class]]\nname = \"widgetless\"\n").is_err());
    }

    #[test]
    fn a_lamina_panic_should_fail_and_a_lamina_failure_should_not() {
        let lamina = facts(&[
            ("a.pdf", json!({"outcome": "panic"})),
            ("b.pdf", json!({"outcome": "failed", "error": "e"})),
        ]);
        let c = compare(&lamina, &[], &[], &[]);
        assert!(c.failed() && c.lamina_failed.len() == 1);
    }

    /// The committed exceptions file parses.
    #[test]
    fn the_committed_exceptions_should_parse() {
        let text = include_str!("../../../../fixtures/differential/forms-exceptions.toml");
        assert!(parse_exceptions(text).is_ok());
    }
}
