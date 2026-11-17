//! `cargo xtask differential --live --inspect` (ADR 0018 tier 6): the objects and fonts Lamina
//! PDF's inspector reads against qpdf's and pdffonts', every night.
//!
//! The inputs are the external corpus and the committed PDFs in [`LOCAL_DIRS`]. lamina's side is
//! the `inspect-facts` example (`crates/laminapdf/examples/differential/`) under [`runner`]; the
//! tools are external processes only (qpdf is Apache-2.0; pdffonts is poppler's, GPL, so never
//! linked, ADR 0019), each pinned in `doctor.rs`. Per file:
//!
//! - **objects**: `qpdf --json=2 --json-key=qpdf` gives every object's number, generation, kind
//!   (stream, dictionary, other) and `/Type`; `qpdf --show-xref` where each is stored (a byte
//!   offset, or an object stream and index). A file either side repaired is left out of this
//!   fact: the places are then each reader's own reconstruction.
//! - **fonts**: `pdffonts` gives every font of every page (resources, forms, annotations'
//!   appearances) with its object, name, type, and whether it is embedded, a subset and has a
//!   ToUnicode CMap; its type maps to the `/Subtype` family (`CID …` → `Type0`).
//!
//! A fact where the two differ fails the run unless a `[[known]]` entry of [`EXCEPTIONS_FILE`]
//! (`file`, a glob; `facts`, default both; `issue`; `reason`) excuses it. The report goes to
//! `target/nightly/differential-inspect/`, with the number of files whose font list matches.
//!
//! `--update` writes the tools' facts about [`COMMITTED`] to [`FACTS_FILE`], which
//! `crates/laminapdf/tests/inspect_differential.rs` compares lamina with on every PR.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use super::exceptions::glob;
use super::{LiveArgs, MAX_ROWS, PENDING_ISSUE, Report, example_binary, run_readers};
use crate::doctor::{POPPLER_VERSION, Probe, QPDF_VERSION};
use crate::workspace::Workspace;
use crate::{corpus, runner, step, util};

/// The committed folders whose PDFs join the external corpus (not their subfolders).
const LOCAL_DIRS: [&str; 6] = [
    "fixtures/corpus",
    "fixtures/golden",
    "fixtures/golden/examples",
    "fixtures/signed",
    "fixtures/attachments",
    "fixtures/revisions",
];

/// The committed PDFs whose tool facts [`FACTS_FILE`] holds for the PR lane: small files from
/// many producers, with simple, Type 0, Type 3-free and standard 14 fonts, object streams,
/// incremental updates, signatures and annotations' appearances.
pub(super) const COMMITTED: [&str; 30] = [
    "fixtures/attachments/associated.pdf",
    "fixtures/attachments/portfolio.pdf",
    "fixtures/corpus/fpdf2.pdf",
    "fixtures/corpus/hello-incremental.pdf",
    "fixtures/corpus/hello-xrefstm.pdf",
    "fixtures/corpus/hello.pdf",
    "fixtures/corpus/outline.pdf",
    "fixtures/corpus/pdflib.pdf",
    "fixtures/corpus/raw.pdf",
    "fixtures/corpus/reportlab.pdf",
    "fixtures/corpus/signed.pdf",
    "fixtures/corpus/text-example.pdf",
    "fixtures/golden/build-text-latin.pdf",
    "fixtures/golden/examples/add_text_box.pdf",
    "fixtures/golden/examples/comment_summary.pdf",
    "fixtures/golden/examples/insert_pages.pdf",
    "fixtures/golden/examples/invoice.pdf",
    "fixtures/golden/examples/merge_documents.pdf",
    "fixtures/golden/examples/multilingual.pdf",
    "fixtures/golden/examples/pdf_a3_invoice_with_xml.pdf",
    "fixtures/golden/examples/pdf_ua_report.pdf",
    "fixtures/golden/metadata.pdf",
    "fixtures/golden/text-cff.pdf",
    "fixtures/golden/text-latin.pdf",
    "fixtures/golden/text-shaped.pdf",
    "fixtures/golden/text-std14.pdf",
    "fixtures/revisions/hello-linearized.pdf",
    "fixtures/signed/approval.pdf",
    "fixtures/signed/locked-page1.pdf",
    "fixtures/signed/two-signatures.pdf",
];

/// The tools' facts about [`COMMITTED`], for the PR lane.
const FACTS_FILE: &str = "fixtures/differential/inspect.json";

/// The `[[known]]` disagreements; a missing file excuses nothing.
const EXCEPTIONS_FILE: &str = "fixtures/differential/inspect-exceptions.toml";

/// The two facts compared.
const FACTS: [&str; 2] = ["objects", "fonts"];

/// How long one tool may take on one file.
const TOOL_TIMEOUT: Duration = Duration::from_secs(60);

/// A tool run's stdout, or why there is none.
enum Ran {
    /// Exited 0 (or 3, qpdf's "succeeded with warnings"), with what it printed and its messages.
    Done {
        stdout: String,
        stderr: String,
    },
    Failed(String),
}

/// Runs `program args path`, killing it after [`TOOL_TIMEOUT`]; its output goes through
/// `work/` files, not pipes, so a large one can't block it.
fn run_tool(program: &str, args: &[&str], path: &Path, work: &Path) -> Result<Ran> {
    let (out, err) = (work.with_extension("out"), work.with_extension("err"));
    let mut child = Command::new(crate::program_path(program))
        .args(args)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(fs::File::create(&out)?)
        .stderr(fs::File::create(&err)?)
        .spawn()
        .with_context(|| format!("starting {program}"))?;
    let deadline = Instant::now() + TOOL_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            // Killing an already-exited child fails harmlessly; reap it either way.
            let _ = child.kill();
            let _ = child.wait();
            return Ok(Ran::Failed("timeout".to_owned()));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = String::from_utf8_lossy(&fs::read(&out)?).into_owned();
    let stderr = String::from_utf8_lossy(&fs::read(&err)?).into_owned();
    let _ = (fs::remove_file(&out), fs::remove_file(&err));
    Ok(match status.code() {
        Some(0 | 3) => Ran::Done { stdout, stderr },
        code => Ran::Failed(format!("exit {code:?}")),
    })
}

/// `qpdf --show-xref`'s lines (`5/0: uncompressed; offset = 15`, `1/0: compressed; stream = 6,
/// index = 0`) as `(generation, place)` by object number, places written as lamina's facts write
/// them (`offset:15`, `stream:6:0`).
pub(super) fn parse_xref(text: &str) -> BTreeMap<u32, (u64, String)> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let Some((id, rest)) = line.split_once(": ") else {
            continue;
        };
        let Some((num, generation)) = id.split_once('/') else {
            continue;
        };
        let (Ok(num), Ok(generation)) = (num.parse::<u32>(), generation.parse::<u64>()) else {
            continue;
        };
        let number_after = |key: &str| -> Option<u64> {
            let at = rest.find(key)? + key.len();
            rest.get(at..)?
                .split(|c: char| !c.is_ascii_digit())
                .next()?
                .parse()
                .ok()
        };
        let place = if rest.starts_with("uncompressed") {
            number_after("offset = ").map(|o| format!("offset:{o}"))
        } else if rest.starts_with("compressed") {
            number_after("stream = ")
                .zip(number_after("index = "))
                .map(|(s, i)| format!("stream:{s}:{i}"))
        } else {
            None
        };
        if let Some(place) = place {
            out.insert(num, (generation, place));
        }
    }
    out
}

/// qpdf's objects (`qpdf --json=2 --json-key=qpdf`'s second element: `"obj:N G R"` keys) joined
/// with their places from `xref`, in lamina's facts' shape.
pub(super) fn qpdf_objects(json: &Value, xref: &BTreeMap<u32, (u64, String)>) -> Value {
    let mut out = Map::new();
    let objects = json.pointer("/qpdf/1").and_then(Value::as_object);
    for (key, object) in objects.into_iter().flatten() {
        let Some(id) = key.strip_prefix("obj:").and_then(|k| k.strip_suffix(" R")) else {
            continue;
        };
        let Some((num, generation)) = id.split_once(' ') else {
            continue;
        };
        let (Ok(num), Ok(generation)) = (num.parse::<u32>(), generation.parse::<u64>()) else {
            continue;
        };
        let (kind, dict) = match (object.pointer("/stream/dict"), object.get("value")) {
            (Some(dict), _) => ("stream", Some(dict)),
            (None, Some(value)) if value.is_object() => ("dictionary", Some(value)),
            _ => ("other", None),
        };
        let type_name = dict
            .and_then(|d| d.get("/Type"))
            .and_then(Value::as_str)
            .and_then(|t| t.strip_prefix('/'));
        let at = xref.get(&num).map(|(_, place)| place.clone());
        out.insert(
            num.to_string(),
            json!({"gen": generation, "kind": kind, "type": type_name, "at": at}),
        );
    }
    Value::Object(out)
}

/// The `/Subtype` family of a pdffonts type (`Type 1C`, `CID TrueType (OT)`).
fn family(kind: &str) -> Option<&'static str> {
    if kind.starts_with("CID ") {
        Some("Type0")
    } else if kind.starts_with("Type 3") {
        Some("Type3")
    } else if kind.starts_with("Type 1") {
        Some("Type1")
    } else if kind.starts_with("TrueType") {
        Some("TrueType")
    } else {
        None
    }
}

/// pdffonts' table (columns as wide as the header's dashes) in lamina's facts' shape, sorted,
/// each font once.
pub(super) fn parse_pdffonts(text: &str) -> Value {
    let mut lines = text.lines().skip_while(|l| !l.starts_with("---"));
    let Some(dashes) = lines.next() else {
        return Value::Array(Vec::new());
    };
    let mut columns = Vec::new();
    let mut start = 0;
    for part in dashes.split(' ') {
        columns.push((start, start + part.len()));
        start += part.len() + 1;
    }
    let mut out: Vec<Value> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            // The object ID column runs to the end of the line.
            let cell = |i: usize| -> String {
                let (from, to) = columns.get(i).copied().unwrap_or_default();
                let to = if i + 1 == columns.len() { line.len() } else { to };
                line.get(from.min(line.len())..to.min(line.len()))
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            };
            let name = cell(0);
            let subset = name.len() > 7 && name.as_bytes().get(6) == Some(&b'+');
            let name = match name.as_str() {
                "[none]" => None,
                n if subset => n.get(7..).map(str::to_owned),
                n => Some(n.to_owned()),
            };
            let object = cell(6);
            let object = (object != "[none]").then(|| object.split_whitespace().collect::<Vec<_>>().join(" "));
            json!({
                "object": object,
                "name": name,
                "family": family(&cell(1)),
                "embedded": cell(3) == "yes",
                "subset": cell(4) == "yes",
                "to_unicode": cell(5) == "yes",
            })
        })
        .collect();
    out.sort_by_key(ToString::to_string);
    // pdffonts lists a direct font once per place it is met (two signatures' appearances each
    // with their own Courier); lamina lists it once per type and name.
    out.dedup();
    Value::Array(out)
}

/// The tools' facts about the file at `path`: `{outcome, objects, fonts}`, with `null` for a
/// fact a tool couldn't give (and why in `errors`). `outcome` is `repaired` when qpdf rebuilt
/// the cross-reference table.
pub(super) fn tool_facts(path: &Path, work: &Path) -> Result<Value> {
    let mut errors = Map::new();
    let mut outcome = "opened";
    let objects = match (
        run_tool("qpdf", &["--json=2", "--json-key=qpdf"], path, work)?,
        run_tool("qpdf", &["--show-xref"], path, work)?,
    ) {
        (Ran::Done { stdout, stderr }, Ran::Done { stdout: xref, .. }) => {
            if stderr.contains("reconstruct") {
                outcome = "repaired";
            }
            match serde_json::from_str::<Value>(&stdout) {
                Ok(json) => qpdf_objects(&json, &parse_xref(&xref)),
                Err(e) => {
                    errors.insert("objects".to_owned(), json!(format!("qpdf printed no JSON: {e}")));
                    Value::Null
                }
            }
        }
        (Ran::Failed(why), _) | (_, Ran::Failed(why)) => {
            errors.insert("objects".to_owned(), json!(format!("qpdf: {why}")));
            Value::Null
        }
    };
    let fonts = match run_tool("pdffonts", &[], path, work)? {
        Ran::Done { stdout, .. } => parse_pdffonts(&stdout),
        Ran::Failed(why) => {
            errors.insert("fonts".to_owned(), json!(format!("pdffonts: {why}")));
            Value::Null
        }
    };
    Ok(json!({"outcome": outcome, "objects": objects, "fonts": fonts, "errors": errors}))
}

/// [`tool_facts`] for every input, on every core, by input name.
fn all_tool_facts(inputs: &[(String, PathBuf)], out: &Path) -> Result<BTreeMap<String, Value>> {
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    let chunk = inputs.len().div_ceil(threads).max(1);
    let results: Vec<Result<Vec<(String, Value)>>> = std::thread::scope(|scope| {
        let handles: Vec<_> = inputs
            .chunks(chunk)
            .enumerate()
            .map(|(i, part)| {
                let work = out.join(format!("tool-{i}"));
                scope.spawn(move || {
                    part.iter()
                        .map(|(name, path)| Ok((name.clone(), tool_facts(path, &work)?)))
                        .collect::<Result<Vec<_>>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("a tool thread panicked")))
            })
            .collect()
    });
    let mut by_name = BTreeMap::new();
    for part in results {
        by_name.extend(part?);
    }
    Ok(by_name)
}

/// A fact lamina and the tools read differently.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Mismatch {
    pub(super) file: String,
    pub(super) fact: &'static str,
    /// What differs, briefly (`object 4: kind`), for the report.
    pub(super) detail: String,
}

/// What differs between two objects maps (by number), at most a few lines.
fn object_differences(ours: &Value, theirs: &Value) -> Vec<String> {
    let empty = Map::new();
    let (ours, theirs) = (ours.as_object().unwrap_or(&empty), theirs.as_object().unwrap_or(&empty));
    let mut out = Vec::new();
    for num in ours.keys().chain(theirs.keys().filter(|k| !ours.contains_key(*k))) {
        match (ours.get(num), theirs.get(num)) {
            (Some(a), Some(b)) if a != b => {
                let fields: Vec<&str> = ["gen", "kind", "type", "at"]
                    .into_iter()
                    .filter(|f| a.get(f) != b.get(f))
                    .collect();
                out.push(format!("object {num}: {}", fields.join(", ")));
            }
            (Some(_), None) => out.push(format!("object {num}: only lamina lists it")),
            (None, Some(_)) => out.push(format!("object {num}: only qpdf lists it")),
            _ => {}
        }
    }
    out
}

/// What differs between two font lists.
fn font_differences(ours: &Value, theirs: &Value) -> Vec<String> {
    let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
    let (ours, theirs) = (list(ours), list(theirs));
    let label = |f: &Value| {
        format!(
            "{} ({})",
            f.get("name").and_then(Value::as_str).unwrap_or("[none]"),
            f.get("object").and_then(Value::as_str).unwrap_or("direct")
        )
    };
    let mut out: Vec<String> = ours
        .iter()
        .filter(|f| !theirs.contains(f))
        .map(|f| format!("lamina: {}", label(f)))
        .collect();
    out.extend(
        theirs
            .iter()
            .filter(|f| !ours.contains(f))
            .map(|f| format!("pdffonts: {}", label(f))),
    );
    out
}

/// Every mismatch between lamina's facts and the tools' about one file, and which facts were
/// compared.
pub(super) fn compare_file(file: &str, lamina: &Value, tools: &Value) -> (Vec<Mismatch>, [bool; 2]) {
    let mut mismatches = Vec::new();
    let repaired = |v: &Value| v.get("outcome").and_then(Value::as_str) == Some("repaired");
    let mut compared = [false; 2];
    for (i, fact) in FACTS.into_iter().enumerate() {
        let (ours, theirs) = (&lamina[fact], &tools[fact]);
        if ours.is_null() || theirs.is_null() || (fact == "objects" && (repaired(lamina) || repaired(tools))) {
            continue;
        }
        if let Some(slot) = compared.get_mut(i) {
            *slot = true;
        }
        let differences = if fact == "objects" {
            object_differences(ours, theirs)
        } else {
            font_differences(ours, theirs)
        };
        if !differences.is_empty() {
            let shown: Vec<String> = differences.iter().take(5).cloned().collect();
            let more = differences.len().saturating_sub(shown.len());
            let mut detail = shown.join("; ");
            if more > 0 {
                let _ = write!(detail, "; {more} more");
            }
            mismatches.push(Mismatch {
                file: file.to_owned(),
                fact,
                detail,
            });
        }
    }
    (mismatches, compared)
}

/// A `[[known]]` entry of [`EXCEPTIONS_FILE`].
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Known {
    file: String,
    /// Empty: both facts.
    facts: Vec<String>,
    issue: String,
}

impl Known {
    fn excuses(&self, m: &Mismatch) -> bool {
        glob(&self.file, &m.file) && (self.facts.is_empty() || self.facts.iter().any(|f| f == m.fact))
    }

    fn label(&self) -> String {
        format!("{} [{}] ({})", self.file, self.facts.join(", "), self.issue)
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
                .find(|k| !["file", "facts", "issue", "reason"].contains(&k.as_str()))
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
            let facts: Vec<String> = match entry.get("facts") {
                None => Vec::new(),
                Some(v) => v
                    .as_array()
                    .context("`facts` must be an array")?
                    .iter()
                    .map(|f| f.as_str().map(str::to_owned).context("`facts` holds strings"))
                    .collect::<Result<_>>()?,
            };
            if let Some(bad) = facts.iter().find(|f| !FACTS.contains(&f.as_str())) {
                bail!("[[known]]: unknown fact {bad:?} (objects or fonts)");
            }
            text("reason")?;
            Ok(Known {
                file: text("file")?,
                facts,
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
    /// Files whose objects were compared, and those that matched.
    pub(super) objects: (usize, usize),
    /// Files whose fonts were compared, those that matched, and those that matched with at least
    /// one font.
    pub(super) fonts: (usize, usize, usize),
    /// Files lamina failed to open, by name with its error code.
    pub(super) lamina_failed: BTreeMap<String, String>,
    /// Files lamina panicked, crashed or hung on (always failures), with the outcome.
    pub(super) lamina_errors: BTreeMap<String, String>,
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

    fn summary_line(&self) -> String {
        format!(
            "Inspection differential: objects of {} files compared with qpdf, {} match; fonts of {} files \
             compared with pdffonts, {} match ({} with fonts); {} differ ({} unexcused); skipped {} lamina \
             couldn't open; {} lamina crashes",
            self.objects.0,
            self.objects.1,
            self.fonts.0,
            self.fonts.1,
            self.fonts.2,
            self.mismatches.len(),
            self.unexcused().count(),
            self.lamina_failed.len(),
            self.lamina_errors.len()
        )
    }
}

/// Compares lamina's facts (by input name) with the tools' (by input name).
pub(super) fn compare(
    lamina: &BTreeMap<String, Value>,
    tools: &BTreeMap<String, Value>,
    known: &[Known],
) -> Comparison {
    let mut c = Comparison::default();
    let mut used = vec![false; known.len()];
    for (name, ours) in lamina {
        match ours.get("outcome").and_then(Value::as_str) {
            Some("opened" | "repaired") => {}
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
        let Some(theirs) = tools.get(name) else { continue };
        let (mismatches, [objects, fonts]) = compare_file(name, ours, theirs);
        let differs = |fact: &str| mismatches.iter().any(|m| m.fact == fact);
        if objects {
            c.objects.0 += 1;
            c.objects.1 += usize::from(!differs("objects"));
        }
        if fonts {
            c.fonts.0 += 1;
            if !differs("fonts") {
                c.fonts.1 += 1;
                c.fonts.2 += usize::from(ours["fonts"].as_array().is_some_and(|f| !f.is_empty()));
            }
        }
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
    qpdf_version: String,
    pdffonts_version: String,
    broken: Vec<String>,
}

fn markdown(c: &Comparison, known: &[Known], info: &RunInfo) -> String {
    let mut md = String::from("## Inspection differential (lamina vs qpdf and pdffonts)\n\n");
    let _ = writeln!(
        md,
        "{} inputs{} at `{}`; lamina {}, qpdf {}, pdffonts {}.\n\n{}\n",
        info.inputs,
        info.sample.map(|n| format!(" (sample of {n})")).unwrap_or_default(),
        info.commit,
        info.lamina_version,
        info.qpdf_version,
        info.pdffonts_version,
        c.summary_line()
    );
    for broken in &info.broken {
        let _ = writeln!(md, "- **Reader broken:** {broken}");
    }
    for (name, outcome) in &c.lamina_errors {
        let _ = writeln!(md, "- **lamina {outcome}:** `{name}`");
    }
    let rows: Vec<_> = c.mismatches.iter().take(MAX_ROWS).collect();
    if !rows.is_empty() {
        // Font names are text from the files: the details stay in report.json.
        md.push_str("\nWhat differs is in report.json.\n\n| File | Fact | Excused by |\n| --- | --- | --- |\n");
        for (m, excuse) in rows {
            let by = excuse
                .and_then(|i| known.get(i))
                .map_or_else(|| "**unexcused**".to_owned(), Known::label);
            let _ = writeln!(md, "| `{}` | {} | {by} |", util::md_cell(&m.file), m.fact);
        }
        if c.mismatches.len() > MAX_ROWS {
            let _ = writeln!(md, "\n{} more in report.json.", c.mismatches.len() - MAX_ROWS);
        }
    }
    for stale in &c.stale {
        let _ = writeln!(md, "- Stale `[[known]]` entry (excused nothing): {stale}");
    }
    md
}

fn report_json(c: &Comparison, known: &[Known], info: &RunInfo) -> Value {
    json!({
        "inputs": info.inputs,
        "commit": info.commit,
        "sample": info.sample,
        "lamina_version": info.lamina_version,
        "qpdf_version": info.qpdf_version,
        "pdffonts_version": info.pdffonts_version,
        "broken_readers": info.broken,
        "objects": {"compared": c.objects.0, "match": c.objects.1},
        "fonts": {"compared": c.fonts.0, "match": c.fonts.1, "match_with_fonts": c.fonts.2},
        "lamina_failed": c.lamina_failed,
        "lamina_errors": c.lamina_errors,
        "stale": c.stale,
        "mismatches": c.mismatches.iter().map(|(m, excuse)| json!({
            "file": m.file,
            "fact": m.fact,
            "detail": m.detail,
            "excused_by": excuse.and_then(|i| known.get(i)).map(Known::label),
        })).collect::<Vec<_>>(),
    })
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

/// The versions of qpdf and pdffonts, once both are at `doctor.rs`'s pins.
fn tool_versions(ws: &Workspace) -> Result<(String, String)> {
    for tool in [crate::doctor::qpdf(), crate::doctor::pdffonts()] {
        match tool.probe(ws) {
            Probe::Ok(_) => {}
            probe => bail!("{}", tool.problem(&probe)),
        }
    }
    Ok((QPDF_VERSION.to_owned(), POPPLER_VERSION.to_owned()))
}

/// `--update`: writes the tools' facts about [`COMMITTED`] to [`FACTS_FILE`].
fn update(ws: &Workspace) -> Result<()> {
    let (qpdf_version, pdffonts_version) = tool_versions(ws)?;
    let out = ws.root.join("target/nightly/differential-inspect");
    fs::create_dir_all(&out)?;
    let inputs: Vec<(String, PathBuf)> = COMMITTED
        .iter()
        .map(|name| ((*name).to_owned(), ws.root.join(name)))
        .collect();
    let mut files = Map::new();
    for (name, mut facts) in all_tool_facts(&inputs, &out)? {
        let errors = facts.get("errors").cloned().unwrap_or_default();
        if errors.as_object().is_some_and(|e| !e.is_empty()) {
            bail!("{name}: the tools failed: {errors}");
        }
        if let Some(facts) = facts.as_object_mut() {
            facts.remove("errors");
        }
        files.insert(name, facts);
    }
    let committed = json!({"qpdf": qpdf_version, "pdffonts": pdffonts_version, "files": files});
    fs::write(
        ws.root.join(FACTS_FILE),
        serde_json::to_string_pretty(&committed)? + "\n",
    )?;
    step(&format!("Wrote {FACTS_FILE}: {} files", COMMITTED.len()));
    Ok(())
}

/// `cargo xtask differential --live --inspect [--update] …`.
pub(super) fn run(ws: &Workspace, args: &LiveArgs) -> Result<()> {
    if args.update {
        return update(ws);
    }
    let path = ws.root.join(EXCEPTIONS_FILE);
    let known = match fs::read_to_string(&path) {
        Ok(text) => parse_exceptions(&text).with_context(|| format!("reading {EXCEPTIONS_FILE}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e).with_context(|| format!("reading {EXCEPTIONS_FILE}")),
    };
    let (qpdf_version, pdffonts_version) = tool_versions(ws)?;
    let out = ws.root.join("target/nightly/differential-inspect");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    fs::create_dir_all(&out)?;
    let mut inputs = match corpus::verified_inputs(ws, &args.sources) {
        Ok(inputs) => inputs,
        // Locally, without the corpus fetched, the committed PDFs still compare; in CI the
        // corpus is always fetched first, so its absence fails there.
        Err(e) if args.sources.is_empty() && std::env::var_os("CI").is_none() => {
            println!("::warning::the external corpus isn't there ({e:#}): comparing the committed PDFs only");
            Vec::new()
        }
        Err(e) => return Err(e),
    };
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
    step(&format!("Inspection differential: {} inputs", inputs.len()));
    ws.cargo(&[
        "build",
        "--release",
        "--locked",
        "-q",
        "-p",
        "laminapdf",
        "--features",
        "extract",
        "--example",
        "inspect-facts",
    ])?;
    let readers = [runner::Reader {
        name: "lamina",
        program: example_binary(ws, "inspect-facts").into_os_string(),
        args: Vec::new(),
        dir: ws.root.clone(),
        reports_repair: true,
    }];
    let (reports, broken) = run_readers(&readers, &inputs, &out)?;
    let lamina: Report = reports.into_iter().next().context("no lamina report")?;
    let tools = all_tool_facts(&inputs, &out)?;
    fs::write(out.join("tools.json"), serde_json::to_string_pretty(&tools)? + "\n")?;
    let comparison = compare(&lamina.facts, &tools, &known);
    let info = RunInfo {
        inputs: inputs.len(),
        commit: util::git_stdout(&ws.root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned()),
        sample: args.sample,
        lamina_version: lamina.version,
        qpdf_version,
        pdffonts_version,
        broken,
    };
    let md = markdown(&comparison, &known, &info);
    fs::write(out.join("report.md"), &md)?;
    fs::write(
        out.join("report.json"),
        serde_json::to_string_pretty(&report_json(&comparison, &known, &info))? + "\n",
    )?;
    util::summary(&md)?;
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
            "lamina's objects or fonts disagree with qpdf's or pdffonts' on {} files' facts, or lamina failed \
             to read {} files (panic, crash or timeout), and {EXCEPTIONS_FILE} doesn't excuse them; see the report",
            comparison.unexcused().count(),
            comparison.lamina_errors.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-written in the shape pdffonts 26.10 prints.
    const PDFFONTS: &str = "\
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
QLGHZG+NotoSans-Regular              CID TrueType      Identity-H       yes yes yes      3  0
Helvetica                            Type 1            WinAnsi          no  no  no  [none]
Helvetica                            Type 1            WinAnsi          no  no  no  [none]
";

    #[test]
    fn pdffonts_rows_should_read_as_lamina_writes_fonts_each_once() {
        let fonts = parse_pdffonts(PDFFONTS);
        assert_eq!(
            fonts,
            json!([
                {"object": null, "name": "Helvetica", "family": "Type1", "embedded": false, "subset": false, "to_unicode": false},
                {"object": "3 0", "name": "NotoSans-Regular", "family": "Type0", "embedded": true, "subset": true, "to_unicode": true},
            ])
        );
    }

    #[test]
    fn show_xref_should_give_offsets_and_object_stream_places() {
        let xref = parse_xref("1/0: compressed; stream = 6, index = 0\n5/2: uncompressed; offset = 15\n");
        assert_eq!(xref.get(&1), Some(&(0, "stream:6:0".to_owned())));
        assert_eq!(xref.get(&5), Some(&(2, "offset:15".to_owned())));
    }

    #[test]
    fn qpdf_json_should_give_kinds_and_types() {
        let json = json!({"qpdf": [{}, {
            "obj:1 0 R": {"value": {"/Type": "/Catalog"}},
            "obj:2 0 R": {"stream": {"dict": {"/Length": 3}}},
            "obj:3 0 R": {"value": 42},
            "trailer": {"value": {}}
        }]});
        let xref = parse_xref("1/0: uncompressed; offset = 9\n");
        let objects = qpdf_objects(&json, &xref);
        assert_eq!(
            objects["1"],
            json!({"gen": 0, "kind": "dictionary", "type": "Catalog", "at": "offset:9"})
        );
        assert_eq!(
            (objects["2"]["kind"].as_str(), objects["3"]["kind"].as_str()),
            (Some("stream"), Some("other"))
        );
    }

    #[test]
    fn a_repaired_file_should_leave_its_objects_out() {
        let lamina = json!({"outcome": "repaired", "objects": {"1": {}}, "fonts": []});
        let tools = json!({"outcome": "opened", "objects": {"2": {}}, "fonts": []});
        let (mismatches, compared) = compare_file("f.pdf", &lamina, &tools);
        assert_eq!((mismatches, compared), (Vec::new(), [false, true]));
    }

    #[test]
    fn a_known_entry_should_excuse_only_its_fact() {
        let lamina = BTreeMap::from([(
            "a.pdf".to_owned(),
            json!({"outcome": "opened", "objects": {"1": {"kind": "stream"}}, "fonts": [{"name": "A"}]}),
        )]);
        let tools = BTreeMap::from([(
            "a.pdf".to_owned(),
            json!({"outcome": "opened", "objects": {"1": {"kind": "other"}}, "fonts": []}),
        )]);
        let known =
            parse_exceptions("[[known]]\nfile = \"a*\"\nfacts = [\"fonts\"]\nissue = \"PENDING\"\nreason = \"r\"\n")
                .unwrap();
        let c = compare(&lamina, &tools, &known);
        let unexcused: Vec<&str> = c.unexcused().map(|m| m.fact).collect();
        assert_eq!((unexcused, c.objects, c.fonts), (vec!["objects"], (1, 0), (1, 0, 0)));
    }

    #[test]
    fn exceptions_should_refuse_unknown_facts() {
        let error = parse_exceptions("[[known]]\nfile = \"a\"\nfacts = [\"font\"]\nissue = \"x\"\nreason = \"r\"\n");
        assert!(error.unwrap_err().to_string().contains("font"));
    }

    #[test]
    fn the_committed_list_should_hold_existing_pdfs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let missing: Vec<&str> = COMMITTED.iter().copied().filter(|f| !root.join(f).is_file()).collect();
        assert!(missing.is_empty(), "{missing:?}");
    }

    /// The nightly builds pdffonts with pdftotext.
    #[test]
    fn nightly_should_build_pdffonts() {
        let action = include_str!("../../../../.github/actions/poppler/action.yml");
        assert!(
            action.contains("--target pdftotext pdffonts"),
            "the poppler action should build pdffonts"
        );
    }
}
