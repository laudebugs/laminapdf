//! Lamina PDF's form fields for every file in a list, as JSON: Lamina PDF's side of the nightly
//! lane's form differential against pypdf and pdf.js (`cargo xtask differential --live --forms`).
//!
//! The list has one `<name>\t<path>` line per file; the output maps each name to
//! `{outcome, error?, fields, orphan_names, parent_names, has_xfa}`. `outcome` is
//! `opened` or `failed` (with Lamina PDF's `error` code). `fields` lists each terminal field of
//! `Document::form` with its fully qualified `name`, its `kind` and its `value`: a text, an
//! on-state name (`null` for `Off`), or a choice's export values. `orphan_names` gives the names
//! the `/Parent` chains of the widgets no field reaches give, and `parent_names` those of the
//! widgets below a node of the tree whose `/Parent` names another node: the names pdf.js, which
//! climbs `/Parent`, gives those widgets.
//!
//! The other tools read some forms differently by design, and a field carries what lets the
//! differential tell those differences from a misread form: a button's `/Opt` export values
//! (`options`), the on-states its widgets show (`shown`), the options a choice's `/I` indices
//! select (`indices`), a text value read from a stream (`stream`), the text values its widgets
//! hold themselves (`widget_values`), whether it has a mapping name (`mapping_name`, `/TM`), the name
//! a reader that climbs `/Parent` gives it below a node whose `/Parent` names another node
//! (`parent_name`), whether it is the node such a `/Parent` names (`parent_target`), whether it
//! is below a node of the tree without a name (`unnamed_ancestor`), and whether none of its widgets is on a page
//! (`widgetless`). A panic is
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
    // The façade's view leaves these out; the model's view underneath lists them.
    let mismatched: Vec<u32> = doc.ops().form().map_or_else(
        |_| Vec::new(),
        |f| {
            f.parent_mismatches
                .iter()
                .filter_map(|id| num_of(&id.to_string()))
                .collect()
        },
    );
    let below_mismatches = below(&doc, &mismatched);
    let widgets_under: Vec<u32> = below_mismatches
        .iter()
        .copied()
        .filter(|&n| is_widget(&doc, n))
        .collect();
    // The fields a mismatched node's /Parent names change too: a reader that climbs /Parent
    // puts the node below them.
    let targets: Vec<u32> = mismatched
        .iter()
        .filter_map(|&n| match entry(&doc.object(n).ok().flatten()?.value, "Parent")? {
            CosValue::Reference { num, .. } => Some(num),
            _ => None,
        })
        .collect();
    let under = below_mismatches;
    let unnamed = below_unnamed(&doc);
    let tree = Tree {
        under,
        targets,
        unnamed,
        widgetless: form.widgetless.clone(),
    };
    let fields: Vec<Value> = form
        .fields
        .iter()
        .map(|field| field_record(&doc, field, &tree))
        .collect();
    json!({
        "outcome": "opened",
        "fields": fields,
        "orphan_names": form.orphan_widgets.iter().filter_map(|id| name_up_parents(&doc, num_of(id)?)).collect::<Vec<_>>(),
        "parent_names": widgets_under.iter().filter_map(|&n| name_up_parents(&doc, n)).collect::<Vec<_>>(),
        "has_xfa": form.has_xfa,
    })
}

/// What the file's field tree says about each field's name, for the name classes.
struct Tree {
    /// The objects below a node whose `/Parent` names another node.
    under: Vec<u32>,
    /// The nodes such a `/Parent` names.
    targets: Vec<u32>,
    /// The objects below a node of the tree without `/T`.
    unnamed: Vec<u32>,
    /// The fields none of whose widgets a page shows.
    widgetless: Vec<String>,
}

/// One field's record: its name, kind and value, and the facts the classes read.
fn field_record(doc: &Document, field: &FieldInfo, tree: &Tree) -> Value {
    let mut out = Map::new();
    out.insert("name".to_owned(), json!(field.name));
    out.insert("kind".to_owned(), json!(field.kind.as_str()));
    out.insert("value".to_owned(), value(field));
    let chain = parent_chain(doc, &field.id);
    value_facts(doc, field, &chain, &mut out);
    let nums: Vec<u32> = std::iter::once(field.id.as_str())
        .chain(field.widgets.iter().map(|w| w.id.as_str()))
        .filter_map(num_of)
        .collect();
    if nums.iter().any(|n| tree.under.contains(n)) {
        // The name a reader that climbs /Parent gives it, from its first widget, as pdf.js does.
        let renamed = nums.get(1).or(nums.first()).and_then(|&n| name_up_parents(doc, n));
        out.insert("parent_name".to_owned(), json!(renamed));
    }
    if num_of(&field.id).is_some_and(|n| tree.targets.contains(&n)) {
        out.insert("parent_target".to_owned(), json!(true));
    }
    if chain.first().is_some_and(|node| entry(node, "TM").is_some()) {
        out.insert("mapping_name".to_owned(), json!(true));
    }
    if num_of(&field.id).is_some_and(|n| tree.unnamed.contains(&n)) {
        out.insert("unnamed_ancestor".to_owned(), json!(true));
    }
    if tree.widgetless.contains(&field.id) {
        out.insert("widgetless".to_owned(), json!(true));
    }
    Value::Object(out)
}

/// What explains where another reader finds a different value: `options`, `shown`,
/// `indices`, `stream` and `widget_values`.
fn value_facts(doc: &Document, field: &FieldInfo, chain: &[CosValue], out: &mut Map<String, Value>) {
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
    if let Some(CosValue::Array(indices)) = inherited(chain, "I") {
        let selected = indices.iter().filter_map(|i| match i {
            CosValue::Integer(i) => usize::try_from(*i).ok().and_then(|i| field.options.get(i)),
            _ => None,
        });
        out.insert("indices".to_owned(), selected.map(|o| o.export.as_str()).collect());
    }
    if matches!(field.value, Some(FieldValue::Text(_)))
        && let Some(CosValue::Reference { num, .. }) = inherited(chain, "V")
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
        .filter_map(|w| match entry(&object_of(doc, &w.id)?, "V")? {
            CosValue::String { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    if !widget_values.is_empty() {
        out.insert("widget_values".to_owned(), json!(widget_values));
    }
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

/// The object number an ID (`f12.0`, `a12.0`) names.
fn num_of(id: &str) -> Option<u32> {
    id.get(1..)?.split('.').next()?.parse().ok()
}

/// The value of the object an ID names.
fn object_of(doc: &Document, id: &str) -> Option<CosValue> {
    doc.object(num_of(id)?).ok().flatten().map(|o| o.value)
}

/// Whether object `num` is a widget annotation.
fn is_widget(doc: &Document, num: u32) -> bool {
    doc.object(num)
        .ok()
        .flatten()
        .and_then(|o| entry(&o.value, "Subtype"))
        .is_some_and(|s| s == CosValue::Name("Widget".to_owned()))
}

/// The objects `roots` and the nodes below them down `/Kids` reach, each once, within the
/// form view's depth.
fn below(doc: &Document, roots: &[u32]) -> Vec<u32> {
    let mut seen: Vec<u32> = Vec::new();
    let mut stack: Vec<(u32, usize)> = roots.iter().map(|&n| (n, 0)).collect();
    while let Some((num, depth)) = stack.pop() {
        if depth >= MAX_DEPTH || seen.contains(&num) {
            continue;
        }
        seen.push(num);
        let kids = doc.object(num).ok().flatten().and_then(|o| entry(&o.value, "Kids"));
        if let Some(CosValue::Array(kids)) = kids {
            for kid in kids {
                if let CosValue::Reference { num, .. } = kid {
                    stack.push((num, depth + 1));
                }
            }
        }
    }
    seen
}

/// The objects of the field tree (`/AcroForm /Fields` down `/Kids`) below a node without `/T`,
/// and the fields a widget listed in `/Fields` stands for.
fn below_unnamed(doc: &Document) -> Vec<u32> {
    let root = match entry(&doc.trailer(), "Root") {
        Some(CosValue::Reference { num, .. }) => doc.object(num).ok().flatten().map(|o| o.value),
        _ => None,
    };
    let form = root.and_then(|r| match entry(&r, "AcroForm")? {
        CosValue::Reference { num, .. } => doc.object(num).ok().flatten().map(|o| o.value),
        direct => Some(direct),
    });
    let refs = |value: Option<CosValue>| -> Vec<u32> {
        match value {
            Some(CosValue::Array(items)) => items
                .into_iter()
                .filter_map(|i| match i {
                    CosValue::Reference { num, .. } => Some(num),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    let mut out = Vec::new();
    let mut seen = Vec::new();
    let mut stack: Vec<(u32, bool, usize)> = refs(form.and_then(|f| entry(&f, "Fields")))
        .into_iter()
        .map(|n| (n, false, 0))
        .collect();
    while let Some((num, unnamed_above, depth)) = stack.pop() {
        if depth >= MAX_DEPTH || seen.contains(&num) {
            continue;
        }
        seen.push(num);
        if unnamed_above {
            out.push(num);
        }
        let Some(node) = doc.object(num).ok().flatten().map(|o| o.value) else {
            continue;
        };
        let unnamed = unnamed_above || entry(&node, "T").is_none();
        stack.extend(refs(entry(&node, "Kids")).into_iter().map(|k| (k, unnamed, depth + 1)));
        // A widget `/Fields` lists in place of its field stands for the field its `/Parent`
        // names, as the form view reads it: the field is reached through a node without `/T`.
        if depth == 0
            && entry(&node, "T").is_none()
            && entry(&node, "FT").is_none()
            && is_widget(doc, num)
            && let Some(CosValue::Reference { num: parent, .. }) = entry(&node, "Parent")
        {
            stack.push((parent, true, depth + 1));
        }
    }
    out
}

/// The name a reader that climbs `/Parent` gives object `num` (pdf.js, ISO 32000-2 §12.7.4.2):
/// the `/T` of it and of each node up its chain, outermost first; `None` when none has one.
fn name_up_parents(doc: &Document, num: u32) -> Option<String> {
    let chain = parent_chain(doc, &format!("f{num}.0"));
    let parts: Vec<String> = chain
        .iter()
        .rev()
        .filter_map(|node| match entry(node, "T") {
            Some(CosValue::String { text, .. }) => Some(text),
            _ => None,
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join("."))
}

/// The dictionaries from field `id`'s own up its `/Parent` chain, each once, as far as the form
/// view reads.
fn parent_chain(doc: &Document, id: &str) -> Vec<CosValue> {
    let mut chain: Vec<CosValue> = object_of(doc, id).into_iter().collect();
    // A /Parent that loops back stops the climb, as it stops pdf.js's.
    let mut seen: Vec<u32> = num_of(id).into_iter().collect();
    while chain.len() < MAX_DEPTH {
        let parent = chain.last().and_then(|node| match entry(node, "Parent") {
            Some(CosValue::Reference { num, .. }) if !seen.contains(&num) => {
                seen.push(num);
                doc.object(num).ok().flatten().map(|o| o.value)
            }
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
