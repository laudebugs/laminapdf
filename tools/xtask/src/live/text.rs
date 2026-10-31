//! `cargo xtask differential --live --text` (todo B9f; ADR 0018 tier 6): lamina's page text
//! against pdf.js (`getTextContent`) and pdftotext over the external corpus, every night.
//!
//! The readers are lamina's `text-facts` example (`crates/lamina/examples/differential/`) and
//! `tools/differential/text.mjs --facts pdfjs|pdftotext`, each under [`runner`] like the other
//! live readers. The comparison is B9c's (`crates/lamina/tests/text_differential.rs`): page by
//! page, with whitespace and bidi controls removed on both sides; a page agrees when lamina
//! matches at least one tool. A page where every tool that read the file disagrees is a
//! failure unless a `[[class]]` or `[[known]]` entry with `facts = ["text"]` in
//! `fixtures/differential/exceptions.toml` excuses each tool; a tool that failed, crashed or
//! timed out on the file is left out of it. lamina failing to open a file counts as empty
//! pages (the structural differential reports the outcome itself); a lamina panic, crash or
//! hang is always a failure. The report goes to `target/nightly/differential-text/`, its
//! agreement-per-source table to the job summary.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::{EXCEPTIONS, Exceptions, Excuse, Kind, LiveArgs, MAX_ROWS, Report, Tally, example_binary, group, rate};
use crate::workspace::Workspace;
use crate::{corpus, runner, step, util};

/// The fact name of page text in `exceptions.toml`.
pub(super) const TEXT: &str = "text";

/// The text readers besides lamina; `[[class]]` and `[[known]]` entries may name them.
pub(super) const TOOLS: [&str; 2] = ["pdfjs", "pdftotext"];

/// Characters of a page's text kept in report.json's rows (the Markdown cell keeps 80).
const JSON_TEXT_CHARS: usize = 400;

/// The text with every whitespace character and bidi control (U+200E, U+200F, U+202A–U+202E,
/// U+2066–U+2069; pdftotext wraps right-to-left runs in embeddings) removed: B9c's rule, the
/// same function as `squeeze` in `crates/lamina/tests/text_differential.rs`.
pub(super) fn squeeze(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
        .collect()
}

/// A reader's pages, squeezed; a `null` page (lamina failed on it) is empty.
fn pages(facts: &Value) -> Vec<String> {
    facts
        .get("pages")
        .and_then(Value::as_array)
        .map(|pages| pages.iter().map(|p| squeeze(p.as_str().unwrap_or_default())).collect())
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
    /// `[[known]]` and `[[class]]` text entries that excused nothing this run.
    stale: Vec<String>,
    class_hits: BTreeMap<usize, usize>,
    known_hits: BTreeMap<usize, usize>,
}

fn compare(lamina: &Report, tools: &[Report], exceptions: &Exceptions) -> TextComparison {
    let mut out = TextComparison::default();
    for tool in tools {
        out.tools.entry(tool.tool.clone()).or_default();
    }
    for (file, ours) in &lamina.facts {
        if let Some(error @ ("panic" | "crash" | "timeout")) = ours.get("outcome").and_then(Value::as_str) {
            out.lamina_errors.push(format!("{file}: {error}"));
            continue;
        }
        let source = out.sources.entry(group(file).to_owned()).or_default();
        source.files += 1;
        let mine = pages(ours);
        let mut read: Vec<(&str, Vec<String>)> = Vec::new();
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
        if read.is_empty() {
            continue;
        }
        for index in 0..count {
            let text = mine.get(index).map_or("", String::as_str);
            let facts = page_facts(ours, index, text);
            let mut agreeing = Vec::new();
            let mut disagreeing = Vec::new();
            for (tool, theirs) in &read {
                let theirs = theirs.get(index).map_or("", String::as_str);
                let tally = source.tools.entry((*tool).to_owned()).or_default();
                if text == theirs {
                    tally.agree += 1;
                    agreeing.push((*tool).to_owned());
                    continue;
                }
                tally.disagree += 1;
                let excuse = exceptions.excuse(file, TEXT, tool, &facts, &json!(theirs));
                tally.excused += usize::from(excuse.is_some());
                match excuse {
                    Some(Excuse::Class(i)) => *out.class_hits.entry(i).or_default() += 1,
                    Some(Excuse::Known(i)) => *out.known_hits.entry(i).or_default() += 1,
                    _ => {}
                }
                disagreeing.push(((*tool).to_owned(), theirs.to_owned(), excuse));
            }
            source.compared += 1;
            source.agree += usize::from(!agreeing.is_empty());
            if disagreeing.is_empty() {
                continue;
            }
            let kind = if disagreeing.iter().all(|(_, _, e)| e.is_some()) {
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
    out.stale = stale(exceptions, lamina, &out);
    out
}

/// Text `[[known]]` entries whose file was compared and that excused nothing: lamina is fixed
/// (or the tools changed), so the entry goes.
fn stale(exceptions: &Exceptions, lamina: &Report, out: &TextComparison) -> Vec<String> {
    exceptions
        .known
        .iter()
        .enumerate()
        .filter(|(i, k)| {
            k.facts.iter().any(|f| f == TEXT) && lamina.facts.contains_key(&k.file) && !out.known_hits.contains_key(i)
        })
        .map(|(_, k)| format!("{}: text (known)", k.file))
        .collect()
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
        let rows = self.sources.iter().map(|(name, s)| (name.as_str(), s));
        for (name, s) in rows {
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

    fn markdown(&self, lamina: &Report, tools: &[Report], exceptions: &Exceptions, context: &RunInfo) -> String {
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
        let _ = writeln!(
            md,
            "Page by page, whitespace and bidi controls removed (todo B9c's rule); a page agrees when \
             lamina's text matches at least one tool's. A tool that can't open a file is left out of \
             it. A failure is a page where every tool that read the file disagrees with lamina and \
             no `[[class]]` or `[[known]]` entry in `fixtures/differential/exceptions.toml` excuses it.\n"
        );
        self.sources_table(&mut md);
        let _ = writeln!(
            md,
            "\n| Reader | Version | Files | Failed to open | Timeouts / crashes |"
        );
        let _ = writeln!(md, "| --- | --- | --- | --- | --- |");
        let _ = writeln!(
            md,
            "| lamina | {} (`{}`) | {} | {} | {} |",
            util::md_cell(&lamina.version),
            context.commit.get(..12).unwrap_or(&context.commit),
            lamina.facts.len(),
            lamina
                .facts
                .values()
                .filter(|f| f.get("outcome").and_then(Value::as_str) == Some("failed"))
                .count(),
            self.lamina_errors.len()
        );
        for tool in tools {
            let row = self.tools.get(&tool.tool);
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} / {} |",
                util::md_cell(&tool.tool),
                util::md_cell(&tool.version),
                row.map_or(0, |r| r.files),
                row.map_or(0, |r| r.failed),
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
        for (title, list) in [
            ("lamina failed to read (panic, crash or timeout)", &self.lamina_errors),
            ("Missing from a tool's report", &self.missing),
            (
                "Text entries that excused nothing (remove them from exceptions.toml)",
                &self.stale,
            ),
        ] {
            if !list.is_empty() {
                let _ = writeln!(md, "\n### {title} ({})\n", list.len());
                for entry in list.iter().take(MAX_ROWS) {
                    let _ = writeln!(md, "- `{}`", util::md_cell(entry));
                }
            }
        }
        self.markdown_rows(&mut md, exceptions);
        md
    }

    fn markdown_rows(&self, md: &mut String, exceptions: &Exceptions) {
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
                            Some(e) => format!("{t}: `{v}` ({})", util::md_cell(&exceptions.describe(*e))),
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
        self.markdown_entries(md, exceptions);
    }

    /// Disagreements (page × tool) each text class excused, and each `[[known]]` issue (its files together).
    fn markdown_entries(&self, md: &mut String, exceptions: &Exceptions) {
        let is_text = |facts: &[String]| facts.iter().any(|f| f == TEXT);
        let mut rows: Vec<(String, String, usize)> = Vec::new();
        for (i, class) in exceptions.classes.iter().enumerate() {
            if is_text(&class.facts) {
                let hits = self.class_hits.get(&i).copied().unwrap_or_default();
                rows.push((exceptions.describe(Excuse::Class(i)), "-".to_owned(), hits));
            }
        }
        // `(files, pages)` per issue.
        let mut issues: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for (i, known) in exceptions.known.iter().enumerate() {
            if is_text(&known.facts) {
                let entry = issues.entry(exceptions.describe(Excuse::Known(i))).or_default();
                entry.0 += 1;
                entry.1 += self.known_hits.get(&i).copied().unwrap_or_default();
            }
        }
        rows.extend(
            issues
                .into_iter()
                .map(|(label, (files, pages))| (label, files.to_string(), pages)),
        );
        if rows.is_empty() {
            return;
        }
        let _ = writeln!(md, "\n### Text classes and known issues\n");
        let _ = writeln!(
            md,
            "| Entry | Files listed | Disagreements excused (page × tool) |\n| --- | --- | --- |"
        );
        for (label, files, pages) in rows {
            let _ = writeln!(md, "| {} | {files} | {pages} |", util::md_cell(&label));
        }
    }

    fn json(&self, lamina: &Report, tools: &[Report], exceptions: &Exceptions, context: &RunInfo) -> Value {
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
                        "tool": t, "value": clip(v, JSON_TEXT_CHARS), "excused": e.map(|e| exceptions.describe(e)),
                    })).collect::<Vec<_>>(),
                    "agreeing": d.agreeing,
                })
            })
            .collect();
        json!({
            "commit": context.commit,
            "inputs": context.inputs,
            "sample": context.sample,
            "lamina": {"version": lamina.version, "files": lamina.facts.len()},
            "tools": tools.iter().map(|t| {
                let row = self.tools.get(&t.tool);
                json!({
                    "tool": t.tool, "version": t.version, "files": row.map_or(0, |r| r.files),
                    "failed": row.map_or(0, |r| r.failed), "timeouts": row.map_or(0, |r| r.timeouts),
                    "crashes": row.map_or(0, |r| r.crashes),
                })
            }).collect::<Vec<_>>(),
            "sources": sources,
            "lamina_errors": self.lamina_errors,
            "missing": self.missing,
            "broken_readers": context.broken,
            "stale": self.stale,
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
}

/// `cargo xtask differential --live --text …`.
pub(super) fn run(ws: &Workspace, args: &LiveArgs) -> Result<()> {
    let mut exceptions = Exceptions::default();
    for file in EXCEPTIONS {
        exceptions.merge(Exceptions::read(&ws.root.join(file)).with_context(|| format!("reading {file}"))?);
    }
    crate::doctor::require(ws, &crate::doctor::pdftotext())?;
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
        "lamina",
        "--example",
        "text-facts",
    ])?;
    let (mut reports, broken) = run_readers(ws, &inputs, &out)?;
    let lamina = reports.remove(0);
    let comparison = compare(&lamina, &reports, &exceptions);
    let context = RunInfo {
        inputs: inputs.len(),
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

/// Runs lamina, pdf.js and pdftotext over `inputs` at once (each under [`runner`]) and writes
/// their reports to `out`; returns the reports, lamina's first, and the readers that broke.
fn run_readers(ws: &Workspace, inputs: &[(String, PathBuf)], out: &Path) -> Result<(Vec<Report>, Vec<String>)> {
    let script = ws.root.join("tools/differential/text.mjs").into_os_string();
    let mut readers = vec![runner::Reader {
        name: "lamina",
        program: example_binary(ws, "text-facts").into_os_string(),
        args: Vec::new(),
        dir: ws.root.clone(),
        reports_repair: true,
    }];
    for tool in TOOLS {
        readers.push(runner::Reader {
            name: tool,
            program: "node".into(),
            args: vec![script.clone(), "--facts".into(), tool.into()],
            dir: ws.root.clone(),
            reports_repair: false,
        });
    }
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
            serde_json::to_string(&run.report)? + "\n",
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

    #[test]
    fn squeeze_should_drop_whitespace_and_bidi_controls() {
        assert_eq!(squeeze(" a\tb\n\u{202B}c\u{202C}\u{200F} d"), "abcd");
    }

    #[test]
    fn compare_should_agree_when_text_differs_only_in_whitespace() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["Hello world"])}));
        let tools = two_tools(&opened(&["Hello\nworld"]), &opened(&["Helloworld "]));
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(c.disagreements.is_empty() && c.sources["s"].agree == 1);
    }

    #[test]
    fn compare_should_only_split_when_one_tool_agrees() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["abc"])}));
        let tools = two_tools(&opened(&["abc"]), &opened(&["abcd"]));
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(!c.failed() && kinds(&c) == [("s/a.pdf", 0, Kind::Split)]);
    }

    #[test]
    fn compare_should_fail_a_page_every_tool_disagrees_on() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["abc", "x"])}));
        let tools = two_tools(&opened(&["abc", "y"]), &opened(&["abc", "y"]));
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(c.failed() && kinds(&c) == [("s/a.pdf", 1, Kind::Failure)]);
    }

    #[test]
    fn compare_should_leave_out_a_tool_that_cannot_open_the_file() {
        let failed = json!({"outcome": "failed", "pages": null});
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["abc"])}));
        let tools = two_tools(&failed, &opened(&["abc"]));
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(c.disagreements.is_empty() && c.tools["pdfjs"].failed == 1);
    }

    #[test]
    fn compare_should_compare_missing_pages_as_empty() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["a"])}));
        let tools = two_tools(&opened(&["a", "b"]), &opened(&["a", ""]));
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert_eq!(kinds(&c), [("s/a.pdf", 1, Kind::Split)]);
    }

    #[test]
    fn compare_should_fail_on_a_lamina_panic() {
        let lamina = report("lamina", &json!({"s/a.pdf": {"outcome": "panic", "pages": null}}));
        assert!(compare(&lamina, &[], &Exceptions::default()).failed());
    }

    #[test]
    fn compare_should_fail_when_a_tool_report_lacks_a_file() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["a"])}));
        let tools = [report("pdfjs", &json!({}))];
        let c = compare(&lamina, &tools, &Exceptions::default());
        assert!(c.failed() && c.missing == ["s/a.pdf (pdfjs)"]);
    }

    const RULE: &str = "rule = \"ADR 0003 §1: Unsupported filter\"\nreason = \"r\"\n";

    #[test]
    fn classes_should_excuse_text_by_lamina_warning() {
        let class =
            format!("[[class]]\nfacts = [\"text\"]\nlamina = \"\"\nlamina_warning = \"filter-unsupported\"\n{RULE}");
        let exceptions = Exceptions::parse(&class).unwrap();
        let ours = json!({"outcome": "opened", "pages": [""], "warnings": ["filter-unsupported"]});
        let lamina = report("lamina", &json!({"s/a.pdf": ours}));
        let tools = two_tools(&opened(&["abc"]), &opened(&["abc"]));
        let c = compare(&lamina, &tools, &exceptions);
        assert!(!c.failed() && kinds(&c) == [("s/a.pdf", 0, Kind::Excused)]);
    }

    #[test]
    fn classes_should_not_excuse_text_without_the_warning() {
        let class = format!("[[class]]\nfacts = [\"text\"]\nlamina_warning = \"filter-unsupported\"\n{RULE}");
        let exceptions = Exceptions::parse(&class).unwrap();
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&[""])}));
        let tools = two_tools(&opened(&["abc"]), &opened(&["abc"]));
        assert!(compare(&lamina, &tools, &exceptions).failed());
    }

    #[test]
    fn classes_should_excuse_only_the_named_tool() {
        let class =
            format!("[[class]]\nfacts = [\"text\"]\ntools = [\"pdftotext\"]\nrelation = \"same-characters\"\n{RULE}");
        let exceptions = Exceptions::parse(&class).unwrap();
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["abc"])}));
        let tools = two_tools(&opened(&["abd"]), &opened(&["cba"]));
        let c = compare(&lamina, &tools, &exceptions);
        assert_eq!(kinds(&c), [("s/a.pdf", 0, Kind::Failure)]);
    }

    #[test]
    fn known_entries_should_mark_text_failures_known() {
        let known = "[[known]]\nfile = \"s/a.pdf\"\nfacts = [\"text\"]\n\
                     issue = \"https://github.com/laudebugs/lamina/issues/1\"\nreason = \"r\"\n";
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["x"])}));
        let tools = two_tools(&opened(&["y"]), &opened(&["y"]));
        let c = compare(&lamina, &tools, &Exceptions::parse(known).unwrap());
        assert!(!c.failed() && kinds(&c) == [("s/a.pdf", 0, Kind::Known)]);
    }

    #[test]
    fn known_text_entries_that_excuse_nothing_should_be_stale() {
        let known = "[[known]]\nfile = \"s/a.pdf\"\nfacts = [\"text\"]\n\
                     issue = \"https://github.com/laudebugs/lamina/issues/1\"\nreason = \"r\"\n";
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["y"])}));
        let tools = two_tools(&opened(&["y"]), &opened(&["y"]));
        let c = compare(&lamina, &tools, &Exceptions::parse(known).unwrap());
        assert_eq!(c.stale, ["s/a.pdf: text (known)"]);
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
    fn sources_table_should_show_agreement_per_source_and_tool() {
        let lamina = report("lamina", &json!({"s/a.pdf": opened(&["a", "b"])}));
        let tools = two_tools(&opened(&["a", "c"]), &opened(&["a", "b"]));
        let mut md = String::new();
        compare(&lamina, &tools, &Exceptions::default()).sources_table(&mut md);
        assert!(md.contains("| s | 1 | 2 | 100.0% (2/2) | 50.0% (1/2) | 100.0% (2/2) | 1 | 0 | 0 | 0 |"));
    }
}
