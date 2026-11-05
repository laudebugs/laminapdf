//! `laminapdf info`: what a document is (its version, pages, size, encryption, tagging and
//! Info dictionary) and the warnings reading it raised.

use laminapdf::Diagnostic;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Settings, open, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::Io;

/// `laminapdf info <input>`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InfoArgs {
    /// The PDF to read, or `-` for standard input. The command line takes several (the front end's
    /// batch), so clap fills this in per input.
    #[arg(skip)]
    pub input: String,
    /// Keep only these top-level keys of the JSON report (`schema` and `input` always stay).
    #[arg(long, value_delimiter = ',', value_name = "KEYS")]
    #[serde(default)]
    pub fields: Vec<String>,
}

/// `laminapdf.info/1`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InfoReport {
    /// `laminapdf.info/1`.
    #[schemars(schema_with = "info_id")]
    pub schema: &'static str,
    /// The input as given: a path, or `-`.
    pub input: String,
    /// The PDF version the file declares (`1.7`), when it declares one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The page count.
    pub pages: u32,
    /// The file's size in bytes.
    pub bytes: u64,
    /// Whether the file is encrypted.
    pub encrypted: bool,
    /// Whether the document says it is tagged (`/MarkInfo /Marked true`).
    pub tagged: bool,
    /// Each page's displayed size ([ADR 0027]: the `CropBox` with `/Rotate` and `/UserUnit`).
    ///
    /// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
    pub page_sizes: Vec<PageSummary>,
    /// The Info dictionary's text entries: document text.
    pub metadata: InfoMetadata,
    /// The warnings reading the document raised (ADR 0026 objects).
    #[schemars(schema_with = "warnings_schema")]
    pub warnings: Vec<Diagnostic>,
}

/// One page's size.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageSummary {
    /// The 0-based page index.
    pub index: u32,
    /// The 1-based page number.
    pub number: u32,
    /// Displayed width in points.
    pub width: f64,
    /// Displayed height in points.
    pub height: f64,
    /// `/Rotate`: 0, 90, 180 or 270.
    pub rotation: u16,
    /// The page label, when the document defines one: document text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// The Info dictionary's text entries; each is document text, absent when the file has none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InfoMetadata {
    /// `/Title`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// `/Author`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// `/Subject`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// `/Keywords`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// `/Creator`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    /// `/Producer`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
}

fn info_id(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schema_id(InfoReport::SCHEMA)
}

impl InfoReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.info/1";
}

/// Reads `args.input` and reports what it is.
///
/// # Errors
///
/// The input couldn't be read or opened (`input-unreadable`, `password-required`, …).
pub fn info(args: &InfoArgs, settings: &Settings, io: &mut dyn Io) -> Result<InfoReport, CliError> {
    let opened = open(&args.input, settings, io)?;
    let doc = &opened.doc;
    let page_sizes = doc
        .page_infos()?
        .into_iter()
        .map(|page| PageSummary {
            index: page.index,
            number: page.index.saturating_add(1),
            width: page.width,
            height: page.height,
            rotation: page.rotation,
            label: page.label,
        })
        .collect::<Vec<_>>();
    let info = doc.info()?;
    Ok(InfoReport {
        schema: InfoReport::SCHEMA,
        input: args.input.clone(),
        version: doc.version()?,
        pages: u32::try_from(page_sizes.len()).unwrap_or(u32::MAX),
        bytes: opened.bytes,
        encrypted: doc.is_encrypted(),
        tagged: doc.is_tagged()?,
        page_sizes,
        metadata: InfoMetadata {
            title: info.title,
            author: info.author,
            subject: info.subject,
            keywords: info.keywords,
            creator: info.creator,
            producer: info.producer,
        },
        warnings: warnings(doc),
    })
}
