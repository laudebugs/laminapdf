//! TypeScript types from a committed JSON Schema: `laminapdf.extract/1`
//! (`crates/laminapdf-extract/schemas/extract.schema.json`, itself generated from the Rust types)
//! becomes `packages/sdk/src/generated/extract.ts`, so `doc.extract({ format: "json" })` is typed
//! by the same source as the JSON it parses, and `laminapdf.document/1`
//! (`crates/laminapdf-build/schemas/document.schema.json`) becomes `document.ts`, the types
//! `builder.flow` takes.
//!
//! The converter knows the keywords that schemars writes for those types and fails, naming the
//! path, on any other, so a new shape in the schema is a deliberate change here.

use std::fmt::Write as _;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

/// Keywords that only describe a value (checked by the schema, not by the type).
const IGNORED: [&str; 7] = ["description", "format", "minimum", "maximum", "title", "$id", "$schema"];

/// Keywords the converter turns into types.
const HANDLED: [&str; 11] = [
    "type",
    "properties",
    "required",
    "items",
    "minItems",
    "maxItems",
    "const",
    "oneOf",
    "anyOf",
    "$ref",
    "additionalProperties",
];

/// The longest array of fixed length written as a tuple.
const MAX_TUPLE: u64 = 8;

/// `schema` as TypeScript: one type per `$defs` entry and `root` for the document itself.
pub(crate) fn render(schema: &Value, root: &str, header: &str, source: &str) -> Result<String> {
    let mut out = format!("{header}\n// From {source}.\n");
    let defs = schema.get("$defs").and_then(Value::as_object);
    for (name, def) in defs.into_iter().flatten() {
        let path = format!("$defs/{name}");
        let _ = write!(out, "\n{}export type {name} = {};\n", doc(def, ""), ts(def, &path, "")?);
    }
    let mut body = Map::clone(schema.as_object().context("the schema is not an object")?);
    body.remove("$defs");
    let _ = write!(
        out,
        "\n{}export type {root} = {};\n",
        doc(schema, ""),
        ts(&Value::Object(body), "#", "")?
    );
    Ok(out)
}

/// A `JSDoc` comment from `node`'s description, or nothing.
fn doc(node: &Value, indent: &str) -> String {
    match node.get("description").and_then(Value::as_str) {
        Some(d) => {
            let lines: Vec<&str> = d.lines().collect();
            if let [line] = lines.as_slice() {
                format!("/** {} */\n{indent}", line.replace("*/", "*\\/"))
            } else {
                let mut s = String::from("/**\n");
                for line in lines {
                    let _ = writeln!(s, "{indent} * {}", line.replace("*/", "*\\/"));
                }
                format!("{s}{indent} */\n{indent}")
            }
        }
        None => String::new(),
    }
}

/// The TypeScript type of `node` at `path`, its nested objects indented past `indent`.
fn ts(node: &Value, path: &str, indent: &str) -> Result<String> {
    let map = match node {
        Value::Bool(true) => return Ok("unknown".to_owned()),
        Value::Object(map) => map,
        _ => bail!("{path}: expected a schema object"),
    };
    if let Some(key) = map
        .keys()
        .find(|k| !IGNORED.contains(&k.as_str()) && !HANDLED.contains(&k.as_str()))
    {
        bail!("{path}: the keyword `{key}` has no TypeScript form yet (tools/xtask/src/codegen/schema_ts.rs)");
    }
    // `additionalProperties` shapes objects only; anywhere else it would be dropped unread.
    if map.contains_key("additionalProperties") && map.get("type").and_then(Value::as_str) != Some("object") {
        bail!("{path}: additionalProperties outside an object schema has no TypeScript form yet");
    }
    if let Some(target) = map.get("$ref").and_then(Value::as_str) {
        return Ok(target
            .strip_prefix("#/$defs/")
            .with_context(|| format!("{path}: only #/$defs/ references are supported"))?
            .to_owned());
    }
    if let Some(value) = map.get("const") {
        return Ok(value.to_string());
    }
    let variants = map.get("oneOf").or_else(|| map.get("anyOf"));
    if let Some(variants) = variants {
        let members = variants
            .as_array()
            .with_context(|| format!("{path}: oneOf/anyOf is not a list"))?
            .iter()
            .enumerate()
            .map(|(i, v)| ts(v, &format!("{path}/oneOf/{i}"), indent))
            .collect::<Result<Vec<_>>>()?;
        let union = members.join(" | ");
        return Ok(if map.contains_key("properties") {
            format!("{} & ({union})", object(map, path, indent)?)
        } else {
            union
        });
    }
    match map.get("type") {
        Some(Value::String(t)) => single(t, map, path, indent),
        Some(Value::Array(types)) => Ok(types
            .iter()
            .map(|t| single(t.as_str().unwrap_or_default(), map, path, indent))
            .collect::<Result<Vec<_>>>()?
            .join(" | ")),
        _ => bail!("{path}: no type"),
    }
}

/// One JSON type name as TypeScript.
fn single(t: &str, map: &Map<String, Value>, path: &str, indent: &str) -> Result<String> {
    Ok(match t {
        "string" => "string".to_owned(),
        "number" | "integer" => "number".to_owned(),
        "boolean" => "boolean".to_owned(),
        "null" => "null".to_owned(),
        "object" => object(map, path, indent)?,
        "array" => {
            let items = map
                .get("items")
                .with_context(|| format!("{path}: an array without items"))?;
            let item = ts(items, &format!("{path}/items"), indent)?;
            let min = map.get("minItems").and_then(Value::as_u64);
            let max = map.get("maxItems").and_then(Value::as_u64);
            match (min, max) {
                (Some(n), Some(m)) if n == m && n <= MAX_TUPLE => {
                    format!("[{}]", vec![item; usize::try_from(n).unwrap_or(0)].join(", "))
                }
                _ if item.contains(' ') => format!("({item})[]"),
                _ => format!("{item}[]"),
            }
        }
        other => bail!("{path}: the type `{other}` has no TypeScript form"),
    })
}

/// An object's properties, required ones without `?`, as a multi-line type literal; a map
/// (`additionalProperties` with a schema and no properties) as a `Record`.
///
/// `additionalProperties: false` beside properties needs nothing more: TypeScript already refuses
/// an unknown key in an object literal, which is how a document is written.
fn object(map: &Map<String, Value>, path: &str, indent: &str) -> Result<String> {
    let extra = map.get("additionalProperties");
    let Some(properties) = map.get("properties").and_then(Value::as_object) else {
        return Ok(match extra {
            None | Some(Value::Bool(true)) => "Record<string, unknown>".to_owned(),
            Some(Value::Bool(false)) => "Record<string, never>".to_owned(),
            Some(value) => format!(
                "Record<string, {}>",
                ts(value, &format!("{path}/additionalProperties"), indent)?
            ),
        });
    };
    // Beside properties only `false` (or nothing) fits a type literal, which already refuses an
    // unknown key; `true` or a schema would promise keys the written type doesn't allow.
    if let Some(value) = extra.filter(|v| **v != Value::Bool(false)) {
        bail!("{path}: properties beside additionalProperties {value} have no TypeScript form yet");
    }
    let required: Vec<&str> = map
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let inner = format!("{indent}  ");
    let mut out = String::from("{\n");
    for (key, value) in properties {
        let optional = if required.contains(&key.as_str()) { "" } else { "?" };
        let ty = ts(value, &format!("{path}/properties/{key}"), &inner)?;
        let _ = writeln!(out, "{inner}{}{key}{optional}: {ty};", doc(value, &inner));
    }
    let _ = write!(out, "{indent}}}");
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "test code")]

    use serde_json::json;

    use super::render;

    #[test]
    fn render_should_write_defs_refs_unions_tuples_and_optional_keys() {
        let schema = json!({
            "$defs": {
                "Kind": { "oneOf": [{ "const": "a", "type": "string" }, { "const": "b", "type": "string" }] }
            },
            "description": "The root.",
            "properties": {
                "kind": { "$ref": "#/$defs/Kind" },
                "box": { "items": { "type": "number" }, "minItems": 4, "maxItems": 4, "type": "array" },
                "label": { "type": ["string", "null"] }
            },
            "required": ["kind", "box"],
            "type": "object"
        });
        let out = render(&schema, "Root", "// header", "test").unwrap();
        assert!(out.contains("export type Kind = \"a\" | \"b\";"), "{out}");
        assert!(out.contains("/** The root. */\nexport type Root = {"), "{out}");
        assert!(out.contains("  box: [number, number, number, number];"), "{out}");
        assert!(out.contains("  kind: Kind;"), "{out}");
        assert!(out.contains("  label?: string | null;"), "{out}");
    }

    #[test]
    fn render_should_refuse_a_keyword_it_does_not_know() {
        let schema = json!({ "type": "object", "patternProperties": { "^x": { "type": "string" } } });
        let err = render(&schema, "Root", "", "test").unwrap_err().to_string();
        assert!(err.contains("patternProperties"), "{err}");
    }

    #[test]
    fn render_should_write_a_map_as_a_record_and_a_closed_object_as_its_properties() {
        let schema = json!({
            "$defs": { "Style": { "properties": { "size": { "type": "number" } }, "type": "object", "additionalProperties": false } },
            "properties": { "styles": { "type": "object", "additionalProperties": { "$ref": "#/$defs/Style" } } },
            "type": "object",
            "additionalProperties": false
        });
        let out = render(&schema, "Root", "", "test").unwrap();
        assert!(out.contains("export type Style = {\n  size?: number;\n};"), "{out}");
        assert!(out.contains("  styles?: Record<string, Style>;"), "{out}");
    }

    #[test]
    fn render_should_refuse_properties_beside_an_additional_properties_schema() {
        let schema = json!({
            "properties": { "a": { "type": "string" } },
            "additionalProperties": { "type": "number" },
            "type": "object"
        });
        let err = render(&schema, "Root", "", "test").unwrap_err().to_string();
        assert!(err.contains("beside additionalProperties"), "{err}");
    }

    #[test]
    fn render_should_refuse_open_properties_and_additional_properties_off_an_object() {
        let open = json!({
            "properties": { "a": { "type": "string" } },
            "additionalProperties": true,
            "type": "object"
        });
        let err = render(&open, "Root", "", "test").unwrap_err().to_string();
        assert!(err.contains("beside additionalProperties true"), "{err}");
        let stray = json!({ "type": "string", "additionalProperties": false });
        let err = render(&stray, "Root", "", "test").unwrap_err().to_string();
        assert!(err.contains("outside an object schema"), "{err}");
    }

    #[test]
    fn render_should_intersect_shared_properties_with_the_variants() {
        let schema = json!({
            "properties": { "id": { "type": "string" } },
            "required": ["id"],
            "oneOf": [{ "properties": { "type": { "const": "x", "type": "string" } }, "required": ["type"], "type": "object" }],
            "type": "object"
        });
        let out = render(&schema, "Root", "", "test").unwrap();
        assert!(out.contains("  id: string;\n} & ({\n  type: \"x\";\n})"), "{out}");
    }
}
