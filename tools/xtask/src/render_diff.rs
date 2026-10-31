//! `cargo xtask render-diff` (todo X3b, ADR 0004 §6, ADR 0018 §2): the nightly hayro-vs-PDFium
//! render diff over the external corpus.
//!
//! Every page (at most `--pages` per file; `--sample` files, stratified per source) is rendered
//! by `lamina-render`'s hayro and PDFium adapters at 96 DPI and compared with the hayro-parity
//! spike's metric (`tools/render`'s `render-diff`, which reuses `render-golden`'s parity
//! comparison; classes `pass`, `minor`, `fail`, `hayro-error`, `pdfium-error`, `both-error`,
//! `timeout`). The readers run through [`runner`] in `--jobs` parallel lanes, so a crash or a
//! hang costs one file; such a file is then rendered by each engine alone to tell which one did
//! it. The tracked number is the **pass+minor share of all rows** (ADR 0004 §6), overall and per
//! source.
//!
//! Writes `report.json`, `summary.html` (per-source table, the `--top` worst pages with their
//! PDFium | hayro | diff images in `diffs/`) and `summary.md` to `target/nightly/render-diff/`.
//! With `--history <dir>` it compares the rates with the last record of `<dir>/history.jsonl`
//! from the same runner and sampling (the benchmark history, `render-diff/<source>` keys in
//! basis points); a drop of more than 2 points writes `regression.md` (the issue body) and sets
//! `regression=true` in `$GITHUB_OUTPUT`, never a failure. `--record` appends this run.
//! `--dry-run` prints the issue that the nightly lane would open.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::BufRead as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::util::{append, github_output, summary};
use crate::workspace::Workspace;
use crate::{corpus, live, nightly, runner, step};

/// A drop of more than this many basis points (2 percentage points) against the last record
/// is a regression.
const REGRESSION_BP: u64 = 200;

/// The history key prefix; `render-diff/all` is the whole run.
const KEY: &str = "render-diff";

/// The resolution of the tracked number (ADR 0004 §6; the spike's).
const DPI: u32 = 96;

const USAGE: &str = "usage: cargo xtask render-diff [--corpus external] [--source <name>]… [--pages <N>] [--sample <N>] \
                     [--jobs <N>] [--top <N>] [--history <dir> [--runner <label>] [--record] [--dry-run]]";

/// What `cargo xtask render-diff` was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Args {
    pub(crate) sources: Vec<String>,
    pub(crate) pages: Option<u32>,
    pub(crate) sample: Option<usize>,
    pub(crate) jobs: usize,
    pub(crate) top: usize,
    pub(crate) history: Option<PathBuf>,
    pub(crate) runner: String,
    pub(crate) record: bool,
    pub(crate) dry_run: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            pages: None,
            sample: None,
            jobs: 4,
            top: 50,
            history: None,
            runner: "local".to_owned(),
            record: false,
            dry_run: false,
        }
    }
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut parsed = Args::default();
    let mut rest = args.iter();
    let positive = |flag: &str, value: Option<&String>| -> Result<usize> {
        let value = value.with_context(|| format!("{flag} needs a value; {USAGE}"))?;
        match value.parse::<usize>() {
            Ok(n) if n > 0 => Ok(n),
            _ => bail!("{flag} {value:?}: not a positive number; {USAGE}"),
        }
    };
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--corpus" => match rest.next().map(String::as_str) {
                Some("external") => {}
                other => bail!("--corpus {other:?}: only `external` is rendered; {USAGE}"),
            },
            "--source" => parsed.sources.push(rest.next().context(USAGE)?.clone()),
            "--pages" => parsed.pages = Some(u32::try_from(positive("--pages", rest.next())?)?),
            "--sample" => parsed.sample = Some(positive("--sample", rest.next())?),
            "--jobs" => parsed.jobs = positive("--jobs", rest.next())?.min(16),
            "--top" => parsed.top = positive("--top", rest.next())?,
            "--history" => parsed.history = Some(PathBuf::from(rest.next().context(USAGE)?)),
            "--runner" => parsed.runner.clone_from(rest.next().context(USAGE)?),
            "--record" => parsed.record = true,
            "--dry-run" => parsed.dry_run = true,
            other => bail!("unknown argument {other:?}; {USAGE}"),
        }
    }
    if (parsed.record || parsed.dry_run) && parsed.history.is_none() {
        bail!("--record and --dry-run need --history; {USAGE}");
    }
    Ok(parsed)
}

/// The tracked classes, in report order. `io-error` (a file that couldn't be read, a diff image
/// that couldn't be written) is neither engine's doing: it is reported, never blamed, and left
/// out of the rates.
const CLASSES: [&str; 8] = [
    "pass",
    "minor",
    "fail",
    "hayro-error",
    "pdfium-error",
    "both-error",
    "timeout",
    "io-error",
];

/// One row of the report: a page both engines were asked to render, or a whole file that no
/// engine rendered a page of (it didn't open, had no pages, crashed or hung).
#[derive(Debug, Clone, PartialEq)]
struct Row {
    file: String,
    /// 1-based; `None` for a whole-file row.
    page: Option<u64>,
    class: String,
    diff_percent: Option<f64>,
    ssim: Option<f64>,
    /// The diff image's file name (minor and fail pages).
    image: Option<String>,
    /// What went wrong, for error and timeout rows.
    detail: Option<String>,
}

impl Row {
    fn file(file: &str, page: Option<u64>, class: &str, detail: String) -> Self {
        Self {
            file: file.to_owned(),
            page,
            class: class.to_owned(),
            diff_percent: None,
            ssim: None,
            image: None,
            detail: Some(detail),
        }
    }

    fn json(&self) -> Value {
        json!({
            "file": self.file,
            "page": self.page,
            "class": self.class,
            "diff_percent": self.diff_percent,
            "ssim": self.ssim,
            "image": self.image,
            "detail": self.detail,
        })
    }
}

/// Which engine crashed or hung on a file, from its single-engine reruns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Blame {
    hayro: bool,
    pdfium: bool,
}

/// What the `--rows` files say about one file: the most pages any reading of it reported
/// (lamina's, or an engine's once it opened the file), and the rows of the pages both engines
/// finished, by page (a later attempt's row replaces an earlier one's).
#[derive(Debug, Default, Clone, PartialEq)]
struct Partial {
    pages: Option<u64>,
    rows: BTreeMap<u64, Value>,
}

/// Reads the readers' `--rows` files (JSON lines; a torn last line of a killed process is
/// skipped): page counts from every file's headers, page rows from the compared ones (they
/// carry a `class`; the single-engine reruns' don't).
fn read_rows(paths: &[PathBuf]) -> BTreeMap<String, Partial> {
    let mut out: BTreeMap<String, Partial> = BTreeMap::new();
    for path in paths {
        let Ok(text) = fs::read_to_string(path) else { continue };
        for line in text.lines() {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let Some(file) = value.get("file").and_then(Value::as_str) else {
                continue;
            };
            let partial = out.entry(file.to_owned()).or_default();
            if value.get("header").and_then(Value::as_bool) == Some(true) {
                let counts = ["lamina_pages", "hayro_pages", "pdfium_pages"]
                    .iter()
                    .filter_map(|k| value.get(*k).and_then(Value::as_u64));
                partial.pages = partial.pages.into_iter().chain(counts).max();
            } else if value.get("class").is_some()
                && let Some(page) = value.get("page").and_then(Value::as_u64)
            {
                partial.rows.insert(page, value);
            }
        }
    }
    out
}

/// The rows of one file. A compared file has a row per page it rendered; a file that crashed or
/// hung keeps the pages it finished and gets a failure row for each other page it has (by
/// `partial`'s count, capped at `cap`), and one opened by neither engine a row per page by
/// lamina's count. Without any count it is one row. So a worse outcome never counts for fewer
/// rows than the page errors it replaces. `blame` is the single-engine verdict for a crash or a
/// timeout.
fn rows_of(file: &str, fact: &Value, blame: Option<Blame>, partial: &Partial, cap: Option<u32>) -> Vec<Row> {
    let capped = |n: Option<u64>| n.map(|n| cap.map_or(n, |c| n.min(u64::from(c))));
    let outcome = fact.get("outcome").and_then(Value::as_str).unwrap_or("missing");
    if outcome != "compared" {
        let (class, who) = match (outcome, blame) {
            (
                _,
                Some(Blame {
                    hayro: true,
                    pdfium: true,
                }),
            ) => ("both-error", "both engines"),
            (
                _,
                Some(Blame {
                    hayro: true,
                    pdfium: false,
                }),
            ) => ("hayro-error", "hayro"),
            (
                _,
                Some(Blame {
                    hayro: false,
                    pdfium: true,
                }),
            ) => ("pdfium-error", "PDFium"),
            _ => ("both-error", "neither engine alone (only together)"),
        };
        let class = if outcome == "timeout" { "timeout" } else { class };
        let detail = format!("{outcome} ({who} on its own)");
        let mut rows = page_rows(file, &partial.rows.values().cloned().collect::<Vec<_>>());
        match capped(partial.pages) {
            Some(n) if n > 0 => rows.extend(
                (1..=n)
                    .filter(|p| !partial.rows.contains_key(p))
                    .map(|p| Row::file(file, Some(p), class, detail.clone())),
            ),
            _ if rows.is_empty() => rows.push(Row::file(file, None, class, detail)),
            _ => {}
        }
        rows.sort_by_key(|r| r.page);
        return rows;
    }
    if let Some(error) = fact.get("io_error").and_then(Value::as_str) {
        return vec![Row::file(file, None, "io-error", error.to_owned())];
    }
    let pages = fact.get("pages").and_then(Value::as_array).cloned().unwrap_or_default();
    if !pages.is_empty() {
        return page_rows(file, &pages);
    }
    let error = |key: &str| fact.get(key).and_then(Value::as_str).map(str::to_owned);
    let (h, p) = (error("hayro_open_error"), error("pdfium_open_error"));
    let class = match (&h, &p) {
        (Some(_), None) => "hayro-error",
        (None, Some(_)) => "pdfium-error",
        _ => "both-error",
    };
    let detail = match (h, p) {
        (None, None) => "no pages in either engine".to_owned(),
        (h, p) => format!(
            "hayro: {}; PDFium: {}",
            h.as_deref().unwrap_or("opened"),
            p.as_deref().unwrap_or("opened")
        ),
    };
    match capped(fact.get("lamina_pages").and_then(Value::as_u64)) {
        Some(n) if n > 0 => (1..=n)
            .map(|p| Row::file(file, Some(p), class, detail.clone()))
            .collect(),
        _ => vec![Row::file(file, None, class, detail)],
    }
}

/// The rows of the reader's page entries.
fn page_rows(file: &str, pages: &[Value]) -> Vec<Row> {
    pages
        .iter()
        .map(|page| {
            let class = page.get("class").and_then(Value::as_str).unwrap_or("both-error");
            let error = |engine: &str| page.pointer(&format!("/{engine}/error")).and_then(Value::as_str);
            let detail = match (
                error("hayro"),
                error("pdfium"),
                page.get("io_error").and_then(Value::as_str),
            ) {
                (_, _, Some(io)) => Some(io.to_owned()),
                (None, None, None) => None,
                (h, p, None) => Some(format!("hayro: {}; PDFium: {}", h.unwrap_or("ok"), p.unwrap_or("ok"))),
            };
            Row {
                file: file.to_owned(),
                page: page.get("page").and_then(Value::as_u64),
                class: class.to_owned(),
                diff_percent: page.get("diff_percent").and_then(Value::as_f64),
                ssim: page.get("ssim").and_then(Value::as_f64),
                image: page.get("image").and_then(Value::as_str).map(str::to_owned),
                detail,
            }
        })
        .collect()
}

/// Rows per class for one source (or all of them).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Tally {
    rows: u64,
    classes: BTreeMap<String, u64>,
}

impl Tally {
    fn add(&mut self, class: &str) {
        self.rows += 1;
        *self.classes.entry(class.to_owned()).or_default() += 1;
    }

    fn count(&self, class: &str) -> u64 {
        self.classes.get(class).copied().unwrap_or_default()
    }

    /// The rows the rates are over: all but `io-error`, which is neither engine's doing.
    fn rated(&self) -> u64 {
        self.rows - self.count("io-error")
    }

    /// The tracked number: pass + minor over the rated rows, in basis points (10,000 = 100 %).
    fn rate_bp(&self) -> u64 {
        share_bp(self.count("pass") + self.count("minor"), self.rated())
    }

    /// The pass share alone, in basis points.
    fn pass_bp(&self) -> u64 {
        share_bp(self.count("pass"), self.rated())
    }

    fn json(&self) -> Value {
        json!({
            "rows": self.rows,
            "classes": self.classes,
            "pass_minor_percent": percent(self.rate_bp()),
            "pass_percent": percent(self.pass_bp()),
        })
    }
}

/// `part / whole` in basis points, rounded; 0 for an empty whole.
fn share_bp(part: u64, whole: u64) -> u64 {
    (part * 10_000 + whole / 2).checked_div(whole).unwrap_or(0)
}

#[expect(clippy::cast_precision_loss, reason = "basis points are at most 10,000")]
fn percent(bp: u64) -> f64 {
    bp as f64 / 100.0
}

/// Per source (`pdfjs/…` → `pdfjs`), and `all`.
fn tally(rows: &[Row]) -> BTreeMap<String, Tally> {
    let mut out: BTreeMap<String, Tally> = BTreeMap::new();
    for row in rows {
        out.entry(live::group(&row.file).to_owned())
            .or_default()
            .add(&row.class);
        out.entry("all".to_owned()).or_default().add(&row.class);
    }
    out
}

/// The `n` worst pages with a diff image: `fail` before `minor`, then by diff share (a size
/// mismatch, which has none, first), then by SSIM.
fn worst(rows: &[Row], n: usize) -> Vec<&Row> {
    let mut with_image: Vec<&Row> = rows.iter().filter(|r| r.image.is_some()).collect();
    with_image.sort_by(|a, b| {
        let rank = |r: &Row| u8::from(r.class != "fail");
        rank(a)
            .cmp(&rank(b))
            .then(
                b.diff_percent
                    .unwrap_or(f64::INFINITY)
                    .total_cmp(&a.diff_percent.unwrap_or(f64::INFINITY)),
            )
            .then(a.ssim.unwrap_or(0.0).total_cmp(&b.ssim.unwrap_or(0.0)))
            .then(a.file.cmp(&b.file))
            .then(a.page.cmp(&b.page))
    });
    with_image.truncate(n);
    with_image
}

/// What this run measured, as a history record's comparable part.
#[derive(Debug, Clone, PartialEq)]
struct Sampling {
    dpi: u32,
    pages: Option<u32>,
    sample: Option<usize>,
    sources: Vec<String>,
}

impl Sampling {
    /// The sampling of a run; the sources are sorted and deduplicated, so `--source` order
    /// doesn't change which history records compare.
    fn new(pages: Option<u32>, sample: Option<usize>, sources: &[String]) -> Self {
        let mut sources = sources.to_vec();
        sources.sort();
        sources.dedup();
        Self {
            dpi: DPI,
            pages,
            sample,
            sources,
        }
    }

    fn json(&self) -> Value {
        json!({"dpi": self.dpi, "pages": self.pages, "sample": self.sample, "sources": self.sources})
    }
}

/// `render-diff/<source>` (and `render-diff/all`) → pass+minor share in basis points: the
/// history's `benches` entries (u64, like the benchmarks' nanoseconds).
fn history_entries(tallies: &BTreeMap<String, Tally>) -> BTreeMap<String, u64> {
    tallies
        .iter()
        .map(|(source, t)| (format!("{KEY}/{source}"), t.rate_bp()))
        .collect()
}

/// A rate that dropped: key, previous and current basis points.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Drop {
    key: String,
    before: u64,
    now: u64,
}

/// The last render-diff record of `history` from `runner` with the same sampling, and every
/// rate that is more than [`REGRESSION_BP`] below it.
fn regressions<'a>(
    history: &'a [Value],
    runner: &str,
    sampling: &Sampling,
    current: &BTreeMap<String, u64>,
) -> (Option<&'a Value>, Vec<Drop>) {
    let last = history.iter().rev().find(|r| {
        r.get("kind").and_then(Value::as_str) == Some(KEY)
            && r.get("runner").and_then(Value::as_str) == Some(runner)
            && r.get("sampling") == Some(&sampling.json())
    });
    let Some(last) = last else {
        return (None, Vec::new());
    };
    let drops = current
        .iter()
        .filter_map(|(key, &now)| {
            let before = last.get("benches")?.get(key)?.as_u64()?;
            (before > now + REGRESSION_BP).then(|| Drop {
                key: key.clone(),
                before,
                now,
            })
        })
        .collect();
    (Some(last), drops)
}

/// The regression issue's title.
fn issue_title(reference: &str) -> String {
    format!("Nightly render-diff regression on {reference}")
}

/// The regression issue's body (the nightly lane puts the run's link above it).
fn issue_body(drops: &[Drop], last: &Value, sampling: &Sampling, worst: &[&Row]) -> String {
    let mut md = String::new();
    let _ = writeln!(
        md,
        "The hayro-vs-PDFium pass rate (pass + minor share of all pages, ADR 0004 §6) dropped by more than \
         {} points against the last record (commit `{}`, {}). It does not fail the build.\n",
        REGRESSION_BP / 100,
        last.get("commit").and_then(Value::as_str).unwrap_or("unknown"),
        last.get("date").and_then(Value::as_str).unwrap_or("unknown date"),
    );
    let _ = writeln!(md, "| Key | Before | Now | Change |\n| --- | ---: | ---: | ---: |");
    for d in drops {
        let change = percent(d.before - d.now);
        let _ = writeln!(
            md,
            "| `{}` | {:.2} % | {:.2} % | −{change:.2} points |",
            d.key,
            percent(d.before),
            percent(d.now)
        );
    }
    let _ = writeln!(
        md,
        "\nSampling: {} DPI, {}, {}.",
        sampling.dpi,
        sampling
            .pages
            .map_or("every page".to_owned(), |n| format!("the first {n} pages of each file")),
        sampling
            .sample
            .map_or("every file".to_owned(), |n| format!("{n} files stratified per source"))
    );
    if !worst.is_empty() {
        let _ = writeln!(
            md,
            "\nThe worst pages now (diff images in the run's `render-diff` artefact):\n"
        );
        for row in worst.iter().take(10) {
            let _ = writeln!(md, "- {}", describe(row));
        }
    }
    md
}

/// `` `file` p3 fail: 4.21 % differ, SSIM 0.8123 ``.
fn describe(row: &Row) -> String {
    let page = row.page.map_or(String::new(), |p| format!(" p{p}"));
    let metric = match (row.diff_percent, row.ssim) {
        (Some(d), Some(s)) => format!("{d:.2} % differ, SSIM {s:.4}"),
        _ if row.class == "fail" => "page sizes differ".to_owned(),
        _ => row.detail.clone().unwrap_or_default(),
    };
    format!("`{}`{page} {}: {metric}", row.file, row.class)
}

fn sources_table(tallies: &BTreeMap<String, Tally>) -> String {
    let mut md = String::from("| Source | Pages | pass+minor % | pass % |");
    for class in CLASSES {
        let _ = write!(md, " {class} |");
    }
    md.push_str("\n| --- | ---: | ---: | ---: |");
    md.push_str(&" ---: |".repeat(CLASSES.len()));
    md.push('\n');
    for (source, t) in tallies {
        let _ = write!(
            md,
            "| {source} | {} | {:.2} % | {:.2} % |",
            t.rows,
            percent(t.rate_bp()),
            percent(t.pass_bp())
        );
        for class in CLASSES {
            let _ = write!(md, " {} |", t.count(class));
        }
        md.push('\n');
    }
    md
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The HTML summary: the per-source table, the sampling, and the worst pages with their
/// images (`diffs/<image>`).
fn html(tallies: &BTreeMap<String, Tally>, worst: &[&Row], errors: &[&Row], context: &str) -> String {
    let mut h = String::from(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>Render diff: hayro vs PDFium</title>\n<style>\n\
         :root { color-scheme: light dark; --bg: #fbfaf7; --fg: #1f1d1a; --muted: #6b665e; --rule: #ddd8cf; --fail: #b3261e; }\n\
         @media (prefers-color-scheme: dark) { :root { --bg: #161513; --fg: #ece8e1; --muted: #a29c92; --rule: #3a3732; --fail: #f2b8b5; } }\n\
         body { background: var(--bg); color: var(--fg); font: 15px/1.5 system-ui, sans-serif; margin: 0 auto; max-width: 1200px; padding: 16px; }\n\
         table { border-collapse: collapse; display: block; overflow-x: auto; }\n\
         th, td { border-bottom: 1px solid var(--rule); padding: 4px 10px; text-align: right; white-space: nowrap; }\n\
         th:first-child, td:first-child { text-align: left; }\n\
         .muted { color: var(--muted); } .fail { color: var(--fail); }\n\
         figure { border-top: 1px solid var(--rule); margin: 24px 0; padding-top: 12px; }\n\
         figure img { display: block; height: auto; max-width: 100%; }\n\
         code { overflow-wrap: anywhere; }\n</style>\n</head>\n<body>\n\
         <h1>Render diff: hayro vs PDFium</h1>\n",
    );
    let _ = writeln!(h, "<p class=\"muted\">{}</p>", escape(context));
    h.push_str("<table>\n<thead><tr><th>Source</th><th>Pages</th><th>pass+minor %</th><th>pass %</th>");
    for class in CLASSES {
        let _ = write!(h, "<th>{class}</th>");
    }
    h.push_str("</tr></thead>\n<tbody>\n");
    for (source, t) in tallies {
        let _ = write!(
            h,
            "<tr><td>{}</td><td>{}</td><td>{:.2} %</td><td>{:.2} %</td>",
            escape(source),
            t.rows,
            percent(t.rate_bp()),
            percent(t.pass_bp())
        );
        for class in CLASSES {
            let _ = write!(h, "<td>{}</td>", t.count(class));
        }
        h.push_str("</tr>\n");
    }
    h.push_str("</tbody>\n</table>\n");
    let _ = writeln!(
        h,
        "<h2>The {} worst pages</h2>\n<p class=\"muted\">Each image is PDFium | hayro | diff (differing pixels in red).</p>",
        worst.len()
    );
    for row in worst {
        let image = row.image.as_deref().unwrap_or_default();
        let _ = writeln!(
            h,
            "<figure><figcaption><span class=\"fail\">{}</span> <code>{}</code>{}: {}</figcaption>\
             <img loading=\"lazy\" src=\"diffs/{}\" alt=\"PDFium, hayro and diff of {} page {}\"></figure>",
            escape(&row.class),
            escape(&row.file),
            row.page.map_or(String::new(), |p| format!(" page {p}")),
            escape(describe(row).split(": ").last().unwrap_or_default()),
            escape(image),
            escape(&row.file),
            row.page.unwrap_or_default()
        );
    }
    if !errors.is_empty() {
        let _ = writeln!(h, "<h2>Errors and timeouts ({})</h2>\n<ul>", errors.len());
        for row in errors {
            let _ = writeln!(h, "<li>{}</li>", escape(&describe(row)));
        }
        h.push_str("</ul>\n");
    }
    h.push_str("</body>\n</html>\n");
    h
}

/// Builds `tools/render`'s `render-diff` in release mode and returns its path.
fn build_reader(ws: &Workspace) -> Result<PathBuf> {
    let manifest = ws.root.join(crate::render::MANIFEST);
    step("cargo build --release --locked --bin render-diff (tools/render)");
    let mut child = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--locked",
            "--bin",
            "render-diff",
            "--message-format=json-render-diagnostics",
        ])
        .arg("--manifest-path")
        .arg(&manifest)
        .current_dir(&ws.root)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("running cargo build")?;
    let mut executable = None;
    if let Some(stdout) = child.stdout.take() {
        for line in std::io::BufReader::new(stdout).lines() {
            let message: Value = serde_json::from_str(&line?).unwrap_or(Value::Null);
            if let Some(path) = message.get("executable").and_then(Value::as_str) {
                executable = Some(PathBuf::from(path));
            }
        }
    }
    let status = child.wait()?;
    if !status.success() {
        bail!("building render-diff failed with {status}");
    }
    executable.context("cargo build printed no executable for render-diff")
}

/// The time limits of the readers: small chunks, so a hang (which reruns its chunk one file at
/// a time) costs a few minutes; a lone file gets 3 minutes (20 pages, two engines; C1a's
/// per-page limit was 30 s).
fn limits() -> runner::Limits {
    runner::Limits {
        chunk: 8,
        chunk_timeout: Duration::from_mins(6),
        file_timeout: Duration::from_mins(3),
    }
}

/// The reader's header (`tool`, `version`, `settings`: DPI, page cap and the class thresholds),
/// from a run over an empty list: the runner keeps only the facts and the version.
fn header(reader: &runner::Reader, out: &Path) -> Result<Value> {
    let work = out.join("work");
    fs::create_dir_all(&work)?;
    let (list, report) = (work.join("empty.tsv"), work.join("header.json"));
    fs::write(&list, "")?;
    crate::run(&reader.dir, &reader.program.to_string_lossy(), &{
        let mut a = reader.args.clone();
        a.extend([list.into_os_string(), report.clone().into_os_string()]);
        a
    })?;
    let text = fs::read_to_string(&report).with_context(|| format!("reading {}", report.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", report.display()))
}

/// Runs `reader` over `inputs` in `jobs` lanes that take chunks from one queue, each chunk
/// through [`runner`] under `limits` (work folders `work/<name>-<lane>`); every file's fact and
/// why the reader broke, if it did.
fn run_lanes(
    reader: &runner::Reader,
    inputs: &[(String, PathBuf)],
    jobs: usize,
    limits: runner::Limits,
    out: &Path,
) -> Result<(serde_json::Map<String, Value>, Vec<String>)> {
    // A shared queue of chunks, so a lane slowed by a hang doesn't hold up the others.
    let chunks: Vec<&[(String, PathBuf)]> = inputs.chunks(limits.chunk.max(1)).collect();
    let next = AtomicUsize::new(0);
    let runs: Vec<Result<runner::Run>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..jobs.min(chunks.len()))
            .map(|j| {
                let work = out.join(format!("work/{}-{j}", reader.name));
                let (chunks, next) = (&chunks, &next);
                scope.spawn(move || -> Vec<Result<runner::Run>> {
                    if let Err(e) = fs::create_dir_all(&work) {
                        return vec![Err(e.into())];
                    }
                    let mut runs = Vec::new();
                    while let Some(chunk) = chunks.get(next.fetch_add(1, Ordering::Relaxed)) {
                        runs.push(runner::run(reader, chunk, &work, limits));
                    }
                    runs
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| {
                h.join()
                    .unwrap_or_else(|_| vec![Err(anyhow::anyhow!("a lane panicked"))])
            })
            .collect()
    });
    let mut facts = serde_json::Map::new();
    let mut broken = Vec::new();
    for run in runs {
        let run = run?;
        if let Some(why) = run.broken {
            broken.push(why);
        }
        if let Some(Value::Object(f)) = run.report.get("facts") {
            facts.extend(f.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    }
    Ok((facts, broken))
}

/// For each input that crashed or hung, whether hayro and PDFium each do so rendering it
/// alone (`engine` makes the single-engine readers).
fn blame(
    engine: &dyn Fn(&'static str) -> runner::Reader,
    facts: &serde_json::Map<String, Value>,
    inputs: &[(String, PathBuf)],
    jobs: usize,
    out: &Path,
) -> Result<BTreeMap<String, Blame>> {
    let mut blames = BTreeMap::new();
    let failed: Vec<(String, PathBuf)> = inputs
        .iter()
        .filter(|(name, _)| facts.get(name).and_then(|f| f.get("outcome")).and_then(Value::as_str) != Some("compared"))
        .cloned()
        .collect();
    if !failed.is_empty() {
        step(&format!(
            "render-diff: {} files crashed or hung; each engine alone",
            failed.len()
        ));
        // One file per process, under the per-file limit (a chunk of one gets no longer), so
        // every crash is "broken" to the runner: expected here. Both engines at once.
        let one = runner::Limits {
            chunk: 1,
            chunk_timeout: limits().file_timeout,
            ..limits()
        };
        let (hayro_reader, pdfium_reader) = (engine("hayro"), engine("pdfium"));
        let (hayro, pdfium) = std::thread::scope(|scope| {
            let hayro = scope.spawn(|| run_lanes(&hayro_reader, &failed, jobs, one, out));
            let pdfium = scope.spawn(|| run_lanes(&pdfium_reader, &failed, jobs, one, out));
            let join = |h: std::thread::ScopedJoinHandle<'_, _>| {
                h.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("a blame lane panicked")))
            };
            (join(hayro), join(pdfium))
        });
        let ((hayro, _), (pdfium, _)) = (hayro?, pdfium?);
        let bad = |facts: &serde_json::Map<String, Value>, name: &str| {
            facts.get(name).and_then(|f| f.get("outcome")).and_then(Value::as_str) != Some("compared")
        };
        for (name, _) in &failed {
            blames.insert(
                name.clone(),
                Blame {
                    hayro: bad(&hayro, name),
                    pdfium: bad(&pdfium, name),
                },
            );
        }
    }

    Ok(blames)
}

/// What the readers reported: every file's fact, the first report (tool version, settings) and
/// which engine to blame for each file that crashed or hung.
struct Rendered {
    facts: serde_json::Map<String, Value>,
    header: Value,
    blames: BTreeMap<String, Blame>,
    /// From the readers' `--rows` files: what a file that crashed or hung got done.
    partials: BTreeMap<String, Partial>,
}

/// Renders and compares every input in `--jobs` lanes, then reruns each file that crashed or
/// hung with one engine at a time; the diff images go to `out/work/images`.
fn render_all(ws: &Workspace, args: &Args, inputs: &[(String, PathBuf)], out: &Path) -> Result<Rendered> {
    let pdfium_dir = crate::pdfium::fetch(ws)?;
    let reader = build_reader(ws)?;
    step(&format!(
        "render-diff: {} files at {DPI} dpi, {}, {} at once",
        inputs.len(),
        args.pages
            .map_or("every page".to_owned(), |n| format!("at most {n} pages each")),
        args.jobs
    ));
    let mut common = vec![
        "--dpi".into(),
        DPI.to_string().into(),
        "--pdfium-dir".into(),
        pdfium_dir.into_os_string(),
    ];
    if let Some(n) = args.pages {
        common.extend(["--pages".into(), n.to_string().into()]);
    }
    let engine = |name: &'static str| {
        let mut a = vec![
            "--engine".into(),
            if name == "render-diff" { "both" } else { name }.into(),
        ];
        a.extend(common.iter().cloned());
        a.extend([
            "--rows".into(),
            out.join(format!("work/rows-{name}.jsonl")).into_os_string(),
        ]);
        runner::Reader {
            name,
            program: reader.clone().into_os_string(),
            args: a,
            dir: ws.root.clone(),
            reports_repair: false,
        }
    };
    let mut both = engine("render-diff");
    both.args
        .extend(["--images".into(), out.join("work/images").into_os_string()]);

    let header = header(&both, out)?;
    let (facts, broken) = run_lanes(&both, inputs, args.jobs, limits(), out)?;
    // The header run proved the reader works, so a chunk whose every file crashed is that many
    // crashes (the runner has marked each one), not a broken reader.
    for why in &broken {
        println!("::warning::render-diff: {why}; each file counts as a crash");
    }
    let blames = blame(&engine, &facts, inputs, args.jobs, out)?;
    let partials =
        read_rows(&["render-diff", "hayro", "pdfium"].map(|name| out.join(format!("work/rows-{name}.jsonl"))));
    Ok(Rendered {
        facts,
        header,
        blames,
        partials,
    })
}

/// Compares this run with the last point of `<dir>/history.jsonl` (writes `regression.md` and
/// prints it with `--dry-run` when a rate dropped by more than 2 points), then appends the
/// record with `--record`; returns the comparison for the report.
fn compare_history(args: &Args, out: &Path, record: &Value, sampling: &Sampling, top: &[&Row]) -> Result<Value> {
    let Some(dir) = &args.history else {
        return Ok(Value::Null);
    };
    let entries: BTreeMap<String, u64> = record
        .get("benches")
        .and_then(Value::as_object)
        .map(|m| m.iter().filter_map(|(k, v)| Some((k.clone(), v.as_u64()?))).collect())
        .unwrap_or_default();
    let path = dir.join("history.jsonl");
    let history = nightly::read_history(&path)?;
    let (last, drops) = regressions(&history, &args.runner, sampling, &entries);
    let comparison = json!({
        "previous": last.map(|r| r.get("commit")),
        "drops": drops.iter().map(|d| json!({"key": d.key, "before_bp": d.before, "now_bp": d.now})).collect::<Vec<_>>(),
    });
    github_output("regression", if drops.is_empty() { "false" } else { "true" })?;
    match last {
        None => step(&format!(
            "render-diff: no earlier record from `{}` with this sampling in {}",
            args.runner,
            path.display()
        )),
        Some(last) if drops.is_empty() => step(&format!(
            "render-diff: no rate dropped by more than {} points since {}",
            REGRESSION_BP / 100,
            last.get("commit").and_then(Value::as_str).unwrap_or("unknown")
        )),
        Some(last) => {
            let body = issue_body(&drops, last, sampling, top);
            fs::write(out.join("regression.md"), &body)?;
            println!(
                "::warning::render-diff: {} pass rates dropped by more than 2 points",
                drops.len()
            );
            if args.dry_run {
                let reference = std::env::var("GITHUB_REF_NAME").unwrap_or_else(|_| "<ref>".to_owned());
                println!(
                    "\n--- dry run: the nightly lane would open (or comment on) this issue ---\n\
                     title: {}\nlabel: nightly\n\n{body}--- end of the issue ---",
                    issue_title(&reference)
                );
            }
        }
    }
    if args.record {
        fs::create_dir_all(dir)?;
        append(&path, &(serde_json::to_string(record)? + "\n"))?;
        step(&format!("Appended this run to {}", path.display()));
    }
    Ok(comparison)
}

/// Moves the diff images of `top` from `out/work/images` (every minor and fail page) to
/// `out/diffs` (what the HTML summary shows) and returns the names it moved; the rest go with
/// the work folder. An image that isn't there (a reader killed while writing it) is reported
/// and skipped.
fn keep_images(top: &[&Row], out: &Path) -> Result<BTreeSet<String>> {
    let (images, diffs) = (out.join("work/images"), out.join("diffs"));
    fs::create_dir_all(&diffs)?;
    let mut kept = BTreeSet::new();
    for image in top.iter().filter_map(|r| r.image.as_ref()) {
        match fs::rename(images.join(image), diffs.join(image)) {
            Ok(()) => {
                kept.insert(image.clone());
            }
            Err(e) => println!("::warning::render-diff: diff image {image}: {e}"),
        }
    }
    Ok(kept)
}

/// Every input's rows (see [`rows_of`]), in input order.
fn all_rows(inputs: &[(String, PathBuf)], rendered: &Rendered, cap: Option<u32>) -> Vec<Row> {
    let none = Partial::default();
    inputs
        .iter()
        .flat_map(|(name, _)| {
            rows_of(
                name,
                rendered.facts.get(name).unwrap_or(&Value::Null),
                rendered.blames.get(name).copied(),
                rendered.partials.get(name).unwrap_or(&none),
                cap,
            )
        })
        .collect()
}

/// `cargo xtask render-diff …`.
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let mut inputs = corpus::verified_inputs(ws, &args.sources)?;
    inputs.sort();
    if let Some(n) = args.sample {
        inputs = live::sample(inputs, n);
    }
    if inputs.is_empty() {
        bail!("no inputs: nothing would be rendered (check --source and --sample)");
    }
    let out = ws.root.join("target/nightly/render-diff");
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("removing {}", out.display()))?;
    }
    fs::create_dir_all(out.join("work/images"))?;
    let rendered = render_all(ws, args, &inputs, &out)?;
    let rows = all_rows(&inputs, &rendered, args.pages);
    let header = rendered.header;
    let tallies = tally(&rows);
    let top = worst(&rows, args.top);
    let kept = keep_images(&top, &out)?;
    let top: Vec<&Row> = top
        .into_iter()
        .filter(|r| r.image.as_ref().is_some_and(|i| kept.contains(i)))
        .collect();
    let errors: Vec<&Row> = rows
        .iter()
        .filter(|r| !matches!(r.class.as_str(), "pass" | "minor" | "fail"))
        .collect();

    let commit = crate::util::git_stdout(&ws.root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    let date = crate::util::git_stdout(&ws.root, &["show", "-s", "--format=%cI", "HEAD"]).unwrap_or_default();
    let sampling = Sampling::new(args.pages, args.sample, &args.sources);
    let all = tallies.get("all").cloned().unwrap_or_default();
    let context = format!(
        "{} files ({}), {} rows at {DPI} DPI, {}; {} on commit {}. pass+minor {:.2} %, pass {:.2} %. \
         Thresholds (hayro-parity spike, ADR 0004 §6): pass ≤ 0.5 % differing pixels and SSIM ≥ 0.90, \
         minor ≤ 2 % and SSIM ≥ 0.85, else fail.",
        inputs.len(),
        args.sample.map_or("the whole corpus".to_owned(), |n| format!(
            "sampled to {n}, stratified per source"
        )),
        all.rows,
        args.pages
            .map_or("every page".to_owned(), |n| format!("the first {n} pages of each file")),
        header
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("hayro / PDFium"),
        commit,
        percent(all.rate_bp()),
        percent(all.pass_bp())
    );

    // History: compare with the last point before appending this one.
    let entries = history_entries(&tallies);
    let record = json!({
        "commit": commit,
        "date": date,
        "runner": args.runner,
        "quick": false,
        "kind": KEY,
        "sampling": sampling.json(),
        "benches": entries,
    });
    let history_json = compare_history(args, &out, &record, &sampling, &top)?;
    let report = json!({
        "commit": commit,
        "date": date,
        "runner": args.runner,
        "tool": header.get("version"),
        "settings": header.get("settings"),
        "sampling": sampling.json(),
        "files": inputs.len(),
        "sources": tallies.iter().map(|(k, t)| (k.clone(), t.json())).collect::<serde_json::Map<_, _>>(),
        "history": history_json,
        "record": record,
        "worst": top.iter().map(|r| r.json()).collect::<Vec<_>>(),
        "rows": rows.iter().map(Row::json).collect::<Vec<_>>(),
    });
    fs::write(out.join("report.json"), serde_json::to_string_pretty(&report)? + "\n")?;
    fs::write(out.join("summary.html"), html(&tallies, &top, &errors, &context))?;
    let markdown = format!(
        "## Render diff (hayro vs PDFium)\n\n{context}\n\n{}",
        sources_table(&tallies)
    );
    fs::write(out.join("summary.md"), &markdown)?;
    summary(&markdown)?;
    fs::remove_dir_all(out.join("work")).with_context(|| "removing the work folder".to_owned())?;
    println!(
        "render-diff: {} rows, pass+minor {:.2} %, pass {:.2} %",
        all.rows,
        percent(all.rate_bp()),
        percent(all.pass_bp())
    );
    step(&format!("Report: {}", out.join("summary.html").display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(class: &str, diff: Option<f64>, image: bool) -> Value {
        json!({"page": 1, "class": class, "diff_percent": diff, "ssim": diff.map(|_| 0.95),
               "image": image.then_some("x.png"), "hayro": {}, "pdfium": {}})
    }

    fn row(file: &str, class: &str, diff: Option<f64>) -> Row {
        Row {
            file: file.to_owned(),
            page: Some(1),
            class: class.to_owned(),
            diff_percent: diff,
            ssim: Some(0.9),
            image: Some(format!("{file}.png")),
            detail: None,
        }
    }

    fn classes(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.class.as_str()).collect()
    }

    #[test]
    fn rows_of_should_keep_one_row_per_page() {
        let fact =
            json!({"outcome": "compared", "pages": [page("pass", Some(0.1), false), page("fail", Some(3.0), true)]});
        assert_eq!(
            classes(&rows_of("pdfjs/a.pdf", &fact, None, &Partial::default(), None)),
            ["pass", "fail"]
        );
    }

    #[test]
    fn rows_of_should_blame_the_engine_that_crashed_alone() {
        let blame = Blame {
            hayro: true,
            pdfium: false,
        };
        let rows = rows_of(
            "pdfjs/a.pdf",
            &json!({"outcome": "crash"}),
            Some(blame),
            &Partial::default(),
            None,
        );
        assert_eq!(classes(&rows), ["hayro-error"]);
    }

    #[test]
    fn rows_of_should_keep_a_timeout_a_timeout() {
        let blame = Blame {
            hayro: false,
            pdfium: true,
        };
        let rows = rows_of(
            "pdfjs/a.pdf",
            &json!({"outcome": "timeout"}),
            Some(blame),
            &Partial::default(),
            None,
        );
        assert_eq!(classes(&rows), ["timeout"]);
    }

    #[test]
    fn rows_of_should_count_a_file_only_pdfium_failed_to_open() {
        let fact = json!({"outcome": "compared", "pages": [], "pdfium_open_error": "render-failed"});
        assert_eq!(
            classes(&rows_of("qpdf/a.pdf", &fact, None, &Partial::default(), None)),
            ["pdfium-error"]
        );
    }

    const PDFIUM_CRASH: Blame = Blame {
        hayro: false,
        pdfium: true,
    };

    #[test]
    fn rows_of_should_count_a_crashed_file_once_per_page_it_has() {
        let partial = Partial {
            pages: Some(30),
            rows: BTreeMap::from([(1, page("pass", Some(0.1), false))]),
        };
        let rows = rows_of(
            "pdfjs/a.pdf",
            &json!({"outcome": "crash"}),
            Some(PDFIUM_CRASH),
            &partial,
            Some(20),
        );
        let pages: Vec<Option<u64>> = rows.iter().map(|r| r.page).collect();
        assert_eq!(
            (classes(&rows)[..2].to_vec(), pages.len(), pages[19]),
            (vec!["pass", "pdfium-error"], 20, Some(20))
        );
    }

    #[test]
    fn a_crash_should_not_raise_the_rate_above_the_page_errors_it_replaces() {
        // Three pages: one passes, two hit a PDFium error; or PDFium crashes on page 2.
        let pages = json!({"outcome": "compared", "pages": [
            page("pass", Some(0.1), false), page("pdfium-error", None, false), page("pdfium-error", None, false)]});
        let errors = tally(&rows_of("pdfjs/a.pdf", &pages, None, &Partial::default(), None));
        let partial = Partial {
            pages: Some(3),
            rows: BTreeMap::from([(1, page("pass", Some(0.1), false))]),
        };
        let crash = tally(&rows_of(
            "pdfjs/a.pdf",
            &json!({"outcome": "crash"}),
            Some(PDFIUM_CRASH),
            &partial,
            None,
        ));
        assert!(crash["all"].rate_bp() <= errors["all"].rate_bp());
    }

    #[test]
    fn rows_of_should_count_a_file_neither_engine_opened_by_lamina_pages() {
        let fact = json!({"outcome": "compared", "pages": [], "lamina_pages": 4,
                          "hayro_open_error": "x", "pdfium_open_error": "y"});
        assert_eq!(
            classes(&rows_of("qpdf/a.pdf", &fact, None, &Partial::default(), None)),
            ["both-error"; 4]
        );
    }

    #[test]
    fn rows_of_should_keep_an_unreadable_file_out_of_the_engines_blame() {
        let fact = json!({"outcome": "compared", "pages": [], "io_error": "reading x: denied"});
        let rows = rows_of("qpdf/a.pdf", &fact, None, &Partial::default(), None);
        assert_eq!((classes(&rows), tally(&rows)["all"].rated()), (vec!["io-error"], 0));
    }

    #[test]
    fn read_rows_should_keep_page_rows_and_the_largest_page_count() {
        let dir = std::env::temp_dir().join(format!("xtask-render-rows-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rows.jsonl");
        let mut row = page("fail", Some(3.0), false);
        row["file"] = json!("pdfjs/a.pdf");
        let text = format!(
            "{}\n{}\n{}\n{{\"torn",
            json!({"file": "pdfjs/a.pdf", "header": true, "lamina_pages": 3}),
            json!({"file": "pdfjs/a.pdf", "header": true, "hayro_pages": 5, "pdfium_pages": null}),
            row
        );
        fs::write(&path, text).unwrap();
        let partials = read_rows(&[path]);
        fs::remove_dir_all(&dir).unwrap();
        let partial = &partials["pdfjs/a.pdf"];
        assert_eq!((partial.pages, partial.rows.len()), (Some(5), 1));
    }

    #[test]
    fn sampling_should_not_depend_on_source_order() {
        let a = Sampling::new(
            Some(20),
            None,
            &["qpdf".to_owned(), "pdfjs".to_owned(), "qpdf".to_owned()],
        );
        let b = Sampling::new(Some(20), None, &["pdfjs".to_owned(), "qpdf".to_owned()]);
        assert_eq!(a.json(), b.json());
    }

    #[test]
    fn tally_should_rate_pass_and_minor_over_every_row() {
        let rows = [
            row("pdfjs/a", "pass", None),
            row("pdfjs/b", "minor", None),
            row("pdfjs/c", "fail", None),
            row("qpdf/d", "both-error", None),
        ];
        let t = tally(&rows);
        let rates: Vec<u64> = ["all", "pdfjs", "qpdf"].iter().map(|s| t[*s].rate_bp()).collect();
        assert_eq!(rates, [5000, 6667, 0]);
    }

    #[test]
    fn worst_should_put_size_mismatches_and_big_diffs_first() {
        let rows = [
            row("a", "minor", Some(1.5)),
            row("b", "fail", Some(3.0)),
            row("c", "fail", None),
            row("d", "fail", Some(9.0)),
        ];
        let names: Vec<&str> = worst(&rows, 3).iter().map(|r| r.file.as_str()).collect();
        assert_eq!(names, ["c", "d", "b"]);
    }

    fn sampling() -> Sampling {
        Sampling {
            dpi: 96,
            pages: Some(20),
            sample: None,
            sources: Vec::new(),
        }
    }

    fn record(runner: &str, all: u64) -> Value {
        json!({"kind": KEY, "runner": runner, "sampling": sampling().json(), "commit": "abc",
               "benches": {"render-diff/all": all}})
    }

    #[test]
    fn regressions_should_flag_a_drop_over_two_points() {
        let now = BTreeMap::from([("render-diff/all".to_owned(), 8799)]);
        let (_, drops) = regressions(&[record("r", 9000)], "r", &sampling(), &now);
        assert_eq!(
            drops,
            [Drop {
                key: "render-diff/all".to_owned(),
                before: 9000,
                now: 8799
            }]
        );
    }

    #[test]
    fn regressions_should_tolerate_exactly_two_points() {
        let now = BTreeMap::from([("render-diff/all".to_owned(), 8800)]);
        assert_eq!(regressions(&[record("r", 9000)], "r", &sampling(), &now).1, []);
    }

    #[test]
    fn regressions_should_compare_with_the_last_point_only() {
        let now = BTreeMap::from([("render-diff/all".to_owned(), 8800)]);
        let history = [record("r", 9500), record("r", 8900)];
        assert_eq!(regressions(&history, "r", &sampling(), &now).1, []);
    }

    #[test]
    fn regressions_should_skip_other_runners_and_samplings() {
        let mut other = record("r", 9900);
        other["sampling"]["sample"] = json!(10);
        let history = [record("elsewhere", 9900), other];
        let now = BTreeMap::from([("render-diff/all".to_owned(), 5000)]);
        assert_eq!(regressions(&history, "r", &sampling(), &now), (None, Vec::new()));
    }

    #[test]
    fn regressions_should_ignore_benchmark_records() {
        let bench = json!({"runner": "r", "benches": {"render-diff/all": 9900}});
        let now = BTreeMap::from([("render-diff/all".to_owned(), 5000)]);
        assert!(regressions(&[bench], "r", &sampling(), &now).0.is_none());
    }

    #[test]
    fn issue_body_should_list_each_drop() {
        let drops = [Drop {
            key: "render-diff/pdfjs".to_owned(),
            before: 9100,
            now: 8650,
        }];
        let body = issue_body(&drops, &record("r", 9100), &sampling(), &[]);
        assert!(body.contains("| `render-diff/pdfjs` | 91.00 % | 86.50 % | −4.50 points |"));
    }

    #[test]
    fn html_should_escape_file_names() {
        let rows = [row("pdfjs/<b>.pdf", "fail", Some(3.0))];
        let tallies = tally(&rows);
        let page = html(&tallies, &worst(&rows, 1), &[], "ctx");
        assert!(page.contains("pdfjs/&lt;b&gt;.pdf") && !page.contains("<b>.pdf"));
    }

    #[test]
    fn sources_table_should_have_a_row_per_source_and_all() {
        let rows = [row("pdfjs/a", "pass", None), row("qpdf/b", "fail", None)];
        let table = sources_table(&tally(&rows));
        let sources: Vec<&str> = table.lines().skip(2).filter_map(|l| l.split(" | ").next()).collect();
        assert_eq!(sources, ["| all", "| pdfjs", "| qpdf"]);
    }

    #[test]
    fn parse_args_should_read_the_sampling_and_history() {
        let args: Vec<String> = [
            "--corpus",
            "external",
            "--pages",
            "20",
            "--sample",
            "50",
            "--history",
            "h",
            "--record",
        ]
        .map(String::from)
        .to_vec();
        let parsed = parse_args(&args).unwrap();
        assert!(parsed.pages == Some(20) && parsed.sample == Some(50) && parsed.record);
    }

    #[test]
    fn parse_args_should_require_history_for_record() {
        assert!(parse_args(&["--record".to_owned()]).is_err());
    }

    #[test]
    fn parse_args_should_reject_another_corpus() {
        assert!(parse_args(&["--corpus".to_owned(), "mutants".to_owned()]).is_err());
    }
}
