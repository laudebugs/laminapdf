//! `cargo xtask differential --live --text` (todo B9f; ADR 0018 tier 6): lamina's page text
//! against pdf.js (`getTextContent`) and pdftotext over the external corpus, every night.
//!
//! The readers are lamina's `text-facts` example (`crates/laminapdf/examples/differential/`) and
//! `tools/differential/text.mjs --facts pdfjs|pdftotext`, each under [`runner`] like the other
//! live readers. The comparison is B9c's (`crates/laminapdf/tests/text_differential.rs`): page by
//! page, with whitespace and bidi controls removed on both sides; a page agrees when lamina
//! matches at least one tool. A page where every tool that read it disagrees is a failure
//! unless a text `[[class]]` or `[[known]]` entry (`facts = ["text"]` in
//! `fixtures/differential/exceptions.toml`) excuses each tool; a tool that failed, crashed or
//! timed out on the file, or failed on that page, is left out of it. lamina failing to open a
//! file counts as empty pages (the structural differential reports the outcome itself); a
//! lamina panic, crash or hang is always a failure. pdftotext takes part only at the poppler
//! version `doctor.rs` pins: another version (or none) is a warning and pdf.js alone is
//! compared. The report goes to `target/nightly/differential-text/`, its agreement per source
//! to the job summary.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::ops::RangeInclusive;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::{
    Class, EXCEPTIONS, Entry, Exceptions, Excuse, Kind, LiveArgs, MAX_ROWS, PENDING_ISSUE, Report, Tally,
    example_binary, group, rate, run_readers,
};
use crate::doctor::{POPPLER_VERSION, Probe};
use crate::workspace::Workspace;
use crate::{corpus, runner, step, util};

/// The fact name of page text in `exceptions.toml`.
pub(super) const TEXT: &str = "text";

/// The text readers besides lamina; text `[[class]]` and `[[known]]` entries may name them.
pub(super) const TOOLS: [&str; 2] = ["pdfjs", "pdftotext"];

/// Characters of a page's text kept in report.json's rows (the Markdown cell keeps 80).
const JSON_TEXT_CHARS: usize = 400;

/// Whether an entry's `facts` make it a page-text entry.
pub(super) fn is_text(facts: &[String]) -> bool {
    facts.iter().any(|f| f == TEXT)
}

/// The text with every whitespace character and bidi control (U+200E, U+200F, U+202A–U+202E,
/// U+2066–U+2069; pdftotext wraps right-to-left runs in embeddings) removed: B9c's rule, the
/// same function as `squeeze` in `crates/laminapdf/tests/text_differential.rs`.
pub(super) fn squeeze(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
        .collect()
}

/// A reader's pages, squeezed; `None` for a page the reader failed on (`null`).
fn pages(facts: &Value) -> Vec<Option<String>> {
    facts
        .get("pages")
        .and_then(Value::as_array)
        .map(|pages| pages.iter().map(|p| p.as_str().map(squeeze)).collect())
        .unwrap_or_default()
}

/// What exceptions match for page `index`: lamina's file facts (`outcome`, `error`,
/// `warnings`), the page's error code (`page_error`) and its squeezed `text`.
fn page_facts(ours: &Value, index: usize, text: &str) -> Value {
    let field = |key: &str| ours.get(key).cloned().unwrap_or(Value::Null);
    json!({
        "outcome": field("outcome"),
        "error": field("error"),
        "warnings": field("warnings"),
        "page_error": ours.get("page_errors").and_then(|e| e.get(index.to_string())).cloned().unwrap_or(Value::Null),
        TEXT: text,
    })
}

/// `text` cut to `max` characters, with `…` when cut.
fn clip(text: &str, max: usize) -> String {
    if text.chars().count() > max {
        format!("{}…", text.chars().take(max.saturating_sub(1)).collect::<String>())
    } else {
        text.to_owned()
    }
}

/// `"0-3, 7"` → {0, 1, 2, 3, 7}: 0-based pages, ranges inclusive.
fn parse_pages(text: &str) -> Result<BTreeSet<usize>> {
    let mut out = BTreeSet::new();
    for part in text.split(',').map(str::trim) {
        let number = |s: &str| {
            s.trim()
                .parse::<usize>()
                .with_context(|| format!("page {s:?} in {text:?} is not a number"))
        };
        let range: RangeInclusive<usize> = match part.split_once('-') {
            Some((a, b)) => number(a)?..=number(b)?,
            None => number(part)?..=number(part)?,
        };
        if range.is_empty() {
            bail!("page range {part:?} in {text:?} runs backwards");
        }
        out.extend(range);
    }
    Ok(out)
}

/// The inverse of [`parse_pages`]: `{0, 1, 2, 3, 7}` → `"0-3, 7"`.
fn format_pages(pages: &BTreeSet<usize>) -> String {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &page in pages {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == page => *end = page,
            _ => runs.push((page, page)),
        }
    }
    runs.iter()
        .map(|(a, b)| if a == b { a.to_string() } else { format!("{a}-{b}") })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A text `[[known]]` entry for one file: the pages (and tools) it excuses, tracked by an issue.
#[derive(Debug)]
pub(super) struct TextKnown {
    file: String,
    pages: BTreeSet<usize>,
    /// Empty: every text tool.
    tools: Vec<String>,
    /// `None` while the issue is [`PENDING_ISSUE`].
    issue: Option<String>,
}

impl TextKnown {
    fn covers_tool(&self, tool: &str) -> bool {
        self.tools.is_empty() || self.tools.iter().any(|t| t == tool)
    }

    fn covers(&self, file: &str, page: usize, tool: &str) -> bool {
        self.file == file && self.pages.contains(&page) && self.covers_tool(tool)
    }

    /// One entry per file of a text `[[known]]`: its `[known.pages]` table maps each file to
    /// its pages (`"0-3, 7"`), or to `{ pages = "…", tools = [...] }`; the entry's `tools` is
    /// the default.
    pub(super) fn read(entry: &toml::Value, e: &Entry, issue: Option<&str>) -> Result<Vec<Self>> {
        if !e.files.is_empty() {
            bail!("a text [[known]] lists its files with their pages in [known.pages], not `file`: {entry:?}");
        }
        let table = entry
            .get("pages")
            .and_then(toml::Value::as_table)
            .filter(|t| !t.is_empty())
            .with_context(|| format!("a text [[known]] needs a [known.pages] table: {entry:?}"))?;
        let mut out = Vec::new();
        for (file, value) in table {
            let (pages, tools) = match value {
                toml::Value::String(pages) => (pages.as_str(), e.tools.clone()),
                toml::Value::Table(t) => {
                    let pages = t
                        .get("pages")
                        .and_then(toml::Value::as_str)
                        .with_context(|| format!("{file}: `pages` must be a string like \"0-3, 7\""))?;
                    let tools: Vec<String> = match t.get("tools").and_then(toml::Value::as_array) {
                        Some(list) => list.iter().filter_map(toml::Value::as_str).map(str::to_owned).collect(),
                        None => e.tools.clone(),
                    };
                    (pages, tools)
                }
                _ => bail!("{file}: a page list (\"0-3, 7\") or {{ pages, tools }}"),
            };
            if let Some(bad) = tools.iter().find(|t| !TOOLS.contains(&t.as_str())) {
                bail!("{file}: text tool {bad:?}; text tools are {TOOLS:?}");
            }
            out.push(Self {
                file: file.clone(),
                pages: parse_pages(pages).with_context(|| format!("[known.pages] {file}"))?,
                tools,
                issue: issue.map(str::to_owned),
            });
        }
        Ok(out)
    }
}

/// The text `[[class]]` and `[[known]]` entries, split off the facts lane's when
/// exceptions.toml is read (indexes are their own).
#[derive(Debug, Default)]
pub(super) struct TextEntries {
    pub(super) classes: Vec<Class>,
    pub(super) known: Vec<TextKnown>,
}

impl TextEntries {
    /// The entry that excuses lamina's page `page` of `file` (with `lamina` its
    /// [`page_facts`]) against `tool`'s text `theirs`.
    fn excuse(&self, file: &str, page: usize, tool: &str, lamina: &Value, theirs: &Value) -> Option<Excuse> {
        if let Some(i) = self
            .classes
            .iter()
            .position(|c| c.matches(file, TEXT, tool, lamina, theirs))
        {
            return Some(Excuse::Class(i));
        }
        self.known
            .iter()
            .position(|k| k.covers(file, page, tool))
            .map(Excuse::Known)
    }

    fn describe(&self, excuse: Excuse) -> String {
        match excuse {
            Excuse::File => String::new(),
            Excuse::Class(i) => self
                .classes
                .get(i)
                .map_or_else(String::new, |c| format!("text class {}: {}", i + 1, c.rule)),
            Excuse::Known(i) => self.known.get(i).map_or_else(String::new, |k| match &k.issue {
                Some(url) => format!("known: {url}"),
                None => "known: tracking issue pending".to_owned(),
            }),
        }
    }

    /// Files of text `[[known]]` entries whose issue is still [`PENDING_ISSUE`].
    fn pending(&self) -> Vec<&str> {
        self.known
            .iter()
            .filter(|k| k.issue.is_none())
            .map(|k| k.file.as_str())
            .collect()
    }
}

/// Pages of one source (`pdfjs`, `pdfium`, …) and how they compared.
#[derive(Debug, Default)]
struct SourceTally {
    files: usize,
    pages: usize,
    /// Pages at least one tool read.
    compared: usize,
    /// Pages where lamina matches at least one tool.
    agree: usize,
    /// Per tool: pages it read and lamina matched.
    tools: BTreeMap<String, Tally>,
    split: usize,
    excused: usize,
    known: usize,
    failures: usize,
}

/// How a tool fared on its files.
#[derive(Debug, Default)]
struct ToolRow {
    files: usize,
    failed: usize,
    timeouts: usize,
    crashes: usize,
    /// Pages of files it opened that it failed on (left out of those pages).
    page_errors: usize,
}

#[derive(Debug)]
struct PageDisagreement {
    file: String,
    page: usize,
    kind: Kind,
    lamina: String,
    /// `(tool, its squeezed text, excused)` for each disagreeing tool.
    tools: Vec<(String, String, Option<Excuse>)>,
    agreeing: Vec<String>,
}

#[derive(Debug, Default)]
struct TextComparison {
    sources: BTreeMap<String, SourceTally>,
    tools: BTreeMap<String, ToolRow>,
    disagreements: Vec<PageDisagreement>,
    /// `file: panic|crash|timeout` where lamina failed to read a file: always a failure.
    lamina_errors: Vec<String>,
    /// `file (tool)` for inputs lamina read that a tool's report lacks: always a failure.
    missing: Vec<String>,
    /// Text classes and knowns that could have applied this run and excused nothing ([`Checked`]).
    stale: Vec<String>,
    class_hits: BTreeMap<usize, usize>,
    known_hits: BTreeMap<usize, usize>,
    checked: Checked,
}

/// Entries that could have applied this run. A known: one of its pages compared against one
/// of its tools. A class: lamina's side of it holding on a page where the tool disagreed (a
/// page the tools agree on proves nothing about a class).
#[derive(Debug, Default)]
struct Checked {
    classes: BTreeSet<usize>,
    known: BTreeSet<usize>,
}

impl Checked {
    /// Records the knowns covering `(file, page, tool)`, compared this run.
    fn compared(&mut self, entries: &TextEntries, known_here: &[usize], at: (&str, usize, &str)) {
        let (file, page, tool) = at;
        for &i in known_here {
            if entries.known.get(i).is_some_and(|k| k.covers(file, page, tool)) {
                self.known.insert(i);
            }
        }
    }

    /// Records the classes whose lamina side holds on a page where `tool` disagreed.
    fn disagreed(&mut self, entries: &TextEntries, file: &str, tool: &str, facts: &Value) {
        for (i, class) in entries.classes.iter().enumerate() {
            if class.could_apply(file, TEXT, tool, facts) {
                self.classes.insert(i);
            }
        }
    }
}

fn compare(lamina: &Report, tools: &[Report], entries: &TextEntries) -> TextComparison {
    let mut out = TextComparison::default();
    for tool in tools {
        out.tools.entry(tool.tool.clone()).or_default();
    }
    let mut known_by_file: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, k) in entries.known.iter().enumerate() {
        known_by_file.entry(k.file.as_str()).or_default().push(i);
    }
    for (file, ours) in &lamina.facts {
        if let Some(error @ ("panic" | "crash" | "timeout")) = ours.get("outcome").and_then(Value::as_str) {
            out.lamina_errors.push(format!("{file}: {error}"));
            continue;
        }
        let source = out.sources.entry(group(file).to_owned()).or_default();
        source.files += 1;
        let mine = pages(ours);
        let mut read: Vec<(&str, Vec<Option<String>>)> = Vec::new();
        for tool in tools {
            let Some(theirs) = tool.facts.get(file) else {
                out.missing.push(format!("{file} ({})", tool.tool));
                continue;
            };
            let row = out.tools.entry(tool.tool.clone()).or_default();
            row.files += 1;
            match theirs.get("outcome").and_then(Value::as_str) {
                Some("opened" | "repaired") => read.push((tool.tool.as_str(), pages(theirs))),
                Some("timeout") => row.timeouts += 1,
                Some("crash") => row.crashes += 1,
                _ => row.failed += 1,
            }
        }
        let count = read
            .iter()
            .map(|(_, p)| p.len())
            .chain([mine.len()])
            .max()
            .unwrap_or_default();
        source.pages += count;
        let known_here = known_by_file.get(file.as_str()).map_or(&[][..], Vec::as_slice);
        for index in 0..count {
            let text = mine.get(index).and_then(Option::as_deref).unwrap_or_default();
            let facts = page_facts(ours, index, text);
            let mut agreeing = Vec::new();
            let mut disagreeing = Vec::new();
            for (tool, theirs) in &read {
                // A page past the tool's last is empty; a page it failed on is left out.
                let theirs = match theirs.get(index) {
                    None => "",
                    Some(Some(t)) => t.as_str(),
                    Some(None) => {
                        if let Some(row) = out.tools.get_mut(*tool) {
                            row.page_errors += 1;
                        }
                        continue;
                    }
                };
                out.checked.compared(entries, known_here, (file, index, tool));
                let tally = source.tools.entry((*tool).to_owned()).or_default();
                if text == theirs {
                    tally.agree += 1;
                    agreeing.push((*tool).to_owned());
                    continue;
                }
                tally.disagree += 1;
                out.checked.disagreed(entries, file, tool, &facts);
                let excuse = entries.excuse(file, index, tool, &facts, &json!(theirs));
                tally.excused += usize::from(excuse.is_some());
                match excuse {
                    Some(Excuse::Class(i)) => *out.class_hits.entry(i).or_default() += 1,
                    Some(Excuse::Known(i)) => *out.known_hits.entry(i).or_default() += 1,
                    _ => {}
                }
                disagreeing.push(((*tool).to_owned(), theirs.to_owned(), excuse));
            }
            if agreeing.is_empty() && disagreeing.is_empty() {
                continue;
            }
            source.compared += 1;
            source.agree += usize::from(!agreeing.is_empty());
            if disagreeing.is_empty() {
                continue;
            }
            let kind = kind_of(&agreeing, &disagreeing);
            *match kind {
                Kind::Failure => &mut source.failures,
                Kind::Split => &mut source.split,
                Kind::Excused => &mut source.excused,
                Kind::Known => &mut source.known,
            } += 1;
            out.disagreements.push(PageDisagreement {
                file: file.clone(),
                page: index,
                kind,
                lamina: text.to_owned(),
                tools: disagreeing,
                agreeing,
            });
        }
    }
    out.stale = stale(entries, &out);
    out
}

/// How a page with disagreeing tools counts (as in the facts lane's comparison).
fn kind_of(agreeing: &[String], disagreeing: &[(String, String, Option<Excuse>)]) -> Kind {
    if disagreeing.iter().all(|(_, _, e)| e.is_some()) {
        if disagreeing.iter().any(|(_, _, e)| matches!(e, Some(Excuse::Known(_)))) {
            Kind::Known
        } else {
            Kind::Excused
        }
    } else if agreeing.is_empty() {
        Kind::Failure
    } else {
        Kind::Split
    }
}

/// Entries that could have applied this run (see [`Checked`]) and
/// excused nothing: lamina is fixed, or the tools changed, so the entry goes.
fn stale(entries: &TextEntries, out: &TextComparison) -> Vec<String> {
    let classes = out
        .checked
        .classes
        .iter()
        .filter(|i| !out.class_hits.contains_key(i))
        .map(|i| entries.describe(Excuse::Class(*i)));
    let known = out
        .checked
        .known
        .iter()
        .filter(|i| !out.known_hits.contains_key(i))
        .filter_map(|i| {
            let k = entries.known.get(*i)?;
            Some(format!(
                "{} pages {} ({})",
                k.file,
                format_pages(&k.pages),
                entries.describe(Excuse::Known(*i))
            ))
        });
    classes.chain(known).collect()
}

/// The tools a run compares with: pdftotext only at the pinned poppler, otherwise pdf.js alone
/// and a warning saying why.
fn text_tools(pdftotext: &Probe) -> (Vec<&'static str>, Option<String>) {
    match pdftotext {
        Probe::Ok(_) => (TOOLS.to_vec(), None),
        Probe::OffPin(found) => (
            vec!["pdfjs"],
            Some(format!(
                "pdftotext is `{}`, not poppler {POPPLER_VERSION} (doctor.rs's pin): pdftotext is left out and lamina is \
                 compared with pdf.js only",
                found.lines().next().unwrap_or_default()
            )),
        ),
        Probe::Missing => (
            vec!["pdfjs"],
            Some(format!(
                "pdftotext is not installed (poppler {POPPLER_VERSION} is pinned): lamina is compared with pdf.js only"
            )),
        ),
    }
}

impl TextComparison {
    fn count(&self, kind: Kind) -> usize {
        self.disagreements.iter().filter(|d| d.kind == kind).count()
    }

    fn failed(&self) -> bool {
        self.count(Kind::Failure) > 0 || !self.lamina_errors.is_empty() || !self.missing.is_empty()
    }

    fn summary_line(&self) -> String {
        format!(
            "text differential: {} failing pages, {} lamina errors, {} missing, {} split, {} excused, {} known, {} stale entries",
            self.count(Kind::Failure),
            self.lamina_errors.len(),
            self.missing.len(),
            self.count(Kind::Split),
            self.count(Kind::Excused),
            self.count(Kind::Known),
            self.stale.len()
        )
    }

    /// The per-source agreement table (the job summary's head).
    fn sources_table(&self, md: &mut String) {
        let tool_names: Vec<&str> = self.tools.keys().map(String::as_str).collect();
        let _ = write!(md, "| Source | Files | Pages | Agree (any tool) |");
        for tool in &tool_names {
            let _ = write!(md, " vs {tool} |");
        }
        let _ = writeln!(md, " Split | Excused | Known | Failures |");
        let _ = writeln!(md, "|{}", " --- |".repeat(8 + tool_names.len()));
        let mut total = SourceTally::default();
        for (name, s) in &self.sources {
            Self::source_row(md, name, s, &tool_names);
            total.files += s.files;
            total.pages += s.pages;
            total.compared += s.compared;
            total.agree += s.agree;
            total.split += s.split;
            total.excused += s.excused;
            total.known += s.known;
            total.failures += s.failures;
            for (tool, t) in &s.tools {
                let sum = total.tools.entry(tool.clone()).or_default();
                sum.agree += t.agree;
                sum.disagree += t.disagree;
                sum.excused += t.excused;
            }
        }
        if self.sources.len() > 1 {
            Self::source_row(md, "**all**", &total, &tool_names);
        }
    }

    fn source_row(md: &mut String, name: &str, s: &SourceTally, tools: &[&str]) {
        let any = Tally {
            agree: s.agree,
            disagree: s.compared - s.agree,
            excused: 0,
        };
        let _ = write!(md, "| {name} | {} | {} | {} |", s.files, s.pages, rate(&any));
        for tool in tools {
            let _ = write!(md, " {} |", s.tools.get(*tool).map_or_else(|| "-".to_owned(), rate));
        }
        let _ = writeln!(md, " {} | {} | {} | {} |", s.split, s.excused, s.known, s.failures);
    }

    fn markdown(&self, lamina: &Report, tools: &[Report], entries: &TextEntries, context: &RunInfo) -> String {
        let mut md = String::new();
        let _ = writeln!(md, "## Nightly text differential\n");
        let _ = writeln!(
            md,
            "{} external files{} at `{}`. {}\n",
            context.inputs,
            context.sample.map_or(String::new(), |n| format!(", sampled to {n}")),
            context.commit,
            self.summary_line()
        );
        for note in &context.notes {
            let _ = writeln!(md, "> **Warning:** {}\n", util::md_cell(note));
        }
        let _ = writeln!(
            md,
            "Page by page, whitespace and bidi controls removed (todo B9c's rule); a page agrees when \
             lamina's text matches at least one tool's. A tool that can't open a file, or fails on a page, \
             is left out of it. A failure is a page where every tool that read it disagrees with lamina and \
             no text `[[class]]` or `[[known]]` entry in `fixtures/differential/exceptions.toml` excuses it.\n"
        );
        self.sources_table(&mut md);
        let _ = writeln!(
            md,
            "\n| Reader | Version | Files | Failed to open | Page errors | Timeouts / crashes |"
        );
        let _ = writeln!(md, "| --- | --- | --- | --- | --- | --- |");
        let lamina_failed = lamina
            .facts
            .values()
            .filter(|f| f.get("outcome").and_then(Value::as_str) == Some("failed"))
            .count();
        let lamina_page_errors: usize = lamina
            .facts
            .values()
            .filter_map(|f| f.get("page_errors").and_then(Value::as_object))
            .map(serde_json::Map::len)
            .sum();
        let _ = writeln!(
            md,
            "| lamina | {} (`{}`) | {} | {lamina_failed} | {lamina_page_errors} | {} |",
            util::md_cell(&lamina.version),
            context.commit.get(..12).unwrap_or(&context.commit),
            lamina.facts.len(),
            self.lamina_errors.len()
        );
        for tool in tools {
            let row = self.tools.get(&tool.tool);
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} | {} / {} |",
                util::md_cell(&tool.tool),
                util::md_cell(&tool.version),
                row.map_or(0, |r| r.files),
                row.map_or(0, |r| r.failed),
                row.map_or(0, |r| r.page_errors),
                row.map_or(0, |r| r.timeouts),
                row.map_or(0, |r| r.crashes)
            );
        }
        if !context.broken.is_empty() {
            let _ = writeln!(md, "\n### Readers that failed (the run fails)\n");
            for why in &context.broken {
                let _ = writeln!(md, "- {}", util::md_cell(why));
            }
        }
        let pending: Vec<String> = entries.pending().into_iter().map(str::to_owned).collect();
        for (title, list) in [
            ("lamina failed to read (panic, crash or timeout)", &self.lamina_errors),
            ("Missing from a tool's report", &self.missing),
            (
                "Text entries that could have applied and excused nothing (remove them from exceptions.toml)",
                &self.stale,
            ),
            (
                "Text `[[known]]` entries without a tracking issue (open it and put its URL in `issue`)",
                &pending,
            ),
        ] {
            if !list.is_empty() {
                let _ = writeln!(md, "\n### {title} ({})\n", list.len());
                for entry in list.iter().take(MAX_ROWS) {
                    let _ = writeln!(md, "- `{}`", util::md_cell(entry));
                }
            }
        }
        self.markdown_rows(&mut md, entries);
        md
    }

    fn markdown_rows(&self, md: &mut String, entries: &TextEntries) {
        for (kind, title) in [
            (Kind::Failure, "Failures: every tool disagrees with lamina"),
            (Kind::Known, "Known: tracked by an issue"),
            (Kind::Excused, "Excused by a class"),
            (Kind::Split, "Split: one tool agrees with lamina, the other doesn't"),
        ] {
            let rows: Vec<&PageDisagreement> = self.disagreements.iter().filter(|d| d.kind == kind).collect();
            if rows.is_empty() {
                continue;
            }
            let _ = writeln!(md, "\n### {title} ({} pages)\n", rows.len());
            if rows.len() > MAX_ROWS {
                let _ = writeln!(md, "First {MAX_ROWS}; report.json has every row.\n");
            }
            let _ = writeln!(md, "| File | Page | lamina | Disagreeing tools | Agreeing |");
            let _ = writeln!(md, "| --- | --- | --- | --- | --- |");
            for d in rows.iter().take(MAX_ROWS) {
                let theirs: Vec<String> = d
                    .tools
                    .iter()
                    .map(|(t, v, excuse)| {
                        let v = util::md_cell(&clip(v, 80));
                        match excuse {
                            Some(e) => format!("{t}: `{v}` ({})", util::md_cell(&entries.describe(*e))),
                            None => format!("{t}: `{v}`"),
                        }
                    })
                    .collect();
                let _ = writeln!(
                    md,
                    "| `{}` | {} | `{}` | {} | {} |",
                    util::md_cell(&d.file),
                    d.page,
                    util::md_cell(&clip(&d.lamina, 80)),
                    theirs.join("<br>"),
                    d.agreeing.join(", ")
                );
            }
        }
        self.markdown_entries(md, entries);
    }

    /// Disagreements (page × tool) each text class excused, and each `[[known]]` issue (its
    /// files and pages together).
    fn markdown_entries(&self, md: &mut String, entries: &TextEntries) {
        let mut rows: Vec<(String, String, usize)> = Vec::new();
        for i in 0..entries.classes.len() {
            let hits = self.class_hits.get(&i).copied().unwrap_or_default();
            rows.push((entries.describe(Excuse::Class(i)), "-".to_owned(), hits));
        }
        // `(files, pages listed, disagreements excused)` per issue.
        let mut issues: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
        for (i, known) in entries.known.iter().enumerate() {
            let entry = issues.entry(entries.describe(Excuse::Known(i))).or_default();
            entry.0 += 1;
            entry.1 += known.pages.len();
            entry.2 += self.known_hits.get(&i).copied().unwrap_or_default();
        }
        rows.extend(
            issues
                .into_iter()
                .map(|(label, (files, pages, hits))| (label, format!("{files} ({pages} pages)"), hits)),
        );
        if rows.is_empty() {
            return;
        }
        let _ = writeln!(md, "\n### Text classes and known issues\n");
        let _ = writeln!(
            md,
            "| Entry | Files listed | Disagreements excused (page × tool) |\n| --- | --- | --- |"
        );
        for (label, files, hits) in rows {
            let _ = writeln!(md, "| {} | {files} | {hits} |", util::md_cell(&label));
        }
    }

    fn json(&self, lamina: &Report, tools: &[Report], entries: &TextEntries, context: &RunInfo) -> Value {
        let sources: serde_json::Map<String, Value> = self
            .sources
            .iter()
            .map(|(name, s)| {
                let per_tool: serde_json::Map<String, Value> = s
                    .tools
                    .iter()
                    .map(|(t, tally)| {
                        (
                            t.clone(),
                            json!({"agree": tally.agree, "disagree": tally.disagree, "excused": tally.excused}),
                        )
                    })
                    .collect();
                (
                    name.clone(),
                    json!({
                        "files": s.files, "pages": s.pages, "compared": s.compared, "agree": s.agree,
                        "tools": per_tool, "split": s.split, "excused": s.excused, "known": s.known,
                        "failures": s.failures,
                    }),
                )
            })
            .collect();
        let disagreements: Vec<Value> = self
            .disagreements
            .iter()
            .map(|d| {
                json!({
                    "file": d.file,
                    "page": d.page,
                    "kind": format!("{:?}", d.kind).to_lowercase(),
                    "lamina": clip(&d.lamina, JSON_TEXT_CHARS),
                    "disagreeing": d.tools.iter().map(|(t, v, e)| json!({
                        "tool": t, "value": clip(v, JSON_TEXT_CHARS), "excused": e.map(|e| entries.describe(e)),
                    })).collect::<Vec<_>>(),
                    "agreeing": d.agreeing,
                })
            })
            .collect();
        json!({
            "commit": context.commit,
            "inputs": context.inputs,
            "sample": context.sample,
            "notes": context.notes,
            "lamina": {"version": lamina.version, "files": lamina.facts.len()},
            "tools": tools.iter().map(|t| {
                let row = self.tools.get(&t.tool);
                json!({
                    "tool": t.tool, "version": t.version, "files": row.map_or(0, |r| r.files),
                    "failed": row.map_or(0, |r| r.failed), "page_errors": row.map_or(0, |r| r.page_errors),
                    "timeouts": row.map_or(0, |r| r.timeouts), "crashes": row.map_or(0, |r| r.crashes),
                })
            }).collect::<Vec<_>>(),
            "sources": sources,
            "lamina_errors": self.lamina_errors,
            "missing": self.missing,
            "broken_readers": context.broken,
            "stale": self.stale,
            "pending": entries.pending(),
            "disagreements": disagreements,
        })
    }
}

/// What the report says about the run besides the comparison.
struct RunInfo {
    inputs: usize,
    commit: String,
    sample: Option<usize>,
    broken: Vec<String>,
    /// Warnings about the run itself (pdftotext left out).
    notes: Vec<String>,
}

/// `cargo xtask differential --live --text …`.
pub(super) fn run(ws: &Workspace, args: &LiveArgs) -> Result<()> {
    let mut exceptions = Exceptions::default();
    for file in EXCEPTIONS {
        exceptions.merge(Exceptions::read(&ws.root.join(file)).with_context(|| format!("reading {file}"))?);
    }
    let entries = exceptions.text;
    let (tools, note) = text_tools(&crate::doctor::pdftotext().probe(ws));
    let notes: Vec<String> = note.into_iter().collect();
    for note in &notes {
        println!("::warning::text differential: {note}");
    }
    if !ws.root.join("tools/differential/node_modules/pdfjs-dist").is_dir() {
        bail!("pdf.js is not installed: run `pnpm --dir tools/differential install --frozen-lockfile`");
    }
    let out = ws.root.join("target/nightly/differential-text");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    fs::create_dir_all(&out)?;
    let mut inputs = corpus::verified_inputs(ws, &args.sources)?;
    inputs.sort();
    if let Some(n) = args.sample {
        inputs = super::sample(inputs, n);
    }
    if inputs.is_empty() {
        bail!("no inputs: nothing would be compared (check --source and --sample)");
    }
    step(&format!("text differential: {} inputs", inputs.len()));
    ws.cargo(&[
        "build",
        "--release",
        "--locked",
        "-q",
        "-p",
        "laminapdf",
        "--example",
        "text-facts",
    ])?;
    let script = ws.root.join("tools/differential/text.mjs").into_os_string();
    let mut readers = vec![runner::Reader {
        name: "lamina",
        program: example_binary(ws, "text-facts").into_os_string(),
        args: Vec::new(),
        dir: ws.root.clone(),
        reports_repair: true,
    }];
    readers.extend(tools.iter().map(|&tool| runner::Reader {
        name: tool,
        program: "node".into(),
        args: vec![script.clone(), "--facts".into(), tool.into()],
        dir: ws.root.clone(),
        reports_repair: false,
    }));
    let (mut reports, broken) = run_readers(&readers, &inputs, &out)?;
    let lamina = reports.remove(0);
    let comparison = compare(&lamina, &reports, &entries);
    let context = RunInfo {
        inputs: inputs.len(),
        commit: util::git_stdout(&ws.root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned()),
        sample: args.sample,
        broken,
        notes,
    };
    let markdown = comparison.markdown(&lamina, &reports, &entries, &context);
    fs::write(out.join("report.md"), &markdown)?;
    fs::write(
        out.join("report.json"),
        serde_json::to_string_pretty(&comparison.json(&lamina, &reports, &entries, &context))? + "\n",
    )?;
    util::summary(&markdown)?;
    println!("{}", comparison.summary_line());
    for pending in entries.pending() {
        println!("::warning::exceptions.toml: text [[known]] {pending} has no tracking issue yet ({PENDING_ISSUE})");
    }
    step(&format!("Report: {}", out.join("report.md").display()));
    if !context.broken.is_empty() {
        bail!("readers failed: {}; see the report", context.broken.join("; "));
    }
    if comparison.failed() {
        bail!(
            "lamina's text disagrees with every tool on {} pages, or lamina failed to read {} files \
             (panic, crash or timeout), and fixtures/differential/exceptions.toml doesn't excuse them; \
             see the report",
            comparison.count(Kind::Failure),
            comparison.lamina_errors.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(tool: &str, facts: &Value) -> Report {
        Report::parse(&json!({"tool": tool, "version": "1", "reports_repair": false, "facts": facts})).unwrap()
    }

    fn opened(pages: &[&str]) -> Value {
        json!({"outcome": "opened", "pages": pages, "warnings": []})
    }

    fn kinds(c: &TextComparison) -> Vec<(&str, usize, Kind)> {
        c.disagreements
            .iter()
            .map(|d| (d.file.as_str(), d.page, d.kind))
            .collect()
    }

    fn two_tools(pdfjs: &Value, pdftotext: &Value) -> [Report; 2] {
        [
            report("pdfjs", &json!({"s/a.pdf": pdfjs})),
            report("pdftotext", &json!({"s/a.pdf": pdftotext})),
        ]
    }

    fn entries(toml: &str) -> TextEntries {
        Exceptions::parse(toml).unwrap().text
    }

    const RULE: &str = "rule = \"ADR 0003 §1: Unsupported filter\"\nreason = \"r\"\n";
    const ISSUE: &str = "issue = \"https://github.com/laudebugs/lamina/issues/1\"\nreason = \"r\"\n";

    /// A text `[[known]]` for `s/a.pdf`'s `pages`, optionally for some tools only.
    fn known(pages: &str, tools: &str) -> String {
        format!("[[known]]\nfacts = [\"text\"]\n{tools}{ISSUE}[known.pages]\n\"s/a.pdf\" = \"{pages}\"\n")
    }

    #[test]
    fn squeeze_should_drop_whitespace_and_bidi_controls() {
        assert_eq!(squeeze(" a\tb\n\u{202B}c\u{202C}\u{200F} d"), "abcd");
    }

    #[test]
    fn compare_should_agree_when_text_differs_only_in_whitespace() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["Hello world"])}));
        let tools = two_tools(&opened(&["Hello\nworld"]), &opened(&["Helloworld "]));
        let c = compare(&lamina, &tools, &TextEntries::default());
        assert!(c.disagreements.is_empty() && c.sources["s"].agree == 1);
    }

    #[test]
    fn compare_should_only_split_when_one_tool_agrees() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["abc"])}));
        let tools = two_tools(&opened(&["abc"]), &opened(&["abcd"]));
        let c = compare(&lamina, &tools, &TextEntries::default());
        assert!(!c.failed() && kinds(&c) == [("s/a.pdf", 0, Kind::Split)]);
    }

    #[test]
    fn compare_should_fail_a_page_every_tool_disagrees_on() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["abc", "x"])}));
        let tools = two_tools(&opened(&["abc", "y"]), &opened(&["abc", "y"]));
        let c = compare(&lamina, &tools, &TextEntries::default());
        assert!(c.failed() && kinds(&c) == [("s/a.pdf", 1, Kind::Failure)]);
    }

    #[test]
    fn compare_should_leave_out_a_tool_that_cannot_open_the_file() {
        let failed = json!({"outcome": "failed", "pages": null});
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["abc"])}));
        let tools = two_tools(&failed, &opened(&["abc"]));
        let c = compare(&lamina, &tools, &TextEntries::default());
        assert!(c.disagreements.is_empty() && c.tools["pdfjs"].failed == 1);
    }

    #[test]
    fn compare_should_leave_out_a_page_a_tool_failed_on() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["a", "b"])}));
        let pdfjs = json!({"outcome": "opened", "pages": ["a", null]});
        let tools = two_tools(&pdfjs, &opened(&["a", "c"]));
        let c = compare(&lamina, &tools, &TextEntries::default());
        assert!(kinds(&c) == [("s/a.pdf", 1, Kind::Failure)] && c.tools["pdfjs"].page_errors == 1);
    }

    #[test]
    fn compare_should_compare_missing_pages_as_empty() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["a"])}));
        let tools = two_tools(&opened(&["a", "b"]), &opened(&["a", ""]));
        let c = compare(&lamina, &tools, &TextEntries::default());
        assert_eq!(kinds(&c), [("s/a.pdf", 1, Kind::Split)]);
    }

    #[test]
    fn compare_should_fail_on_a_lamina_panic() {
        let lamina = report("lamina", &json!({"s/a.pdf": {"outcome": "panic", "pages": null}}));
        assert!(compare(&lamina, &[], &TextEntries::default()).failed());
    }

    #[test]
    fn compare_should_fail_when_a_tool_report_lacks_a_file() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["a"])}));
        let tools = [report("pdfjs", &json!({}))];
        let c = compare(&lamina, &tools, &TextEntries::default());
        assert!(c.failed() && c.missing == ["s/a.pdf (pdfjs)"]);
    }

    #[test]
    fn classes_should_excuse_text_by_lamina_warning() {
        let e = entries(&format!(
            "[[class]]\nfacts = [\"text\"]\nlamina = \"\"\nlamina_warning = \"filter-unsupported\"\n{RULE}"
        ));
        let ours = json!({"outcome": "opened", "pages": [""], "warnings": ["filter-unsupported"]});
        let lamina = report("lamina", &json!({"s/a.pdf": ours}));
        let tools = two_tools(&opened(&["abc"]), &opened(&["abc"]));
        let c = compare(&lamina, &tools, &e);
        assert!(!c.failed() && kinds(&c) == [("s/a.pdf", 0, Kind::Excused)]);
    }

    #[test]
    fn classes_should_not_excuse_text_without_the_warning() {
        let e = entries(&format!(
            "[[class]]\nfacts = [\"text\"]\nlamina_warning = \"filter-unsupported\"\n{RULE}"
        ));
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&[""])}));
        let tools = two_tools(&opened(&["abc"]), &opened(&["abc"]));
        assert!(compare(&lamina, &tools, &e).failed());
    }

    #[test]
    fn classes_should_excuse_only_the_named_tool() {
        let e = entries(&format!(
            "[[class]]\nfacts = [\"text\"]\ntools = [\"pdftotext\"]\n{RULE}"
        ));
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["abc"])}));
        let tools = two_tools(&opened(&["abd"]), &opened(&["cba"]));
        let c = compare(&lamina, &tools, &e);
        assert_eq!(kinds(&c), [("s/a.pdf", 0, Kind::Failure)]);
    }

    #[test]
    fn known_entries_should_excuse_their_pages() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["x", "x"])}));
        let tools = two_tools(&opened(&["y", "x"]), &opened(&["y", "x"]));
        let c = compare(&lamina, &tools, &entries(&known("0", "")));
        assert!(!c.failed() && kinds(&c) == [("s/a.pdf", 0, Kind::Known)]);
    }

    #[test]
    fn known_entries_should_not_excuse_other_pages() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["x", "x"])}));
        let tools = two_tools(&opened(&["x", "y"]), &opened(&["x", "y"]));
        let c = compare(&lamina, &tools, &entries(&known("0", "")));
        assert_eq!(kinds(&c), [("s/a.pdf", 1, Kind::Failure)]);
    }

    #[test]
    fn known_entries_should_excuse_only_their_tools() {
        let e = entries(&known("0", "tools = [\"pdftotext\"]\n"));
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["x"])}));
        let tools = two_tools(&opened(&["y"]), &opened(&["y"]));
        assert_eq!(kinds(&compare(&lamina, &tools, &e)), [("s/a.pdf", 0, Kind::Failure)]);
    }

    #[test]
    fn known_entries_should_read_per_file_tools() {
        let text = format!(
            "[[known]]\nfacts = [\"text\"]\n{ISSUE}[known.pages]\n\"s/a.pdf\" = {{ pages = \"0-2, 5\", tools = [\"pdfjs\"] }}\n"
        );
        let e = entries(&text);
        let k = &e.known[0];
        assert!(
            k.covers("s/a.pdf", 5, "pdfjs") && !k.covers("s/a.pdf", 3, "pdfjs") && !k.covers("s/a.pdf", 0, "pdftotext")
        );
    }

    #[test]
    fn known_text_entries_should_need_a_pages_table() {
        let text = format!("[[known]]\nfile = \"s/a.pdf\"\nfacts = [\"text\"]\n{ISSUE}");
        assert!(Exceptions::parse(&text).is_err());
    }

    #[test]
    fn known_text_entries_that_excuse_nothing_should_be_stale() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["y"])}));
        let tools = two_tools(&opened(&["y"]), &opened(&["y"]));
        let c = compare(&lamina, &tools, &entries(&known("0", "")));
        assert_eq!(
            c.stale,
            ["s/a.pdf pages 0 (known: https://github.com/laudebugs/lamina/issues/1)"]
        );
    }

    #[test]
    fn known_text_entries_should_not_be_stale_when_lamina_panicked() {
        let lamina = report("lamina", &json!({"s/a.pdf": {"outcome": "panic", "pages": null}}));
        let tools = two_tools(&opened(&["y"]), &opened(&["y"]));
        assert_eq!(
            compare(&lamina, &tools, &entries(&known("0", ""))).stale,
            Vec::<String>::new()
        );
    }

    #[test]
    fn known_text_entries_should_not_be_stale_when_every_tool_failed() {
        let failed = json!({"outcome": "failed", "pages": null});
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["y"])}));
        let tools = two_tools(&failed, &failed);
        assert_eq!(
            compare(&lamina, &tools, &entries(&known("0", ""))).stale,
            Vec::<String>::new()
        );
    }

    #[test]
    fn known_text_entries_should_not_be_stale_when_their_tool_failed() {
        let failed = json!({"outcome": "failed", "pages": null});
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["y"])}));
        let tools = two_tools(&opened(&["y"]), &failed);
        let e = entries(&known("0", "tools = [\"pdftotext\"]\n"));
        assert_eq!(compare(&lamina, &tools, &e).stale, Vec::<String>::new());
    }

    #[test]
    fn classes_that_could_apply_and_excuse_nothing_should_be_stale() {
        let e = entries(&format!(
            "[[class]]\nfacts = [\"text\"]\nlamina_facts = {{ error = \"missing-root\" }}\ntheirs = \"zzz\"\n{RULE}"
        ));
        let ours = json!({"outcome": "failed", "error": "missing-root", "pages": null});
        let lamina = report("lamina", &json!({"s/a.pdf": ours}));
        let tools = two_tools(&opened(&["abc"]), &opened(&["abc"]));
        assert_eq!(
            compare(&lamina, &tools, &e).stale,
            ["text class 1: ADR 0003 §1: Unsupported filter"]
        );
    }

    #[test]
    fn classes_should_not_be_stale_when_the_tools_agree() {
        let e = entries(&format!(
            "[[class]]\nfacts = [\"text\"]\nlamina_facts = {{ error = \"missing-root\" }}\n{RULE}"
        ));
        let ours = json!({"outcome": "failed", "error": "missing-root", "pages": null});
        let lamina = report("lamina", &json!({"s/a.pdf": ours}));
        let tools = two_tools(&opened(&[""]), &opened(&[""]));
        assert_eq!(compare(&lamina, &tools, &e).stale, Vec::<String>::new());
    }

    #[test]
    fn classes_that_could_not_apply_should_not_be_stale() {
        let e = entries(&format!(
            "[[class]]\nfacts = [\"text\"]\nlamina_facts = {{ error = \"missing-root\" }}\n{RULE}"
        ));
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["a"])}));
        let tools = two_tools(&opened(&["a"]), &opened(&["a"]));
        assert_eq!(compare(&lamina, &tools, &e).stale, Vec::<String>::new());
    }

    #[test]
    fn text_entries_should_stay_out_of_the_facts_lane() {
        let text = format!(
            "[[class]]\nfacts = [\"text\"]\n{RULE}\n{}",
            known("0", "issue = \"PENDING\"\n").replace(ISSUE, "reason = \"r\"\n")
        );
        let e = Exceptions::parse(&text).unwrap();
        assert!(
            e.classes.is_empty() && e.known.is_empty() && e.pending().is_empty() && e.text.pending() == ["s/a.pdf"]
        );
    }

    #[test]
    fn exceptions_should_reject_a_per_file_text_exception() {
        let text = format!("[[exception]]\nfile = \"a.pdf\"\nfacts = [\"text\"]\ntools = [\"pdfjs\"]\n{RULE}");
        assert!(Exceptions::parse(&text).is_err());
    }

    #[test]
    fn exceptions_should_reject_pdftotext_on_structural_facts() {
        let text = format!("[[class]]\nfacts = [\"outcome\"]\ntools = [\"pdftotext\"]\n{RULE}");
        assert!(Exceptions::parse(&text).is_err());
    }

    #[test]
    fn page_facts_should_carry_the_page_error() {
        let ours = json!({"outcome": "opened", "pages": [null], "page_errors": {"0": "syntax"}, "warnings": []});
        assert_eq!(page_facts(&ours, 0, "")["page_error"], "syntax");
    }

    #[test]
    fn pages_should_round_trip_through_ranges() {
        let pages = parse_pages("0-3, 7, 9-10").unwrap();
        assert_eq!((pages.len(), format_pages(&pages)), (7, "0-3, 7, 9-10".to_owned()));
    }

    #[test]
    fn pages_should_reject_a_backwards_range() {
        assert!(parse_pages("3-1").is_err() && parse_pages("x").is_err());
    }

    #[test]
    fn text_tools_should_keep_pdftotext_at_the_pin() {
        let (tools, note) = text_tools(&Probe::Ok("pdftotext version 26.10.0".to_owned()));
        assert!(tools == TOOLS && note.is_none());
    }

    #[test]
    fn text_tools_should_leave_out_pdftotext_off_the_pin() {
        let (tools, note) = text_tools(&Probe::OffPin("pdftotext version 26.11.0\nCopyright".to_owned()));
        assert!(
            tools == ["pdfjs"]
                && note.is_some_and(|n| n.contains("pdftotext version 26.11.0") && n.contains(POPPLER_VERSION))
        );
    }

    #[test]
    fn sources_table_should_show_agreement_per_source_and_tool() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["a", "b"])}));
        let tools = two_tools(&opened(&["a", "c"]), &opened(&["a", "b"]));
        let mut md = String::new();
        compare(&lamina, &tools, &TextEntries::default()).sources_table(&mut md);
        assert!(md.contains("| s | 1 | 2 | 100.0% (2/2) | 50.0% (1/2) | 100.0% (2/2) | 1 | 0 | 0 | 0 |"));
    }
}
