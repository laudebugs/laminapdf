//! The rows of a run (one per page or file, by class), their tallies per source and the worst
//! pages.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::live;

/// The tracked classes, in report order. `io-error` (a file that couldn't be read, a diff image
/// that couldn't be written) is neither engine's doing: it is reported, never blamed, and left
/// out of the rates.
pub(super) const CLASSES: [&str; 8] = [
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
pub(super) struct Row {
    pub(super) file: String,
    /// 1-based; `None` for a whole-file row.
    pub(super) page: Option<u64>,
    pub(super) class: String,
    pub(super) diff_percent: Option<f64>,
    pub(super) ssim: Option<f64>,
    /// The diff image's file name (minor and fail pages).
    pub(super) image: Option<String>,
    /// What went wrong, for error and timeout rows.
    pub(super) detail: Option<String>,
}

impl Row {
    pub(super) fn file(file: &str, page: Option<u64>, class: &str, detail: String) -> Self {
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

    pub(super) fn json(&self) -> Value {
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
pub(super) struct Blame {
    pub(super) hayro: bool,
    pub(super) pdfium: bool,
}

/// What the `--rows` files say about one file: the most pages any reading of it reported
/// (lamina's, or an engine's once it opened the file), and the rows of the pages both engines
/// finished, by page (a later attempt's row replaces an earlier one's).
#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct Partial {
    pub(super) pages: Option<u64>,
    pub(super) rows: BTreeMap<u64, Value>,
}

/// Reads the readers' `--rows` files (JSON lines; a torn last line of a killed process is
/// skipped): page counts from every file's headers, page rows from the compared ones (they
/// carry a `class`; the single-engine reruns' don't).
pub(super) fn read_rows(paths: &[PathBuf]) -> BTreeMap<String, Partial> {
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
pub(super) fn rows_of(file: &str, fact: &Value, blame: Option<Blame>, partial: &Partial, cap: Option<u32>) -> Vec<Row> {
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
pub(super) fn page_rows(file: &str, pages: &[Value]) -> Vec<Row> {
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
pub(super) struct Tally {
    pub(super) rows: u64,
    pub(super) classes: BTreeMap<String, u64>,
}

impl Tally {
    pub(super) fn add(&mut self, class: &str) {
        self.rows += 1;
        *self.classes.entry(class.to_owned()).or_default() += 1;
    }

    pub(super) fn count(&self, class: &str) -> u64 {
        self.classes.get(class).copied().unwrap_or_default()
    }

    /// The rows the rates are over: all but `io-error`, which is neither engine's doing.
    pub(super) fn rated(&self) -> u64 {
        self.rows - self.count("io-error")
    }

    /// The tracked number: pass + minor over the rated rows, in basis points (10,000 = 100 %).
    pub(super) fn rate_bp(&self) -> u64 {
        share_bp(self.count("pass") + self.count("minor"), self.rated())
    }

    /// The pass share alone, in basis points.
    pub(super) fn pass_bp(&self) -> u64 {
        share_bp(self.count("pass"), self.rated())
    }

    pub(super) fn json(&self) -> Value {
        json!({
            "rows": self.rows,
            "classes": self.classes,
            "pass_minor_percent": percent(self.rate_bp()),
            "pass_percent": percent(self.pass_bp()),
        })
    }
}

/// `part / whole` in basis points, rounded; 0 for an empty whole.
pub(super) fn share_bp(part: u64, whole: u64) -> u64 {
    (part * 10_000 + whole / 2).checked_div(whole).unwrap_or(0)
}

#[expect(clippy::cast_precision_loss, reason = "basis points are at most 10,000")]
pub(super) fn percent(bp: u64) -> f64 {
    bp as f64 / 100.0
}

/// Per source (`pdfjs/…` → `pdfjs`), and `all`.
pub(super) fn tally(rows: &[Row]) -> BTreeMap<String, Tally> {
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
pub(super) fn worst(rows: &[Row], n: usize) -> Vec<&Row> {
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
