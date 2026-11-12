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
//! current item, so an `$each` can run over a list inside each item of another. A placeholder
//! object holds nothing else: `$data` and `$item` alone, `$each` and `$if` with `do`.

use serde_json::{Map, Value};

use super::check::refuse;
use super::{FlowDocument, escape};
use crate::error::{Error, Result};

const DATA: &str = "$data";
const ITEM: &str = "$item";
const EACH: &str = "$each";
const IF: &str = "$if";
const DO: &str = "do";

/// What a text placeholder takes: text, a number or `true`/`false`.
const SCALAR: &str = "text, a number or true/false";
/// What a placeholder's pointer must be.
const POINTER: &str = "a JSON Pointer such as /items";

/// The most values a template may fill in, `$each` repeats counted, and the deepest it may nest:
/// a few levels of `$each` over long lists would otherwise grow without end.
pub(crate) const MAX_FILLED: usize = 100_000;
/// The deepest a template's JSON may nest.
pub(crate) const MAX_DEPTH: usize = 64;

/// Fills `template` from `data` and reads the result as a document. A filled template is a
/// document like any other, so it is checked the same way.
///
/// # Errors
/// `template-data-missing` for a pointer with no value; `template-data-type` for `$each` over
/// something that isn't a list, a `$data` or `$item` that finds an object or a list, or a
/// pointer that isn't one; `template-too-large` past 100,000 values filled in or 64 levels
/// of nesting; `invalid-document` for a placeholder object with another key, or for the filled
/// result. The pointer is in each error's data.
///
/// Added in 0.2.0.
pub fn fill(template: &Value, data: &Value) -> Result<FlowDocument> {
    let mut filler = Filler {
        data,
        items: Vec::new(),
        path: Vec::new(),
        filled: 0,
    };
    let document = filler.value(template)?;
    FlowDocument::from_json(&document)
}

/// `template` filled from `data` when there is data, else read as the document it is: what a
/// caller that takes either a document or a template and its data does (`laminapdf build
/// --data`, `builder.flow(document, { data })`).
///
/// # Errors
/// As [`fill`] with data; as [`FlowDocument::from_json`] without.
///
/// Added in 0.2.0.
pub fn fill_or_read(template: &Value, data: Option<&Value>) -> Result<FlowDocument> {
    match data {
        Some(data) => fill(template, data),
        None => FlowDocument::from_json(template),
    }
}

struct Filler<'a> {
    data: &'a Value,
    /// The current item of each `$each` the walk is inside, innermost last.
    items: Vec<&'a Value>,
    /// Where the walk is in the template, for errors about the template itself.
    path: Vec<String>,
    /// Values made so far.
    filled: usize,
}

/// The placeholder `map` is, and what it points with: `None` for an ordinary object.
fn placeholder(map: &Map<String, Value>) -> Option<(&'static str, &Value)> {
    [DATA, ITEM, EACH, IF]
        .into_iter()
        .find_map(|k| map.get(k).map(|v| (k, v)))
}

impl<'a> Filler<'a> {
    /// The template's pointer to where the walk is, with `key` after it.
    fn here(&self, key: &str) -> String {
        let mut pointer = String::new();
        for segment in self.path.iter().map(String::as_str).chain([key]) {
            pointer.push('/');
            pointer.push_str(&escape(segment));
        }
        pointer
    }

    /// A placeholder object's keys: `$data`/`$item` alone, `$each`/`$if` with `do`.
    fn check_keys(&self, map: &Map<String, Value>, key: &'static str) -> Result<()> {
        let allowed: &[&str] = if key == EACH || key == IF { &[key, DO] } else { &[key] };
        if let Some(extra) = map.keys().find(|k| !allowed.contains(&k.as_str())) {
            return Err(Error::InvalidDocument {
                pointer: self.here(extra),
                reason: "unknown-key",
                did_you_mean: Vec::new(),
            });
        }
        if allowed.len() == 2 && !map.contains_key(DO) {
            return Err(refuse(self.here(DO), "missing-key"));
        }
        Ok(())
    }

    /// What `pointer` finds: in the data, or in the current item for `$item` (or a
    /// `{ "$item": … }` pointer). `Ok(None)` for a well-formed pointer that finds nothing.
    fn lookup(&self, placeholder: &'static str, pointer: &Value) -> Result<Option<&'a Value>> {
        let (root, text) = match pointer {
            Value::String(p) if placeholder == ITEM => (self.items.last().copied(), p.as_str()),
            Value::String(p) => (Some(self.data), p.as_str()),
            Value::Object(map) if placeholder != DATA && placeholder != ITEM && map.len() == 1 => match map.get(ITEM) {
                Some(Value::String(p)) => (self.items.last().copied(), p.as_str()),
                _ => return Err(wrong_type(placeholder, pointer, POINTER)),
            },
            _ => return Err(wrong_type(placeholder, pointer, POINTER)),
        };
        if !(text.is_empty() || text.starts_with('/')) {
            return Err(wrong_type(placeholder, pointer, POINTER));
        }
        let Some(root) = root else {
            // `$item` outside every `$each`: there is no item to point into.
            return Err(missing(placeholder, text));
        };
        Ok(root.pointer(text))
    }

    /// Counts one more value made, refusing past the limits.
    fn count(&mut self) -> Result<()> {
        self.filled += 1;
        if self.filled > MAX_FILLED || self.path.len() > MAX_DEPTH {
            return Err(Error::TemplateTooLarge {
                limit: if self.filled > MAX_FILLED {
                    MAX_FILLED
                } else {
                    MAX_DEPTH
                },
            });
        }
        Ok(())
    }

    /// `value` with its placeholders filled.
    fn value(&mut self, value: &'a Value) -> Result<Value> {
        self.count()?;
        match value {
            Value::Array(items) => self.list(items).map(Value::Array),
            Value::Object(map) => {
                if let Some((key @ (DATA | ITEM), pointer)) = placeholder(map) {
                    return self.scalar(map, key, pointer);
                }
                // `$each`/`$if` outside a list have no list to add to: left as they are, the
                // document check refuses the unknown key.
                let mut out = Map::with_capacity(map.len());
                for (k, v) in map {
                    self.path.push(k.clone());
                    let filled = self.value(v);
                    self.path.pop();
                    out.insert(k.clone(), filled?);
                }
                Ok(Value::Object(out))
            }
            other => Ok(other.clone()),
        }
    }

    /// The text, number or `true`/`false` a `$data` or `$item` placeholder points to.
    fn scalar(&self, map: &Map<String, Value>, key: &'static str, pointer: &Value) -> Result<Value> {
        self.check_keys(map, key)?;
        match self.lookup(key, pointer)? {
            Some(found @ (Value::String(_) | Value::Number(_) | Value::Bool(_))) => Ok(found.clone()),
            Some(_) => Err(wrong_type(key, pointer, SCALAR)),
            None => Err(missing(key, &pointer_text(pointer))),
        }
    }

    /// A list with its `$each` and `$if` members expanded.
    fn list(&mut self, items: &'a [Value]) -> Result<Vec<Value>> {
        let mut out = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            self.path.push(i.to_string());
            let filled = self.member(item, &mut out);
            self.path.pop();
            filled?;
        }
        Ok(out)
    }

    /// Adds what the list member `item` makes to `out`.
    fn member(&mut self, item: &'a Value, out: &mut Vec<Value>) -> Result<()> {
        let Value::Object(map) = item else {
            out.push(self.value(item)?);
            return Ok(());
        };
        match placeholder(map) {
            Some((EACH, pointer)) => {
                self.check_keys(map, EACH)?;
                let Some(found) = self.lookup(EACH, pointer)? else {
                    return Err(missing(EACH, &pointer_text(pointer)));
                };
                let Value::Array(list) = found else {
                    return Err(wrong_type(EACH, pointer, "list"));
                };
                let body = map.get(DO).unwrap_or(&Value::Null);
                for each in list {
                    self.items.push(each);
                    let filled = self.body(body, out);
                    self.items.pop();
                    filled?;
                }
            }
            Some((IF, pointer)) => {
                self.check_keys(map, IF)?;
                // Only a well-formed pointer that finds nothing counts as false.
                if matches!(self.lookup(IF, pointer)?, Some(found) if !matches!(found, Value::Null | Value::Bool(false)))
                {
                    self.body(map.get(DO).unwrap_or(&Value::Null), out)?;
                }
            }
            _ => out.push(self.value(item)?),
        }
        Ok(())
    }

    /// Adds the members `do` makes to `out`: each of a list's, else the one value.
    fn body(&mut self, body: &'a Value, out: &mut Vec<Value>) -> Result<()> {
        self.path.push(DO.to_owned());
        // `do` adds to the list it stands in, so a lone value is a member too: an `$each` or `$if`
        // there expands as it would in a list.
        let filled = match body {
            Value::Array(items) => self.list(items).map(|items| out.extend(items)),
            other => self.member(other, out),
        };
        self.path.pop();
        filled
    }
}

fn missing(placeholder: &'static str, pointer: &str) -> Error {
    Error::TemplateDataMissing {
        pointer: pointer.to_owned(),
        placeholder,
    }
}

/// The pointer a placeholder gave, as text: the string, or the one inside `{ "$item": … }`.
fn pointer_text(pointer: &Value) -> String {
    match pointer {
        Value::String(p) => p.clone(),
        other => other.get(ITEM).and_then(Value::as_str).unwrap_or_default().to_owned(),
    }
}

fn wrong_type(placeholder: &'static str, pointer: &Value, expected: &'static str) -> Error {
    Error::TemplateDataType {
        pointer: pointer_text(pointer),
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
    fn item_finding_an_object_and_each_with_a_pointer_that_isn_t_one_are_wrong_types() {
        let item = document(&json!([{ "$each": "/a", "do": { "type": "paragraph", "text": { "$item": "" } } }]));
        let error = fill(&item, &json!({ "a": [{ "x": 1 }] })).unwrap_err();
        assert!(
            matches!(
                &error,
                Error::TemplateDataType {
                    placeholder: "$item",
                    ..
                }
            ),
            "{error:?}"
        );
        let each = document(&json!([{ "$each": 5, "do": { "type": "rule" } }]));
        let error = fill(&each, &json!({})).unwrap_err();
        assert!(
            matches!(
                &error,
                Error::TemplateDataType {
                    placeholder: "$each",
                    ..
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn if_with_a_malformed_pointer_fails_instead_of_dropping_its_block() {
        for pointer in [json!("paid"), json!(5)] {
            let template = document(&json!([{ "$if": pointer, "do": { "type": "rule" } }]));
            let error = fill(&template, &json!({ "paid": true })).unwrap_err();
            assert!(
                matches!(&error, Error::TemplateDataType { placeholder: "$if", .. }),
                "{error:?}"
            );
        }
        let item = document(&json!([{ "$if": { "$item": "/x" }, "do": { "type": "rule" } }]));
        assert!(matches!(
            fill(&item, &json!({})).unwrap_err(),
            Error::TemplateDataMissing { .. }
        ));
    }

    #[test]
    fn a_placeholder_holds_nothing_else_and_each_needs_do() {
        let extra = document(&json!([{ "type": "paragraph", "text": { "$data": "/x", "style": "body" } }]));
        let error = fill(&extra, &json!({ "x": "a" })).unwrap_err();
        assert!(
            matches!(&error, Error::InvalidDocument { pointer, reason: "unknown-key", .. } if pointer == "/blocks/0/text/style"),
            "{error:?}"
        );
        let no_do = document(&json!([{ "$each": "/x" }]));
        let error = fill(&no_do, &json!({ "x": [] })).unwrap_err();
        assert!(
            matches!(&error, Error::InvalidDocument { pointer, reason: "missing-key", .. } if pointer == "/blocks/0/do"),
            "{error:?}"
        );
    }

    #[test]
    fn nested_each_past_the_limit_is_too_large() {
        let mut body = json!({ "type": "rule" });
        for _ in 0..6 {
            body = json!({ "$each": "/ten", "do": [body] });
        }
        let template = document(&json!([body]));
        let error = fill(&template, &json!({ "ten": [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] })).unwrap_err();
        assert!(
            matches!(error, Error::TemplateTooLarge { limit: MAX_FILLED }),
            "{error:?}"
        );
    }

    #[test]
    fn a_nested_each_that_finds_nothing_names_its_item_pointer() {
        let template = document(&json!([{
            "$each": "/groups",
            "do": { "$each": { "$item": "/lines" }, "do": { "type": "rule" } }
        }]));
        let error = fill(&template, &json!({ "groups": [{ "lines": [1] }, {}] })).unwrap_err();
        assert!(
            matches!(&error, Error::TemplateDataMissing { pointer, placeholder: "$each" } if pointer == "/lines"),
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
