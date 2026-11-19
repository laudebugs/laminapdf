//! XFDF (ISO 19444-1): read with the guarded reader of `laminapdf_model::xml`, which refuses a
//! DTD and every entity but XML's five, and written by hand.

use std::fmt::Write as _;

use laminapdf_cos::{Dict, Object, PdfString};
use laminapdf_model::UserSpaceRect;
use laminapdf_model::xml::{self, Element, Node};
use laminapdf_model::xmp::escape;
use laminapdf_syntax::text::encode_text_string;
use laminapdf_syntax::{DecodeBudget, Limits, format_real};

use crate::exchange::{Comment, Exchange, ExchangedFile, Popup, SkipCause, SkippedComment, text};
use crate::fields::{DataValue, FieldData, NameTree, qualified};
use crate::lines::Lines;
use crate::map::{self, element_of, joined, numbers, read_attribute, read_border, subtype_of};
use crate::{InterchangeError, Result};

/// The file being read: its text and where its lines start.
struct Source<'t> {
    text: &'t str,
    lines: Lines,
}

/// XFDF's namespace.
pub(crate) const NAMESPACE: &str = "http://ns.adobe.com/xfdf/";

/// Reads an XFDF file.
///
/// # Errors
/// `interchange-too-large` past `limits.max_stream_decoded()`; `interchange-dtd` for a DTD;
/// `interchange-malformed`, with its line, for a file that isn't well-formed XFDF.
pub(crate) fn read(bytes: &[u8], limits: &Limits) -> Result<Exchange> {
    if bytes.len() > limits.max_stream_decoded() {
        return Err(InterchangeError::TooLarge);
    }
    let lines = Lines::new(bytes, "XFDF");
    let text = std::str::from_utf8(bytes).map_err(|e| lines.malformed(e.valid_up_to(), "it isn't UTF-8"))?;
    let src = Source { text, lines };
    let top = xml::parse(text, |_| None).map_err(|e| {
        let offset = usize::try_from(e.offset()).unwrap_or(usize::MAX);
        match e {
            xml::XmlError::Dtd { .. } => InterchangeError::Dtd {
                line: src.lines.at(offset),
            },
            xml::XmlError::TooDeep { .. } | xml::XmlError::TooLarge { .. } => InterchangeError::TooLarge,
            xml::XmlError::UndefinedEntity { .. } => {
                src.lines.malformed(offset, "it refers to an entity XML doesn't define")
            }
            _ => src.lines.malformed(offset, "it isn't well-formed XML"),
        }
    })?;
    let root = top
        .iter()
        .find_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
        .filter(|e| e.name.local == "xfdf")
        .ok_or_else(|| src.lines.malformed(0, "it has no xfdf element"))?;
    let mut budget = DecodeBudget::new(limits);
    let mut out = Exchange::default();
    for child in root.child_elements() {
        match child.name.local.as_str() {
            "f" => out.file = attr(child, "href").map(str::to_owned),
            "ids" => out.ids = ids(child),
            "annots" => {
                for annot in child.child_elements() {
                    read_annot(&src, annot, limits, &mut budget, &mut out)?;
                }
            }
            "fields" => {
                skip_fields(&src, child, &mut out);
                read_fields(child, "", &mut out.fields);
            }
            _ => {}
        }
    }
    Ok(out)
}

/// An unprefixed attribute's value.
fn attr<'e>(e: &'e Element, local: &str) -> Option<&'e str> {
    e.attr("", local).map(|a| a.value.as_str())
}

/// `<ids original="…" modified="…"/>`, in hexadecimal.
fn ids(e: &Element) -> Option<[Vec<u8>; 2]> {
    let hex = |name| attr(e, name).and_then(from_hex);
    Some([hex("original")?, hex("modified")?])
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    let digits: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    digits
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02X}");
        s
    })
}

/// Each `<field>` is a form field's value, which comment import leaves out.
fn skip_fields(src: &Source<'_>, fields: &Element, out: &mut Exchange) {
    for field in fields.child_elements().filter(|f| f.name.local == "field") {
        out.skipped.push(SkippedComment {
            page: None,
            kind: "field".to_owned(),
            cause: SkipCause::FormField,
            line: Some(src.lines.at(field.span.start)),
        });
    }
}

/// Each `<field>` below `parent` (ISO 19444-1 §7.4), named under `prefix`: one with `<value>`
/// elements holds a value, several for a multi-select list; one with neither values nor fields
/// below it is empty. The XML reader's depth limit bounds the nesting.
fn read_fields(parent: &Element, prefix: &str, out: &mut Vec<FieldData>) {
    for field in parent.child_elements().filter(|f| f.name.local == "field") {
        let Some(name) = attr(field, "name") else { continue };
        let name = qualified(prefix, name);
        let values: Vec<String> = field
            .child_elements()
            .filter(|v| v.name.local == "value")
            .map(Element::text)
            .collect();
        let has_kids = field.child_elements().any(|f| f.name.local == "field");
        let value = match <[String; 1]>::try_from(values) {
            Ok([one]) => Some(DataValue::Text(one)),
            Err(values) if values.is_empty() => (!has_kids).then_some(DataValue::Empty),
            Err(values) => Some(DataValue::Choices(values)),
        };
        if let Some(value) = value {
            out.push(FieldData::new(name.clone(), value));
        }
        read_fields(field, &name, out);
    }
}

/// Writes `fields` as `<field>` elements nested by their names' parts.
fn write_fields(out: &mut String, node: &NameTree<'_>) {
    for (part, child) in &node.children {
        let _ = write!(out, "<field name=\"{}\">", escape(part));
        match child.value {
            Some(DataValue::Text(t) | DataValue::State(t)) => {
                let _ = write!(out, "<value>{}</value>", escape(t));
            }
            Some(DataValue::Checked(on)) => {
                let _ = write!(out, "<value>{}</value>", if *on { "Yes" } else { "Off" });
            }
            Some(DataValue::Choices(values)) => {
                for v in values {
                    let _ = write!(out, "<value>{}</value>", escape(v));
                }
            }
            Some(DataValue::Empty) | None => {}
        }
        if !child.children.is_empty() {
            out.push('\n');
            write_fields(out, child);
        }
        out.push_str("</field>\n");
    }
}

/// The attributes read apart from the table: the page, what it answers, a line's ends, the
/// border's, and a file's facts.
const SPECIAL: [&str; 15] = [
    "page",
    "inreplyto",
    "start",
    "end",
    "head",
    "tail",
    "width",
    "style",
    "dashes",
    "intensity",
    "file",
    "mimetype",
    "size",
    "creation",
    "modification",
];

fn read_annot(
    src: &Source<'_>,
    e: &Element,
    limits: &Limits,
    budget: &mut DecodeBudget,
    out: &mut Exchange,
) -> Result<()> {
    let line = src.lines.at(e.span.start);
    let skip = |out: &mut Exchange, page, cause| {
        out.skipped.push(SkippedComment {
            page,
            kind: subtype_of(&e.name.local).unwrap_or(&e.name.local).to_owned(),
            cause,
            line: Some(line),
        });
    };
    let page = attr(e, "page").and_then(|p| p.trim().parse::<u32>().ok());
    let Some(subtype) = subtype_of(&e.name.local).filter(|s| map::IMPORTED.contains(s)) else {
        skip(out, page, SkipCause::Unsupported);
        return Ok(());
    };
    let Some(page) = page else {
        skip(out, None, SkipCause::Incomplete);
        return Ok(());
    };
    let mut comment = Comment::new(page, subtype);
    comment.line = Some(line);
    let bad = |reason| src.lines.malformed(e.span.start, reason);
    for a in e.attrs.iter().filter(|a| a.name.namespace.is_empty()) {
        let local = a.name.local.as_str();
        if !SPECIAL.contains(&local) {
            read_attribute(&mut comment.entries, subtype, local, &a.value).map_err(bad)?;
        }
    }
    comment.in_reply_to = attr(e, "inreplyto").map(str::to_owned);
    if let (Some(start), Some(end)) = (attr(e, "start"), attr(e, "end")) {
        let mut l = numbers(start).map_err(bad)?;
        l.extend(numbers(end).map_err(bad)?);
        comment
            .entries
            .insert("L", Object::Array(l.into_iter().map(Object::number).collect()));
    }
    let ends: Vec<Object> = [attr(e, "head"), attr(e, "tail")]
        .into_iter()
        .map(|n| Object::name(n.map_or("None", str::trim)))
        .collect();
    if attr(e, "head").is_some() || attr(e, "tail").is_some() {
        comment.entries.insert("LE", Object::Array(ends));
    }
    read_border(
        &mut comment.entries,
        attr(e, "width"),
        attr(e, "style"),
        attr(e, "dashes"),
        attr(e, "intensity"),
    )
    .map_err(bad)?;
    for child in e.child_elements() {
        read_child(src, e, child, limits, budget, &mut comment)?;
    }
    if subtype == "FileAttachment" && comment.file.is_none() {
        skip(out, Some(page), SkipCause::Incomplete);
        return Ok(());
    }
    out.comments.push(comment);
    Ok(())
}

fn read_child(
    src: &Source<'_>,
    annot: &Element,
    child: &Element,
    limits: &Limits,
    budget: &mut DecodeBudget,
    comment: &mut Comment,
) -> Result<()> {
    let bad = |reason| src.lines.malformed(child.span.start, reason);
    let set_text = |comment: &mut Comment, key: &str, value: &str| {
        let s = PdfString::literal(encode_text_string(value));
        comment.entries.insert(key, Object::String(s));
    };
    match child.name.local.as_str() {
        "contents" => set_text(comment, "Contents", &child.text()),
        "contents-richtext" => set_text(comment, "RC", inner(src.text, child)),
        "defaultappearance" => set_text(comment, "DA", &child.text()),
        "defaultstyle" => set_text(comment, "DS", &child.text()),
        "popup" => {
            let rect = attr(child, "rect").map(numbers).transpose().map_err(bad)?;
            if let Some(&[x0, y0, x1, y1]) = rect.as_deref() {
                let open = attr(child, "open").is_some_and(|o| o.trim() == "yes" || o.trim() == "true");
                comment.popup = Some(Popup::new(UserSpaceRect::from_corners(x0, y0, x1, y1), open));
            }
        }
        "inklist" => {
            let strokes = child
                .child_elements()
                .filter(|g| g.name.local == "gesture")
                .map(|g| numbers(&g.text()).map(|n| Object::Array(n.into_iter().map(Object::number).collect())))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(bad)?;
            comment.entries.insert("InkList", Object::Array(strokes));
        }
        "vertices" => {
            let v = numbers(&child.text()).map_err(bad)?;
            comment
                .entries
                .insert("Vertices", Object::Array(v.into_iter().map(Object::number).collect()));
        }
        "data" if comment.subtype() == "FileAttachment" => {
            let bytes = file_data(child, limits, budget).ok_or_else(|| bad("its file data doesn't decode"))?;
            comment.file = Some(ExchangedFile {
                name: attr(annot, "file").unwrap_or_default().to_owned(),
                mime: attr(annot, "mimetype").unwrap_or_default().to_owned(),
                bytes,
                description: None,
                created: attr(annot, "creation").map(str::to_owned),
                modified: attr(annot, "modification").map(str::to_owned),
            });
        }
        _ => {}
    }
    Ok(())
}

/// The bytes of `<data>`: hexadecimal, Flate-compressed first when `encoding="compress"`.
fn file_data(data: &Element, limits: &Limits, budget: &mut DecodeBudget) -> Option<Vec<u8>> {
    let raw = from_hex(&data.text())?;
    if attr(data, "encoding").is_some_and(|e| e.trim() == "compress") {
        laminapdf_syntax::filters::flate_decode(&raw, limits, budget)
            .ok()
            .map(|d| d.data)
    } else {
        Some(raw)
    }
}

/// What is between `e`'s start and end tags, as written: rich text's XHTML.
fn inner<'t>(text: &'t str, e: &Element) -> &'t str {
    match (e.children.first(), e.children.last()) {
        (Some(first), Some(last)) => text.get(span(first).start..span(last).end).unwrap_or_default(),
        _ => "",
    }
}

fn span(n: &Node) -> std::ops::Range<usize> {
    match n {
        Node::Element(e) => e.span.clone(),
        Node::Text { span, .. } | Node::Other { span } => span.clone(),
        _ => 0..0,
    }
}

/// Writes `exchange` as XFDF.
pub(crate) fn write(exchange: &Exchange) -> Vec<u8> {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(out, "<xfdf xmlns=\"{NAMESPACE}\" xml:space=\"preserve\">");
    // Form data alone has no `<annots>`; comments, or nothing at all, do.
    if !exchange.comments.is_empty() || exchange.fields.is_empty() {
        out.push_str("<annots>\n");
        for c in &exchange.comments {
            write_comment(&mut out, c);
        }
        out.push_str("</annots>\n");
    }
    if !exchange.fields.is_empty() {
        out.push_str("<fields>\n");
        write_fields(&mut out, &NameTree::of(&exchange.fields));
        out.push_str("</fields>\n");
    }
    if let Some(file) = &exchange.file {
        let _ = writeln!(out, "<f href=\"{}\"/>", escape(file));
    }
    if let Some([original, modified]) = &exchange.ids {
        let _ = writeln!(
            out,
            "<ids original=\"{}\" modified=\"{}\"/>",
            to_hex(original),
            to_hex(modified)
        );
    }
    out.push_str("</xfdf>\n");
    out.into_bytes()
}

fn write_comment(out: &mut String, c: &Comment) {
    let Some(element) = element_of(c.subtype()) else {
        return;
    };
    let mut attrs = vec![("page", c.page.to_string())];
    attrs.extend(map::write_attributes(&c.entries));
    if let Some(parent) = &c.in_reply_to {
        attrs.push(("inreplyto", parent.clone()));
    }
    if let Some(f) = &c.file {
        attrs.push(("file", f.name.clone()));
        if !f.mime.is_empty() {
            attrs.push(("mimetype", f.mime.clone()));
        }
        attrs.push(("size", f.bytes.len().to_string()));
        attrs.extend(f.created.clone().map(|d| ("creation", d)));
        attrs.extend(f.modified.clone().map(|d| ("modification", d)));
    }
    let _ = write!(out, "<{element}");
    for (name, value) in attrs {
        let _ = write!(out, " {name}=\"{}\"", escape(&value));
    }
    out.push_str(">\n");
    let entry = |key| text(&c.entries, key);
    if let Some(contents) = entry("Contents") {
        let _ = writeln!(out, "<contents>{}</contents>", escape(&contents));
    }
    // Rich text is XHTML: written as it is when it reads as XML, else left out (its plain text is
    // `/Contents`).
    if let Some(rc) = entry("RC")
        .map(|rc| without_declaration(&rc).to_owned())
        .filter(|rc| xml::parse(rc, |_| None).is_ok())
    {
        let _ = writeln!(out, "<contents-richtext>{rc}</contents-richtext>");
    }
    for (key, element) in [("DA", "defaultappearance"), ("DS", "defaultstyle")] {
        if let Some(v) = entry(key) {
            let _ = writeln!(out, "<{element}>{}</{element}>", escape(&v));
        }
    }
    write_geometry(out, &c.entries);
    if let Some(p) = &c.popup {
        let [x0, y0, x1, y1] = [p.rect.x0, p.rect.y0, p.rect.x1, p.rect.y1].map(format_real);
        let open = if p.open { "yes" } else { "no" };
        let _ = writeln!(
            out,
            "<popup open=\"{open}\" page=\"{}\" rect=\"{x0},{y0},{x1},{y1}\"/>",
            c.page
        );
    }
    if let Some(f) = &c.file {
        let packed = laminapdf_syntax::filters::flate_encode(&f.bytes);
        let _ = writeln!(
            out,
            "<data MODE=\"raw\" encoding=\"compress\" length=\"{}\">{}</data>",
            packed.len(),
            to_hex(&packed)
        );
    }
    let _ = writeln!(out, "</{element}>");
}

fn write_geometry(out: &mut String, entries: &Dict) {
    if let Some(strokes) = entries.get("InkList").and_then(Object::as_array) {
        out.push_str("<inklist>");
        for stroke in strokes.iter().filter_map(Object::as_array) {
            let points: Vec<String> = stroke
                .chunks(2)
                .filter_map(|p| {
                    Some(format!(
                        "{},{}",
                        format_real(p.first()?.as_f64()?),
                        format_real(p.get(1)?.as_f64()?)
                    ))
                })
                .collect();
            let _ = write!(out, "<gesture>{}</gesture>", points.join(";"));
        }
        out.push_str("</inklist>\n");
    }
    if let Some(v) = entries.get("Vertices").and_then(Object::as_array) {
        let _ = writeln!(out, "<vertices>{}</vertices>", joined(v));
    }
}

/// Rich text without its leading `<?xml …?>` declaration, which Acrobat writes and which may only
/// start a document, not sit inside the XFDF.
fn without_declaration(rc: &str) -> &str {
    let trimmed = rc.trim_start_matches('\u{FEFF}').trim_start();
    match trimmed
        .strip_prefix("<?xml")
        .and_then(|rest| rest.find("?>").and_then(|end| rest.get(end + 2..)))
    {
        Some(rest) => rest.trim_start(),
        None => rc,
    }
}
