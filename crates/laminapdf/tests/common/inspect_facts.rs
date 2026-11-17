//! Lamina PDF's inspection facts about a file in the shape the inspection differential compares
//! with qpdf's and pdffonts': shared by the `inspect-facts` example (the nightly lane's live run)
//! and `tests/inspect_differential.rs` (the PR lane's committed facts).
//!
//! `objects` maps each object number to `{gen, kind, type, subtype, at}`: `kind` is `stream`,
//! `dictionary` or `other`, `type` the `/Type` name and `subtype` the `/Subtype` (else `/S`)
//! name (or `null`), `at` `offset:<n>` or `stream:<objstm>:<index>`. `xref` is the newest
//! cross-reference section, through the trailer: `{kind, size, prev}`, `kind` `table`, `stream`
//! or `hybrid` (a table whose trailer names an `/XRefStm`). `fonts` lists each font as
//! `{object, name, family, encoding, embedded, subset, to_unicode}`, sorted: `object` is
//! `"<num> <gen>"` (`null` for a direct font), `family` the `/Subtype` (`Type1` for `MMType1`),
//! `encoding` as pdffonts spells it (`WinAnsi` for `/WinAnsiEncoding`, `Custom` for
//! differences or a CMap stream, the font's own for none).

use laminapdf::{CosValue, Document, FontEncoding, FontSummary, ObjectEntry, ObjectKind, ObjectLocation};
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
    json!({"gen": entry.id.gen_nr, "kind": kind, "type": entry.type_name, "subtype": entry.subtype, "at": at})
}

/// The newest cross-reference section of `doc`, through its trailer.
pub(crate) fn xref(doc: &Document) -> Value {
    let CosValue::Dictionary(trailer) = doc.trailer() else {
        return Value::Null;
    };
    let get = |key: &str| trailer.iter().find(|(k, _)| k == key).map(|(_, v)| v);
    let int = |key: &str| match get(key) {
        Some(CosValue::Integer(n)) => u64::try_from(*n).ok(),
        _ => None,
    };
    let kind = match (get("Type"), get("XRefStm")) {
        (Some(CosValue::Name(n)), _) if n == "XRef" => "stream",
        (_, Some(_)) => "hybrid",
        _ => "table",
    };
    json!({"kind": kind, "size": int("Size"), "prev": int("Prev")})
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
        "encoding": encoding(f),
        "embedded": f.embedded,
        "subset": f.subset,
        "to_unicode": f.to_unicode,
    })
}

/// A font's encoding as pdffonts' column spells it: a named encoding without its `Encoding`
/// suffix, a CMap by its name, `Custom` for differences or an embedded CMap. A font without
/// `/Encoding` uses its own (§9.6.5): pdffonts names a standard 14 Type 1 font's (`Symbol`,
/// `ZapfDingbats`, else `Standard`) and calls an embedded font's `Builtin`.
fn encoding(f: &FontSummary) -> String {
    match &f.encoding {
        Some(FontEncoding::Named(name)) => name.strip_suffix("Encoding").unwrap_or(name).to_owned(),
        Some(_) => "Custom".to_owned(),
        None => match (f.embedded, f.kind.as_deref(), f.name.as_deref()) {
            (false, Some("Type1" | "MMType1"), Some(name @ ("Symbol" | "ZapfDingbats"))) => name.to_owned(),
            (false, Some("Type1" | "MMType1"), _) => "Standard".to_owned(),
            _ => "Builtin".to_owned(),
        },
    }
}
