//! Writing packets: one property as XML (shared with the builder), an edit made in place
//! (only the changed properties' bytes replaced), the canonical rewrite when that can't be
//! done, the padding, and the minimal packet for a file that has none.

use std::fmt::Write as _;
use std::ops::Range;

use super::Value;
use super::mapping::{ADOBE_META, DC, Mapping, PDF, RDF, STANDARD_PREFIXES, ValueKind, XMP, XMP_MM, standard_prefix};
use super::rdf::{Description, Located, Place, Rdf, RdfValue};
use super::tree::Node;

/// The header of a packet Lamina PDF writes: the byte-order mark in `begin` and the fixed
/// `id` (ISO 16684-1 §7.3.2), and no `bytes` or `encoding` attribute, which PDF/A forbids
/// (ISO 19005-2 §6.6.2.1).
pub(super) const HEADER: &str = "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>";
/// The trailer: the packet may be edited in place.
pub(super) const TRAILER: &str = "<?xpacket end=\"w\"?>";
/// The padding a new or rewritten packet gets, about what Adobe's tools leave (research 04
/// §5.2), so later edits fit in place.
pub(super) const PADDING: usize = 2048;

/// Whether XML 1.0 can hold `ch` (§2.2 `Char`): not the C0 controls other than tab, line
/// feed and carriage return, and not U+FFFE or U+FFFF.
///
/// ```
/// use laminapdf_model::xmp::is_xml_char;
///
/// assert!(is_xml_char('\t') && !is_xml_char('\u{1}'));
/// ```
#[must_use]
pub const fn is_xml_char(ch: char) -> bool {
    !matches!(ch, '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}')
}

/// `text` escaped for XML character data or a double-quoted attribute: `&`, `<`, `>` and `"`
/// as entities; tab, line feed and carriage return as character references (so they survive
/// attribute normalisation); characters XML can't hold ([`is_xml_char`]) left out.
///
/// ```
/// use laminapdf_model::xmp::escape;
///
/// assert_eq!(escape("a<b & \"c\">\u{1}\n"), "a&lt;b &amp; &quot;c&quot;&gt;&#xA;");
/// ```
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            c if !is_xml_char(c) => {}
            _ => out.push(ch),
        }
    }
    out
}

/// The property `mapping` names with `value`, as one element on one line with the mapping's
/// own prefix: `<dc:title><rdf:Alt><rdf:li xml:lang="x-default">…</rdf:li></rdf:Alt></dc:title>`,
/// `<dc:creator><rdf:Seq><rdf:li>…</rdf:li></rdf:Seq></dc:creator>`, `<pdf:Producer>…</pdf:Producer>`
/// or a date in ISO 8601. The builder writes its packet's properties with it.
///
/// ```
/// use laminapdf_model::xmp::{MappedField, Value, property_element};
///
/// let title = property_element(MappedField::Title.mapping(), &Value::Text("Q3 & Q4".into()));
/// assert_eq!(title, r#"<dc:title><rdf:Alt><rdf:li xml:lang="x-default">Q3 &amp; Q4</rdf:li></rdf:Alt></dc:title>"#);
/// ```
#[must_use]
pub fn property_element(mapping: &Mapping, value: &Value) -> String {
    element(mapping, mapping.prefix, "rdf", value, &[])
}

/// [`property_element`] with the prefixes the packet binds and, for a language alternative,
/// the other languages' entries (`(lang, text)`) after the `x-default` one.
pub(super) fn element(
    mapping: &Mapping,
    prefix: &str,
    rdf: &str,
    value: &Value,
    others: &[(String, String)],
) -> String {
    let tag = format!("{prefix}:{}", mapping.name);
    let inner = match (mapping.kind, value) {
        (ValueKind::LangAlt, _) => {
            let mut s = format!(
                "<{rdf}:Alt><{rdf}:li xml:lang=\"x-default\">{}</{rdf}:li>",
                escape(&value_text(value))
            );
            for (lang, text) in others {
                let _ = write!(s, "<{rdf}:li xml:lang=\"{}\">{}</{rdf}:li>", escape(lang), escape(text));
            }
            let _ = write!(s, "</{rdf}:Alt>");
            s
        }
        (ValueKind::Seq, Value::List(items)) => {
            let mut s = format!("<{rdf}:Seq>");
            for item in items {
                // Writing to a String can't fail.
                let _ = write!(s, "<{rdf}:li>{}</{rdf}:li>", escape(item));
            }
            let _ = write!(s, "</{rdf}:Seq>");
            s
        }
        (ValueKind::Seq, other) => format!(
            "<{rdf}:Seq><{rdf}:li>{}</{rdf}:li></{rdf}:Seq>",
            escape(&value_text(other))
        ),
        (ValueKind::Text | ValueKind::Date, _) => escape(&value_text(value)),
    };
    format!("<{tag}>{inner}</{tag}>")
}

/// The entries of the language alternative `mapping` names, other than `x-default`, to keep
/// when the `x-default` entry becomes `value`. An entry that repeated the old default (the
/// document's own language, as XMP toolkits write it) takes the new value too.
fn alternatives(rdf: &Rdf, mapping: &Mapping, value: &Value) -> Vec<(String, String)> {
    let Some(RdfValue::Array { items, .. }) = rdf
        .properties
        .iter()
        .find(|p| p.property.namespace == mapping.namespace && p.property.name == mapping.name)
        .map(|p| &p.property.value)
    else {
        return Vec::new();
    };
    if mapping.kind != ValueKind::LangAlt {
        return Vec::new();
    }
    let is_default = |lang: Option<&str>| lang.is_some_and(|l| l.eq_ignore_ascii_case("x-default"));
    let old_default = items
        .iter()
        .find(|i| is_default(i.lang.as_deref()))
        .and_then(|i| match &i.value {
            RdfValue::Text(t) => Some(t.as_str()),
            _ => None,
        });
    let new = value_text(value);
    items
        .iter()
        .filter_map(|i| match (&i.lang, &i.value) {
            (Some(lang), RdfValue::Text(t)) if !is_default(Some(lang)) => Some((
                lang.clone(),
                if Some(t.as_str()) == old_default {
                    new.clone()
                } else {
                    t.clone()
                },
            )),
            _ => None,
        })
        .collect()
}

/// A value as one string: dates in ISO 8601, lists joined with `; ` (as the Info dictionary's
/// `/Author` holds them).
fn value_text(value: &Value) -> String {
    match value {
        Value::Text(s) => s.clone(),
        Value::List(items) => items.join("; "),
        Value::Date(d) => d.to_iso8601(),
    }
}

/// `n` bytes of white space: a line break, then lines of 100 characters, as Adobe's tools pad.
pub(super) fn padding(n: usize) -> String {
    (1..=n)
        .map(|i| if i % 100 == 1 || i == n { '\n' } else { ' ' })
        .collect()
}

/// A packet for a file that has none: the four namespaces of the Info-mapped properties
/// declared on one `rdf:Description`, `dc:format`, and [`PADDING`].
pub(super) fn minimal() -> String {
    let mut namespaces = String::new();
    for (prefix, uri) in STANDARD_PREFIXES
        .iter()
        .filter(|(_, uri)| [DC, XMP, PDF, XMP_MM].contains(uri))
    {
        let _ = write!(namespaces, "\n    xmlns:{prefix}=\"{uri}\"");
    }
    format!(
        "{HEADER}\n<x:xmpmeta xmlns:x=\"{ADOBE_META}\">\n <rdf:RDF xmlns:rdf=\"{RDF}\">\n  \
         <rdf:Description rdf:about=\"\"{namespaces}>\n   <dc:format>application/pdf</dc:format>\n  \
         </rdf:Description>\n </rdf:RDF>\n</x:xmpmeta>\n{}{TRAILER}",
        padding(PADDING)
    )
}

/// A change to make: the field's mapping and its new value (`None` removes it).
pub(super) type Change<'a> = (&'static Mapping, Option<&'a Value>);

/// One replacement of the packet's bytes.
#[derive(Debug)]
struct Edit {
    range: Range<usize>,
    text: String,
}

/// What a description gains: namespace declarations and property elements.
#[derive(Debug, Default, Clone)]
struct Additions {
    declarations: Vec<(String, String)>,
    children: String,
}

/// The packet with `changes` made in place: each changed property's bytes replaced or
/// removed, new properties added to a description, the rest of `text` as it was, padded back
/// to its length when it fits. `None` when it can't be done in place.
pub(super) fn in_place(text: &str, top: &[Node], rdf: &Rdf, changes: &[Change<'_>]) -> Option<String> {
    let mut edits = Vec::new();
    let mut additions = vec![Additions::default(); rdf.descriptions.len()];
    for &(mapping, value) in changes {
        let mut found = rdf
            .properties
            .iter()
            .filter(|p| p.property.namespace == mapping.namespace && p.property.name == mapping.name);
        let first = found.next();
        for other in found {
            edits.push(removal(other));
        }
        let Some(value) = value else {
            edits.extend(first.map(removal));
            continue;
        };
        let Some(p) = first else {
            let index = target(rdf, mapping.namespace)?;
            add(rdf.descriptions.get(index)?, additions.get_mut(index)?, mapping, value)?;
            continue;
        };
        let description = rdf.descriptions.get(p.description)?;
        match &p.place {
            Place::Element(span) => {
                let rdf_prefix = description.scope.prefix_for(RDF)?;
                edits.push(Edit {
                    range: span.clone(),
                    text: element(
                        mapping,
                        &p.prefix,
                        rdf_prefix,
                        value,
                        &alternatives(rdf, mapping, value),
                    ),
                });
            }
            Place::Attribute(span) if matches!(mapping.kind, ValueKind::Text | ValueKind::Date) => {
                let quote = text.get(..span.end).and_then(|s| s.chars().last()).unwrap_or('"');
                let escaped = attribute(&value_text(value), quote);
                edits.push(Edit {
                    range: span.clone(),
                    text: format!("{}:{}={quote}{escaped}{quote}", p.prefix, mapping.name),
                });
            }
            Place::Attribute(_) => {
                // An array can't be an attribute: the property becomes an element.
                edits.push(removal(p));
                add(description, additions.get_mut(p.description)?, mapping, value)?;
            }
        }
    }
    for (description, added) in rdf.descriptions.iter().zip(&additions) {
        edits.extend(addition_edits(text, description, added)?);
    }
    edits.extend(header_edit(text, top));
    apply(text, top, edits)
}

/// The description a new property in `namespace` goes to: the first that declares the
/// namespace itself, else the first.
fn target(rdf: &Rdf, namespace: &str) -> Option<usize> {
    rdf.descriptions
        .iter()
        .position(|d| d.declarations.iter().any(|(_, uri)| uri == namespace))
        .or_else(|| (!rdf.descriptions.is_empty()).then_some(0))
}

/// Adds the property to what `description` gains, declaring its namespace when needed.
fn add(description: &Description, added: &mut Additions, mapping: &Mapping, value: &Value) -> Option<()> {
    let prefix = prefix_in(description, &mut added.declarations, mapping.namespace, mapping.prefix);
    let rdf_prefix = description.scope.prefix_for(RDF)?;
    added.children.push_str(&description.indent);
    added
        .children
        .push_str(&element(mapping, &prefix, rdf_prefix, value, &[]));
    Some(())
}

/// The prefix for `namespace` inside `description`: one it binds already, one declared by an
/// earlier addition, else `preferred` (or `preferred1`, `preferred2`, … when that is taken),
/// added to `declarations`.
fn prefix_in(
    description: &Description,
    declarations: &mut Vec<(String, String)>,
    namespace: &str,
    preferred: &str,
) -> String {
    if let Some(p) = description.scope.prefix_for(namespace) {
        return p.to_owned();
    }
    if let Some((p, _)) = declarations.iter().find(|(_, uri)| uri == namespace) {
        return p.clone();
    }
    let taken = |p: &str| {
        description.scope.resolve(p).is_some_and(|uri| !uri.is_empty()) || declarations.iter().any(|(q, _)| q == p)
    };
    // A packet binds far fewer prefixes than this, so one of them is free.
    let prefix = (0..=u16::MAX)
        .map(|n| {
            if n == 0 {
                preferred.to_owned()
            } else {
                format!("{preferred}{n}")
            }
        })
        .find(|p| !taken(p))
        .unwrap_or_default();
    declarations.push((prefix.clone(), namespace.to_owned()));
    prefix
}

/// The edits that add `added` to `description`: declarations at the end of its start tag,
/// properties after its last child (an empty description is opened and closed).
fn addition_edits(text: &str, description: &Description, added: &Additions) -> Option<Vec<Edit>> {
    if added.declarations.is_empty() && added.children.is_empty() {
        return Some(Vec::new());
    }
    let mut declarations = String::new();
    for (prefix, uri) in &added.declarations {
        let _ = write!(declarations, " xmlns:{prefix}=\"{}\"", escape(uri));
    }
    let tag_end = description.start_tag.end;
    if description.empty {
        let slash = tag_end.checked_sub(2)?;
        (text.get(slash..tag_end)? == "/>").then_some(())?;
        return Some(vec![Edit {
            range: slash..tag_end,
            text: format!(
                "{declarations}>{}{}</{}>",
                added.children, description.own_indent, description.qualified
            ),
        }]);
    }
    let close = tag_end.checked_sub(1)?;
    Some(vec![
        Edit {
            range: close..close,
            text: declarations,
        },
        Edit {
            range: description.insert_at..description.insert_at,
            text: added.children.clone(),
        },
    ])
}

/// Removing a property and the white space before it.
fn removal(p: &Located) -> Edit {
    let span = match &p.place {
        Place::Attribute(span) | Place::Element(span) => span,
    };
    Edit {
        range: p.removal_start..span.end,
        text: String::new(),
    }
}

/// `text` escaped for an attribute quoted with `quote`.
fn attribute(text: &str, quote: char) -> String {
    let escaped = escape(text);
    if quote == '\'' {
        escaped.replace('\'', "&apos;")
    } else {
        escaped
    }
}

/// The `<?xpacket begin … ?>` header, when it is the packet's first node.
fn header(text: &str, top: &[Node]) -> Option<Range<usize>> {
    top.iter().find_map(|n| match n {
        Node::Other { span } if text.get(span.clone())?.starts_with("<?xpacket begin") => Some(span.clone()),
        _ => None,
    })
}

/// The `<?xpacket end … ?>` trailer.
fn trailer(text: &str, top: &[Node]) -> Option<Range<usize>> {
    top.iter().rev().find_map(|n| match n {
        Node::Other { span } if text.get(span.clone())?.starts_with("<?xpacket end") => Some(span.clone()),
        _ => None,
    })
}

/// Replacing a header with `bytes` or `encoding` attributes: an edit makes `bytes` wrong, and
/// PDF/A forbids both.
fn header_edit(text: &str, top: &[Node]) -> Option<Edit> {
    let span = header(text, top)?;
    let written = text.get(span.clone())?;
    (written.contains("bytes=") || written.contains("encoding=")).then(|| Edit {
        range: span,
        text: HEADER.to_owned(),
    })
}

/// `text` with `edits` made, padded back to its length before the trailer when it fits, with
/// fresh padding when it doesn't. `None` when two edits overlap.
fn apply(text: &str, top: &[Node], mut edits: Vec<Edit>) -> Option<String> {
    edits.sort_by_key(|e| (e.range.start, e.range.end));
    let mut out = String::with_capacity(text.len() + PADDING);
    let mut at = 0;
    for edit in &edits {
        if edit.range.start < at {
            return None;
        }
        out.push_str(text.get(at..edit.range.start)?);
        out.push_str(&edit.text);
        at = edit.range.end;
    }
    let Some(trailer) = trailer(text, top) else {
        out.push_str(text.get(at..)?);
        return Some(out);
    };
    let pad_start = text
        .get(..trailer.start)?
        .trim_end_matches([' ', '\t', '\n', '\r'])
        .len();
    if pad_start < at {
        return None;
    }
    out.push_str(text.get(at..pad_start)?);
    let tail = text.get(trailer.start..)?;
    // The old padding is kept as far as it goes: an edit that keeps the length changes no
    // byte outside the edited properties.
    let old = text.get(pad_start..trailer.start)?;
    match text.len().checked_sub(out.len() + tail.len()) {
        Some(room) if room >= old.len() => {
            out.push_str(&padding(room - old.len()));
            out.push_str(old);
        }
        // Shorter: the middle goes, so the padding still starts and ends as it did.
        Some(0) => {}
        Some(room) => {
            out.push_str(old.get(..1)?);
            out.push_str(old.get(old.len() - room + 1..)?);
        }
        None => out.push_str(&padding(PADDING)),
    }
    out.push_str(tail);
    Some(out)
}

/// The packet rewritten whole: [`HEADER`], `x:xmpmeta`, `rdf:RDF` with each description in
/// turn (its declarations and the properties `changes` leaves alone, as written), the changed
/// properties in the first description, then [`PADDING`] and [`TRAILER`]. Comments and white
/// space between properties are not kept.
pub(super) fn canonical(text: &str, rdf: &Rdf, changes: &[Change<'_>]) -> String {
    let is_changed =
        |namespace: &str, name: &str| changes.iter().any(|(m, _)| m.namespace == namespace && m.name == name);
    let dropped: Vec<&Range<usize>> = rdf
        .properties
        .iter()
        .filter(|p| is_changed(&p.property.namespace, &p.property.name))
        .map(|p| match &p.place {
            Place::Attribute(span) | Place::Element(span) => span,
        })
        .collect();
    let rdf_prefix = if rdf.scope.resolve(&rdf.prefix) == Some(RDF) {
        rdf.prefix.as_str()
    } else {
        "rdf"
    };
    let mut out = format!("{HEADER}\n<x:xmpmeta xmlns:x=\"{ADOBE_META}\">\n <{rdf_prefix}:RDF");
    let mut scope = rdf.scope.effective();
    if !scope.iter().any(|(p, uri)| p == rdf_prefix && uri == RDF) {
        scope.push((rdf_prefix.to_owned(), RDF.to_owned()));
    }
    write_declarations(&mut out, scope.iter().filter(|(p, _)| p != "xml"));
    out.push_str(">\n");
    let fresh = Description {
        qualified: format!("{rdf_prefix}:Description"),
        start_tag: 0..0,
        empty: false,
        insert_at: 0,
        indent: String::new(),
        own_indent: String::new(),
        scope: rdf.scope.clone(),
        declarations: Vec::new(),
        attributes: Vec::new(),
        children: Vec::new(),
    };
    let descriptions: Vec<&Description> = if rdf.descriptions.is_empty() {
        vec![&fresh]
    } else {
        rdf.descriptions.iter().collect()
    };
    for (i, d) in descriptions.into_iter().enumerate() {
        let mut declarations = Vec::new();
        let mut properties = String::new();
        if i == 0 {
            for &(mapping, value) in changes {
                let Some(value) = value else { continue };
                let preferred = standard_prefix(mapping.namespace).unwrap_or(mapping.prefix);
                let prefix = prefix_in(d, &mut declarations, mapping.namespace, preferred);
                let others = alternatives(rdf, mapping, value);
                let _ = writeln!(
                    properties,
                    "   {}",
                    element(mapping, &prefix, rdf_prefix, value, &others)
                );
            }
        }
        let _ = write!(out, "  <{}", d.qualified);
        write_declarations(&mut out, d.declarations.iter().chain(&declarations));
        if d.attributes.is_empty() {
            let _ = write!(out, " {rdf_prefix}:about=\"\"");
        }
        for span in d.attributes.iter().filter(|s| !dropped.contains(s)) {
            out.push(' ');
            out.push_str(text.get(span.clone()).unwrap_or_default());
        }
        out.push_str(">\n");
        for span in d.children.iter().filter(|s| !dropped.contains(s)) {
            let _ = writeln!(out, "   {}", text.get(span.clone()).unwrap_or_default());
        }
        out.push_str(&properties);
        let _ = writeln!(out, "  </{}>", d.qualified);
    }
    for span in &rdf.others {
        let _ = writeln!(out, "  {}", text.get(span.clone()).unwrap_or_default());
    }
    let _ = write!(out, " </{rdf_prefix}:RDF>\n</x:xmpmeta>\n{}{TRAILER}", padding(PADDING));
    out
}

fn write_declarations<'a>(out: &mut String, declarations: impl Iterator<Item = &'a (String, String)>) {
    for (prefix, uri) in declarations {
        if prefix.is_empty() {
            let _ = write!(out, " xmlns=\"{}\"", escape(uri));
        } else {
            let _ = write!(out, " xmlns:{prefix}=\"{}\"", escape(uri));
        }
    }
}
