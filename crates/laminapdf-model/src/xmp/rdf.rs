//! The packet's RDF (ISO 16684-1 §7): the `rdf:Description` elements under `rdf:RDF`, and
//! their properties in attribute form (`<rdf:Description dc:format="…"/>`) and element form,
//! each with where it is written so the writer can replace it alone.

use std::ops::Range;

use super::ParseError;
use super::extension::{self, Extensions, PDFA_EXTENSION};
use super::mapping::{ADOBE_META, RDF, XML};
use super::tree::{Binding, Element, Node, Scope};

/// A property's value as the packet writes it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RdfValue {
    /// A simple value: text, in attribute or element form.
    Text(String),
    /// An array (`rdf:Alt`, `rdf:Seq` or `rdf:Bag`) and its items in order.
    Array {
        /// Which array.
        kind: ArrayKind,
        /// The `rdf:li` items.
        items: Vec<ArrayItem>,
    },
    /// Anything else (a structure, a resource reference, a qualified value), as the packet
    /// writes the whole property element.
    Other(String),
}

/// The three RDF containers XMP uses (ISO 16684-1 §7.7).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ArrayKind {
    /// `rdf:Alt`: alternatives, such as one per language.
    Alt,
    /// `rdf:Seq`: an ordered list.
    Seq,
    /// `rdf:Bag`: an unordered list.
    Bag,
}

/// One `rdf:li` of an array.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ArrayItem {
    /// Its `xml:lang`, if any.
    pub lang: Option<String>,
    /// Its value.
    pub value: RdfValue,
}

/// A property of the packet: its namespace, local name and value.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RdfProperty {
    /// The namespace URI.
    pub namespace: String,
    /// The local name, e.g. `title`.
    pub name: String,
    /// The value.
    pub value: RdfValue,
}

/// Where a property is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Place {
    /// An attribute of its `rdf:Description`: the attribute's bytes.
    Attribute(Range<usize>),
    /// A child element of its `rdf:Description`: the element's bytes.
    Element(Range<usize>),
}

/// A property with where it is.
#[derive(Debug, Clone)]
pub(super) struct Located {
    pub(super) property: RdfProperty,
    /// The prefix the packet writes it with.
    pub(super) prefix: String,
    pub(super) place: Place,
    /// Where removing it starts: the white space before it, so no blank line is left.
    pub(super) removal_start: usize,
    /// Its `rdf:Description`, an index into [`Rdf::descriptions`].
    pub(super) description: usize,
    /// The namespaces an element-form property declares on itself, which a replacement must
    /// declare again.
    pub(super) declarations: Vec<Binding>,
}

/// An `rdf:Description`: what an edit needs to add properties to it.
#[derive(Debug, Clone)]
pub(super) struct Description {
    /// `rdf:Description` as written (the RDF prefix may differ).
    pub(super) qualified: String,
    pub(super) start_tag: Range<usize>,
    pub(super) empty: bool,
    /// Where a new property goes: after the last child element, else after the start tag.
    pub(super) insert_at: usize,
    /// The line break and indentation a new property gets.
    pub(super) indent: String,
    /// The line break and indentation of the description's own start tag.
    pub(super) own_indent: String,
    /// The namespace bindings in scope inside it.
    pub(super) scope: Scope,
    /// The bindings it declares itself.
    pub(super) declarations: Vec<Binding>,
    /// Its attributes as written (`rdf:about="…"` and attribute-form properties).
    pub(super) attributes: Vec<Range<usize>>,
    /// Whether one of them is `rdf:about`, which ISO 16684-1 §7.4 requires on every
    /// description.
    pub(super) has_about: bool,
    /// Its child elements as written.
    pub(super) children: Vec<Range<usize>>,
}

/// The RDF of a packet.
#[derive(Debug, Clone, Default)]
pub(super) struct Rdf {
    pub(super) descriptions: Vec<Description>,
    pub(super) properties: Vec<Located>,
    /// The bindings in scope inside `rdf:RDF`.
    pub(super) scope: Scope,
    /// `rdf:RDF`'s children other than descriptions (typed nodes, comments), as written.
    pub(super) others: Vec<Range<usize>>,
    /// The PDF/A extension schemas the packet declares (the first `pdfaExtension:schemas`).
    pub(super) extensions: Option<Extensions>,
}

/// Finds `rdf:RDF` (inside `x:xmpmeta` or at the top) and reads its descriptions.
pub(super) fn read(text: &str, top: &[Node]) -> Result<Rdf, ParseError> {
    let mut scope = Scope::default();
    let rdf = find_rdf(top, &mut scope).ok_or(ParseError::NoRdf)?;
    let scope = scope.with(&rdf.declarations);
    let mut out = Rdf {
        scope: scope.clone(),
        ..Rdf::default()
    };
    for node in &rdf.children {
        match node {
            Node::Element(e) if e.name.is(RDF, "Description") => description(text, e, &scope, &mut out),
            Node::Element(e) => out.others.push(e.span.clone()),
            Node::Other { span } => out.others.push(span.clone()),
            Node::Text { .. } => {}
        }
    }
    Ok(out)
}

/// `rdf:RDF`, at the top or inside `x:xmpmeta`, with the bindings of the elements around it
/// pushed onto `scope`.
fn find_rdf<'a>(nodes: &'a [Node], scope: &mut Scope) -> Option<&'a Element> {
    for node in nodes {
        let Node::Element(e) = node else { continue };
        if e.name.is(RDF, "RDF") {
            return Some(e);
        }
        if e.name.namespace == ADOBE_META
            && let Some(found) = e.child_elements().find(|c| c.name.is(RDF, "RDF"))
        {
            *scope = scope.with(&e.declarations);
            return Some(found);
        }
    }
    None
}

fn description(text: &str, e: &Element, outer: &Scope, out: &mut Rdf) {
    let index = out.descriptions.len();
    let scope = outer.with(&e.declarations);
    let mut attributes = Vec::new();
    for a in &e.attrs {
        attributes.push(a.span.clone());
        if a.name.namespace.is_empty() || a.name.namespace == RDF || a.name.namespace == XML {
            continue;
        }
        out.properties.push(Located {
            property: RdfProperty {
                namespace: a.name.namespace.clone(),
                name: a.name.local.clone(),
                value: RdfValue::Text(a.value.clone()),
            },
            prefix: a.name.prefix.clone(),
            place: Place::Attribute(a.span.clone()),
            removal_start: whitespace_before(text, a.span.start),
            description: index,
            declarations: Vec::new(),
        });
    }
    let mut children = Vec::new();
    let mut insert_at = e.start_tag.end;
    let mut indent = None;
    let mut previous: Option<&Node> = None;
    for node in &e.children {
        if let Node::Element(child) = node {
            let removal_start = match previous {
                Some(Node::Text { span, value }) if value.trim().is_empty() => span.start,
                _ => child.span.start,
            };
            if indent.is_none() {
                indent = Some(line_indent(text, child.span.start));
            }
            if out.extensions.is_none() && child.name.is(PDFA_EXTENSION, "schemas") {
                out.extensions = Some(extension::read(text, child, &scope));
            }
            out.properties.push(Located {
                property: RdfProperty {
                    namespace: child.name.namespace.clone(),
                    name: child.name.local.clone(),
                    value: value_of(text, child),
                },
                prefix: child.name.prefix.clone(),
                place: Place::Element(child.span.clone()),
                removal_start,
                description: index,
                declarations: child.declarations.clone(),
            });
            children.push(child.span.clone());
            insert_at = child.span.end;
        }
        previous = Some(node);
    }
    let own_indent = line_indent(text, e.span.start);
    let indent = indent.unwrap_or_else(|| format!("{own_indent} "));
    out.descriptions.push(Description {
        qualified: e.name.qualified(),
        start_tag: e.start_tag.clone(),
        empty: e.empty,
        insert_at,
        indent,
        own_indent,
        has_about: e.attr(RDF, "about").is_some(),
        scope,
        declarations: e.declarations.clone(),
        attributes,
        children,
    });
}

/// The value of property element `e` (ISO 16684-1 §7.9).
fn value_of(text: &str, e: &Element) -> RdfValue {
    let verbatim = || RdfValue::Other(text.get(e.span.clone()).unwrap_or_default().to_owned());
    let qualifies = e
        .attrs
        .iter()
        .any(|a| !(a.name.is(XML, "lang") || (a.name.namespace.is_empty() && a.name.local == "about")));
    if qualifies {
        return verbatim();
    }
    let mut elements = e.child_elements();
    let Some(first) = elements.next() else {
        return RdfValue::Text(e.text());
    };
    if elements.next().is_some() || first.name.namespace != RDF {
        return verbatim();
    }
    let kind = match first.name.local.as_str() {
        "Alt" => ArrayKind::Alt,
        "Seq" => ArrayKind::Seq,
        "Bag" => ArrayKind::Bag,
        _ => return verbatim(),
    };
    let mut items = Vec::new();
    for li in first.child_elements() {
        if !li.name.is(RDF, "li") {
            return verbatim();
        }
        items.push(ArrayItem {
            lang: li.attr(XML, "lang").map(|a| a.value.clone()),
            value: value_of(text, li),
        });
    }
    RdfValue::Array { kind, items }
}

/// The start of the white space that ends just before byte `at`.
fn whitespace_before(text: &str, at: usize) -> usize {
    let before = text.get(..at).unwrap_or_default();
    before.trim_end_matches([' ', '\t', '\n', '\r']).len()
}

/// A line break and the indentation of the line holding byte `at` (empty when other
/// characters come before it on that line).
fn line_indent(text: &str, at: usize) -> String {
    let before = text.get(..at).unwrap_or_default();
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let indent = before.get(line_start..).unwrap_or_default();
    if indent.chars().all(|c| c == ' ' || c == '\t') {
        format!("\n{indent}")
    } else {
        "\n".to_owned()
    }
}
