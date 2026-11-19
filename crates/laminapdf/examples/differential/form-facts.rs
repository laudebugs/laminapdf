//! Lamina PDF's form fields for every file in a list, as JSON: Lamina PDF's side of the nightly
//! lane's form differential against pypdf and pdf.js (`cargo xtask differential --live --forms`).
//!
//! The list has one `<name>\t<path>` line per file; the output maps each name to
//! `{outcome, error?, fields, orphan_widgets, parent_mismatches, has_xfa}`. `outcome` is
//! `opened` or `failed` (with Lamina PDF's `error` code). `fields` lists each terminal field of
//! `Document::form` with its fully qualified `name`, its `kind` and its `value`: a text, an
//! on-state name (`null` for `Off`), or a choice's export values. `orphan_widgets` counts the
//! widgets a page shows that no field reaches, and `parent_mismatches` the field tree's nodes
//! whose `/Parent` names another node.
//!
//! The other tools read some forms differently by design, and a field carries what lets the
//! differential tell those differences from a misread form: a button's `/Opt` export values
//! (`options`), the on-states its widgets show (`shown`), the options a choice's `/I` indices
//! select (`indices`), a text value read from a stream (`stream`), the text values its widgets
//! hold themselves (`widget_values`), a node up its `/Parent` chain without a name
//! (`unnamed_ancestor`), and whether none of its widgets is on a page (`widgetless`). A panic is
//! recorded as the outcome `panic` (a bug to report), not a crash of the whole run.
//!
//! It is not one of the façade's golden examples (`examples/*.rs`), so it lives in a subfolder
//! with its own `path` in `Cargo.toml`.
//!
//! ```sh
//! cargo run --release -p laminapdf --example form-facts -- <list.tsv> <out.json>
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use laminapdf::{CosValue, Document, FieldInfo, FieldKind, FieldValue, ObjectKind, OpenOptions};
use serde_json::{Map, Value, json};

/// How far up `/Parent` a field's chain is read, as the form view's own walk is bounded.
const MAX_DEPTH: usize = 32;

/// Lamina PDF's form facts about one file.
fn facts(bytes: Vec<u8>) -> Value {
    let failed = |e: &laminapdf::Error| json!({"outcome": "failed", "error": e.error_code().as_str()});
    let doc = match Document::open_bytes(bytes, OpenOptions::default()) {
        Ok(doc) => doc,
        Err(e) => return failed(&e),
    };
    let form = match doc.form() {
        Ok(form) => form,
        Err(e) => return failed(&e),
    };
    // The façade's view leaves these out; the model's view underneath counts them.
    let parent_mismatches = doc.ops().form().map_or(0, |f| f.parent_mismatches.len());
    let fields: Vec<Value> = form
        .fields
        .iter()
        .map(|field| {
            let mut out = Map::new();
            out.insert("name".to_owned(), json!(field.name));
            out.insert("kind".to_owned(), json!(field.kind.as_str()));
            out.insert("value".to_owned(), value(field));
            let chain = parent_chain(&doc, &field.id);
            let button = matches!(field.kind, FieldKind::CheckBox | FieldKind::RadioGroup);
            if button && !field.options.is_empty() {
                out.insert(
                    "options".to_owned(),
                    field.options.iter().map(|o| o.export.as_str()).collect(),
                );
            }
            let shown: Vec<&str> = field
                .widgets
                .iter()
                .filter_map(|w| w.state.as_deref())
                .filter(|s| *s != "Off")
                .collect();
            if button && !shown.is_empty() {
                out.insert("shown".to_owned(), json!(shown));
            }
            if let Some(CosValue::Array(indices)) = inherited(&chain, "I") {
                let selected = indices.iter().filter_map(|i| match i {
                    CosValue::Integer(i) => usize::try_from(*i).ok().and_then(|i| field.options.get(i)),
                    _ => None,
                });
                out.insert("indices".to_owned(), selected.map(|o| o.export.as_str()).collect());
            }
            if matches!(field.value, Some(FieldValue::Text(_)))
                && let Some(CosValue::Reference { num, .. }) = inherited(&chain, "V")
                && doc
                    .object(num)
                    .ok()
                    .flatten()
                    .is_some_and(|v| v.entry.kind == ObjectKind::Stream)
            {
                out.insert("stream".to_owned(), json!(true));
            }
            let widget_values: Vec<String> = field
                .widgets
                .iter()
                .filter(|w| w.id.get(1..) != field.id.get(1..))
                .filter_map(|w| match entry(&object_of(&doc, &w.id)?, "V")? {
                    CosValue::String { text, .. } => Some(text),
                    _ => None,
                })
                .collect();
            if !widget_values.is_empty() {
                out.insert("widget_values".to_owned(), json!(widget_values));
            }
            if chain.iter().skip(1).any(|node| entry(node, "T").is_none()) {
                out.insert("unnamed_ancestor".to_owned(), json!(true));
            }
            if form.widgetless.contains(&field.id) {
                out.insert("widgetless".to_owned(), json!(true));
            }
            Value::Object(out)
        })
        .collect();
    json!({
        "outcome": "opened",
        "fields": fields,
        "orphan_widgets": form.orphan_widgets.len(),
        "parent_mismatches": parent_mismatches,
        "has_xfa": form.has_xfa,
    })
}

/// A field's value as JSON: text, an on-state name (`null` for `Off`), or export values.
fn value(field: &FieldInfo) -> Value {
    match &field.value {
        Some(FieldValue::Text(text)) => json!(text),
        Some(FieldValue::Button(state)) => json!(state),
        Some(FieldValue::Choice(values)) => json!(values),
        Some(_) | None => Value::Null,
    }
}

/// Entry `key` of a dictionary.
fn entry(dict: &CosValue, key: &str) -> Option<CosValue> {
    match dict {
        CosValue::Dictionary(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()),
        _ => None,
    }
}

/// The value of the object an ID (`f12.0`, `a12.0`) names.
fn object_of(doc: &Document, id: &str) -> Option<CosValue> {
    let num = id.get(1..)?.split('.').next()?.parse().ok()?;
    doc.object(num).ok().flatten().map(|o| o.value)
}

/// The dictionaries from field `id`'s own up its `/Parent` chain, as far as the form view reads.
fn parent_chain(doc: &Document, id: &str) -> Vec<CosValue> {
    let mut chain: Vec<CosValue> = object_of(doc, id).into_iter().collect();
    while chain.len() < MAX_DEPTH {
        let parent = chain.last().and_then(|node| match entry(node, "Parent") {
            Some(CosValue::Reference { num, .. }) => doc.object(num).ok().flatten().map(|o| o.value),
            _ => None,
        });
        let Some(parent) = parent else { break };
        chain.push(parent);
    }
    chain
}

/// The nearest value of inheritable entry `key` up a field's chain (ISO 32000-2 §12.7.4.1).
fn inherited(chain: &[CosValue], key: &str) -> Option<CosValue> {
    chain.iter().find_map(|node| entry(node, key))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [list, out] = args.as_slice() else {
        panic!("usage: form-facts <list.tsv> <out.json>");
    };
    let list = std::fs::read_to_string(list).unwrap();
    // The panic message still goes to stderr; the run carries on with the next file.
    let mut all = Map::new();
    for line in list.lines().filter(|l| !l.is_empty()) {
        let (name, path) = line.split_once('\t').expect("<name>\\t<path> lines");
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        let value = catch_unwind(AssertUnwindSafe(|| facts(bytes))).unwrap_or_else(|_| json!({"outcome": "panic"}));
        all.insert(name.to_owned(), value);
    }
    let report = json!({
        "tool": "lamina",
        "version": env!("CARGO_PKG_VERSION"),
        "reports_repair": false,
        "facts": Value::Object(all),
    });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
}
