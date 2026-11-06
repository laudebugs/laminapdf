//! The commands ([ADR 0031] §1): one function each, taking its arguments, the [`Settings`] and
//! an [`Io`], and returning a report or a [`CliError`]. Commands never print; the front end
//! (`main.rs`) renders their reports, and `laminapdf mcp` calls the same functions.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

mod check;
mod explain;
mod extract;
mod info;
mod pages;
mod render;

pub use check::{CheckArgs, CheckReport, check};
pub use explain::{ExplainArgs, ExplainReport, explain};
pub use extract::{ExtractArgs, ExtractedReport, Format, PageTextEntry, TextReport, extract, extract_document};
pub use info::{InfoArgs, InfoMetadata, InfoReport, PageSummary, info};
pub use pages::{PagesReport, RotateArgs, RotatedPage, rotate};
pub use render::{Backend, DEFAULT_DPI, RenderArgs, RenderReport, RenderedPage, check_outputs, render};

use laminapdf::{CancelToken, Diagnostic, Document, ErrorCode, Limits, OpenOptions};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Deserialize;

use crate::exit::CliError;
use crate::io::Io;
use crate::ranges::{PageRange, RangeError};

/// The `$id` of `laminapdf.error/1`, whose `$defs.warning` every report's warnings follow.
pub const ERROR_SCHEMA_ID: &str =
    "https://github.com/laudebugs/laminapdf/blob/main/packages/sdk/schemas/error.schema.json";

/// Declares [`OpenArgs`] with strict mode and one flag per `Limits` value named in
/// `fixtures/codes.json` (`limits`), and [`OpenArgs::LIMIT_NAMES`] listing them.
macro_rules! open_args {
    ($($field:ident: $ty:ty => $with:ident),* $(,)?) => {
        /// How documents are opened: strict mode and the `Limits` ([ADR 0003] §3), one flag per
        /// limit, named after it.
        ///
        /// [ADR 0003]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
        #[derive(clap::Args, Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct OpenArgs {
            /// Fail on the first problem a tolerant read would repair, instead of warning.
            #[arg(long, global = true)]
            #[serde(default)]
            pub strict: bool,
            $(
                #[doc = concat!("Sets the `", stringify!($field), "` limit.")]
                #[arg(
                    long,
                    global = true,
                    value_name = "N",
                    hide_short_help = true,
                    help = concat!("Set the ", stringify!($field), " limit (see `laminapdf explain limit-exceeded`)")
                )]
                #[serde(default)]
                pub $field: Option<$ty>,
            )*
        }

        impl OpenArgs {
            /// The limits these flags name, as `fixtures/codes.json` lists them.
            pub const LIMIT_NAMES: &[&str] = &[$(stringify!($field)),*];

            /// The default limits with each flag given applied.
            #[must_use]
            pub fn limits(&self) -> Limits {
                let mut limits = Limits::default();
                $(
                    if let Some(value) = self.$field {
                        limits = limits.$with(value);
                    }
                )*
                limits
            }
        }
    };
}

open_args! {
    max_nesting_depth: usize => with_max_nesting_depth,
    max_stream_decoded: usize => with_max_stream_decoded,
    max_decoded_total: usize => with_max_decoded_total,
    max_filter_chain: usize => with_max_filter_chain,
    max_resolve_depth: usize => with_max_resolve_depth,
    max_page_tree_depth: usize => with_max_page_tree_depth,
    max_tree_depth: usize => with_max_tree_depth,
    max_string_length: usize => with_max_string_length,
    max_content_operators: u64 => with_max_content_operators,
    max_xobject_depth: usize => with_max_xobject_depth,
    max_image_pixels: u64 => with_max_image_pixels,
    max_read_window: usize => with_max_read_window,
}

/// What every command shares, read by the front end from the global flags, the environment
/// (`LAMINA_PDF_PASSWORD`) and the password file.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// Strict mode and the limits.
    pub open: OpenArgs,
    /// The password to open encrypted documents with; never from the command line.
    pub password: Option<String>,
    /// Replace existing files (`--force`).
    pub force: bool,
    /// Do everything but write (`--dry-run`).
    pub dry_run: bool,
    /// Stops a command at its next step: after reading its input and after opening it, between
    /// pages, and inside a save (ADR 0028). The MCP server cancels it when a call runs out of time.
    pub cancel: CancelToken,
}

impl Settings {
    /// `cancelled` once [`Settings::cancel`] has been cancelled.
    ///
    /// # Errors
    ///
    /// `cancelled`.
    pub fn stop_if_cancelled(&self) -> Result<(), CliError> {
        if self.cancel.is_cancelled() {
            return Err(CliError::new(ErrorCode::Cancelled, "the command was cancelled"));
        }
        Ok(())
    }
}

/// An opened input: the document and its size in bytes.
struct Opened {
    doc: Document,
    bytes: u64,
}

/// Reads `input` through `io` and opens it with `settings`.
fn open(input: &str, settings: &Settings, io: &mut dyn Io) -> Result<Opened, CliError> {
    let data = io.read(input)?;
    settings.stop_if_cancelled()?;
    let bytes = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut options = OpenOptions::default()
        .with_strict(settings.open.strict)
        .with_limits(settings.open.limits());
    if let Some(password) = &settings.password {
        options = options.with_password(password.as_bytes());
    }
    let doc = Document::open_bytes(data, options)?;
    settings.stop_if_cancelled()?;
    Ok(Opened { doc, bytes })
}

/// Every warning `doc` has raised so far, as ADR 0026 objects.
fn warnings(doc: &Document) -> Vec<Diagnostic> {
    let mut cursor = doc.warnings();
    let mut all = Vec::new();
    loop {
        let batch = cursor.next_batch(256);
        if batch.is_empty() {
            return all;
        }
        all.extend(batch.iter().map(laminapdf::Warning::diagnostic));
    }
}

/// The page numbers (from 1) `range` selects in a document of `page_count` pages; every page
/// without one.
fn pages_of(range: Option<&str>, page_count: u32) -> Result<Vec<u32>, CliError> {
    let Some(text) = range else {
        return Ok((1..=page_count).collect());
    };
    PageRange::parse(text)
        .and_then(|range| range.resolve(page_count))
        .map_err(|e| range_error(text, &e))
}

/// `invalid-page-range` for `error` in `text`; the text and the 1-based position go in `data`,
/// since a message never quotes the command line.
fn range_error(text: &str, error: &RangeError) -> CliError {
    CliError::new(ErrorCode::InvalidPageRange, error.message.clone())
        .with("range", text)
        .with("position", error.position.saturating_add(1))
}

/// The schema of a report's `warnings`: ADR 0026 warnings, by reference to
/// `laminapdf.error/1`.
fn warnings_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "array",
        "items": { "$ref": format!("{ERROR_SCHEMA_ID}#/$defs/warning") }
    })
}

/// The schema of a report's `schema` field: the one id it always has.
fn schema_id(id: &str) -> Schema {
    json_schema!({ "type": "string", "const": id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancelled_command_should_stop_at_its_next_step() {
        let settings = Settings::default();
        settings.cancel.cancel();
        let mut io = crate::io::memory::MemoryIo::default();
        io.files.insert(
            "in.pdf".to_owned(),
            laminapdf_test_fixtures::fixture("corpus/hello.pdf"),
        );
        let input = "in.pdf".to_owned();
        let codes = [
            check(&CheckArgs { input: input.clone() }, &settings, &mut io).map(|_| ()),
            extract(
                &ExtractArgs {
                    input: input.clone(),
                    format: Format::Text,
                    pages: None,
                    max_chars: None,
                    visible_only: false,
                },
                &settings,
                &mut io,
            )
            .map(|_| ()),
            rotate(
                &RotateArgs {
                    input,
                    rotation: "90".to_owned(),
                    output: Some("out.pdf".to_owned()),
                    in_place: false,
                },
                &settings,
                &mut io,
            )
            .map(|_| ()),
        ]
        .map(|result| result.map_or_else(|e| e.diagnostic().code.clone(), |()| "ok".to_owned()));
        assert_eq!(codes, ["cancelled", "cancelled", "cancelled"]);
        assert!(!io.files.contains_key("out.pdf"));
    }

    #[test]
    fn the_limit_flags_should_be_the_limits_codes_json_lists() {
        let codes = laminapdf_test_fixtures::codes::codes_json();
        let listed: Vec<&str> = codes["limits"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(OpenArgs::LIMIT_NAMES, listed.as_slice());
    }

    #[test]
    fn a_limit_flag_should_change_only_its_limit() {
        let args = OpenArgs {
            max_stream_decoded: Some(1024),
            ..OpenArgs::default()
        };
        assert_eq!(args.limits(), Limits::default().with_max_stream_decoded(1024));
    }
}
