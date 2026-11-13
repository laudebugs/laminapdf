//! Writing packets: one property as XML (shared with the builder), an edit made in place
//! (only the changed properties' bytes replaced), the canonical rewrite when that can't be
//! done, the padding, and the minimal packet for a file that has none.

use std::fmt::Write as _;
use std::ops::Range;

use super::extension::{self, ExtensionProperty, PDFA_EXTENSION, is_predefined_in_pdfa};
use super::mapping::{ADOBE_META, DC, Mapping, PDF, RDF, STANDARD_PREFIXES, ValueKind, XMP, XMP_MM};
use super::rdf::{Description, Located, Place, Rdf, RdfValue};
use super::tree::{Binding, Node, Scope};
use super::{Value, XmpValue};

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
///
/// Added in 0.2.0.
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
///
/// Added in 0.2.0.
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
///
/// Added in 0.2.0.
#[must_use]
pub fn property_element(mapping: &Mapping, value: &Value) -> String {
    element(
        mapping.name,
        Body::Mapped(mapping, value),
        mapping.prefix,
        "rdf",
        &[],
        &[],
    )
}

/// The property `name` with `body` and the prefixes the packet binds (an empty prefix writes
/// the bare name, in the default namespace), `declarations` on its start tag and, for a mapped
/// language alternative, the other languages' entries (`(lang, text)`) after the `x-default`
/// one. A URI is written `<p:name rdf:resource="…"/>`, so `rdf` must then be a real prefix.
pub(super) fn element(
    name: &str,
    body: Body<'_>,
    prefix: &str,
    rdf: &str,
    others: &[(String, String)],
    declarations: &[Binding],
) -> String {
    let tag = qname(prefix, name);
    let mut open = tag.clone();
    write_declarations(&mut open, declarations.iter());
    let inner = match body {
        Body::Mapped(mapping, value) => mapped_inner(mapping, value, rdf, others),
        Body::Custom(XmpValue::Uri(uri)) => {
            return format!("<{open} {}=\"{}\"/>", qname(rdf, "resource"), escape(uri));
        }
        Body::Custom(value) => custom_inner(value, rdf),
    };
    format!("<{open}>{inner}</{tag}>")
}

/// A mapped property's content, in the shape of its row.
fn mapped_inner(mapping: &Mapping, value: &Value, rdf: &str, others: &[(String, String)]) -> String {
    let [alt, seq, li] = ["Alt", "Seq", "li"].map(|local| qname(rdf, local));
    match (mapping.kind, value) {
        (ValueKind::LangAlt, _) => {
            let mut s = format!(
                "<{alt}><{li} xml:lang=\"x-default\">{}</{li}>",
                escape(&value_text(value))
            );
            for (lang, text) in others {
                let _ = write!(s, "<{li} xml:lang=\"{}\">{}</{li}>", escape(lang), escape(text));
            }
            let _ = write!(s, "</{alt}>");
            s
        }
        (ValueKind::Seq, Value::List(items)) => {
            let mut s = format!("<{seq}>");
            for item in items {
                // Writing to a String can't fail.
                let _ = write!(s, "<{li}>{}</{li}>", escape(item));
            }
            let _ = write!(s, "</{seq}>");
            s
        }
        (ValueKind::Seq, other) => format!("<{seq}><{li}>{}</{li}></{seq}>", escape(&value_text(other))),
        (ValueKind::Text | ValueKind::Date, _) => escape(&value_text(value)),
    }
}

/// Any property's content: a simple value's text, a language alternative with `x-default`
/// first, or an array of simple values.
fn custom_inner(value: &XmpValue, rdf: &str) -> String {
    let [alt, li] = ["Alt", "li"].map(|local| qname(rdf, local));
    let is_default = |lang: &str| lang.eq_ignore_ascii_case("x-default");
    match value {
        XmpValue::LangAlt(entries) => {
            let mut s = format!("<{alt}>");
            let first = entries.iter().filter(|e| is_default(&e.lang));
            for e in first.chain(entries.iter().filter(|e| !is_default(&e.lang))) {
                let _ = write!(s, "<{li} xml:lang=\"{}\">{}</{li}>", escape(&e.lang), escape(&e.text));
            }
            let _ = write!(s, "</{alt}>");
            s
        }
        XmpValue::Seq(items) | XmpValue::Bag(items) => {
            let array = qname(
                rdf,
                if matches!(value, XmpValue::Seq(_)) {
                    "Seq"
                } else {
                    "Bag"
                },
            );
            let mut s = format!("<{array}>");
            for item in items {
                match item {
                    XmpValue::Uri(uri) => {
                        let _ = write!(s, "<{li} {}=\"{}\"/>", qname(rdf, "resource"), escape(uri));
                    }
                    other => {
                        let _ = write!(s, "<{li}>{}</{li}>", escape(&other.simple_text().unwrap_or_default()));
                    }
                }
            }
            let _ = write!(s, "</{array}>");
            s
        }
        simple => escape(&simple.simple_text().unwrap_or_default()),
    }
}

/// `prefix:local`, or `local` alone for an empty prefix.
pub(super) fn qname(prefix: &str, local: &str) -> String {
    if prefix.is_empty() {
        local.to_owned()
    } else {
        format!("{prefix}:{local}")
    }
}

/// The prefix that names the RDF namespace in `scope`: a prefix bound to it, the empty one when
/// it is the default namespace, else a fresh one, added to `declarations` for the element
/// that uses it.
pub(super) fn rdf_prefix(scope: &Scope, declarations: &mut Vec<Binding>) -> String {
    if let Some(p) = scope.prefix_for(RDF) {
        return p.to_owned();
    }
    if scope.resolve("") == Some(RDF) {
        return String::new();
    }
    let prefix = free_prefix(scope, declarations, "rdf");
    declarations.push(Binding::new(&prefix, RDF));
    prefix
}

/// A non-empty prefix for the RDF namespace in `scope`, for an attribute: an unprefixed
/// attribute is in no namespace, so the default namespace won't do. A fresh one is added to
/// `declarations` when none is bound.
pub(super) fn rdf_attribute_prefix(scope: &Scope, declarations: &mut Vec<Binding>) -> String {
    if let Some(p) = scope.prefix_for(RDF) {
        return p.to_owned();
    }
    let prefix = free_prefix(scope, declarations, "rdf");
    declarations.push(Binding::new(&prefix, RDF));
    prefix
}

/// `preferred`, or `preferred1`, `preferred2`, …: the first that neither `scope` nor
/// `declarations` binds.
pub(super) fn free_prefix(scope: &Scope, declarations: &[Binding], preferred: &str) -> String {
    let taken =
        |p: &str| scope.resolve(p).is_some_and(|uri| !uri.is_empty()) || declarations.iter().any(|b| b.prefix == p);
    // A packet binds at most MAX_DECLARATIONS prefixes, so one of these is free.
    (0..=u16::MAX)
        .map(|n| {
            if n == 0 {
                preferred.to_owned()
            } else {
                format!("{preferred}{n}")
            }
        })
        .find(|p| !taken(p))
        .unwrap_or_default()
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

/// A property's new value: a mapped field's, or any property's.
#[derive(Debug, Clone, Copy)]
pub(super) enum Body<'a> {
    Mapped(&'a Mapping, &'a Value),
    Custom(&'a XmpValue),
}

impl Body<'_> {
    /// The value as one attribute's text, when it can be written as an attribute.
    fn attribute_text(self) -> Option<String> {
        match self {
            Body::Mapped(m, v) => matches!(m.kind, ValueKind::Text | ValueKind::Date).then(|| value_text(v)),
            Body::Custom(XmpValue::Uri(_)) => None,
            Body::Custom(v) => v.simple_text(),
        }
    }

    /// The prefix for the RDF namespace in `scope`: a real one for any property, which may
    /// need `rdf:resource`.
    pub(super) fn rdf_name(self, scope: &Scope, declarations: &mut Vec<Binding>) -> String {
        match self {
            Body::Mapped(..) => rdf_prefix(scope, declarations),
            Body::Custom(_) => rdf_attribute_prefix(scope, declarations),
        }
    }
}

/// A change to make: the property, the prefix to write its namespace with when the packet
/// binds none, and its new value (`None` removes it).
#[derive(Debug, Clone, Copy)]
pub(super) struct Change<'a> {
    pub(super) namespace: &'a str,
    pub(super) name: &'a str,
    pub(super) prefix: &'a str,
    pub(super) value: Option<Body<'a>>,
}

/// Which properties a PDF/A packet's extension schemas get declarations for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Declare {
    /// None: the packet doesn't claim PDF/A.
    Nothing,
    /// The properties the changes set.
    Changed,
    /// Every property, the packet's own included.
    All,
}

/// One replacement of the packet's bytes.
#[derive(Debug)]
struct Edit {
    range: Range<usize>,
    text: String,
}

/// What a description gains: namespace declarations and property elements.
#[derive(Debug, Default, Clone)]
struct Additions {
    declarations: Vec<Binding>,
    children: String,
}

/// The packet with `changes` made in place: each changed property's bytes replaced or
/// removed, new properties added to a description, the rest of `text` as it was, padded back
/// to its length when it fits; and the extension schema entries it declared. `None` when it
/// can't be done in place.
pub(super) fn in_place(
    text: &str,
    top: &[Node],
    rdf: &Rdf,
    changes: &[Change<'_>],
    declare: Declare,
) -> Option<(String, Vec<ExtensionProperty>)> {
    if read_only(text, top) {
        return None;
    }
    let mut edits = Vec::new();
    let mut additions = vec![Additions::default(); rdf.descriptions.len()];
    let mut written = Vec::new();
    for change in changes {
        let mut found = rdf.properties.iter().filter(|p| is(p, change));
        let first = found.next();
        for other in found {
            edits.push(removal(other));
        }
        let Some(body) = change.value else {
            edits.extend(first.map(removal));
            continue;
        };
        let Some(p) = first else {
            let index = target(rdf, change.namespace)?;
            let prefix = add(rdf.descriptions.get(index)?, additions.get_mut(index)?, change, body);
            written.push((change, prefix));
            continue;
        };
        let description = rdf.descriptions.get(p.description)?;
        match (&p.place, body.attribute_text()) {
            (Place::Element(span), _) => {
                // The element's own declarations go on its replacement: its prefix may need them.
                // A standard prefix the packet used undeclared gets its declaration there too.
                let mut declarations = p.declarations.clone();
                if description.scope.with(&declarations).resolve(&p.prefix).is_none() {
                    declarations.push(Binding::new(&p.prefix, change.namespace));
                }
                let scope = description.scope.with(&declarations);
                let rdf_name = body.rdf_name(&scope, &mut declarations);
                edits.push(Edit {
                    range: span.clone(),
                    text: element(
                        change.name,
                        body,
                        &p.prefix,
                        &rdf_name,
                        &others(rdf, body),
                        &declarations,
                    ),
                });
                written.push((change, p.prefix.clone()));
            }
            (Place::Attribute(span), Some(value)) => {
                let quote = text.get(..span.end).and_then(|s| s.chars().last()).unwrap_or('"');
                let escaped = attribute(&value, quote);
                let name = qname(&p.prefix, change.name);
                edits.push(Edit {
                    range: span.clone(),
                    text: format!("{name}={quote}{escaped}{quote}"),
                });
                written.push((change, p.prefix.clone()));
                // A standard prefix the packet used undeclared is declared on its description.
                let added = additions.get_mut(p.description)?;
                if description.scope.with(&added.declarations).resolve(&p.prefix).is_none() {
                    added.declarations.push(Binding::new(&p.prefix, change.namespace));
                }
            }
            (Place::Attribute(_), None) => {
                // An array can't be an attribute: the property becomes an element.
                edits.push(removal(p));
                let prefix = add(description, additions.get_mut(p.description)?, change, body);
                written.push((change, prefix));
            }
        }
    }
    let declared = to_declare(rdf, changes, &written, declare);
    let (inserts, leftover) = extension::plan(rdf.extensions.as_ref(), &declared);
    edits.extend(inserts.into_iter().map(|i| Edit {
        range: i.at..i.at,
        text: i.text,
    }));
    if !leftover.is_empty() {
        let index = target(rdf, PDFA_EXTENSION)?;
        let description = rdf.descriptions.get(index)?;
        let added = additions.get_mut(index)?;
        added.children.push_str(&description.indent);
        let scope = description.scope.with(&added.declarations);
        added.children.push_str(&extension::schemas_element(&leftover, &scope));
    }
    for (description, added) in rdf.descriptions.iter().zip(&additions) {
        edits.extend(addition_edits(text, description, added)?);
    }
    edits.extend(header_edit(text, top));
    Some((apply(text, top, edits)?, declared))
}

/// Whether `p` is the property `change` names.
pub(super) fn is(p: &Located, change: &Change<'_>) -> bool {
    p.property.namespace == change.namespace && p.property.name == change.name
}

/// A mapped language alternative's other entries, to keep ([`alternatives`]).
pub(super) fn others(rdf: &Rdf, body: Body<'_>) -> Vec<(String, String)> {
    match body {
        Body::Mapped(mapping, value) => alternatives(rdf, mapping, value),
        Body::Custom(_) => Vec::new(),
    }
}

/// The extension schema entries a PDF/A packet needs: for each property `written` (with the
/// prefix it was written with) and, for [`Declare::All`], each property the packet keeps,
/// unless PDF/A predefines its schema or the packet declares it already.
pub(super) fn to_declare(
    rdf: &Rdf,
    changes: &[Change<'_>],
    written: &[(&Change<'_>, String)],
    declare: Declare,
) -> Vec<ExtensionProperty> {
    if declare == Declare::Nothing {
        return Vec::new();
    }
    let mut out: Vec<ExtensionProperty> = Vec::new();
    let mut push = |namespace: &str, prefix: &str, name: &str, value_type: Option<String>| {
        let declared = rdf.extensions.as_ref().is_some_and(|x| x.declares(namespace, name));
        let listed = out.iter().any(|p| p.namespace == namespace && p.name == name);
        if let Some(value_type) = value_type
            && !is_predefined_in_pdfa(namespace)
            && !declared
            && !listed
        {
            out.push(ExtensionProperty {
                namespace: namespace.to_owned(),
                prefix: prefix.to_owned(),
                name: name.to_owned(),
                value_type,
            });
        }
    };
    for (change, prefix) in written {
        if let Some(Body::Custom(value)) = change.value {
            push(change.namespace, prefix, change.name, Some(value.value_type()));
        }
    }
    if declare == Declare::All {
        for p in rdf.properties.iter().filter(|p| !changes.iter().any(|c| is(p, c))) {
            let q = &p.property;
            push(&q.namespace, &p.prefix, &q.name, extension::parsed_type(&q.value));
        }
    }
    out
}

/// The description a new property in `namespace` goes to: the first that declares the
/// namespace itself, else the first.
fn target(rdf: &Rdf, namespace: &str) -> Option<usize> {
    rdf.descriptions
        .iter()
        .position(|d| d.declarations.iter().any(|b| b.uri == namespace))
        .or_else(|| (!rdf.descriptions.is_empty()).then_some(0))
}

/// Adds the property to what `description` gains, declaring its namespace when needed;
/// returns the prefix it is written with.
fn add(description: &Description, added: &mut Additions, change: &Change<'_>, body: Body<'_>) -> String {
    let prefix = prefix_in(
        &description.scope,
        &mut added.declarations,
        change.namespace,
        change.prefix,
    );
    let mut own = Vec::new();
    let rdf_name = body.rdf_name(&description.scope.with(&added.declarations), &mut own);
    added.children.push_str(&description.indent);
    added
        .children
        .push_str(&element(change.name, body, &prefix, &rdf_name, &[], &own));
    prefix
}

/// The prefix for `namespace` inside `scope`: one it binds already, one declared by an earlier
/// addition, else a free one from `preferred` ([`free_prefix`]), added to `declarations`.
pub(super) fn prefix_in(scope: &Scope, declarations: &mut Vec<Binding>, namespace: &str, preferred: &str) -> String {
    if let Some(p) = scope.with(declarations).prefix_for(namespace) {
        return p.to_owned();
    }
    let prefix = free_prefix(scope, declarations, preferred);
    declarations.push(Binding::new(&prefix, namespace));
    prefix
}

/// The edits that add `added` to `description`: declarations at the end of its start tag,
/// properties after its last child (an empty description is opened and closed).
fn addition_edits(text: &str, description: &Description, added: &Additions) -> Option<Vec<Edit>> {
    if added.declarations.is_empty() && added.children.is_empty() {
        return Some(Vec::new());
    }
    let mut declarations = String::new();
    write_declarations(&mut declarations, added.declarations.iter());
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

/// Whether the trailer says `end="r"`: a read-only packet, which ISO 16684-1 §7.3.3 says not
/// to edit in place, so an edit writes a new packet.
fn read_only(text: &str, top: &[Node]) -> bool {
    trailer(text, top)
        .and_then(|span| text.get(span))
        .is_some_and(|t| t.contains("end=\"r\"") || t.contains("end='r'"))
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

/// ` xmlns:prefix="uri"` (or ` xmlns="uri"`) for each binding.
pub(super) fn write_declarations<'a>(out: &mut String, declarations: impl Iterator<Item = &'a Binding>) {
    for b in declarations {
        if b.prefix.is_empty() {
            let _ = write!(out, " xmlns=\"{}\"", escape(&b.uri));
        } else {
            let _ = write!(out, " xmlns:{}=\"{}\"", b.prefix, escape(&b.uri));
        }
    }
}
