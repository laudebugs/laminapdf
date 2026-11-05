//! `laminapdf extract`: a document's text, page by page, or its blocks in reading order as
//! Markdown, chunks or `laminapdf.extract/1` JSON ([ADR 0030]).
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use clap::ValueEnum;
use laminapdf::extract::{ChunkOptions, ExtractOptions, Extraction, MarkdownOptions, Rendered, TextOptions};
use laminapdf::{Diagnostic, ErrorCode};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Settings, open, pages_of, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::Io;

/// What `extract` writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    /// Plain text: each page's lines, pages separated by a form feed.
    #[default]
    Text,
    /// Markdown: headings, list items, paragraphs and tables in reading order, hidden text left out.
    Md,
    /// The `laminapdf.extract/1` document: every block with its box, kind and spans, and what was dropped.
    Json,
    /// The Markdown cut into chunks for retrieval, one JSON object per line.
    Chunks,
}

/// `laminapdf extract <input>`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractArgs {
    /// The PDF to read, or `-` for standard input. The command line takes several (the front end's
    /// batch), so clap fills this in per input.
    #[arg(skip)]
    pub input: String,
    /// The output format.
    #[arg(long, value_enum, default_value_t)]
    #[serde(default)]
    pub format: Format,
    /// The pages to extract, as a range (`1-3`, `z`, `r2-z`, `1-z,x2`); every page without it.
    #[arg(long, value_name = "RANGE")]
    #[serde(default)]
    pub pages: Option<String>,
    /// With `md`, `json` or `chunks`: keep whole blocks up to this many characters of Markdown,
    /// and say so (`"truncated": true` in JSON).
    #[arg(long, value_name = "N")]
    #[serde(default)]
    pub max_chars: Option<usize>,
    /// With `text`: leave out text drawn invisibly (text rendering modes 3 and 7). The other
    /// formats always leave hidden text out.
    #[arg(long)]
    #[serde(default)]
    pub visible_only: bool,
}

/// From this many pages on, `extract` reads every page's text in one page-tree walk rather
/// than one walk per page.
const ONE_WALK_FROM: usize = 16;

/// `laminapdf.text/1`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextReport {
    /// `laminapdf.text/1`.
    #[schemars(schema_with = "text_id")]
    pub schema: &'static str,
    /// The input as given: a path, or `-`.
    pub input: String,
    /// The pages, in the order the range named them.
    pub pages: Vec<PageTextEntry>,
    /// The warnings reading the document raised (ADR 0026 objects).
    #[schemars(schema_with = "warnings_schema")]
    pub warnings: Vec<Diagnostic>,
}

/// One page's text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageTextEntry {
    /// The 0-based page index.
    pub index: u32,
    /// The 1-based page number.
    pub number: u32,
    /// The page's lines, joined by line feeds: document text.
    pub text: String,
}

impl TextReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.text/1";
}

fn text_id(_: &mut SchemaGenerator) -> Schema {
    schema_id(TextReport::SCHEMA)
}

/// Extracts the text of `args.pages` (every page by default) from `args.input`.
///
/// # Errors
///
/// `invalid-page-range`; the input couldn't be read or opened; a page's text couldn't be read.
pub fn extract(args: &ExtractArgs, settings: &Settings, io: &mut dyn Io) -> Result<TextReport, CliError> {
    if args.format != Format::Text {
        return Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "this reads the text format only: the others are an extracted document",
        ));
    }
    let opened = open(&args.input, settings, io)?;
    let doc = &opened.doc;
    let numbers = pages_of(args.pages.as_deref(), doc.page_count()?)?;
    let options = TextOptions::default().with_skip_invisible(args.visible_only);
    // `page_text` walks the page tree each time, so many pages are read in one walk instead.
    let all = if numbers.len() >= ONE_WALK_FROM {
        settings.stop_if_cancelled()?;
        Some(doc.page_texts(&options)?)
    } else {
        None
    };
    let mut pages = Vec::with_capacity(numbers.len());
    for number in numbers {
        settings.stop_if_cancelled()?;
        let index = number.saturating_sub(1);
        let text = match all.as_ref().and_then(|all| all.get(usize::try_from(index).ok()?)) {
            Some(text) => text.lines(),
            None => doc.page_text(index, &options)?.lines(),
        };
        pages.push(PageTextEntry {
            index,
            number,
            text: text.join("\n"),
        });
    }
    Ok(TextReport {
        schema: TextReport::SCHEMA,
        input: args.input.clone(),
        pages,
        warnings: warnings(doc),
    })
}

/// What `extract --format md|json|chunks` gives: the `laminapdf.extract/1` document (with the
/// `input` and, past `--max-chars`, `"truncated": true`), and the output in the format asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedReport {
    /// The `laminapdf.extract/1` document: the report `--json` prints.
    pub json: Value,
    /// The Markdown, the JSON or the chunks as JSON Lines: what the command prints.
    pub output: String,
    /// The warnings reading the document raised (ADR 0026 objects).
    pub warnings: Vec<Diagnostic>,
}

impl ExtractedReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.extract/1";

    /// The `$id` of `laminapdf.extract/1`'s schema (`crates/laminapdf-extract/schemas/`).
    pub const SCHEMA_ID: &'static str = "https://docs.laminapdf.com/schemas/extract-1.json";
}

impl Serialize for ExtractedReport {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.json.serialize(serializer)
    }
}

impl JsonSchema for ExtractedReport {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ExtractedReport".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "allOf": [{ "$ref": Self::SCHEMA_ID }],
            "type": "object",
            "properties": {
                "input": { "type": "string", "description": "The input as given: a path, or -." },
                "truncated": { "type": "boolean", "description": "Blocks past --max-chars were left out." }
            }
        })
    }
}

/// The blocks of `extraction` that fit in `max_chars` characters of Markdown, whole; whether
/// any was left out.
fn truncate(extraction: &mut Extraction, markdown: &mut Rendered, options: &MarkdownOptions, max_chars: usize) -> bool {
    let keep = markdown.ranges.iter().take_while(|r| r.end <= max_chars).count();
    if keep >= extraction.blocks.len() {
        return false;
    }
    extraction.blocks.truncate(keep);
    *markdown = extraction.to_markdown(options);
    true
}

/// Extracts `args.pages` (every page by default) of `args.input` as `args.format` (`md`,
/// `json` or `chunks`).
///
/// # Errors
///
/// `invalid-command-line` for the `text` format ([`extract`] reads it); `invalid-page-range`;
/// the input couldn't be read or opened; extraction failed or was cancelled.
pub fn extract_document(args: &ExtractArgs, settings: &Settings, io: &mut dyn Io) -> Result<ExtractedReport, CliError> {
    if args.format == Format::Text {
        return Err(CliError::new(
            ErrorCode::InvalidCommandLine,
            "the text format is read page by page, not as an extracted document",
        ));
    }
    let opened = open(&args.input, settings, io)?;
    let doc = &opened.doc;
    let numbers = pages_of(args.pages.as_deref(), doc.page_count()?)?;
    let indices = numbers.iter().filter_map(|n| usize::try_from(n.saturating_sub(1)).ok()).collect();
    let mut extraction = doc.extract_with(&ExtractOptions::new().with_pages(indices), &settings.cancel)?;
    let markdown_options = MarkdownOptions::new().with_page_markers(numbers.len() > 1);
    let mut markdown = extraction.to_markdown(&markdown_options);
    let truncated = args
        .max_chars
        .is_some_and(|max| truncate(&mut extraction, &mut markdown, &markdown_options, max));
    let chunks = extraction.chunks(&markdown, &ChunkOptions::new());
    let output = match args.format {
        Format::Md => markdown.text.clone(),
        Format::Chunks => chunks
            .iter()
            .filter_map(|c| serde_json::to_string(c).ok())
            .map(|line| format!("{line}\n"))
            .collect(),
        Format::Json | Format::Text => String::new(),
    };
    let with_chunks = (args.format == Format::Chunks).then_some(chunks);
    let text = doc.extract_json(&extraction, &markdown, with_chunks);
    let mut json: Value = serde_json::from_str(&text).unwrap_or_default();
    if let Some(map) = json.as_object_mut() {
        map.insert("input".to_owned(), args.input.clone().into());
        if truncated {
            map.insert("truncated".to_owned(), true.into());
        }
    }
    let output = if args.format == Format::Json { format!("{json}\n") } else { output };
    Ok(ExtractedReport {
        json,
        output,
        warnings: warnings(doc),
    })
}
