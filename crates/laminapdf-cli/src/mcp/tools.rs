//! The tools `laminapdf mcp` lists: names, fixed descriptions, honest annotations, and schemas
//! generated from the commands' own argument and report types.

use std::sync::Arc;

use rmcp::model::{Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde_json::{Map, Value};

use crate::commands::{
    CheckArgs, CheckReport, ERROR_SCHEMA_ID, ExplainArgs, ExplainReport, ExtractArgs, InfoArgs, InfoReport,
    PagesReport, RotateArgs, TextReport,
};

/// What the copied `laminapdf.error/1` definitions are named after in a tool's `$defs`, so they
/// can never collide with a report's own.
const ERROR_DEFS: &str = "laminapdf-error-";

/// The tools, in name order. Their descriptions are fixed text, never a path or document text.
pub(super) fn tools(allow_write: bool, allow_in_place: bool) -> Vec<Tool> {
    let read = || ToolAnnotations::new().read_only(true).open_world(false);
    let mut tools = vec![
        tool::<CheckArgs, CheckReport>(
            "laminapdf_check",
            "Read a whole PDF (every object, page and content stream) and report what a tolerant read had to repair.",
            read(),
        ),
        tool::<ExplainArgs, ExplainReport>(
            "laminapdf_explain",
            "The documentation page of a Lamina PDF error or warning code.",
            read(),
        ),
        Tool::new(
            "laminapdf_extract",
            "A PDF's text, page by page; `pages` takes a range such as 1-3, z (the last page) or 1-z,x2.",
            text_only(schema::<ExtractArgs>()),
        )
        .with_raw_output_schema(schema::<TextReport>())
        .with_annotations(read()),
        tool::<InfoArgs, InfoReport>(
            "laminapdf_info",
            "What a PDF is: its version, pages and their sizes, file size, encryption, tagging, metadata and warnings.",
            read(),
        ),
    ];
    if allow_write {
        let what = "Rotate pages of a PDF: +90 or -90 turn them, 90 sets their rotation, :RANGE names the pages.";
        let (description, destructive) = if allow_in_place {
            (
                format!("{what} Writes `output`, a new file, or with `inPlace` replaces the input."),
                true,
            )
        } else {
            (format!("{what} Writes `output`, which must be a new file."), false)
        };
        let annotations = ToolAnnotations::new()
            .read_only(false)
            .destructive(destructive)
            .open_world(false);
        tools.push(
            Tool::new("laminapdf_pages_rotate", description, schema::<RotateArgs>())
                .with_raw_output_schema(schema::<PagesReport>())
                .with_annotations(annotations),
        );
    }
    tools
}

/// `extract`'s arguments as the tool takes them: the text format only, since the tool answers
/// with the text report (the Markdown, JSON and chunk formats are the command line's for now).
fn text_only(schema: Arc<Map<String, Value>>) -> Arc<Map<String, Value>> {
    let mut schema = Arc::unwrap_or_clone(schema);
    if let Some(Value::Object(properties)) = schema.get_mut("properties") {
        properties.remove("maxChars");
        properties.insert(
            "format".to_owned(),
            serde_json::json!({ "const": "text", "default": "text", "description": "The output format: plain text." }),
        );
    }
    if let Some(Value::Object(defs)) = schema.get_mut("$defs") {
        defs.remove("Format");
    }
    Arc::new(schema)
}

/// One read tool: `T` its arguments, `R` its report.
fn tool<T: JsonSchema, R: JsonSchema>(
    name: &'static str,
    description: &'static str,
    annotations: ToolAnnotations,
) -> Tool {
    Tool::new(name, description, schema::<T>())
        .with_raw_output_schema(schema::<R>())
        .with_annotations(annotations)
}

/// A JSON Schema object for `T`, self-contained: references to `laminapdf.error/1` (a report's
/// `warnings`) point to a copy of its definitions in the schema's own `$defs`, named under
/// [`ERROR_DEFS`], so a client validating `structuredContent` never has to fetch it.
pub(super) fn schema<T: JsonSchema>() -> Arc<Map<String, Value>> {
    let mut schema = schemars::schema_for!(T).to_value();
    let external = format!("{ERROR_SCHEMA_ID}#/$defs/");
    if rename_refs(&mut schema, &external) {
        let error: Value = serde_json::from_str(laminapdf::ERROR_SCHEMA).unwrap_or_default();
        let defs = schema.as_object_mut().and_then(|map| {
            map.entry("$defs")
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
        });
        if let (Some(defs), Some(error_defs)) = (defs, error.get("$defs").and_then(Value::as_object)) {
            for (name, definition) in error_defs {
                let mut definition = definition.clone();
                rename_refs(&mut definition, "#/$defs/");
                defs.insert(format!("{ERROR_DEFS}{name}"), definition);
            }
        }
    }
    match schema {
        Value::Object(map) => Arc::new(map),
        _ => Arc::default(),
    }
}

/// Rewrites every `$ref` that starts with `prefix` to `#/$defs/<ERROR_DEFS><name>`; whether it
/// rewrote one. Every reference is visited: one schema may hold several.
fn rename_refs(value: &mut Value, prefix: &str) -> bool {
    let mut rewrote = false;
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                match child {
                    Value::String(target) if key == "$ref" => {
                        if let Some(name) = target.strip_prefix(prefix) {
                            *target = format!("#/$defs/{ERROR_DEFS}{name}");
                            rewrote = true;
                        }
                    }
                    _ => rewrote |= rename_refs(child, prefix),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrote |= rename_refs(item, prefix);
            }
        }
        _ => {}
    }
    rewrote
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `$ref` in `value`.
    fn refs(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    match child {
                        Value::String(target) if key == "$ref" => out.push(target.clone()),
                        _ => refs(child, out),
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|item| refs(item, out)),
            _ => {}
        }
    }

    #[test]
    fn the_tools_should_be_in_name_order_and_read_only_by_default() {
        let names = |tools: &[Tool]| tools.iter().map(|t| t.name.to_string()).collect::<Vec<_>>();
        assert_eq!(
            names(&tools(false, false)),
            [
                "laminapdf_check",
                "laminapdf_explain",
                "laminapdf_extract",
                "laminapdf_info"
            ]
        );
        let write = tools(true, false);
        assert_eq!(names(&write).last().map(String::as_str), Some("laminapdf_pages_rotate"));
        let destructive = |tools: &[Tool]| {
            tools
                .last()
                .and_then(|t| t.annotations.clone())
                .and_then(|a| a.destructive_hint)
        };
        assert_eq!(
            (destructive(&write), destructive(&tools(true, true))),
            (Some(false), Some(true))
        );
    }

    // Also fails if the embedded laminapdf.error/1 didn't parse: its references would dangle.
    #[test]
    fn every_reference_should_resolve_inside_its_schema() {
        for tool in tools(true, true) {
            for schema in [Some(tool.input_schema.clone()), tool.output_schema.clone()]
                .into_iter()
                .flatten()
            {
                let schema = Value::Object((*schema).clone());
                let mut found = Vec::new();
                refs(&schema, &mut found);
                for target in found {
                    let name = target
                        .strip_prefix("#/$defs/")
                        .unwrap_or_else(|| panic!("{}: {target}", tool.name));
                    assert!(
                        schema["$defs"].get(name).is_some(),
                        "{}: {target} doesn't resolve",
                        tool.name
                    );
                }
            }
        }
        assert!(schema::<InfoReport>()["$defs"].get("laminapdf-error-warning").is_some());
    }
}
