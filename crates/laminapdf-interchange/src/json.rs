//! Lamina PDF's form data JSON (E8 plan §10, decision 2): `{ "$schema": …, "fields": { "<fully
//! qualified name>": value } }`, where a value is a string, `true` or `false` for a check box
//! with one on-state, an array of strings for a list box, or `null` for empty. It is the object
//! `fill_form` takes and `export_form_data` writes, so an export fills a form unchanged.

use std::fmt;

use laminapdf_syntax::Limits;
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde_json::Value;

use crate::fields::{DataValue, FieldData};
use crate::{InterchangeError, Result};

/// The schema's address, written as `$schema` and published with the docs.
///
/// Added in 0.7.0.
pub const FORM_DATA_SCHEMA: &str = "https://docs.laminapdf.com/schemas/form-data-1.json";

/// The format's name in errors.
const JSON: &str = "JSON";

/// Reads form data JSON: each entry of `fields`, in the file's order.
///
/// A number is taken as its text, so `{ "quantity": 3 }` fills a text field with `3`. An entry
/// whose value is an object, or an array holding anything but strings, refuses the file.
///
/// # Errors
/// `interchange-too-large` past `limits.max_stream_decoded()`; `interchange-malformed`, with
/// its line, for text that isn't JSON, a top level that isn't an object, a `fields` that isn't
/// one, or a value of no kind a field takes.
pub(crate) fn read(bytes: &[u8], limits: &Limits) -> Result<Vec<FieldData>> {
    if bytes.len() > limits.max_stream_decoded() {
        return Err(InterchangeError::TooLarge);
    }
    let top: Top = serde_json::from_slice(bytes).map_err(|e| {
        let reason = if e.is_data() {
            "it isn't an object of fields"
        } else {
            "it isn't JSON"
        };
        malformed(line(e.line()), reason)
    })?;
    top.fields
        .into_iter()
        .map(|(name, value)| Ok(FieldData::new(name, data_value(&value)?)))
        .collect()
}

/// The file's top level: its `fields`, in the file's order, which a map would sort.
struct Top {
    fields: Vec<(String, Value)>,
}

impl<'de> de::Deserialize<'de> for Top {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_map(TopVisitor)
    }
}

struct TopVisitor;

impl<'de> Visitor<'de> for TopVisitor {
    type Value = Top;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object with `fields`")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Top, A::Error> {
        let mut fields = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            if key == "fields" {
                fields = map.next_value::<Fields>()?.0;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(Top { fields })
    }
}

/// `fields`' entries in order.
struct Fields(Vec<(String, Value)>);

impl<'de> de::Deserialize<'de> for Fields {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Fields;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object of fields")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Fields, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry::<String, Value>()? {
                    out.push(entry);
                }
                Ok(Fields(out))
            }
        }
        deserializer.deserialize_map(V)
    }
}

/// The value `value` gives a field.
fn data_value(value: &Value) -> Result<DataValue> {
    Ok(match value {
        Value::String(s) => DataValue::Text(s.clone()),
        Value::Bool(on) => DataValue::Checked(*on),
        Value::Number(n) => DataValue::Text(n.to_string()),
        Value::Null => DataValue::Empty,
        Value::Array(items) => DataValue::Choices(
            items
                .iter()
                .map(|i| i.as_str().map(str::to_owned))
                .collect::<Option<_>>()
                .ok_or_else(|| malformed(1, "a list holds something other than text"))?,
        ),
        Value::Object(_) => return Err(malformed(1, "a value is an object")),
    })
}

/// Writes `fields` as form data JSON, two spaces to a level, in the order given.
pub(crate) fn write(fields: &[FieldData]) -> Vec<u8> {
    let mut out = serde_json::to_vec_pretty(&Written(fields)).unwrap_or_default();
    out.push(b'\n');
    out
}

/// The file `write` makes: `$schema`, then `fields` in their order.
struct Written<'f>(&'f [FieldData]);

impl serde::Serialize for Written<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut top = serializer.serialize_map(Some(2))?;
        top.serialize_entry("$schema", FORM_DATA_SCHEMA)?;
        top.serialize_entry("fields", &Entries(self.0))?;
        top.end()
    }
}

struct Entries<'f>(&'f [FieldData]);

impl serde::Serialize for Entries<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for field in self.0 {
            let value = match &field.value {
                DataValue::Text(t) | DataValue::State(t) => Value::String(t.clone()),
                DataValue::Checked(on) => Value::Bool(*on),
                DataValue::Choices(values) => Value::Array(values.iter().cloned().map(Value::String).collect()),
                DataValue::Empty => Value::Null,
            };
            map.serialize_entry(&field.name, &value)?;
        }
        map.end()
    }
}

fn line(line: usize) -> u32 {
    u32::try_from(line.max(1)).unwrap_or(u32::MAX)
}

fn malformed(line: u32, reason: &'static str) -> InterchangeError {
    InterchangeError::Malformed {
        format: JSON,
        line,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_str(text: &str) -> Result<Vec<FieldData>> {
        read(text.as_bytes(), &Limits::default())
    }

    #[test]
    fn read_should_take_each_kind_of_value_in_the_files_order() {
        let fields = read_str(
            r#"{ "fields": { "name": "Ada", "agree": true, "langs": ["en", "fr"], "note": null, "qty": 3 } }"#,
        );
        assert_eq!(
            fields,
            Ok(vec![
                FieldData::new("name", DataValue::Text("Ada".into())),
                FieldData::new("agree", DataValue::Checked(true)),
                FieldData::new("langs", DataValue::Choices(vec!["en".into(), "fr".into()])),
                FieldData::new("note", DataValue::Empty),
                FieldData::new("qty", DataValue::Text("3".into())),
            ])
        );
    }

    #[test]
    fn read_should_refuse_an_object_value() {
        let error = read_str(r#"{ "fields": { "address": { "city": "Lagos" } } }"#);
        assert_eq!(error.map_err(|e| e.code()), Err("interchange-malformed"));
    }

    #[test]
    fn read_should_name_the_line_of_text_that_isnt_json() {
        let error = read_str("{\n  \"fields\": {\n    \"a\": nope\n  }\n}");
        assert!(
            matches!(error, Err(InterchangeError::Malformed { line: 3, .. })),
            "{error:?}"
        );
    }

    #[test]
    fn write_then_read_should_give_the_same_fields() {
        let fields = vec![
            FieldData::new("b", DataValue::Checked(false)),
            FieldData::new("a", DataValue::Choices(vec!["x".into()])),
        ];
        assert_eq!(read(&write(&fields), &Limits::default()), Ok(fields));
    }
}
