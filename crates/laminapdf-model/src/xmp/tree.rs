//! The packet's XML as an ordered tree that remembers where each node came from, so an edit
//! can replace one property's bytes and leave the rest of the packet as it was.
//!
//! `quick-xml` tokenizes; this module checks what XMP needs on top: no DTD (so no entity can
//! expand: XXE, billion laughs), only XML's predefined and numeric character references,
//! declared prefixes, and bounded depth, attributes and declarations.

use std::ops::Range;
use std::sync::Arc;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use super::ParseError;
use super::mapping::XML;

/// Elements nest no deeper than this. Real packets stay under ten levels; the bound keeps the
/// recursive walks over the tree (and its drop) within a small stack.
pub(super) const MAX_DEPTH: usize = 64;

/// A start tag holds no more attributes (declarations included) than this. Real ones hold a
/// dozen; the bound keeps a tag's scan and its names' resolution small.
pub(super) const MAX_ATTRIBUTES: usize = 256;

/// A packet declares no more namespaces than this in all. Real packets declare a few dozen;
/// each name's resolution walks the declarations in scope, so the bound keeps a packet of many
/// elements under many declarations linear in its size.
pub(super) const MAX_DECLARATIONS: usize = 256;

/// A node of the tree.
#[derive(Debug, Clone)]
pub(super) enum Node {
    Element(Element),
    /// Character data (text, references and CDATA sections run together), decoded.
    Text {
        span: Range<usize>,
        value: String,
    },
    /// A comment, processing instruction or XML declaration, kept as written.
    Other {
        span: Range<usize>,
    },
}

/// A qualified name with its namespace resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Name {
    pub(super) prefix: String,
    pub(super) local: String,
    /// The namespace URI; empty for an unprefixed attribute or no default namespace.
    pub(super) namespace: String,
}

impl Name {
    pub(super) fn is(&self, namespace: &str, local: &str) -> bool {
        self.namespace == namespace && self.local == local
    }

    /// The name as written: `prefix:local` or `local`.
    pub(super) fn qualified(&self) -> String {
        if self.prefix.is_empty() {
            self.local.clone()
        } else {
            format!("{}:{}", self.prefix, self.local)
        }
    }
}

/// An attribute: its name, decoded value and the bytes from its name to its closing quote.
#[derive(Debug, Clone)]
pub(super) struct Attr {
    pub(super) name: Name,
    pub(super) value: String,
    pub(super) span: Range<usize>,
}

/// An element and everything inside it.
#[derive(Debug, Clone)]
pub(super) struct Element {
    pub(super) name: Name,
    pub(super) attrs: Vec<Attr>,
    /// The `xmlns` declarations on this element.
    pub(super) declarations: Vec<Binding>,
    pub(super) children: Vec<Node>,
    /// From `<` of the start tag to `>` of the end tag (or of `/>`).
    pub(super) span: Range<usize>,
    pub(super) start_tag: Range<usize>,
    /// Written as `<name …/>`.
    pub(super) empty: bool,
}

impl Element {
    pub(super) fn attr(&self, namespace: &str, local: &str) -> Option<&Attr> {
        self.attrs.iter().find(|a| a.name.is(namespace, local))
    }

    pub(super) fn child_elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
    }

    /// The text directly inside, references decoded.
    pub(super) fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|n| match n {
                Node::Text { value, .. } => Some(value.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// One namespace declaration: `xmlns:prefix="uri"`, or `xmlns="uri"` with an empty prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Binding {
    pub(super) prefix: String,
    pub(super) uri: String,
}

impl Binding {
    pub(super) fn new(prefix: &str, uri: &str) -> Self {
        Self {
            prefix: prefix.to_owned(),
            uri: uri.to_owned(),
        }
    }
}

/// The namespace bindings in scope. Each element's declarations are one frame that points at
/// its parent's, so a scope is shared, not copied: holding the scope of every description
/// costs one pointer each, however many declarations surround them.
#[derive(Debug, Clone, Default)]
pub(super) struct Scope(Option<Arc<Frame>>);

#[derive(Debug)]
struct Frame {
    bindings: Vec<Binding>,
    parent: Scope,
}

impl Scope {
    /// This scope with `bindings` declared inside it.
    pub(super) fn with(&self, bindings: &[Binding]) -> Self {
        if bindings.is_empty() {
            return self.clone();
        }
        Self(Some(Arc::new(Frame {
            bindings: bindings.to_vec(),
            parent: self.clone(),
        })))
    }

    /// The bindings, innermost first.
    fn iter(&self) -> impl Iterator<Item = &Binding> {
        std::iter::successors(self.0.as_deref(), |f| f.parent.0.as_deref()).flat_map(|f| f.bindings.iter().rev())
    }

    /// The URI `prefix` names here (`""` for no default namespace).
    pub(super) fn resolve(&self, prefix: &str) -> Option<&str> {
        if prefix == "xml" {
            return Some(XML);
        }
        let found = self.iter().find(|b| b.prefix == prefix).map(|b| b.uri.as_str());
        match (found, prefix) {
            (None, "") => Some(""),
            (found, _) => found,
        }
    }

    /// A non-empty prefix that names `namespace` here, innermost first.
    pub(super) fn prefix_for(&self, namespace: &str) -> Option<&str> {
        self.iter()
            .map(|b| b.prefix.as_str())
            .find(|p| !p.is_empty() && self.resolve(p) == Some(namespace))
    }

    /// Each prefix's binding in effect, in the order first declared.
    pub(super) fn effective(&self) -> Vec<Binding> {
        let mut out: Vec<Binding> = Vec::new();
        let outermost_first: Vec<&Binding> = self.iter().collect();
        for b in outermost_first.into_iter().rev() {
            match out.iter_mut().find(|o| o.prefix == b.prefix) {
                Some(slot) => slot.uri.clone_from(&b.uri),
                None => out.push(b.clone()),
            }
        }
        out
    }
}

/// The top-level nodes of `text`.
pub(super) fn parse(text: &str) -> Result<Vec<Node>, ParseError> {
    // quick-xml skips a leading byte-order mark and counts positions after it, so the mark is
    // left out here and its three bytes added back to every position.
    let (body, base) = match text.strip_prefix('\u{FEFF}') {
        Some(body) => (body, '\u{FEFF}'.len_utf8()),
        None => (text, 0),
    };
    let mut reader = Reader::from_str(body);
    let config = reader.config_mut();
    config.check_end_names = true;
    config.expand_empty_elements = false;
    config.trim_text(false);
    let mut builder = Builder::default();
    loop {
        let start = base + position(reader.buffer_position());
        let event = reader.read_event().map_err(|_| ParseError::Syntax {
            offset: base as u64 + reader.error_position(),
        })?;
        let end = base + position(reader.buffer_position());
        match event {
            Event::Start(_) | Event::Empty(_) => {
                let empty = matches!(event, Event::Empty(_));
                let tag = text
                    .get(start..end)
                    .ok_or(ParseError::Syntax { offset: start as u64 })?;
                builder.open(tag, start..end, empty)?;
            }
            Event::End(_) => builder.close(end)?,
            Event::Text(t) => builder.text(start..end, &t.xml10_content()),
            Event::CData(c) => builder.text(start..end, &c.xml10_content()),
            Event::GeneralRef(r) => {
                let ch = if r.is_char_ref() {
                    r.resolve_char_ref().ok().flatten()
                } else {
                    predefined(&r.xml10_content())
                };
                let ch = ch.ok_or(ParseError::UndefinedEntity { offset: start as u64 })?;
                builder.text(start..end, ch.encode_utf8(&mut [0; 4]));
            }
            Event::Comment(_) | Event::PI(_) | Event::Decl(_) => builder.other(start..end),
            Event::DocType(_) => return Err(ParseError::Dtd { offset: start as u64 }),
            Event::Eof => break,
        }
    }
    builder.finish(text.len())
}

fn position(p: u64) -> usize {
    // Positions are within a &str, so they fit; a huge one only fails the slice that uses it.
    usize::try_from(p).unwrap_or(usize::MAX / 2)
}

/// The five entities XML predefines (XML 1.0 §4.6).
fn predefined(name: &str) -> Option<char> {
    match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "apos" => Some('\''),
        "quot" => Some('"'),
        _ => None,
    }
}

/// Builds the tree from the events, one open element per level.
#[derive(Default)]
struct Builder {
    top: Vec<Node>,
    /// The open elements, each with the scope outside it.
    open: Vec<(Element, Scope)>,
    scope: Scope,
    declarations: usize,
}

impl Builder {
    fn siblings(&mut self) -> &mut Vec<Node> {
        match self.open.last_mut() {
            Some((e, _)) => &mut e.children,
            None => &mut self.top,
        }
    }

    fn text(&mut self, span: Range<usize>, decoded: &str) {
        let siblings = self.siblings();
        if let Some(Node::Text { span: s, value }) = siblings.last_mut()
            && s.end == span.start
        {
            s.end = span.end;
            value.push_str(decoded);
            return;
        }
        siblings.push(Node::Text {
            span,
            value: decoded.to_owned(),
        });
    }

    fn other(&mut self, span: Range<usize>) {
        self.siblings().push(Node::Other { span });
    }

    fn open(&mut self, tag: &str, span: Range<usize>, empty: bool) -> Result<(), ParseError> {
        let offset = span.start as u64;
        if self.open.len() >= MAX_DEPTH {
            return Err(ParseError::TooDeep { offset });
        }
        let raw = scan_tag(tag, span.start).ok_or(ParseError::Syntax { offset })?;
        if raw.attrs.len() > MAX_ATTRIBUTES {
            return Err(ParseError::TooLarge { offset });
        }
        let mut declarations = Vec::new();
        for a in &raw.attrs {
            let prefix = match a.name.split_once(':') {
                Some(("xmlns", prefix)) => prefix,
                None if a.name == "xmlns" => "",
                _ => continue,
            };
            // `xmlns:="…"` declares nothing, and Namespaces in XML 1.0 can't undeclare a prefix.
            if a.name.ends_with(':') || (!prefix.is_empty() && a.value.is_empty()) || prefix.contains(':') {
                return Err(ParseError::Syntax { offset });
            }
            declarations.push(Binding::new(prefix, &a.value));
        }
        self.declarations += declarations.len();
        if self.declarations > MAX_DECLARATIONS {
            return Err(ParseError::TooLarge { offset });
        }
        let outside = self.scope.clone();
        self.scope = outside.with(&declarations);
        let name = self.resolve(raw.name, false, offset)?;
        let mut attrs = Vec::new();
        for a in raw.attrs {
            if a.name == "xmlns" || a.name.starts_with("xmlns:") {
                continue;
            }
            let name = self.resolve(a.name, true, offset)?;
            attrs.push(Attr {
                name,
                value: a.value,
                span: a.span,
            });
        }
        let element = Element {
            name,
            attrs,
            declarations,
            children: Vec::new(),
            span: span.clone(),
            start_tag: span,
            empty,
        };
        if empty {
            self.scope = outside;
            self.siblings().push(Node::Element(element));
        } else {
            self.open.push((element, outside));
        }
        Ok(())
    }

    fn close(&mut self, end: usize) -> Result<(), ParseError> {
        let (mut element, outside) = self.open.pop().ok_or(ParseError::Syntax { offset: end as u64 })?;
        element.span.end = end;
        self.scope = outside;
        self.siblings().push(Node::Element(element));
        Ok(())
    }

    /// `qualified` with its prefix resolved. A colon must separate a non-empty prefix from a
    /// non-empty local name (Namespaces in XML 1.0 §4), so `:x` and `a:b:c` are refused.
    fn resolve(&self, qualified: &str, attribute: bool, offset: u64) -> Result<Name, ParseError> {
        let (prefix, local) = qualified.split_once(':').unwrap_or(("", qualified));
        if local.is_empty() || local.contains(':') || (prefix.is_empty() && qualified.contains(':')) {
            return Err(ParseError::Syntax { offset });
        }
        // An unprefixed attribute is in no namespace (Namespaces in XML §6.2).
        let namespace = if attribute && prefix.is_empty() {
            ""
        } else {
            self.scope.resolve(prefix).ok_or(ParseError::UnboundPrefix { offset })?
        };
        Ok(Name {
            prefix: prefix.to_owned(),
            local: local.to_owned(),
            namespace: namespace.to_owned(),
        })
    }

    fn finish(self, len: usize) -> Result<Vec<Node>, ParseError> {
        if self.open.is_empty() {
            Ok(self.top)
        } else {
            Err(ParseError::Syntax { offset: len as u64 })
        }
    }
}

/// A start tag's name and attributes as written, before namespaces are resolved.
struct RawTag<'t> {
    name: &'t str,
    attrs: Vec<RawAttr<'t>>,
}

struct RawAttr<'t> {
    name: &'t str,
    value: String,
    span: Range<usize>,
}

/// Splits `tag` (`<name a="v" …>` or `…/>`, starting at byte `base` of the packet) into its
/// name and attributes, with each attribute's span and decoded value. `None` when it isn't a
/// well-formed start tag.
fn scan_tag(tag: &str, base: usize) -> Option<RawTag<'_>> {
    let inner = tag.strip_prefix('<')?;
    let inner = inner.strip_suffix("/>").or_else(|| inner.strip_suffix('>'))?;
    let is_space = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r');
    let name_len = inner.find(is_space).unwrap_or(inner.len());
    let name = inner.get(..name_len)?;
    let mut at = name_len;
    let mut attrs = Vec::new();
    loop {
        let rest = inner.get(at..)?;
        let trimmed = rest.trim_start_matches(is_space);
        if trimmed.is_empty() {
            break;
        }
        // An attribute must follow white space.
        if trimmed.len() == rest.len() {
            return None;
        }
        at += rest.len() - trimmed.len();
        let eq = trimmed.find('=')?;
        let attr_name = trimmed.get(..eq)?.trim_end_matches(is_space);
        let after_eq = trimmed.get(eq + 1..)?.trim_start_matches(is_space);
        let mut chars = after_eq.chars();
        let quote = chars.next().filter(|q| matches!(q, '"' | '\''))?;
        let body = chars.as_str();
        let close = body.find(quote)?;
        let raw_value = body.get(..close)?;
        let consumed = trimmed.len() - body.len() + close + 1;
        if attr_name.is_empty() || attr_name.contains(is_space) || raw_value.contains('<') {
            return None;
        }
        attrs.push(RawAttr {
            name: attr_name,
            value: attribute_value(raw_value)?,
            span: base + 1 + at..base + 1 + at + consumed,
        });
        at += consumed;
    }
    Some(RawTag { name, attrs })
}

/// An attribute value as XML 1.0 §3.3.3 normalises it: line ends and literal white space
/// become spaces, then only predefined and numeric references are decoded.
fn attribute_value(raw: &str) -> Option<String> {
    let spaced = raw.replace("\r\n", " ").replace(['\t', '\n', '\r'], " ");
    quick_xml::escape::unescape(&spaced)
        .ok()
        .map(std::borrow::Cow::into_owned)
}
