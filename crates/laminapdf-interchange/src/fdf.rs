//! FDF (ISO 32000-2 §12.7.8): PDF syntax with an `/FDF` dictionary under the trailer's `/Root`.
//! Read by scanning for object headers with `laminapdf-syntax`'s tolerant parser, as a damaged
//! PDF is recovered, so a missing or wrong cross-reference table doesn't matter; written with
//! the objects numbered in order.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object, PdfString, Stream};
use laminapdf_model::UserSpaceRect;
use laminapdf_syntax::text::{decode_text_string, encode_text_string};
use laminapdf_syntax::{DecodeBudget, Limits, Parser, find};

use crate::exchange::{Comment, Exchange, ExchangedFile, Popup, SkipCause, SkippedComment, text};
use crate::fields::{DataValue, FieldData, FormDataFormat, NameTree, qualified};
use crate::lines::Lines;
use crate::{InterchangeError, Result};

/// The objects of an FDF file by number, and where each starts.
struct Objects {
    lines: Lines,
    by_ref: BTreeMap<ObjRef, (Object, usize)>,
}

impl Objects {
    /// `o`, or the object it refers to.
    fn resolve<'o>(&'o self, o: &'o Object) -> &'o Object {
        match o {
            Object::Ref(r) => self.by_ref.get(r).map_or(&Object::Null, |(o, _)| o),
            other => other,
        }
    }

    fn dict<'o>(&'o self, o: Option<&'o Object>) -> Option<&'o Dict> {
        match o.map(|o| self.resolve(o)) {
            Some(Object::Dict(d)) => Some(d),
            Some(Object::Stream(s)) => Some(&s.dict),
            _ => None,
        }
    }

    fn line_of(&self, r: Option<ObjRef>) -> Option<u32> {
        let (_, at) = self.by_ref.get(&r?)?;
        Some(self.lines.at(*at))
    }
}

/// Reads an FDF file.
///
/// # Errors
/// `interchange-too-large` past `limits.max_stream_decoded()`; `interchange-malformed`, with
/// its line, for a file without its `/FDF` dictionary or with an object that doesn't parse.
pub(crate) fn read(bytes: &[u8], limits: &Limits) -> Result<Exchange> {
    if bytes.len() > limits.max_stream_decoded() {
        return Err(InterchangeError::TooLarge);
    }
    let objects = scan(bytes, limits)?;
    // The last trailer: an update appended to the file adds one after the first.
    let trailer_at = std::iter::successors(find(bytes, b"trailer", 0), |&at| find(bytes, b"trailer", at + 1))
        .last()
        .ok_or_else(|| objects.lines.malformed(bytes.len(), "it has no trailer"))?;
    let trailer = Parser::new(bytes, trailer_at + b"trailer".len())
        .with_limits(limits)
        .object()
        .map_err(|_| objects.lines.malformed(trailer_at, "its trailer doesn't parse"))?;
    let root = objects.dict(trailer.as_dict().and_then(|t| t.get("Root")));
    let fdf = objects
        .dict(root.and_then(|r| r.get("FDF")))
        .ok_or_else(|| objects.lines.malformed(trailer_at, "it has no FDF dictionary"))?;
    let mut out = Exchange {
        file: file_name(&objects, fdf.get("F")),
        ids: ids(&objects, fdf.get("ID")),
        ..Exchange::default()
    };
    let fields = objects
        .resolve(fdf.get("Fields").unwrap_or(&Object::Null))
        .as_array()
        .unwrap_or_default();
    for field in fields {
        out.skipped.push(SkippedComment {
            page: None,
            kind: "field".to_owned(),
            cause: SkipCause::FormField,
            line: objects.line_of(field.as_ref()),
        });
    }
    let mut seen = BTreeSet::new();
    let mut read = Vec::new();
    read_fields(&objects, fields, "", limits.max_tree_depth(), &mut seen, &mut read);
    out.fields = crate::fields::one_per_name(read);
    let annots = objects
        .resolve(fdf.get("Annots").unwrap_or(&Object::Null))
        .as_array()
        .unwrap_or_default();
    // A parent without `/NM` is named after its object, so its replies can find it.
    let names: BTreeMap<ObjRef, String> = annots
        .iter()
        .filter_map(Object::as_ref)
        .map(|r| {
            let name = objects.dict(Some(&Object::Ref(r))).and_then(|d| text(d, "NM"));
            (r, name.unwrap_or_else(|| format!("fdf-{}", r.num)))
        })
        .collect();
    let mut budget = DecodeBudget::new(limits);
    for annot in annots {
        read_annot(&objects, annot, &names, limits, &mut budget, &mut out);
    }
    Ok(out)
}

/// The fields of `kids` and below them (ISO 32000-2 §12.7.8.3.2, Table 246), named under
/// `prefix`: `/T` is the partial name, `/V` the value, `/Kids` the fields below. A field reached
/// twice, or deeper than `depth`, is left out.
fn read_fields(
    objects: &Objects,
    kids: &[Object],
    prefix: &str,
    depth: usize,
    seen: &mut BTreeSet<ObjRef>,
    out: &mut Vec<FieldData>,
) {
    let Some(depth) = depth.checked_sub(1) else { return };
    for kid in kids {
        if let Some(r) = kid.as_ref()
            && !seen.insert(r)
        {
            continue;
        }
        let Some(dict) = objects.dict(Some(kid)) else { continue };
        let Some(name) = text(dict, "T") else { continue };
        let name = qualified(prefix, &name);
        let below = objects
            .resolve(dict.get("Kids").unwrap_or(&Object::Null))
            .as_array()
            .unwrap_or_default();
        let value = match dict.get("V").map(|v| objects.resolve(v)) {
            Some(Object::String(s)) => Some(DataValue::Text(decode_text_string(&s.bytes))),
            Some(Object::Name(n)) => Some(DataValue::State(String::from_utf8_lossy(n.as_bytes()).into_owned())),
            Some(Object::Array(items)) => Some(DataValue::Choices(
                items
                    .iter()
                    .filter_map(|i| match objects.resolve(i) {
                        Object::String(s) => Some(decode_text_string(&s.bytes)),
                        _ => None,
                    })
                    .collect(),
            )),
            // A text stream, or a value of no kind a field takes: the field is left out.
            Some(_) => None,
            None => below.is_empty().then_some(DataValue::Empty),
        };
        if let Some(value) = value {
            out.push(FieldData::new(name.clone(), value));
        }
        read_fields(objects, below, &name, depth, seen, out);
    }
}

/// `fields` as FDF field dictionaries, nested by their names' parts (Table 246).
fn field_dicts(node: &NameTree<'_>) -> Vec<Object> {
    node.children
        .iter()
        .map(|(part, child)| {
            let mut dict = Dict::new();
            dict.insert("T", Object::String(PdfString::literal(encode_text_string(part))));
            let string = |t: &str| Object::String(PdfString::literal(encode_text_string(t)));
            match child.value {
                Some(DataValue::Text(t)) => {
                    dict.insert("V", string(t));
                }
                Some(DataValue::State(s)) => {
                    dict.insert("V", Object::name(s));
                }
                Some(DataValue::Checked(on)) => {
                    dict.insert("V", Object::name(if *on { "Yes" } else { "Off" }));
                }
                Some(DataValue::Choices(values)) => {
                    dict.insert("V", Object::Array(values.iter().map(|v| string(v)).collect()));
                }
                Some(DataValue::Empty) | None => {}
            }
            if !child.children.is_empty() {
                dict.insert("Kids", Object::Array(field_dicts(child)));
            }
            Object::Dict(dict)
        })
        .collect()
}

/// Every `num gen obj … endobj` in `bytes`; a later one with the same number wins, as in an
/// incremental update.
fn scan(bytes: &[u8], limits: &Limits) -> Result<Objects> {
    let mut by_ref = BTreeMap::new();
    let mut from = 0;
    while let Some(at) = find(bytes, b"obj", from) {
        from = at + 3;
        // `endobj` ends one; only a header's `obj` stands after a number.
        if at >= 3 && bytes.get(at - 3..at) == Some(b"end".as_slice()) {
            continue;
        }
        let start = header_start(bytes, at);
        let mut parser = Parser::new(bytes, start).with_limits(limits);
        let Ok(object) = parser.indirect_object(&mut |_| None) else {
            continue;
        };
        if by_ref.len() >= limits.max_object_number() as usize {
            return Err(InterchangeError::TooLarge);
        }
        from = from.max(parser.pos());
        by_ref.insert(object.id, (object.object, start));
    }
    Ok(Objects {
        lines: Lines::new(bytes, FormDataFormat::Fdf),
        by_ref,
    })
}

/// Where the `num gen` before the `obj` at `at` starts: two numbers back over white space.
fn header_start(bytes: &[u8], at: usize) -> usize {
    let mut i = at;
    for _ in 0..2 {
        while i > 0 && bytes.get(i - 1).is_some_and(u8::is_ascii_whitespace) {
            i -= 1;
        }
        while i > 0 && bytes.get(i - 1).is_some_and(u8::is_ascii_digit) {
            i -= 1;
        }
    }
    i
}

/// `/F`: a file specification string, or a dictionary's `/UF` or `/F`.
fn file_name(objects: &Objects, f: Option<&Object>) -> Option<String> {
    match objects.resolve(f?) {
        Object::String(s) => Some(decode_text_string(&s.bytes)),
        Object::Dict(d) => text(d, "UF").or_else(|| text(d, "F")),
        _ => None,
    }
}

fn ids(objects: &Objects, id: Option<&Object>) -> Option<[Vec<u8>; 2]> {
    match objects.resolve(id?).as_array()? {
        [Object::String(a), Object::String(b)] => Some([a.bytes.clone(), b.bytes.clone()]),
        _ => None,
    }
}

fn read_annot(
    objects: &Objects,
    annot: &Object,
    names: &BTreeMap<ObjRef, String>,
    limits: &Limits,
    budget: &mut DecodeBudget,
    out: &mut Exchange,
) {
    let line = objects.line_of(annot.as_ref());
    let Some(dict) = objects.dict(Some(annot)) else {
        return;
    };
    let subtype = dict
        .get_name("Subtype")
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .unwrap_or_default();
    let page = dict.get_int("Page").and_then(|p| u32::try_from(p).ok());
    // A Popup comes with its parent.
    if subtype == "Popup" {
        return;
    }
    let skip = |out: &mut Exchange, cause| {
        out.skipped.push(SkippedComment {
            page,
            kind: subtype.clone(),
            cause,
            line,
        });
    };
    if !crate::map::IMPORTED.contains(&subtype.as_str()) {
        return skip(out, SkipCause::Unsupported);
    }
    let Some(page) = page else {
        return skip(out, SkipCause::Incomplete);
    };
    let mut comment = Comment::new(page, &subtype);
    comment.line = line;
    for (key, value) in dict.iter() {
        let key_text = String::from_utf8_lossy(key.as_bytes());
        if !crate::map::LEFT_OUT.contains(&key_text.as_ref()) {
            comment.entries.insert(key.clone(), value.clone());
        }
    }
    if let Some(r) = annot.as_ref()
        && !comment.entries.contains_key("NM")
        && let Some(name) = names.get(&r)
    {
        comment
            .entries
            .insert("NM", Object::String(PdfString::literal(encode_text_string(name))));
    }
    comment.in_reply_to = match dict.get("IRT") {
        Some(Object::Ref(r)) => names.get(r).cloned(),
        _ => None,
    };
    if let Some(p) = objects.dict(dict.get("Popup")) {
        let rect = p
            .get("Rect")
            .and_then(Object::as_array)
            .and_then(crate::map::fixed::<4>);
        if let Some([x0, y0, x1, y1]) = rect {
            let rect = UserSpaceRect::from_corners(x0, y0, x1, y1);
            comment.popup = Some(Popup::new(rect, matches!(p.get("Open"), Some(Object::Bool(true)))));
        }
    }
    if subtype == "FileAttachment" {
        comment.file = embedded(objects, dict.get("FS"), limits, budget);
        if comment.file.is_none() {
            return skip(out, SkipCause::Incomplete);
        }
    }
    out.comments.push(comment);
}

/// A file specification's embedded file.
fn embedded(
    objects: &Objects,
    fs: Option<&Object>,
    limits: &Limits,
    budget: &mut DecodeBudget,
) -> Option<ExchangedFile> {
    let spec = objects.dict(fs)?;
    let ef = objects.dict(spec.get("EF"))?;
    let Object::Stream(stream) = objects.resolve(ef.get("UF").or_else(|| ef.get("F"))?) else {
        return None;
    };
    let bytes = laminapdf_syntax::filters::decode(&stream.dict, &stream.data, limits, budget)
        .ok()?
        .data;
    let params = objects.dict(stream.dict.get("Params"));
    let date = |key| params.and_then(|p| text(p, key));
    Some(ExchangedFile {
        name: text(spec, "UF").or_else(|| text(spec, "F")).unwrap_or_default(),
        mime: stream
            .dict
            .get_name("Subtype")
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .unwrap_or_default(),
        bytes,
        description: text(spec, "Desc"),
        created: date("CreationDate"),
        modified: date("ModDate"),
    })
}

/// Writes `exchange` as FDF.
pub(crate) fn write(exchange: &Exchange) -> Vec<u8> {
    // Each comment is object 2 + its index; Popups and embedded files follow them all.
    let first = 2_u32;
    let count = u32::try_from(exchange.comments.len()).unwrap_or(u32::MAX);
    let number_of: BTreeMap<String, u32> = exchange
        .comments
        .iter()
        .zip(first..)
        .filter_map(|(c, n)| Some((c.name()?, n)))
        .collect();
    let mut next = first.saturating_add(count);
    let mut objects: Vec<(u32, Object)> = Vec::new();
    let mut refs = Vec::new();
    for (c, n) in exchange.comments.iter().zip(first..) {
        refs.push(Object::Ref(ObjRef::new(n, 0)));
        let mut dict = Dict::new();
        dict.insert("Type", Object::name("Annot"));
        // In key order, so the same comment writes the same bytes however it was read.
        let mut entries: Vec<_> = c.entries.iter().collect();
        entries.sort_by(|(a, _), (b, _)| a.as_bytes().cmp(b.as_bytes()));
        for (key, value) in entries {
            dict.insert(key.clone(), value.clone());
        }
        dict.insert("Page", Object::Integer(i64::from(c.page)));
        if let Some(parent) = c.in_reply_to.as_ref().and_then(|p| number_of.get(p)) {
            dict.insert("IRT", Object::Ref(ObjRef::new(*parent, 0)));
        }
        if let Some(p) = &c.popup {
            let popup = next;
            next += 1;
            let mut pd = Dict::new();
            pd.insert("Type", Object::name("Annot"));
            pd.insert("Subtype", Object::name("Popup"));
            pd.insert(
                "Rect",
                Object::Array(
                    [p.rect.x0, p.rect.y0, p.rect.x1, p.rect.y1]
                        .map(Object::number)
                        .to_vec(),
                ),
            );
            pd.insert("Open", Object::Bool(p.open));
            pd.insert("Parent", Object::Ref(ObjRef::new(n, 0)));
            pd.insert("Page", Object::Integer(i64::from(c.page)));
            dict.insert("Popup", Object::Ref(ObjRef::new(popup, 0)));
            objects.push((popup, Object::Dict(pd)));
        }
        if let Some(f) = &c.file {
            let stream = next;
            next += 1;
            dict.insert("FS", Object::Dict(file_spec(f, stream)));
            objects.push((stream, Object::Stream(embedded_stream(f))));
        }
        objects.push((n, Object::Dict(dict)));
    }
    let mut fdf = Dict::new();
    // Form data alone has no `/Annots`; comments, or nothing at all, do.
    if !exchange.comments.is_empty() || exchange.fields.is_empty() {
        fdf.insert("Annots", Object::Array(refs));
    }
    if !exchange.fields.is_empty() {
        fdf.insert("Fields", Object::Array(field_dicts(&NameTree::of(&exchange.fields))));
    }
    if let Some(file) = &exchange.file {
        fdf.insert("F", Object::String(PdfString::literal(encode_text_string(file))));
    }
    if let Some([a, b]) = &exchange.ids {
        fdf.insert(
            "ID",
            Object::Array(vec![
                Object::String(PdfString::hex(a.clone())),
                Object::String(PdfString::hex(b.clone())),
            ]),
        );
    }
    let mut catalog = Dict::new();
    catalog.insert("FDF", Object::Dict(fdf));
    objects.push((1, Object::Dict(catalog)));
    objects.sort_by_key(|(n, _)| *n);
    let mut out = b"%FDF-1.2\n%\xE2\xE3\xCF\xD3\n".to_vec();
    for (n, o) in objects {
        out.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        o.write_to(&mut out);
        out.extend_from_slice(b"\nendobj\n");
    }
    out.extend_from_slice(b"trailer\n<< /Root 1 0 R >>\n%%EOF\n");
    out
}

fn file_spec(f: &ExchangedFile, stream: u32) -> Dict {
    let name = Object::String(PdfString::literal(encode_text_string(&f.name)));
    let mut ef = Dict::new();
    ef.insert("F", Object::Ref(ObjRef::new(stream, 0)));
    let mut spec = Dict::new();
    spec.insert("Type", Object::name("Filespec"));
    spec.insert("F", name.clone());
    spec.insert("UF", name);
    if let Some(d) = &f.description {
        spec.insert("Desc", Object::String(PdfString::literal(encode_text_string(d))));
    }
    spec.insert("EF", Object::Dict(ef));
    spec
}

fn embedded_stream(f: &ExchangedFile) -> Stream {
    let mut dict = Dict::new();
    dict.insert("Type", Object::name("EmbeddedFile"));
    if !f.mime.is_empty() {
        dict.insert("Subtype", Object::name(&f.mime));
    }
    let mut params = Dict::new();
    params.insert(
        "Size",
        Object::Integer(i64::try_from(f.bytes.len()).unwrap_or(i64::MAX)),
    );
    for (key, date) in [("CreationDate", &f.created), ("ModDate", &f.modified)] {
        if let Some(d) = date {
            params.insert(key, Object::String(PdfString::literal(d.as_bytes().to_vec())));
        }
    }
    dict.insert("Params", Object::Dict(params));
    Stream::flate(dict, &f.bytes)
}
