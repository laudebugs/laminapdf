//! Templates (ADR 0032 §6): a document whose JSON holds placeholders that point into data with
//! JSON Pointers (RFC 6901). No expressions and no code: the data arrives formatted.
//!
//! - `{ "$data": "/invoice/number" }` is replaced by that value of the data;
//! - in a list, `{ "$each": "/items", "do": … }` is replaced by `do` once per item of that list
//!   (`do` may be a list, whose members are all added); inside it `{ "$item": "/name" }` points
//!   into the current item;
//! - in a list, `{ "$if": "/paid", "do": … }` is replaced by `do` when the value is there and
//!   isn't `false` or `null`, and by nothing otherwise.
//!
//! `$each` and `$if` take a pointer into the data, or `{ "$item": "/lines" }` for one into the
//! current item, so an `$each` can run over a list inside each item of another.

use serde_json::{Map, Value};

use super::FlowDocument;
use crate::error::{Error, Result};

const DATA: &str = "$data";
const ITEM: &str = "$item";
const EACH: &str = "$each";
const IF: &str = "$if";

/// What a text placeholder takes: text, a number or `true`/`false`.
const SCALAR: &str = "text, a number or true/false";

/// Fills `template` from `data` and reads the result as a document. A filled template is a
/// document like any other, so it is checked the same way.
///
/// # Errors
/// `template-data-missing` for a pointer with no value; `template-data-type` for `$each` over
/// something that isn't a list, or a `$data` or `$item` that finds an object or a list;
/// `invalid-document` for the filled result. The pointer is in each error's data.
pub fn fill(template: &Value, data: &Value) -> Result<FlowDocument> {
    let filled = Filler {
        data,
        items: Vec::new(),
    }
    .value(template)?;
    FlowDocument::from_json(&filled)
}

struct Filler<'a> {
    data: &'a Value,
    /// The current item of each `$each` the walk is inside, innermost last.
    items: Vec<&'a Value>,
}

/// The one key of a placeholder object, and its value.
fn placeholder(map: &Map<String, Value>) -> Option<(&str, &Value)> {
    let key = [DATA, ITEM, EACH, IF].into_iter().find(|k| map.contains_key(*k))?;
    map.get(key).map(|v| (key, v))
}

impl<'a> Filler<'a> {
    /// What `pointer` finds: in the data, or in the current item for `$item`.
    fn lookup(&self, placeholder: &'static str, pointer: &Value) -> Result<&'a Value> {
        let (root, pointer) = match pointer {
            Value::String(p) if placeholder != ITEM => (Some(self.data), p.as_str()),
            Value::String(p) => (self.items.last().copied(), p.as_str()),
            // `$each`/`$if` with `{ "$item": "/lines" }`: a pointer into the current item.
            Value::Object(map) => match map.get(ITEM) {
                Some(Value::String(p)) if map.len() == 1 => (self.items.last().copied(), p.as_str()),
                _ => return Err(missing(placeholder, "")),
            },
            _ => return Err(missing(placeholder, "")),
        };
        root.and_then(|root| root.pointer(pointer))
            .ok_or_else(|| missing(placeholder, pointer))
    }

    /// `value` with its placeholders filled.
    fn value(&mut self, value: &'a Value) -> Result<Value> {
        match value {
            Value::Array(items) => self.list(items).map(Value::Array),
            Value::Object(map) => match placeholder(map) {
                Some((key @ (DATA | ITEM), pointer)) => {
                    let found = self.lookup(if key == DATA { DATA } else { ITEM }, pointer)?;
                    match found {
                        Value::String(_) | Value::Number(_) | Value::Bool(_) => Ok(found.clone()),
                        _ => Err(wrong_type(if key == DATA { DATA } else { ITEM }, pointer, SCALAR)),
                    }
                }
                // `$each`/`$if` outside a list have no list to add to: left as they are, the
                // document check refuses the unknown key.
                _ => map
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), self.value(v)?)))
                    .collect::<Result<Map<_, _>>>()
                    .map(Value::Object),
            },
            other => Ok(other.clone()),
        }
    }

    /// A list with its `$each` and `$if` members expanded.
    fn list(&mut self, items: &'a [Value]) -> Result<Vec<Value>> {
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let Value::Object(map) = item else {
                out.push(self.value(item)?);
                continue;
            };
            match placeholder(map) {
                Some((EACH, pointer)) => {
                    let Value::Array(list) = self.lookup(EACH, pointer)? else {
                        return Err(wrong_type(EACH, pointer, "list"));
                    };
                    let body = map.get("do").unwrap_or(&Value::Null);
                    for each in list {
                        self.items.push(each);
                        let filled = self.body(body);
                        self.items.pop();
                        out.extend(filled?);
                    }
                }
                Some((IF, pointer)) => {
                    let keep = match self.lookup(IF, pointer) {
                        Ok(Value::Null | Value::Bool(false)) | Err(Error::TemplateDataMissing { .. }) => false,
                        Ok(_) => true,
                        Err(e) => return Err(e),
                    };
                    if keep {
                        out.extend(self.body(map.get("do").unwrap_or(&Value::Null))?);
                    }
                }
                _ => out.push(self.value(item)?),
            }
        }
        Ok(out)
    }

    /// The members `do` adds: each of a list's, else the one value.
    fn body(&mut self, body: &'a Value) -> Result<Vec<Value>> {
        match body {
            Value::Array(items) => self.list(items),
            other => Ok(vec![self.value(other)?]),
        }
    }
}

fn missing(placeholder: &'static str, pointer: &str) -> Error {
    Error::TemplateDataMissing {
        pointer: pointer.to_owned(),
        placeholder,
    }
}

fn wrong_type(placeholder: &'static str, pointer: &Value, expected: &'static str) -> Error {
    let pointer = match pointer {
        Value::String(p) => p.clone(),
        other => other.get(ITEM).and_then(Value::as_str).unwrap_or_default().to_owned(),
    };
    Error::TemplateDataType {
        pointer,
        placeholder,
        expected,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn document(blocks: &Value) -> Value {
        json!({
            "schema": "laminapdf.document/1",
            "styles": { "body": { "font": "Helvetica", "size": 10 } },
            "blocks": blocks
        })
    }

    fn texts(doc: &FlowDocument) -> Vec<String> {
        doc.blocks
            .iter()
            .filter_map(|b| match b {
                super::super::BlockSpec::Paragraph(p) => p.text.as_ref().map(super::super::Text::as_string),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn data_puts_text_and_numbers_where_the_document_takes_text() {
        let template = document(&json!([
            { "type": "paragraph", "text": { "$data": "/invoice/number" } },
            { "type": "paragraph", "text": { "$data": "/invoice/lines" } }
        ]));
        let data = json!({ "invoice": { "number": "2026-114", "lines": 40 } });
        assert_eq!(texts(&fill(&template, &data).unwrap()), ["2026-114", "40"]);
    }

    #[test]
    fn each_repeats_its_block_per_item_and_item_points_into_it() {
        let template = document(&json!([
            { "type": "paragraph", "text": "Items:" },
            { "$each": "/items", "do": { "type": "paragraph", "text": { "$item": "/name" } } }
        ]));
        let data = json!({ "items": [{ "name": "Stapler" }, { "name": "Pens" }] });
        assert_eq!(texts(&fill(&template, &data).unwrap()), ["Items:", "Stapler", "Pens"]);
    }

    #[test]
    fn each_nests_over_a_list_inside_each_item() {
        let template = document(&json!([{
            "$each": "/groups",
            "do": [
                { "type": "paragraph", "text": { "$item": "/title" } },
                { "$each": { "$item": "/lines" }, "do": { "type": "paragraph", "text": { "$item": "" } } }
            ]
        }]));
        let data = json!({ "groups": [{ "title": "A", "lines": ["a1", "a2"] }, { "title": "B", "lines": ["b1"] }] });
        assert_eq!(texts(&fill(&template, &data).unwrap()), ["A", "a1", "a2", "B", "b1"]);
    }

    #[test]
    fn if_keeps_its_block_only_for_a_value_that_is_there_and_not_false_or_null() {
        let template = document(&json!([
            { "$if": "/paid", "do": { "type": "paragraph", "text": "Paid" } },
            { "$if": "/overdue", "do": { "type": "paragraph", "text": "Overdue" } },
            { "$if": "/note", "do": { "type": "paragraph", "text": "Note" } },
            { "$if": "/gone", "do": { "type": "paragraph", "text": "Gone" } }
        ]));
        let data = json!({ "paid": true, "overdue": false, "note": null });
        assert_eq!(texts(&fill(&template, &data).unwrap()), ["Paid"]);
    }

    #[test]
    fn a_pointer_with_no_value_names_the_pointer() {
        let template = document(&json!([{ "type": "paragraph", "text": { "$data": "/invoice/total" } }]));
        let error = fill(&template, &json!({ "invoice": {} })).unwrap_err();
        assert!(
            matches!(&error, Error::TemplateDataMissing { pointer, placeholder: "$data" } if pointer == "/invoice/total"),
            "{error:?}"
        );
    }

    #[test]
    fn each_over_something_that_isn_t_a_list_and_text_from_an_object_are_wrong_types() {
        let each = document(&json!([{ "$each": "/items", "do": { "type": "rule" } }]));
        let error = fill(&each, &json!({ "items": { "a": 1 } })).unwrap_err();
        assert!(
            matches!(&error, Error::TemplateDataType { expected: "list", .. }),
            "{error:?}"
        );
        let text = document(&json!([{ "type": "paragraph", "text": { "$data": "/invoice" } }]));
        let error = fill(&text, &json!({ "invoice": { "number": 1 } })).unwrap_err();
        assert!(
            matches!(
                &error,
                Error::TemplateDataType {
                    placeholder: "$data",
                    ..
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn item_outside_an_each_has_nothing_to_point_into() {
        let template = document(&json!([{ "type": "paragraph", "text": { "$item": "/name" } }]));
        let error = fill(&template, &json!({})).unwrap_err();
        assert!(matches!(
            error,
            Error::TemplateDataMissing {
                placeholder: "$item",
                ..
            }
        ));
    }
}
