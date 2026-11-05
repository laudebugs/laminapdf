//! The JSON Schemas of the CLI's output ([ADR 0031] §6, §8), generated from the report types:
//! one per report, and one for the error line on standard error under `--json`. They are committed
//! under `crates/laminapdf-cli/schemas/`; a test fails when a committed one is stale.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use schemars::{JsonSchema, schema_for};
use serde_json::{Value, json};

use crate::batch::{BatchSummary, Status};
use crate::commands::{
    CheckArgs, CheckReport, ERROR_SCHEMA_ID, ExplainArgs, ExplainReport, ExtractArgs, ExtractedReport, InfoArgs,
    InfoReport, PagesReport, RotateArgs, TextReport,
};

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
        ("result-line.schema.json", result_line()),
        ("summary-line.schema.json", summary_line()),
    ]
}

/// The commands `laminapdf schema` describes, by the name it takes.
pub const COMMANDS: [&str; 5] = ["check", "explain", "extract", "info", "pages-rotate"];

/// The committed schema `file`.
fn committed(file: &str) -> Value {
    all()
        .into_iter()
        .find(|(name, _)| *name == file)
        .map(|(_, schema)| schema)
        .unwrap_or_default()
}

/// The schema of a command's arguments: its flags and inputs as one object, the shape
/// `laminapdf mcp` takes as a tool's input too.
fn input<T: JsonSchema>(command: &str) -> Value {
    let mut schema = schema_for!(T).to_value();
    if let Some(map) = schema.as_object_mut() {
        map.insert("$id".to_owned(), format!("{BASE}{command}-input.schema.json").into());
        map.insert("title".to_owned(), format!("laminapdf {command} arguments").into());
    }
    schema
}

/// `laminapdf schema <command>`: one JSON Schema 2020-12 document whose properties are the
/// command's `input` (its arguments as an object), its `output` (the `--json` report, as
/// committed), its `error` line and, for the commands that take several inputs, the NDJSON
/// `result` and `summary` lines. Each is an embedded schema resource with its own `$id`.
/// `None` for an unknown command.
#[must_use]
pub fn command(name: &str) -> Option<Value> {
    let (input, output) = match name {
        "check" => (input::<CheckArgs>(name), committed("check.schema.json")),
        "explain" => (input::<ExplainArgs>(name), committed("explain.schema.json")),
        "extract" => (
            input::<ExtractArgs>(name),
            json!({ "oneOf": [committed("text.schema.json"), schema_for!(ExtractedReport).to_value()] }),
        ),
        "info" => (input::<InfoArgs>(name), committed("info.schema.json")),
        "pages-rotate" => (input::<RotateArgs>(name), committed("pages.schema.json")),
        _ => return None,
    };
    let mut properties = json!({ "input": input, "output": output, "error": committed("error-line.schema.json") });
    if matches!(name, "check" | "extract" | "info")
        && let Some(map) = properties.as_object_mut()
    {
        map.insert("result".to_owned(), committed("result-line.schema.json"));
        map.insert("summary".to_owned(), committed("summary-line.schema.json"));
    }
    Some(json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("{BASE}command-{name}.schema.json"),
        "title": format!("laminapdf {}", name.replace('-', " ")),
        "description": "What the command takes (input), prints with --json (output), prints on standard error when it fails (error) and, over several inputs, the NDJSON lines (result, summary).",
        "type": "object",
        "properties": properties
    }))
}

/// `laminapdf schema` with no command: the commands it describes and every committed schema's id.
#[must_use]
pub fn index() -> Value {
    let ids: Vec<Value> = all()
        .into_iter()
        .filter_map(|(_, schema)| schema.get("$id").cloned())
        .collect();
    json!({ "schema": "laminapdf.schemas/1", "commands": COMMANDS, "schemas": ids })
}

/// One NDJSON `result` line of a command over several inputs: the input's index and status,
/// and either its report's keys or its error.
fn result_line() -> Value {
    let mut status = schemars::SchemaGenerator::default();
    let status = status.root_schema_for::<Status>().to_value();
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("{BASE}result-line.schema.json"),
        "title": "laminapdf batch result line",
        "description": "One input's outcome over several inputs (--json): `status` ok or warnings with the report's keys, or error with `error`.",
        "type": "object",
        "required": ["type", "schema", "index", "input", "status"],
        "properties": {
            "type": { "const": "result" },
            "schema": { "type": "string", "description": "The schema id of the command's report." },
            "index": { "type": "integer", "minimum": 0, "description": "The input's position, from 0." },
            "input": { "type": "string", "description": "The input as given: a path, or -." },
            "status": status,
            "error": error_line().get("properties").and_then(|p| p.get("error")).cloned().unwrap_or_default()
        },
        "if": { "properties": { "status": { "const": "error" } } },
        "then": { "required": ["error"] }
    })
}

/// The last NDJSON line of a command over several inputs: the counts.
fn summary_line() -> Value {
    let mut schema = schema_for!(BatchSummary).to_value();
    if let Some(map) = schema.as_object_mut() {
        map.insert("$id".to_owned(), format!("{BASE}summary-line.schema.json").into());
        map.insert("title".to_owned(), "laminapdf batch summary line".into());
    }
    schema
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
