//! `laminapdf extract`: a document's text, page by page.

use clap::ValueEnum;
use laminapdf::Diagnostic;
use laminapdf::extract::TextOptions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Settings, open, pages_of, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::Io;

/// What `extract` writes. Only `text` exists so far; the flag is part of the grammar now so
/// that scripts never change when more formats come.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    /// Plain text: each page's lines, pages separated by a form feed.
    #[default]
    Text,
}

/// `laminapdf extract <input>`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractArgs {
    /// The PDF to read, or `-` for standard input.
    pub input: String,
    /// The output format.
    #[arg(long, value_enum, default_value_t)]
    #[serde(default)]
    pub format: Format,
    /// The pages to extract, as a range (`1-3`, `z`, `r2-z`, `1-z,x2`); every page without it.
    #[arg(long, value_name = "RANGE")]
    #[serde(default)]
    pub pages: Option<String>,
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

fn text_id(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schema_id(TextReport::SCHEMA)
}

/// Extracts the text of `args.pages` (every page by default) from `args.input`.
///
/// # Errors
///
/// `invalid-page-range`; the input couldn't be read or opened; a page's text couldn't be read.
pub fn extract(args: &ExtractArgs, settings: &Settings, io: &mut dyn Io) -> Result<TextReport, CliError> {
    let opened = open(&args.input, settings, io)?;
    let doc = &opened.doc;
    let numbers = pages_of(args.pages.as_deref(), doc.page_count()?)?;
    let options = TextOptions::default();
    // `page_text` walks the page tree each time, so many pages are read in one walk instead.
    let all = if numbers.len() > ONE_WALK_FROM {
        Some(doc.page_texts(&options)?)
    } else {
        None
    };
    let mut pages = Vec::with_capacity(numbers.len());
    for number in numbers {
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
