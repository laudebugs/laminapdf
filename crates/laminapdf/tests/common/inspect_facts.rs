//! Lamina PDF's inspection facts about a file in the shape the inspection differential compares
//! with qpdf's and pdffonts': shared by the `inspect-facts` example (the nightly lane's live run)
//! and `tests/inspect_differential.rs` (the PR lane's committed facts).
//!
//! `objects` maps each object number to `{gen, kind, type, at}`: `kind` is `stream`,
//! `dictionary` or `other`, `type` the `/Type` name (or `null`), `at` `offset:<n>` or
//! `stream:<objstm>:<index>`. `fonts` lists each font as `{object, name, family, embedded,
//! subset, to_unicode}`, sorted: `object` is `"<num> <gen>"` (`null` for a direct font),
//! `family` the `/Subtype` (`Type1` for `MMType1`).

use laminapdf::{Document, FontSummary, ObjectEntry, ObjectKind, ObjectLocation};
use serde_json::{Map, Value, json};

/// The objects of `doc`, keyed by number.
pub(crate) fn objects(doc: &Document) -> Result<Value, laminapdf::Error> {
    let mut out = Map::new();
    for entry in doc.objects()? {
        out.insert(entry.id.num.to_string(), object(&entry));
    }
    Ok(Value::Object(out))
}

fn object(entry: &ObjectEntry) -> Value {
    let kind = match entry.kind {
        ObjectKind::Stream => "stream",
        ObjectKind::Dictionary => "dictionary",
        _ => "other",
    };
    let at = match entry.location {
        ObjectLocation::Offset { offset, .. } => format!("offset:{offset}"),
        ObjectLocation::ObjectStream { stream, index } => format!("stream:{stream}:{index}"),
        _ => "created".to_owned(),
    };
    json!({"gen": entry.id.gen_nr, "kind": kind, "type": entry.type_name, "at": at})
}

/// The fonts of every page of `doc`, sorted.
pub(crate) fn fonts(doc: &Document) -> Result<Value, laminapdf::Error> {
    let mut list: Vec<Value> = doc.fonts(None)?.iter().map(font).collect();
    list.sort_by_key(ToString::to_string);
    Ok(Value::Array(list))
}

fn font(f: &FontSummary) -> Value {
    let family = match f.kind.as_deref() {
        Some("MMType1") => Some("Type1"),
        other => other,
    };
    json!({
        "object": f.object.map(|r| format!("{} {}", r.num, r.gen_nr)),
        "name": f.name,
        "family": family,
        "embedded": f.embedded,
        "subset": f.subset,
        "to_unicode": f.to_unicode,
    })
}
