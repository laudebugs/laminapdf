//! The tools `laminapdf mcp` lists: names, fixed descriptions, honest annotations, and schemas
//! generated from the commands' own argument and report types.

use std::sync::Arc;

use rmcp::model::{Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde_json::{Map, Value};

use crate::commands::{
    CheckArgs, CheckReport, ERROR_SCHEMA_ID, ExplainArgs, ExplainReport, ExtractArgs, ExtractedReport, InfoArgs,
    InfoReport, PagesReport, RenderArgs, RenderReport, RevealReport, RevealToolArgs, RotateArgs, TextReport,
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
            "Read a whole PDF (every object, page and content stream) and report what a tolerant read had to repair and what a failed redaction left in the file: covered text, unapplied redaction marks and text kept in earlier revisions.",
            read(),
        ),
        tool::<ExplainArgs, ExplainReport>(
            "laminapdf_explain",
            "The documentation page of a Lamina PDF error or warning code.",
            read(),
        ),
        Tool::new(
            "laminapdf_extract",
            "A PDF's text: page by page (format text), or its blocks in reading order as Markdown (md), the laminapdf.extract/1 JSON (json) or retrieval chunks (chunks); `pages` takes a range such as 1-3, z (the last page) or 1-z,x2.",
            schema::<ExtractArgs>(),
        )
        .with_raw_output_schema(extract_output())
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
        let render = ToolAnnotations::new()
            .read_only(false)
            .destructive(false)
            .open_world(false);
        tools.push(
            Tool::new(
                "laminapdf_render",
                "Render pages of a PDF as PNG images. `output` is a path template: {stem}, {name}, {index} and {n} (the page number; {n:03} pads it) fill in, and each file must be new.",
                schema::<RenderArgs>(),
            )
            .with_raw_output_schema(schema::<RenderReport>())
            .with_annotations(render),
        );
    }
    // Last in name order: after laminapdf_render when that is listed.
    tools.push(tool::<RevealToolArgs, RevealReport>(
        "laminapdf_reveal",
        "List what a PDF holds beyond its visible pages: earlier revisions, metadata, attachments, scripts and actions, annotations, form values, hidden layers and counts of hidden text.",
        read(),
    ));
    tools
}

/// The prefix of the `laminapdf.extract/1` definitions copied into the extract tool's
/// `$defs`, so they can never collide with the text report's.
const EXTRACT_DEFS: &str = "laminapdf-extract-";

/// The extract tool's output: the text report for `format: text`, the `laminapdf.extract/1`
/// document for the others. One self-contained object schema, `laminapdf.extract/1`'s
/// definitions hoisted into its `$defs` under [`EXTRACT_DEFS`].
fn extract_output() -> Arc<Map<String, Value>> {
    let mut text = Arc::unwrap_or_clone(schema::<TextReport>());
    let mut defs = match text.remove("$defs") {
        Some(Value::Object(defs)) => defs,
        _ => Map::new(),
    };
    text.remove("$schema");
    let mut document = schemars::schema_for!(ExtractedReport).to_value();
    let mut own = Map::new();
    let mut strip = |embedded: Option<&mut Value>| {
        if let Some(map) = embedded.and_then(Value::as_object_mut) {
            map.remove("$id");
            map.remove("$schema");
            if let Some(Value::Object(found)) = map.remove("$defs") {
                own.extend(found);
            }
        }
    };
    strip(document.pointer_mut("/allOf/0"));
    strip(Some(&mut document));
    rename_refs(&mut document, "#/$defs/", EXTRACT_DEFS);
    for (name, mut definition) in own {
        rename_refs(&mut definition, "#/$defs/", EXTRACT_DEFS);
        defs.insert(format!("{EXTRACT_DEFS}{name}"), definition);
    }
    let mut out = Map::new();
    out.insert(
        "$schema".to_owned(),
        "https://json-schema.org/draft/2020-12/schema".into(),
    );
    out.insert("type".to_owned(), "object".into());
    out.insert("anyOf".to_owned(), Value::Array(vec![Value::Object(text), document]));
    out.insert("$defs".to_owned(), Value::Object(defs));
    Arc::new(out)
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
    if rename_refs(&mut schema, &external, ERROR_DEFS) {
        let error: Value = serde_json::from_str(laminapdf::ERROR_SCHEMA).unwrap_or_default();
        let defs = schema.as_object_mut().and_then(|map| {
            map.entry("$defs")
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
        });
        if let (Some(defs), Some(error_defs)) = (defs, error.get("$defs").and_then(Value::as_object)) {
            for (name, definition) in error_defs {
                let mut definition = definition.clone();
                rename_refs(&mut definition, "#/$defs/", ERROR_DEFS);
                defs.insert(format!("{ERROR_DEFS}{name}"), definition);
            }
        }
    }
    match schema {
        Value::Object(map) => Arc::new(map),
        _ => Arc::default(),
    }
}

/// Rewrites every `$ref` that starts with `prefix` to `#/$defs/<to><name>`; whether it rewrote
/// one. Every reference is visited: one schema may hold several.
fn rename_refs(value: &mut Value, prefix: &str, to: &str) -> bool {
    let mut rewrote = false;
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                match child {
                    Value::String(target) if key == "$ref" => {
                        if let Some(name) = target.strip_prefix(prefix) {
                            *target = format!("#/$defs/{to}{name}");
                            rewrote = true;
                        }
                    }
                    _ => rewrote |= rename_refs(child, prefix, to),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrote |= rename_refs(item, prefix, to);
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
                "laminapdf_info",
                "laminapdf_reveal"
            ]
        );
        let write = tools(true, false);
        let mut sorted = names(&write);
        sorted.sort();
        assert_eq!(names(&write), sorted);
        assert_eq!(
            names(&write)
                .iter()
                .rev()
                .take(3)
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["laminapdf_reveal", "laminapdf_render", "laminapdf_pages_rotate"]
        );
        let destructive = |tools: &[Tool], name: &str| {
            tools
                .iter()
                .find(|t| t.name == name)
                .and_then(|t| t.annotations.clone())
                .and_then(|a| a.destructive_hint)
        };
        let in_place = tools(true, true);
        assert_eq!(
            (
                destructive(&write, "laminapdf_pages_rotate"),
                destructive(&in_place, "laminapdf_pages_rotate"),
                destructive(&in_place, "laminapdf_render")
            ),
            (Some(false), Some(true), Some(false))
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
