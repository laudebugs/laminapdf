//! Comparing lamina's facts with every tool's: the disagreements, their kinds and the report
//! (summary line, Markdown and JSON).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::{Value, json};

use super::exceptions::{Exceptions, Excuse};
use super::{FACTS, MAX_ROWS, Report};
use crate::util;

/// How lamina's answer on one fact of one file relates to the tools'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
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
pub(super) struct Disagreement {
    pub(super) file: String,
    pub(super) fact: &'static str,
    pub(super) kind: Kind,
    pub(super) lamina: Value,
    /// `(tool, its value, excused)` for each disagreeing tool.
    pub(super) tools: Vec<(String, Value, Option<Excuse>)>,
    pub(super) agreeing: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub(super) struct Tally {
    pub(super) agree: usize,
    pub(super) disagree: usize,
    pub(super) excused: usize,
}

#[derive(Debug, Default)]
pub(super) struct ToolSummary {
    pub(super) files: usize,
    pub(super) timeouts: usize,
    /// Files whose single-file run crashed the tool (skipped, like timeouts).
    pub(super) crashes: usize,
    /// Per fact, in [`FACTS`] order.
    pub(super) tallies: [Tally; 3],
}

#[derive(Debug, Default)]
pub(super) struct Comparison {
    pub(super) tools: BTreeMap<String, ToolSummary>,
    pub(super) disagreements: Vec<Disagreement>,
    /// `file: panic|crash|timeout` where lamina itself failed to read a file: always a failure.
    pub(super) lamina_errors: Vec<String>,
    /// `file (tool)` for inputs lamina read that a tool's report lacks: always a failure.
    pub(super) missing: Vec<String>,
    /// `file: fact (tool)` for `[[exception]]` and `[[known]]` entries the tool now agrees on.
    pub(super) stale: Vec<String>,
    /// How many disagreements each `[[class]]` and `[[known]]` entry excused.
    pub(super) class_hits: BTreeMap<usize, usize>,
    pub(super) known_hits: BTreeMap<usize, usize>,
}

pub(super) struct RunContext {
    /// Readers that broke or reported the wrong files (the run fails).
    pub(super) broken: Vec<String>,
    pub(super) inputs: usize,
    pub(super) external: usize,
    pub(super) commit: String,
    pub(super) sample: Option<usize>,
}

pub(super) fn readable(facts: &Value) -> bool {
    matches!(
        facts.get("outcome").and_then(Value::as_str),
        Some("opened" | "repaired")
    )
}

/// The value compared for `fact`; outcomes lose the repaired/opened distinction for tools
/// that repair silently.
pub(super) fn comparable(facts: &Value, fact: &str, reports_repair: bool) -> Value {
    let value = facts.get(fact).cloned().unwrap_or(Value::Null);
    if fact == "outcome" && !reports_repair && value == "repaired" {
        return json!("opened");
    }
    value
}

pub(super) fn compare(lamina: &Report, tools: &[Report], exceptions: &Exceptions) -> Comparison {
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
pub(super) fn rate(tally: &Tally) -> String {
    let compared = tally.agree + tally.disagree;
    if compared == 0 {
        return "-".to_owned();
    }
    #[expect(clippy::cast_precision_loss, reason = "file counts are far below 2^52")]
    let percent = 100.0 * tally.agree as f64 / compared as f64;
    format!("{percent:.1}% ({}/{compared})", tally.agree)
}

/// A JSON value for a Markdown table cell: compact, `|` escaped, at most 80 characters.
pub(super) fn cell(value: &Value) -> String {
    let text = value.to_string();
    let text = if text.chars().count() > 80 {
        format!("{}…", text.chars().take(79).collect::<String>())
    } else {
        text
    };
    util::md_cell(&text)
}

impl Comparison {
    pub(super) fn failures(&self) -> impl Iterator<Item = &Disagreement> {
        self.disagreements.iter().filter(|d| d.kind == Kind::Failure)
    }

    pub(super) fn failed(&self) -> bool {
        self.failures().next().is_some() || !self.lamina_errors.is_empty() || !self.missing.is_empty()
    }

    pub(super) fn count(&self, kind: Kind) -> usize {
        self.disagreements.iter().filter(|d| d.kind == kind).count()
    }

    pub(super) fn summary_line(&self) -> String {
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

    pub(super) fn markdown(
        &self,
        lamina: &Report,
        tools: &[Report],
        exceptions: &Exceptions,
        context: &RunContext,
    ) -> String {
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
    pub(super) fn markdown_details(&self, md: &mut String, exceptions: &Exceptions) {
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

    pub(super) fn json(
        &self,
        lamina: &Report,
        tools: &[Report],
        exceptions: &Exceptions,
        context: &RunContext,
    ) -> Value {
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
