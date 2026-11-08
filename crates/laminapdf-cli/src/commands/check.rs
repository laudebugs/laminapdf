//! `laminapdf check`: reads a whole document (every object, every page and every content
//! stream, within the `Limits`) and reports what a tolerant read had to repair and what a
//! failed redaction left in the file.

use laminapdf::Diagnostic;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Settings, open, open_options, schema_id, warnings, warnings_schema};
use crate::exit::CliError;
use crate::io::Io;

/// `laminapdf check <input>`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckArgs {
    /// The PDF to check, or `-` for standard input.
    // The command line takes several inputs, so the front end fills this in per input.
    #[arg(skip)]
    pub input: String,
}

/// The outcome of a check that read the whole document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    /// Nothing needed repair.
    Ok,
    /// The document reads, but only after repairs, or it holds what a failed redaction left
    /// behind (covered text, unapplied redaction marks, text kept in an earlier revision);
    /// `warnings` lists them.
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
    /// `ok`, or `warnings` when a read had to repair something or found what a failed
    /// redaction left behind.
    pub status: CheckStatus,
    /// The page count.
    pub pages: u32,
    /// The objects read.
    pub objects: u32,
    /// The repairs, then what failed redactions left (ADR 0026 warnings).
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
/// through extraction's page pass, which interprets them and their fonts, then looks for what a
/// failed redaction leaves behind (`covered-text`, `redaction-not-applied`,
/// `text-in-earlier-revision`). Those findings aren't repairs, so `--strict` leaves them
/// warnings.
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
    let found = doc.redaction_warnings(&open_options(settings), &settings.cancel)?;
    settings.stop_if_cancelled()?;
    let mut warnings = warnings(doc);
    warnings.extend(found.iter().map(laminapdf::Warning::diagnostic));
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
