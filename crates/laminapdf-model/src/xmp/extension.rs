//! PDF/A extension schemas (ISO 19005-2 §6.6.2.3): a file that claims PDF/A may use only the
//! XMP properties PDF/A predefines, unless its packet declares the others in
//! `pdfaExtension:schemas`. This module reads the declarations a packet has and writes the
//! ones an edit adds: property entries into an existing schema, schema entries into the
//! existing list, or the list itself.

use std::fmt::Write as _;

use super::mapping::{DC, PDF, PDFA_ID, RDF, XMP, XMP_MM};
use super::rdf::RdfValue;
use super::tree::{Binding, Element, Scope};
use super::write::{escape, prefix_in, rdf_attribute_prefix, write_declarations};

/// `pdfaExtension:`, the extension schema container (ISO 19005-2 §6.6.2.3.2).
///
/// Added in 0.2.0.
pub const PDFA_EXTENSION: &str = "http://www.aiim.org/pdfa/ns/extension/";
/// `pdfaSchema:`, a schema's description.
///
/// Added in 0.2.0.
pub const PDFA_SCHEMA: &str = "http://www.aiim.org/pdfa/ns/schema#";
/// `pdfaProperty:`, a property's description.
///
/// Added in 0.2.0.
pub const PDFA_PROPERTY: &str = "http://www.aiim.org/pdfa/ns/property#";
pub(super) const PDFA_TYPE: &str = "http://www.aiim.org/pdfa/ns/type#";
pub(super) const PDFA_FIELD: &str = "http://www.aiim.org/pdfa/ns/field#";

/// The schemas PDF/A predefines (ISO 19005-2 §6.6.2.3.1: those of the XMP Specification
/// 2005, and PDF/A's own), whose properties need no declaration.
const PREDEFINED: [&str; 19] = [
    DC,
    XMP,
    "http://ns.adobe.com/xap/1.0/rights/",
    XMP_MM,
    "http://ns.adobe.com/xap/1.0/bj/",
    "http://ns.adobe.com/xap/1.0/t/pg/",
    "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
    PDF,
    "http://ns.adobe.com/photoshop/1.0/",
    "http://ns.adobe.com/camera-raw-settings/1.0/",
    "http://ns.adobe.com/tiff/1.0/",
    "http://ns.adobe.com/exif/1.0/",
    "http://ns.adobe.com/exif/1.0/aux/",
    PDFA_ID,
    PDFA_EXTENSION,
    PDFA_SCHEMA,
    PDFA_PROPERTY,
    PDFA_TYPE,
    PDFA_FIELD,
];

/// Whether PDF/A predefines the schema `namespace`.
///
/// ```
/// use laminapdf_model::xmp::is_predefined_in_pdfa;
///
/// assert!(is_predefined_in_pdfa("http://purl.org/dc/elements/1.1/"));
/// assert!(!is_predefined_in_pdfa("http://www.aiim.org/pdfua/ns/id/"));
/// ```
///
/// Added in 0.2.0.
#[must_use]
pub fn is_predefined_in_pdfa(namespace: &str) -> bool {
    PREDEFINED.contains(&namespace)
}

/// A property a PDF/A file's packet declares, or an edit declared, in its extension schemas.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExtensionProperty {
    /// The schema's namespace URI.
    pub namespace: String,
    /// The prefix the packet writes the namespace with.
    pub prefix: String,
    /// The property's local name.
    pub name: String,
    /// Its declared type, e.g. `Text` or `Seq Integer`.
    pub value_type: String,
}

/// An element new entries go inside: before its end tag, with its scope.
#[derive(Debug, Clone)]
pub(super) struct Container {
    /// Where an entry goes: the end of the last child's line, before the end tag.
    insert_at: usize,
    /// The line break and indentation of an entry.
    indent: String,
    scope: Scope,
}

/// One schema the packet declares.
#[derive(Debug, Clone)]
pub(super) struct Schema {
    namespace: String,
    properties: Vec<String>,
    /// The `rdf:Seq` of its properties, when it is one new entries can join.
    seq: Option<Container>,
}

/// The packet's `pdfaExtension:schemas`.
#[derive(Debug, Clone)]
pub(super) struct Extensions {
    /// The `rdf:Bag` of schemas, when new ones can join it.
    bag: Option<Container>,
    schemas: Vec<Schema>,
}

impl Extensions {
    /// Whether the packet declares `name` in `namespace`.
    pub(super) fn declares(&self, namespace: &str, name: &str) -> bool {
        self.schemas
            .iter()
            .any(|s| s.namespace == namespace && s.properties.iter().any(|p| p == name))
    }
}

/// Reads `pdfaExtension:schemas` element `e`, inside `scope`.
pub(super) fn read(text: &str, e: &Element, scope: &Scope) -> Extensions {
    let scope = scope.with(&e.declarations);
    let bag = e.child_elements().find(|c| c.name.is(RDF, "Bag"));
    let mut schemas = Vec::new();
    if let Some(bag) = bag {
        let inside = scope.with(&bag.declarations);
        for li in bag.child_elements().filter(|c| c.name.is(RDF, "li")) {
            if let Some(schema) = schema(text, li, &inside) {
                schemas.push(schema);
            }
        }
    }
    Extensions {
        bag: bag.and_then(|b| container(text, b, &scope)),
        schemas,
    }
}

/// The fields of a resource: an `rdf:li` with `rdf:parseType="Resource"`, or one holding an
/// `rdf:Description`; with the scope inside the element that holds them.
fn fields<'e>(li: &'e Element, scope: &Scope) -> Option<(&'e Element, Scope)> {
    let scope = scope.with(&li.declarations);
    if li.attr(RDF, "parseType").is_some_and(|a| a.value == "Resource") {
        return Some((li, scope));
    }
    let d = li.child_elements().find(|c| c.name.is(RDF, "Description"))?;
    Some((d, scope.with(&d.declarations)))
}

/// A field's text: a child element's, else an attribute's.
fn field(holder: &Element, namespace: &str, local: &str) -> Option<String> {
    holder
        .child_elements()
        .find(|c| c.name.is(namespace, local))
        .map(Element::text)
        .or_else(|| holder.attr(namespace, local).map(|a| a.value.clone()))
        .map(|s| s.trim().to_owned())
}

fn schema(text: &str, li: &Element, scope: &Scope) -> Option<Schema> {
    let (holder, scope) = fields(li, scope)?;
    let namespace = field(holder, PDFA_SCHEMA, "namespaceURI")?;
    let mut properties = Vec::new();
    let mut seq = None;
    if let Some(property) = holder.child_elements().find(|c| c.name.is(PDFA_SCHEMA, "property")) {
        let scope = scope.with(&property.declarations);
        if let Some(s) = property.child_elements().find(|c| c.name.is(RDF, "Seq")) {
            let inside = scope.with(&s.declarations);
            for item in s.child_elements().filter(|c| c.name.is(RDF, "li")) {
                if let Some((h, _)) = fields(item, &inside)
                    && let Some(name) = field(h, PDFA_PROPERTY, "name")
                {
                    properties.push(name);
                }
            }
            seq = container(text, s, &scope);
        }
    }
    Some(Schema {
        namespace,
        properties,
        seq,
    })
}

/// Where entries go inside `e` (an `rdf:Bag` or `rdf:Seq` inside `scope`): `None` when it is
/// written `<…/>`.
fn container(text: &str, e: &Element, scope: &Scope) -> Option<Container> {
    if e.empty {
        return None;
    }
    let end_tag = e.span.end.checked_sub(e.name.qualified().len() + 3)?;
    text.get(end_tag..e.span.end)?.starts_with("</").then_some(())?;
    let before = text.get(..end_tag)?;
    let insert_at = before.trim_end_matches([' ', '\t', '\n', '\r']).len();
    let indent = match e.child_elements().next() {
        Some(child) => line_indent(text, child.span.start),
        None => format!("{} ", line_indent(text, e.span.start)),
    };
    Some(Container {
        insert_at,
        indent,
        scope: scope.with(&e.declarations),
    })
}

/// A line break and the indentation of the line holding byte `at`.
fn line_indent(text: &str, at: usize) -> String {
    let before = text.get(..at).unwrap_or_default();
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let indent: String = before
        .get(line_start..)
        .unwrap_or_default()
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    format!("\n{indent}")
}

/// Text to insert at a byte of the packet.
#[derive(Debug)]
pub(super) struct Insert {
    pub(super) at: usize,
    pub(super) text: String,
}

/// How `props` get declared: entries inserted into the packet's schemas, and the properties
/// whose schema needs a new `pdfaExtension:schemas` (none exists, or it can't be added to).
pub(super) fn plan<'p>(
    extensions: Option<&Extensions>,
    props: &'p [ExtensionProperty],
) -> (Vec<Insert>, Vec<&'p ExtensionProperty>) {
    let mut inserts = Vec::new();
    let mut leftover = Vec::new();
    for group in by_namespace(props) {
        let Some(first) = group.first() else { continue };
        let existing = extensions.and_then(|x| x.schemas.iter().find(|s| s.namespace == first.namespace));
        if let Some(seq) = existing.and_then(|s| s.seq.as_ref()) {
            let mut text = String::new();
            for p in &group {
                text.push_str(&seq.indent);
                text.push_str(&property_entry(p, &seq.scope));
            }
            inserts.push(Insert {
                at: seq.insert_at,
                text,
            });
        } else if let Some(bag) = extensions.and_then(|x| x.bag.as_ref()) {
            let mut text = bag.indent.clone();
            text.push_str(&schema_entry(&group, &bag.scope));
            inserts.push(Insert {
                at: bag.insert_at,
                text,
            });
        } else {
            leftover.extend(group);
        }
    }
    (inserts, leftover)
}

/// The `pdfaExtension:schemas` property declaring `props`, written in a description with
/// `scope`; its own namespace declarations go on its start tag.
pub(super) fn schemas_element(props: &[&ExtensionProperty], scope: &Scope) -> String {
    let mut declarations = Vec::new();
    let extension = prefix_in(scope, &mut declarations, PDFA_EXTENSION, "pdfaExtension");
    let rdf = rdf_attribute_prefix(&scope.with(&declarations), &mut declarations);
    let inside = scope.with(&declarations);
    let mut schemas = String::new();
    for group in by_namespace(props.iter().copied()) {
        schemas.push_str(&schema_entry(&group, &inside));
    }
    let mut open = format!("{extension}:schemas");
    write_declarations(&mut open, declarations.iter());
    format!("<{open}><{rdf}:Bag>{schemas}</{rdf}:Bag></{extension}:schemas>")
}

/// `props` grouped by namespace, in the order each namespace first appears.
fn by_namespace<'p>(props: impl IntoIterator<Item = &'p ExtensionProperty>) -> Vec<Vec<&'p ExtensionProperty>> {
    let mut groups: Vec<Vec<&ExtensionProperty>> = Vec::new();
    for p in props {
        match groups
            .iter_mut()
            .find(|g| g.first().is_some_and(|f| f.namespace == p.namespace))
        {
            Some(g) => g.push(p),
            None => groups.push(vec![p]),
        }
    }
    groups
}

/// One schema's `rdf:li`: its name, namespace, prefix and properties. Prefixes the scope
/// doesn't bind are declared on the entry.
fn schema_entry(group: &[&ExtensionProperty], scope: &Scope) -> String {
    let Some(first) = group.first() else {
        return String::new();
    };
    let mut decl: Vec<Binding> = Vec::new();
    let rdf = rdf_attribute_prefix(scope, &mut decl);
    let s = prefix_in(scope, &mut decl, PDFA_SCHEMA, "pdfaSchema");
    let inside = scope.with(&decl);
    let mut properties = String::new();
    for p in group {
        properties.push_str(&property_entry(p, &inside));
    }
    let mut open = format!("{rdf}:li");
    write_declarations(&mut open, decl.iter());
    format!(
        "<{open} {rdf}:parseType=\"Resource\"><{s}:schema>{}</{s}:schema><{s}:namespaceURI>{}</{s}:namespaceURI>\
         <{s}:prefix>{}</{s}:prefix><{s}:property><{rdf}:Seq>{properties}</{rdf}:Seq></{s}:property></{rdf}:li>",
        escape(&format!("Properties in the {} namespace", first.prefix)),
        escape(&first.namespace),
        escape(&first.prefix),
    )
}

/// One property's `rdf:li`: name, value type, category and description, declaring on itself
/// the prefixes `scope` doesn't bind.
fn property_entry(p: &ExtensionProperty, scope: &Scope) -> String {
    let mut own = Vec::new();
    let rdf = rdf_attribute_prefix(scope, &mut own);
    let n = prefix_in(scope, &mut own, PDFA_PROPERTY, "pdfaProperty");
    let mut open = format!("{rdf}:li");
    write_declarations(&mut open, own.iter());
    let mut out = String::new();
    let _ = write!(
        out,
        "<{open} {rdf}:parseType=\"Resource\"><{n}:name>{}</{n}:name><{n}:valueType>{}</{n}:valueType>\
         <{n}:category>external</{n}:category><{n}:description>{}</{n}:description></{rdf}:li>",
        escape(&p.name),
        escape(&p.value_type),
        escape(&format!("{}:{}", p.prefix, p.name)),
    );
    out
}

/// The type to declare for a property already in a packet: `Text` for a simple value, `Lang
/// Alt`, `Alt Text`, `Seq Text` or `Bag Text` for an array of text. `None` for a structure or
/// a qualified value, whose type a declaration can't be guessed for.
pub(super) fn parsed_type(value: &RdfValue) -> Option<String> {
    use super::rdf::ArrayKind;
    match value {
        RdfValue::Text(_) => Some("Text".to_owned()),
        RdfValue::Array { kind, items } => {
            if !items.iter().all(|i| matches!(i.value, RdfValue::Text(_))) {
                return None;
            }
            Some(
                match kind {
                    ArrayKind::Alt if items.iter().all(|i| i.lang.is_some()) => "Lang Alt",
                    ArrayKind::Alt => "Alt Text",
                    ArrayKind::Seq => "Seq Text",
                    ArrayKind::Bag => "Bag Text",
                }
                .to_owned(),
            )
        }
        RdfValue::Other(_) => None,
    }
}
