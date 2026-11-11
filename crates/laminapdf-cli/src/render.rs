//! What the front end prints ([ADR 0031] §6–§8): reports as text or JSON on stdout, and
//! diagnostics on stderr, rustc style or as JSON lines.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use std::fmt::Write as _;
use std::io::Write as _;

use anstream::{AutoStream, ColorChoice};
use laminapdf::Diagnostic;
use laminapdf_cli::commands::{
    BuildReport, CheckReport, ExplainReport, InfoReport, PageSummary, PagesReport, RenderReport, TextReport,
};
use laminapdf_cli::exit::{CliError, ExitCode};
use serde_json::Value;

const RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[1;33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// The two output streams, coloured per `--color` and the terminal.
///
/// Standard output carries data, so it is written as is: no colour to strip, nothing dropped
/// on the way to a pipe. Only standard error, which has the diagnostics, is coloured.
pub(crate) struct Streams {
    pub(crate) out: std::io::Stdout,
    pub(crate) err: AutoStream<std::io::Stderr>,
}

impl Streams {
    pub(crate) fn new(choice: ColorChoice) -> Self {
        Self {
            out: std::io::stdout(),
            err: AutoStream::new(std::io::stderr(), choice),
        }
    }
}

/// `text` safe for a terminal: every control character but the line feed, tab and form feed
/// (escape sequences, BEL, the C1 controls) becomes U+FFFD, so document text can't move the
/// cursor, retitle the window or plant a hyperlink.
pub(crate) fn for_terminal(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() && !matches!(c, '\n' | '\t' | '\u{c}') {
                '\u{fffd}'
            } else {
                c
            }
        })
        .collect()
}

/// Writes `error` to stderr: `error[code]: message` and its hint, docs URL and candidates, or
/// with `json` one `laminapdf.error/1` line carrying `input` and `exitCode` too.
pub(crate) fn error(streams: &mut Streams, error: &CliError, input: Option<&str>, json: bool) {
    let diagnostic = error.diagnostic();
    let text = if json {
        format!("{}\n", error.to_line(input))
    } else {
        let mut text = format!(
            "{RED}error{RESET}{BOLD}[{}]: {}{RESET}\n",
            diagnostic.code, diagnostic.message
        );
        // Where in a JSON document or its data the error is (invalid-document, template-data-*):
        // the user's own keys, so made safe for the terminal like any text read from a file.
        if let Some(pointer) = diagnostic.data.get("pointer").and_then(Value::as_str) {
            let _ = writeln!(text, "  = at: {}", for_terminal(pointer));
        }
        if !diagnostic.did_you_mean.is_empty() {
            let _ = writeln!(text, "  = did you mean: {}", diagnostic.did_you_mean.join(", "));
        }
        if let Some(hint) = &diagnostic.hint {
            let _ = writeln!(text, "  = hint: {hint}");
        }
        let _ = writeln!(text, "  = see: {}", diagnostic.docs_url);
        text
    };
    let _ = streams.err.write_all(text.as_bytes());
}

/// Writes `warnings` to stderr, `warning[code]: message` each. Under `--json` the report lists
/// them instead.
pub(crate) fn warnings(streams: &mut Streams, warnings: &[Diagnostic]) {
    for warning in warnings {
        let line = format!(
            "{YELLOW}warning{RESET}{BOLD}[{}]: {}{RESET}\n",
            warning.code, warning.message
        );
        let _ = streams.err.write_all(line.as_bytes());
    }
}

/// `value` with only `schema`, `input` and the `fields` named; every top-level key when
/// `fields` is empty.
///
/// # Errors
///
/// `invalid-command-line` for a name the report doesn't have, with the close ones.
pub(crate) fn keep_fields(value: Value, fields: &[String]) -> Result<Value, CliError> {
    let Value::Object(map) = value else {
        return Ok(value);
    };
    if fields.is_empty() {
        return Ok(Value::Object(map));
    }
    let known: Vec<String> = map.keys().cloned().collect();
    if let Some(unknown) = fields.iter().find(|f| !map.contains_key(f.as_str())) {
        return Err(CliError::new(
            laminapdf::ErrorCode::InvalidCommandLine,
            "--fields names a key the report doesn't have",
        )
        .with("argument", unknown.as_str())
        .with_did_you_mean(laminapdf::did_you_mean(unknown, &known)));
    }
    Ok(Value::Object(
        map.into_iter()
            .filter(|(key, _)| key == "schema" || key == "input" || fields.contains(key))
            .collect(),
    ))
}

/// `1 page`, `2 pages`.
pub(crate) fn count(n: impl Into<u64>, noun: &str) -> String {
    let n = n.into();
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

/// A byte count as people read it: `1.8 MB`, `412 KB`, `900 B`.
pub(crate) fn size(bytes: u64) -> String {
    #[expect(clippy::cast_precision_loss, reason = "a size shown to one decimal")]
    let value = bytes as f64;
    match bytes {
        0..1_000 => format!("{bytes} B"),
        1_000..1_000_000 => format!("{:.0} KB", value / 1e3),
        1_000_000..1_000_000_000 => format!("{:.1} MB", value / 1e6),
        _ => format!("{:.1} GB", value / 1e9),
    }
}

/// A point length without a needless `.0`.
fn points(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded}")
    }
}

fn page_size(page: &PageSummary) -> String {
    let turned = if page.rotation == 0 {
        String::new()
    } else {
        format!(", rotated {}°", page.rotation)
    };
    format!("{} × {} pt{turned}", points(page.width), points(page.height))
}

/// `info` as at most about 20 lines.
pub(crate) fn info_text(report: &InfoReport) -> String {
    let mut parts = Vec::new();
    if let Some(version) = &report.version {
        parts.push(format!("PDF {version}"));
    }
    parts.push(count(report.pages, "page"));
    parts.push(size(report.bytes));
    parts.push((if report.encrypted { "encrypted" } else { "not encrypted" }).to_owned());
    parts.push((if report.tagged { "tagged" } else { "not tagged" }).to_owned());
    let mut out = format!("{}  {}\n", report.input, parts.join(" · "));
    let m = &report.metadata;
    for (name, value) in [
        ("Title", &m.title),
        ("Author", &m.author),
        ("Subject", &m.subject),
        ("Keywords", &m.keywords),
        ("Creator", &m.creator),
        ("Producer", &m.producer),
    ] {
        if let Some(value) = value {
            let _ = writeln!(out, "{name}: {}", one_line(value));
        }
    }
    let sizes: Vec<String> = report.page_sizes.iter().map(page_size).collect();
    match sizes.first() {
        Some(first) if sizes.iter().all(|s| s == first) => {
            let _ = writeln!(out, "Pages: {first}");
        }
        _ => {
            for (page, size) in report.page_sizes.iter().zip(&sizes).take(8) {
                let _ = writeln!(out, "Page {}: {size}", page.number);
            }
            if sizes.len() > 8 {
                let _ = writeln!(out, "… and {} more pages", sizes.len() - 8);
            }
        }
    }
    out
}

/// Document text on one line, at most 120 characters, without control characters.
pub(crate) fn one_line(text: &str) -> String {
    let clean: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let mut chars = clean.chars();
    let short: String = chars.by_ref().take(120).collect();
    if chars.next().is_some() {
        format!("{short}…")
    } else {
        short
    }
}

pub(crate) fn check_text(report: &CheckReport) -> String {
    let status = match report.warnings.len() {
        0 => "ok".to_owned(),
        1 => "1 warning".to_owned(),
        n => format!("{n} warnings"),
    };
    format!(
        "{}: {status} · {} · {}\n",
        report.input,
        count(report.pages, "page"),
        count(report.objects, "object")
    )
}

/// The pages' text, separated by form feeds as `pdftotext` does.
pub(crate) fn extract_text(report: &TextReport) -> String {
    let pages: Vec<&str> = report.pages.iter().map(|p| p.text.as_str()).collect();
    let mut text = pages.join("\n\u{c}");
    text.push('\n');
    text
}

pub(crate) fn render_text(report: &RenderReport) -> String {
    let mut text = String::new();
    for page in &report.pages {
        let _ = writeln!(
            text,
            "page {}: {} × {} px → {}",
            page.number, page.width, page.height, page.output
        );
    }
    text
}

pub(crate) fn pages_text(report: &PagesReport) -> String {
    format!(
        "rotated {} → {} ({})\n",
        count(u64::try_from(report.pages.len()).unwrap_or(u64::MAX), "page"),
        report.output,
        if report.save_mode == "incremental" {
            "incremental update"
        } else {
            "full rewrite"
        }
    )
}

pub(crate) fn build_text(report: &BuildReport) -> String {
    let profiles = if report.profiles.is_empty() {
        String::new()
    } else {
        format!(", {}", report.profiles.join(", "))
    };
    format!(
        "built {} → {} ({}{profiles})\n",
        count(report.page_count, "page"),
        report.output,
        size(report.bytes)
    )
}

pub(crate) fn explain_text(report: &ExplainReport) -> String {
    let mut page = report.page.clone();
    if !page.ends_with('\n') {
        page.push('\n');
    }
    page
}

/// `laminapdf help exit-codes`.
pub(crate) fn exit_codes_text() -> String {
    let order: Vec<String> = ExitCode::by_priority()
        .iter()
        .map(|code| code.number().to_string())
        .collect();
    let mut out = format!(
        "Exit codes (when several apply, the first in {} wins):\n",
        order.join(", ")
    );
    for code in ExitCode::ALL {
        let _ = writeln!(out, "  {}  {}", code.number(), code.meaning());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exit_codes_should_list_every_code_in_the_order_they_win() {
        assert!(
            exit_codes_text().starts_with("Exit codes (when several apply, the first in 2, 5, 4, 1, 6, 3, 0 wins):")
        );
    }

    #[test]
    fn sizes_should_read_like_a_file_manager() {
        assert_eq!([size(900), size(41_200), size(1_800_000)], ["900 B", "41 KB", "1.8 MB"]);
    }

    #[test]
    fn a_terminal_should_get_no_control_characters_but_line_breaks() {
        assert_eq!(
            for_terminal("a\u{1b}]0;x\u{7}b\n\tc\u{c}\u{9b}"),
            "a\u{fffd}]0;x\u{fffd}b\n\tc\u{c}\u{fffd}"
        );
    }

    #[test]
    fn document_text_should_stay_on_one_line() {
        assert_eq!(one_line("a\nb\u{1b}[31m"), "a b [31m");
        assert_eq!(one_line(&"x".repeat(130)).chars().count(), 121);
    }

    #[test]
    fn fields_should_keep_schema_input_and_the_names() {
        let report = json!({ "schema": "s", "input": "i", "pages": 1, "bytes": 2 });
        assert_eq!(
            keep_fields(report.clone(), &["pages".to_owned()]).unwrap(),
            json!({ "schema": "s", "input": "i", "pages": 1 })
        );
        let error = keep_fields(report, &["page".to_owned()]).unwrap_err();
        assert_eq!(error.diagnostic().did_you_mean, ["pages"]);
    }
}
