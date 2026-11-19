//! Lamina PDF's form data JSON (E8 plan §10, decision 2): `{ "$schema": …, "fields": { "<fully
//! qualified name>": value } }`, where a value is a string, `true` or `false` for a check box
//! with one on-state, an array of strings for a list box, or `null` for empty. It is the object
//! `fill_form` takes and `export_form_data` writes, so an export fills a form unchanged.

use std::fmt;

use laminapdf_syntax::Limits;
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};

use crate::fields::{DataValue, FieldData};
use crate::{InterchangeError, Result};

/// The schema's address, written as `$schema` and published with the docs.
///
/// Added in 0.7.0.
pub const FORM_DATA_SCHEMA: &str =
    "https://github.com/laudebugs/laminapdf/blob/main/crates/laminapdf-interchange/schemas/form-data.schema.json";

/// Reads form data JSON: each entry of `fields`, in the file's order.
///
/// A number is taken as its text, so `{ "quantity": 3 }` fills a text field with `3`: a reader
/// is tolerant, and `write` never writes one. An entry whose value is an object, or an array
/// holding anything but strings, refuses the file at its line.
///
/// # Errors
/// `interchange-too-large` past `limits.max_stream_decoded()`; `interchange-malformed`, with
/// its line, for text that isn't JSON, a top level that isn't an object or has no `fields`, a
/// `fields` that isn't an object, or a value of no kind a field takes.
pub(crate) fn read(bytes: &[u8], limits: &Limits) -> Result<Vec<FieldData>> {
    if bytes.len() > limits.max_stream_decoded() {
        return Err(InterchangeError::TooLarge);
    }
    let top: Top = serde_json::from_slice(bytes).map_err(|e| {
        let reason = if e.is_data() {
            "a value is of no kind a field takes"
        } else {
            "it isn't JSON"
        };
        malformed(line(e.line()), reason)
    })?;
    let fields = top.fields.ok_or_else(|| malformed(1, "it has no fields"))?;
    Ok(crate::fields::one_per_name(
        fields
            .into_iter()
            .map(|(name, Value(value))| FieldData::new(name, value))
            .collect(),
    ))
}

/// The file's top level: its `fields`, in the file's order, which a map would sort.
struct Top {
    fields: Option<Vec<(String, Value)>>,
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
        let mut fields = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == "fields" {
                if fields.is_some() {
                    return Err(de::Error::custom("two fields objects"));
                }
                fields = Some(map.next_value::<Fields>()?.0);
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

/// One field's value, read as it is parsed, so a value of the wrong kind fails at its line.
struct Value(DataValue);

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a string, true or false, an array of strings, or null")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Value, E> {
        Ok(Value(DataValue::Text(v.to_owned())))
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Value, E> {
        Ok(Value(DataValue::Checked(v)))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Value, E> {
        Ok(Value(DataValue::Text(v.to_string())))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Value, E> {
        Ok(Value(DataValue::Text(v.to_string())))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Value, E> {
        Ok(Value(DataValue::Text(v.to_string())))
    }

    fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value(DataValue::Empty))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(item) = seq.next_element::<String>()? {
            out.push(item);
        }
        Ok(Value(DataValue::Choices(out)))
    }
}

/// Writes `fields` as form data JSON, two spaces to a level, in the order given.
pub(crate) fn write(fields: &[FieldData]) -> Vec<u8> {
    // Strings, booleans and arrays of strings under string keys: serialising them can't fail.
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
            match &field.value {
                DataValue::Text(t) | DataValue::State(t) => map.serialize_entry(&field.name, t)?,
                DataValue::Checked(on) => map.serialize_entry(&field.name, on)?,
                DataValue::Choices(values) => map.serialize_entry(&field.name, values)?,
                DataValue::Empty => map.serialize_entry(&field.name, &())?,
            }
        }
        map.end()
    }
}

fn line(line: usize) -> u32 {
    u32::try_from(line.max(1)).unwrap_or(u32::MAX)
}

fn malformed(line: u32, reason: &'static str) -> InterchangeError {
    InterchangeError::Malformed {
        format: crate::FormDataFormat::Json,
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
    fn read_should_refuse_an_object_value_at_its_line() {
        let error =
            read_str("{\n  \"fields\": {\n    \"name\": \"Ada\",\n    \"address\": { \"city\": \"Lagos\" }\n  }\n}");
        assert!(
            matches!(error, Err(InterchangeError::Malformed { line: 4, .. })),
            "{error:?}"
        );
    }

    #[test]
    fn read_should_refuse_a_file_without_fields() {
        let error = read_str(r#"{ "answers": {} }"#);
        assert!(
            matches!(
                error,
                Err(InterchangeError::Malformed {
                    reason: "it has no fields",
                    ..
                })
            ),
            "{error:?}"
        );
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
