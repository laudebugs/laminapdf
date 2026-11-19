//! `cargo xtask differential --live --inspect` (ADR 0018 tier 6): the objects and fonts Lamina
//! PDF's inspector reads against qpdf's and pdffonts', every night.
//!
//! The inputs are the external corpus and the committed PDFs in [`LOCAL_DIRS`]. lamina's side is
//! the `inspect-facts` example (`crates/laminapdf/examples/differential/`) under [`runner`]; the
//! tools are external processes only (qpdf is Apache-2.0; pdffonts is poppler's, GPL, so never
//! linked, ADR 0019), each pinned in `doctor.rs`. Per file:
//!
//! - **objects**: `qpdf --json=2 --json-key=qpdf` gives every object's number, generation, kind
//!   (stream, dictionary, other), `/Type` and `/Subtype` (else `/S`, lamina's rule); `qpdf
//!   --show-xref` where each is stored (a byte offset, or an object stream and index).
//! - **xref**: the newest cross-reference section, through the trailer qpdf's JSON gives (qpdf
//!   lists no sections or revisions of its own): its kind (a table, a cross-reference stream, or
//!   a hybrid table whose trailer names an `/XRefStm`), `/Size` and `/Prev`, so whether the file
//!   was updated and where the section before it is.
//! - **fonts**: `pdffonts` gives every font of every page (resources, forms, annotations'
//!   appearances) with its object, name, type, encoding, and whether it is embedded, a subset
//!   and has a ToUnicode CMap; its type maps to the `/Subtype` family (`CID …` → `Type0`) and
//!   lamina's encoding to pdffonts' spelling (`WinAnsi`, `Identity-H`, `Custom`).
//!
//! A file either side repaired has its objects and section left out: they are then each
//! reader's own reconstruction. Each object or font the two read differently fails the run
//! unless [`EXCEPTIONS_FILE`] excuses it: a `[[class]]` by its values ([`differences`]: a
//! `relation`, the `fields` that may differ, what lamina's and the tool's records hold, and the
//! `rule` behind the difference), or a `[[known]]` entry (`file`, a glob; `facts`, default all;
//! `issue`; `reason`) for the fact in that file. The report goes to
//! `target/nightly/differential-inspect/`, with the number of files whose font list matches.
//!
//! `--update` writes the tools' facts about [`COMMITTED`] to [`FACTS_FILE`], which
//! `crates/laminapdf/tests/inspect_differential.rs` compares lamina with on every PR.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use super::known::{Known, known_entries, warn_pending};
use super::{LiveArgs, MAX_ROWS, Report, example_binary, local_pdfs, run_readers};
use crate::doctor::{POPPLER_VERSION, Probe, QPDF_VERSION};
use crate::workspace::Workspace;
use crate::{corpus, runner, step, util};
use differences::{Class, Difference};

mod differences;

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
/// incremental updates, a hybrid-reference file, signatures and annotations' appearances.
pub(super) const COMMITTED: [&str; 31] = [
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
    "fixtures/revisions/hybrid.pdf",
    "fixtures/signed/approval.pdf",
    "fixtures/signed/locked-page1.pdf",
    "fixtures/signed/two-signatures.pdf",
];

/// The tools' facts about [`COMMITTED`], for the PR lane.
const FACTS_FILE: &str = "fixtures/differential/inspect.json";

/// The `[[known]]` disagreements; a missing file excuses nothing.
pub(super) const EXCEPTIONS_FILE: &str = "fixtures/differential/inspect-exceptions.toml";

/// The two facts compared.
const FACTS: [&str; 3] = ["objects", "xref", "fonts"];

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
    let mut command = Command::new(crate::program_path(program));
    command
        .args(args)
        .arg(path)
        .stdout(fs::File::create(&out)?)
        .stderr(fs::File::create(&err)?);
    let runner::Exit::Status(status) =
        runner::run_with_timeout(command, TOOL_TIMEOUT).with_context(|| format!("running {program}"))?
    else {
        return Ok(Ran::Failed("timeout".to_owned()));
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
        let name = |key: &str| {
            dict.and_then(|d| d.get(key))
                .and_then(Value::as_str)
                .and_then(|t| t.strip_prefix('/'))
        };
        // lamina's subtype is `/Subtype`, else a structure element's `/S`: the same rule here.
        let subtype = name("/Subtype").or_else(|| name("/S"));
        let at = xref.get(&num).map(|(_, place)| place.clone());
        out.insert(
            num.to_string(),
            json!({"gen": generation, "kind": kind, "type": name("/Type"), "subtype": subtype, "at": at}),
        );
    }
    Value::Object(out)
}

/// The newest cross-reference section as qpdf's JSON gives it, through its trailer (qpdf has
/// no per-section listing): `kind` (`stream` for a cross-reference stream's dictionary,
/// `hybrid` for a table whose trailer names an `/XRefStm`, else `table`), `/Size` and `/Prev`
/// (the section before it, so whether the file was updated).
pub(super) fn qpdf_xref(json: &Value) -> Value {
    let trailer = json.pointer("/qpdf/1/trailer/value");
    let get = |key: &str| trailer.and_then(|t| t.get(key));
    let kind = if get("/Type").and_then(Value::as_str) == Some("/XRef") {
        "stream"
    } else if get("/XRefStm").is_some() {
        "hybrid"
    } else {
        "table"
    };
    json!({"kind": kind, "size": get("/Size").and_then(Value::as_u64), "prev": get("/Prev").and_then(Value::as_u64)})
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

/// pdffonts' font types (`fontTypeNames` in poppler's `pdffonts.cc`), longest first, so a row's
/// type is the longest one it ends with.
const PDFFONTS_TYPES: [&str; 12] = [
    "CID TrueType (OT)",
    "CID Type 0C (OT)",
    "TrueType (OT)",
    "Type 1C (OT)",
    "CID TrueType",
    "CID Type 0C",
    "CID Type 0",
    "TrueType",
    "Type 1C",
    "unknown",
    "Type 1",
    "Type 3",
];

/// The last white-space-separated word of `rest`, which keeps what comes before it.
fn pop_word<'a>(rest: &mut &'a str) -> Option<&'a str> {
    let text = rest.trim_end();
    let at = text.rfind(char::is_whitespace).map_or(0, |i| i + 1);
    let word = text.get(at..)?;
    *rest = text.get(..at)?;
    (!word.is_empty()).then_some(word)
}

/// The type `rest` ends with, and the name before it.
fn split_type(rest: &str) -> Option<(&str, &'static str)> {
    let rest = rest.trim_end();
    PDFFONTS_TYPES
        .into_iter()
        .find_map(|kind| Some((rest.strip_suffix(kind)?.trim_end(), kind)))
}

/// One row of pdffonts' table, read from its end: the object ID (`N G` or `[none]`), the `uni`,
/// `sub` and `emb` columns, the encoding (one word, or none), the type (one of
/// [`PDFFONTS_TYPES`]) and the name. pdffonts pads its columns but never cuts a value, so a
/// name wider than its column (or one decoded lossily, whose bytes no longer line up) moves
/// the rest of the row: only the order of the columns holds.
fn pdffonts_row(line: &str) -> Option<Value> {
    let mut rest = line;
    let object = match pop_word(&mut rest)? {
        "[none]" => None,
        generation => Some(format!("{} {generation}", pop_word(&mut rest)?)),
    };
    let to_unicode = pop_word(&mut rest)? == "yes";
    let subset = pop_word(&mut rest)? == "yes";
    let embedded = pop_word(&mut rest)? == "yes";
    // An empty encoding leaves the type last.
    let (name, kind, encoding) = if let Some((name, kind)) = split_type(rest) {
        (name, kind, "")
    } else {
        let encoding = pop_word(&mut rest)?;
        let (name, kind) = split_type(rest)?;
        (name, kind, encoding)
    };
    let name = match name {
        "[none]" => None,
        n if subset => n.get(7..).map(str::to_owned),
        n => Some(n.to_owned()),
    };
    Some(json!({
        "object": object,
        "name": name,
        "family": family(kind),
        "encoding": encoding,
        "embedded": embedded,
        "subset": subset,
        "to_unicode": to_unicode,
    }))
}

/// pdffonts' table in lamina's facts' shape, sorted, each font once, and how many rows couldn't
/// be read: a font left out would read as one only lamina lists.
pub(super) fn parse_pdffonts(text: &str) -> (Value, usize) {
    let rows: Vec<Option<Value>> = text
        .lines()
        .skip_while(|l| !l.starts_with("---"))
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(pdffonts_row)
        .collect();
    let unread = rows.iter().filter(|r| r.is_none()).count();
    let mut out: Vec<Value> = rows.into_iter().flatten().collect();
    out.sort_by_key(ToString::to_string);
    // pdffonts lists a direct font once per place it is met (two signatures' appearances each
    // with their own Courier); lamina lists it once per type and name.
    out.dedup();
    (Value::Array(out), unread)
}

/// The key of a tool's `errors` that counts the pdffonts rows [`parse_pdffonts`] couldn't read.
const UNREAD_ROWS: &str = "pdffonts_rows_unread";

/// The tools' facts about the file at `path`: `{outcome, objects, fonts}`, with `null` for a
/// fact a tool couldn't give (and why in `errors`). `outcome` is `repaired` when qpdf rebuilt
/// the cross-reference table.
pub(super) fn tool_facts(path: &Path, work: &Path) -> Result<Value> {
    let mut errors = Map::new();
    let mut outcome = "opened";
    let mut xref_section = Value::Null;
    let objects = match (
        run_tool("qpdf", &["--json=2", "--json-key=qpdf"], path, work)?,
        run_tool("qpdf", &["--show-xref"], path, work)?,
    ) {
        (Ran::Done { stdout, stderr }, Ran::Done { stdout: xref, .. }) => {
            if stderr.contains("reconstruct") {
                outcome = "repaired";
            }
            match serde_json::from_str::<Value>(&stdout) {
                Ok(json) => {
                    xref_section = qpdf_xref(&json);
                    qpdf_objects(&json, &parse_xref(&xref))
                }
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
        Ran::Done { stdout, .. } => {
            let (fonts, unread) = parse_pdffonts(&stdout);
            if unread > 0 {
                errors.insert(UNREAD_ROWS.to_owned(), json!(unread));
            }
            fonts
        }
        Ran::Failed(why) => {
            errors.insert("fonts".to_owned(), json!(format!("pdffonts: {why}")));
            Value::Null
        }
    };
    Ok(json!({"outcome": outcome, "objects": objects, "xref": xref_section, "fonts": fonts, "errors": errors}))
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

/// What differs between two newest sections: the keys whose values differ.
fn xref_differences(ours: &Value, theirs: &Value) -> Vec<String> {
    ["kind", "size", "prev"]
        .into_iter()
        .filter(|k| ours.get(k) != theirs.get(k))
        .map(|k| format!("newest section: {k}"))
        .collect()
}

/// Every mismatch between lamina's facts and the tools' about one file, each with the objects
/// or fonts that differ (none for the cross-reference section), and which facts were compared.
pub(super) fn compare_file(file: &str, lamina: &Value, tools: &Value) -> (Vec<(Mismatch, Vec<Difference>)>, [bool; 3]) {
    let mut mismatches = Vec::new();
    let repaired = |v: &Value| v.get("outcome").and_then(Value::as_str) == Some("repaired");
    let mut compared = [false; 3];
    let appearance = lamina
        .get("appearance_fonts")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    for (i, fact) in FACTS.into_iter().enumerate() {
        let (ours, theirs) = (&lamina[fact], &tools[fact]);
        // A repaired file's objects and sections are each reader's own rebuild.
        let rebuilt = fact != "fonts" && (repaired(lamina) || repaired(tools));
        if ours.is_null() || theirs.is_null() || rebuilt {
            continue;
        }
        if let Some(slot) = compared.get_mut(i) {
            *slot = true;
        }
        let (labels, found) = match fact {
            "objects" => labelled(differences::objects(ours, theirs)),
            "fonts" => labelled(differences::fonts(ours, theirs, appearance)),
            _ => (xref_differences(ours, theirs), Vec::new()),
        };
        if !labels.is_empty() {
            let shown: Vec<String> = labels.iter().take(5).cloned().collect();
            let more = labels.len().saturating_sub(shown.len());
            let mut detail = shown.join("; ");
            if more > 0 {
                let _ = write!(detail, "; {more} more");
            }
            let mismatch = Mismatch {
                file: file.to_owned(),
                fact,
                detail,
            };
            mismatches.push((mismatch, found));
        }
    }
    (mismatches, compared)
}

/// The differences' labels, and the differences.
fn labelled(found: Vec<Difference>) -> (Vec<String>, Vec<Difference>) {
    (found.iter().map(|d| d.label.clone()).collect(), found)
}

/// The `[[class]]` and `[[known]]` entries of [`EXCEPTIONS_FILE`].
#[derive(Debug, Default)]
pub(super) struct Exceptions {
    classes: Vec<Class>,
    known: Vec<Known>,
}

/// The entries of [`EXCEPTIONS_FILE`]'s text.
pub(super) fn parse_exceptions(text: &str) -> Result<Exceptions> {
    let mut table: toml::Table = toml::from_str(text)?;
    let classes = match table.remove("class") {
        None => Vec::new(),
        Some(entries) => entries
            .as_array()
            .context("`class` must be [[class]] entries")?
            .iter()
            .map(|entry| Class::read(entry, &["objects", "fonts"]))
            .collect::<Result<_>>()?,
    };
    let known = known_entries(&table, "facts", &FACTS)?;
    Ok(Exceptions { classes, known })
}

/// The comparison of a whole run.
#[derive(Debug, Default)]
pub(super) struct Comparison {
    /// Mismatches with what excused each (its classes and known entry), if they did.
    pub(super) mismatches: Vec<(Mismatch, Option<String>)>,
    /// Files whose objects were compared, and those that matched.
    pub(super) objects: (usize, usize),
    /// Files whose newest cross-reference section was compared, and those that matched.
    pub(super) xref: (usize, usize),
    /// Files whose fonts were compared, those that matched, and those that matched with at least
    /// one font.
    pub(super) fonts: (usize, usize, usize),
    /// Files lamina failed to open, by name with its error code.
    pub(super) lamina_failed: BTreeMap<String, String>,
    /// Files lamina panicked, crashed or hung on (always failures), with the outcome.
    pub(super) lamina_errors: BTreeMap<String, String>,
    /// `[[class]]` and `[[known]]` entries that excused nothing.
    pub(super) stale: Vec<String>,
}

/// How a report names the class at `index`.
fn class_label(classes: &[Class], index: usize) -> String {
    classes
        .get(index)
        .map_or_else(String::new, |c| format!("class {}: {}", index + 1, c.rule))
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
            "Inspection differential: objects of {} files compared with qpdf, {} match; cross-reference \
             sections of {} files, {} match; fonts of {} files compared with pdffonts, {} match ({} with \
             fonts); {} differ ({} unexcused); skipped {} lamina couldn't open; {} lamina crashes",
            self.objects.0,
            self.objects.1,
            self.xref.0,
            self.xref.1,
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
    exceptions: &Exceptions,
) -> Comparison {
    let mut c = Comparison::default();
    let mut used = Used {
        classes: vec![false; exceptions.classes.len()],
        known: vec![false; exceptions.known.len()],
    };
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
        let (mismatches, [objects, xref, fonts]) = compare_file(name, ours, theirs);
        let differs = |fact: &str| mismatches.iter().any(|(m, _)| m.fact == fact);
        if objects {
            c.objects.0 += 1;
            c.objects.1 += usize::from(!differs("objects"));
        }
        if xref {
            c.xref.0 += 1;
            c.xref.1 += usize::from(!differs("xref"));
        }
        if fonts {
            c.fonts.0 += 1;
            if !differs("fonts") {
                c.fonts.1 += 1;
                c.fonts.2 += usize::from(ours["fonts"].as_array().is_some_and(|f| !f.is_empty()));
            }
        }
        for (m, found) in mismatches {
            let by = excuse(&m, &found, ours, exceptions, &mut used);
            c.mismatches.push((m, by));
        }
    }
    let stale_classes = used
        .classes
        .iter()
        .enumerate()
        .filter(|(_, u)| !**u)
        .map(|(i, _)| class_label(&exceptions.classes, i));
    let stale_known = exceptions
        .known
        .iter()
        .zip(&used.known)
        .filter(|(_, u)| !**u)
        .map(|(k, _)| k.label());
    c.stale = stale_classes.chain(stale_known).collect();
    c
}

/// Which `[[class]]` and `[[known]]` entries excused something.
struct Used {
    classes: Vec<bool>,
    known: Vec<bool>,
}

/// What excuses mismatch `m`, whose objects or fonts are `found`: a class for each of them, or
/// a `[[known]]` entry for the rest (and for a cross-reference section, which has none).
fn excuse(
    m: &Mismatch,
    found: &[Difference],
    facts: &Value,
    exceptions: &Exceptions,
    used: &mut Used,
) -> Option<String> {
    let mut by: Vec<usize> = Vec::new();
    let mut rest = 0;
    for d in found {
        match exceptions
            .classes
            .iter()
            .position(|class| class.excuses(&m.file, m.fact, d, facts))
        {
            Some(i) => {
                if !by.contains(&i) {
                    by.push(i);
                }
            }
            None => rest += 1,
        }
    }
    let mut labels: Vec<String> = Vec::new();
    if rest > 0 || found.is_empty() {
        let known = exceptions.known.iter().position(|k| k.excuses(&m.file, m.fact))?;
        if let Some(slot) = used.known.get_mut(known) {
            *slot = true;
        }
        labels.extend(exceptions.known.get(known).map(Known::label));
    }
    // Only once the whole mismatch is excused do its classes count as used: a class that
    // matched part of an unexcused mismatch excused nothing.
    for &i in &by {
        if let Some(slot) = used.classes.get_mut(i) {
            *slot = true;
        }
    }
    labels.extend(by.iter().map(|&i| class_label(&exceptions.classes, i)));
    Some(labels.join("; "))
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

fn markdown(c: &Comparison, info: &RunInfo) -> String {
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
            let by = excuse.clone().unwrap_or_else(|| "**unexcused**".to_owned());
            let _ = writeln!(md, "| `{}` | {} | {by} |", util::md_cell(&m.file), m.fact);
        }
        if c.mismatches.len() > MAX_ROWS {
            let _ = writeln!(md, "\n{} more in report.json.", c.mismatches.len() - MAX_ROWS);
        }
    }
    for stale in &c.stale {
        let _ = writeln!(md, "- Stale entry (excused nothing): {stale}");
    }
    md
}

fn report_json(c: &Comparison, info: &RunInfo) -> Value {
    json!({
        "inputs": info.inputs,
        "commit": info.commit,
        "sample": info.sample,
        "lamina_version": info.lamina_version,
        "qpdf_version": info.qpdf_version,
        "pdffonts_version": info.pdffonts_version,
        "broken_readers": info.broken,
        "objects": {"compared": c.objects.0, "match": c.objects.1},
        "xref": {"compared": c.xref.0, "match": c.xref.1},
        "fonts": {"compared": c.fonts.0, "match": c.fonts.1, "match_with_fonts": c.fonts.2},
        "lamina_failed": c.lamina_failed,
        "lamina_errors": c.lamina_errors,
        "stale": c.stale,
        "mismatches": c.mismatches.iter().map(|(m, excuse)| json!({
            "file": m.file,
            "fact": m.fact,
            "detail": m.detail,
            "excused_by": excuse,
        })).collect::<Vec<_>>(),
    })
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
    let exceptions = match fs::read_to_string(&path) {
        Ok(text) => parse_exceptions(&text).with_context(|| format!("reading {EXCEPTIONS_FILE}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Exceptions::default(),
        Err(e) => return Err(e).with_context(|| format!("reading {EXCEPTIONS_FILE}")),
    };
    let (qpdf_version, pdffonts_version) = tool_versions(ws)?;
    let out = ws.root.join("target/nightly/differential-inspect");
    super::fresh_dir(&out)?;
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
        inputs.extend(local_pdfs(&ws.root, &LOCAL_DIRS)?);
    }
    let inputs = super::chosen(inputs, args.sample)?;
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
    // A pdffonts row the parser can't read would show its font as one only lamina lists, which
    // a class could excuse: the reader is broken instead.
    let mut broken = broken;
    broken.extend(tools.iter().filter_map(|(name, facts)| {
        let unread = facts.pointer(&format!("/errors/{UNREAD_ROWS}"))?.as_u64()?;
        Some(format!("pdffonts: {unread} rows of {name} couldn't be read"))
    }));
    let comparison = compare(&lamina.facts, &tools, &exceptions);
    let info = RunInfo {
        inputs: inputs.len(),
        commit: util::git_stdout(&ws.root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned()),
        sample: args.sample,
        lamina_version: lamina.version,
        qpdf_version,
        pdffonts_version,
        broken,
    };
    let md = markdown(&comparison, &info);
    fs::write(out.join("report.md"), &md)?;
    fs::write(
        out.join("report.json"),
        serde_json::to_string_pretty(&report_json(&comparison, &info))? + "\n",
    )?;
    util::summary(&md)?;
    println!("{}", comparison.summary_line());
    warn_pending(&exceptions.known, EXCEPTIONS_FILE);
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
        let (fonts, unread) = parse_pdffonts(PDFFONTS);
        assert_eq!(unread, 0);
        assert_eq!(
            fonts,
            json!([
                {"object": null, "name": "Helvetica", "family": "Type1", "encoding": "WinAnsi", "embedded": false, "subset": false, "to_unicode": false},
                {"object": "3 0", "name": "NotoSans-Regular", "family": "Type0", "encoding": "Identity-H", "embedded": true, "subset": true, "to_unicode": true},
            ])
        );
    }

    /// Rows whose names outgrow the name column: pdffonts never cuts a value, so the columns
    /// after it move.
    #[test]
    fn pdffonts_rows_should_read_from_their_end() {
        let table = "\
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
ABCDEF+LucidaSansUnicode,Bold-WinCharSetFFFF CID TrueType      Identity-H       yes yes yes     32  0
\u{fffd}\u{fffd}_GB2312                         TrueType          WinAnsi          no  no  no       5  0
Segoe UI,Bold                        Type 1C (OT)      Custom           yes no  yes   1982  0
";
        let (fonts, _) = parse_pdffonts(table);
        let get = |i: usize, key: &str| fonts[i][key].clone();
        assert_eq!(
            (0..3)
                .map(|i| (get(i, "object"), get(i, "name"), get(i, "family"), get(i, "subset")))
                .collect::<Vec<_>>(),
            [
                (
                    json!("5 0"),
                    json!("\u{fffd}\u{fffd}_GB2312"),
                    json!("TrueType"),
                    json!(false)
                ),
                (json!("1982 0"), json!("Segoe UI,Bold"), json!("Type1"), json!(false)),
                (
                    json!("32 0"),
                    json!("LucidaSansUnicode,Bold-WinCharSetFFFF"),
                    json!("Type0"),
                    json!(true)
                ),
            ]
        );
    }

    /// An empty encoding leaves the type last; a `[none]` name is no name.
    #[test]
    fn pdffonts_rows_should_read_an_empty_encoding_and_no_name() {
        let table = "\
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
[none]                               Type 3                             yes no  no       9  0
";
        let (fonts, unread) = parse_pdffonts(table);
        assert_eq!(
            (fonts, unread),
            (
                json!([{"object": "9 0", "name": null, "family": "Type3", "encoding": "", "embedded": true, "subset": false, "to_unicode": false}]),
                0
            )
        );
    }

    #[test]
    fn pdffonts_rows_that_cant_be_read_should_be_counted() {
        let table = "\
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
Helvetica                            Type 9            WinAnsi          no  no  no  [none]
";
        assert_eq!(parse_pdffonts(table), (json!([]), 1));
    }

    #[test]
    fn a_class_should_excuse_its_fonts_and_a_known_entry_the_rest() {
        let font = |object: &str, encoding: &str| json!({"object": object, "family": "TrueType", "encoding": encoding});
        let lamina = BTreeMap::from([(
            "a.pdf".to_owned(),
            json!({"outcome": "opened", "fonts": [font("4 0", "Builtin"), font("5 0", "Custom")]}),
        )]);
        let tools = BTreeMap::from([(
            "a.pdf".to_owned(),
            json!({"outcome": "opened", "fonts": [font("4 0", "WinAnsi"), font("5 0", "WinAnsi")]}),
        )]);
        let class = "[[class]]\nfacts = [\"fonts\"]\nrelation = \"differ\"\nfields = [\"encoding\"]\n\
                     lamina = { encoding = \"Builtin\" }\nrule = \"r\"\nreason = \"r\"\n";
        let known = "[[known]]\nfile = \"a.pdf\"\nissue = \"PENDING\"\nreason = \"r\"\n";
        let alone = compare(&lamina, &tools, &parse_exceptions(class).unwrap());
        let both = compare(&lamina, &tools, &parse_exceptions(&format!("{class}{known}")).unwrap());
        assert_eq!(
            (alone.unexcused().count(), both.unexcused().count(), both.stale.len()),
            (1, 0, 0)
        );
    }

    #[test]
    fn a_class_that_excused_nothing_should_be_stale() {
        let lamina = BTreeMap::from([("a.pdf".to_owned(), json!({"outcome": "opened", "fonts": []}))]);
        let tools = BTreeMap::from([("a.pdf".to_owned(), json!({"outcome": "opened", "fonts": []}))]);
        let class = "[[class]]\nfacts = [\"fonts\"]\nrelation = \"lamina-only\"\nrule = \"r\"\nreason = \"r\"\n";
        let c = compare(&lamina, &tools, &parse_exceptions(class).unwrap());
        assert_eq!(c.stale, ["class 1: r"]);
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
            json!({"gen": 0, "kind": "dictionary", "type": "Catalog", "subtype": null, "at": "offset:9"})
        );
        assert_eq!(
            (objects["2"]["kind"].as_str(), objects["3"]["kind"].as_str()),
            (Some("stream"), Some("other"))
        );
    }

    #[test]
    fn qpdf_trailer_should_give_the_newest_sections_kind() {
        let section = |trailer: Value| qpdf_xref(&json!({"qpdf": [{}, {"trailer": {"value": trailer}}]}));
        assert_eq!(
            section(json!({"/Size": 8, "/XRefStm": 497})),
            json!({"kind": "hybrid", "size": 8, "prev": null})
        );
        assert_eq!(section(json!({"/Type": "/XRef", "/Size": 8}))["kind"], "stream");
        assert_eq!(section(json!({"/Size": 6, "/Prev": 416}))["prev"], 416);
    }

    #[test]
    fn a_repaired_file_should_leave_its_objects_out() {
        let lamina = json!({"outcome": "repaired", "objects": {"1": {}}, "fonts": []});
        let tools = json!({"outcome": "opened", "objects": {"2": {}}, "fonts": []});
        let (mismatches, compared) = compare_file("f.pdf", &lamina, &tools);
        assert_eq!((mismatches.is_empty(), compared), (true, [false, false, true]));
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
