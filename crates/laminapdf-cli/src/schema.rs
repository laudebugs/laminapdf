//! The JSON Schemas of the CLI's output ([ADR 0031] §6, §8), generated from the report types:
//! one per report, and one for the error line on standard error under `--json`. They are committed
//! under `crates/laminapdf-cli/schemas/`; a test fails when a committed one is stale.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use schemars::{JsonSchema, schema_for};
use serde_json::{Value, json};

use crate::commands::{CheckReport, ERROR_SCHEMA_ID, ExplainReport, InfoReport, PagesReport, TextReport};

/// Where the committed schemas are published; a schema's `$id` is this plus its file name.
const BASE: &str = "https://github.com/laudebugs/laminapdf/blob/main/crates/laminapdf-cli/schemas/";

/// Every schema: its file name under `schemas/` and its JSON.
#[must_use]
pub fn all() -> Vec<(&'static str, Value)> {
    vec![
        ("info.schema.json", report::<InfoReport>(InfoReport::SCHEMA)),
        ("check.schema.json", report::<CheckReport>(CheckReport::SCHEMA)),
        ("pages.schema.json", report::<PagesReport>(PagesReport::SCHEMA)),
        ("text.schema.json", report::<TextReport>(TextReport::SCHEMA)),
        ("explain.schema.json", report::<ExplainReport>(ExplainReport::SCHEMA)),
        ("error-line.schema.json", error_line()),
    ]
}

/// A report's schema, with its `$id` and its schema id as the title.
fn report<T: JsonSchema>(id: &str) -> Value {
    let mut schema = schema_for!(T).to_value();
    if let Some(map) = schema.as_object_mut() {
        let file = id
            .trim_start_matches("laminapdf.")
            .split('/')
            .next()
            .unwrap_or_default();
        map.insert("$id".to_owned(), format!("{BASE}{file}.schema.json").into());
        map.insert("title".to_owned(), id.into());
    }
    schema
}

/// The line standard error holds under `--json` when a command fails: `laminapdf.error/1`'s
/// error object, which carries the input and the exit code too (ADR 0031 §8). Warnings never
/// get a line of their own: every report lists them.
fn error_line() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("{BASE}error-line.schema.json"),
        "title": "laminapdf.error/1 line",
        "description": "The line standard error holds under --json when a command fails (ADR 0031 §8).",
        "type": "object",
        "required": ["schema", "error"],
        "properties": {
            "schema": { "const": "laminapdf.error/1" },
            "error": {
                "allOf": [
                    { "$ref": format!("{ERROR_SCHEMA_ID}#/$defs/error") },
                    {
                        "type": "object",
                        "required": ["exitCode"],
                        "properties": {
                            "input": { "type": "string", "description": "The input as given: a path, or -." },
                            "exitCode": { "type": "integer", "minimum": 1, "maximum": 5 }
                        }
                    }
                ]
            }
        }
    })
}
