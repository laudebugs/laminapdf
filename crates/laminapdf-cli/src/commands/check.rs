//! `laminapdf check`: reads a whole document (every object, every page and every content
//! stream, within the `Limits`) and reports what a tolerant read had to repair.

use laminapdf::Diagnostic;
use laminapdf::extract::TextOptions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Settings, open, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::Io;

/// `laminapdf check <input>`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckArgs {
    /// The PDF to check, or `-` for standard input.
    pub input: String,
}

/// The outcome of a check that read the whole document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    /// Nothing needed repair.
    Ok,
    /// The document reads, but only after repairs; `warnings` lists them.
    Warnings,
}

/// `laminapdf.check/1`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    /// `laminapdf.check/1`.
    #[schemars(schema_with = "check_id")]
    pub schema: &'static str,
    /// The input as given: a path, or `-`.
    pub input: String,
    /// `ok`, or `warnings` when a read had to repair something.
    pub status: CheckStatus,
    /// The page count.
    pub pages: u32,
    /// The objects read.
    pub objects: u32,
    /// The repairs (ADR 0026 warnings).
    #[schemars(schema_with = "warnings_schema")]
    pub warnings: Vec<Diagnostic>,
}

impl CheckReport {
    /// The report's schema id.
    pub const SCHEMA: &'static str = "laminapdf.check/1";
}

fn check_id(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schema_id(CheckReport::SCHEMA)
}

/// Reads all of `args.input`: every object, every page, and every page's content streams
/// through text extraction, which interprets them and their fonts.
///
/// # Errors
///
/// The first problem a read can't get past; with `--strict`, the first one it would repair.
pub fn check(args: &CheckArgs, settings: &Settings, io: &mut dyn Io) -> Result<CheckReport, CliError> {
    let opened = open(&args.input, settings, io)?;
    let doc = &opened.doc;
    settings.stop_if_cancelled()?;
    let objects = doc.read_every_object()?;
    settings.stop_if_cancelled()?;
    let pages = doc.page_count()?;
    doc.page_texts(&TextOptions::default())?;
    settings.stop_if_cancelled()?;
    let warnings = warnings(doc);
    Ok(CheckReport {
        schema: CheckReport::SCHEMA,
        input: args.input.clone(),
        status: if warnings.is_empty() {
            CheckStatus::Ok
        } else {
            CheckStatus::Warnings
        },
        pages,
        objects,
        warnings,
    })
}
