//! The form's XFA part (`/AcroForm /XFA`, ISO 32000-2 §12.7.9, deprecated in PDF 2.0): its
//! packets as bytes and whether the form is static or dynamic ([`Document::xfa`]).
//!
//! A static form has real pages and `AcroForm` fields beside its XFA part. A dynamic form has
//! placeholder pages, usually Acrobat's "Please wait..." page, and only Adobe's apps lay out
//! its template. Nothing here lays out XFA.

use std::collections::BTreeSet;
use std::sync::Arc;

use laminapdf_cos::{Object, Result, Stream, tolerant};

use crate::document::Document;
use crate::values::text_of;
use crate::xml::{self, Element, Node};

/// The most packets read from one `/XFA` array.
const MAX_PACKETS: usize = 256;

/// The name given to the one packet of an `/XFA` stream, which holds the whole XDP document.
///
/// Added in 0.7.0.
pub const XDP_PACKET: &str = "xdp";

/// Whether an XFA form needs Adobe's apps to show its pages.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum XfaKind {
    /// The pages are real and the `AcroForm` fields fill as usual. Adobe's apps show the XFA
    /// values, not the fields' own.
    Static,
    /// The pages are placeholders: only Adobe's apps can show or fill the form.
    Dynamic,
}

impl XfaKind {
    /// The name the bindings use: `"static"` or `"dynamic"`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Dynamic => "dynamic",
        }
    }
}

/// A fact that makes an XFA form dynamic.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum XfaSignal {
    /// The catalog's `/NeedsRendering` is true (ISO 32000-2 §7.7.2).
    NeedsRendering,
    /// The `config` packet asks for dynamic rendering: `dynamicRender` is `required`.
    DynamicRender,
    /// The `AcroForm` has no terminal field, while the template has fields.
    NoAcroFormFields,
}

impl XfaSignal {
    /// The name the bindings use: `"needsRendering"`, `"dynamicRender"` or `"noAcroFormFields"`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NeedsRendering => "needsRendering",
            Self::DynamicRender => "dynamicRender",
            Self::NoAcroFormFields => "noAcroFormFields",
        }
    }
}

/// One packet of an XFA part: an XML document or fragment such as `template`, `datasets` or
/// `config`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct XfaPacket {
    /// Its name from the `/XFA` array, or [`XDP_PACKET`] for the one packet of a stream.
    pub name: String,
    /// The decoded bytes.
    pub bytes: Vec<u8>,
}

/// The form's XFA part.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Xfa {
    /// Static or dynamic.
    pub kind: XfaKind,
    /// What made it dynamic, empty for a static form.
    pub signals: Vec<XfaSignal>,
    /// The packets, in the order the part lists them. A packet that can't be read (a stream that
    /// fails to decode or goes over a limit) is left out.
    pub packets: Vec<XfaPacket>,
}

impl Xfa {
    /// The packet called `name`, such as `template`, `datasets` or `config`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn packet(&self, name: &str) -> Option<&XfaPacket> {
        self.packets.iter().find(|p| p.name == name)
    }
}

impl Document {
    /// The form's XFA part: its packets (the `/XFA` entry as one stream, or as an array of
    /// names and streams), read within the document's [`Limits`](laminapdf_syntax::Limits), and
    /// whether the form is static or dynamic. `None` when the form has no `/XFA`.
    ///
    /// The kind is dynamic on any of three signals ([`XfaSignal`]): the catalog's
    /// `/NeedsRendering` is true, the `config` packet asks for dynamic rendering, or the
    /// `AcroForm` has no terminal field while the template has fields. Anything else is static.
    /// The `config` and `template` packets are read with the guarded XML reader
    /// ([`xml`](crate::xml)); one that doesn't parse gives no signal.
    ///
    /// # Errors
    /// Unavailable input, hard limits and strict-mode refusals reading the catalog, the form
    /// or its fields.
    ///
    /// Added in 0.7.0.
    pub fn xfa(&self) -> Result<Option<Xfa>> {
        let catalog = self.catalog()?;
        let Some(form) = catalog.get("AcroForm").map(|v| self.read(v)).transpose()?.flatten() else {
            return Ok(None);
        };
        let Some(entry) = form
            .as_dict()
            .and_then(|f| f.get("XFA"))
            .map(|v| self.read(v))
            .transpose()?
            .flatten()
        else {
            return Ok(None);
        };
        let packets = self.xfa_packets(&entry)?;
        let mut signals = Vec::new();
        let needs_rendering = catalog
            .get("NeedsRendering")
            .map(|v| self.read(v))
            .transpose()?
            .flatten()
            .is_some_and(|o| matches!(*o, Object::Bool(true)));
        if needs_rendering {
            signals.push(XfaSignal::NeedsRendering);
        }
        let parts = Parts::of(&packets);
        if parts.asks_for_dynamic_render() {
            signals.push(XfaSignal::DynamicRender);
        }
        if signals.is_empty() && parts.template_has_fields() && self.form()?.fields.is_empty() {
            signals.push(XfaSignal::NoAcroFormFields);
        }
        Ok(Some(Xfa {
            kind: if signals.is_empty() {
                XfaKind::Static
            } else {
                XfaKind::Dynamic
            },
            signals,
            packets,
        }))
    }

    /// The packets the `/XFA` value holds, all decoded under one budget
    /// ([`laminapdf_cos::Document::decode_streams`]), each stream object once: a stream the array
    /// lists again is left out the second time.
    fn xfa_packets(&self, entry: &Arc<Object>) -> Result<Vec<XfaPacket>> {
        let mut named: Vec<(String, Arc<Object>)> = Vec::new();
        match &**entry {
            Object::Stream(_) => named.push((XDP_PACKET.to_owned(), Arc::clone(entry))),
            Object::Array(items) => {
                let mut seen = BTreeSet::new();
                for pair in items.chunks(2).take(MAX_PACKETS) {
                    let [name, packet] = pair else { break };
                    let Some(name) = self.read(name)?.as_deref().and_then(text_of) else {
                        continue;
                    };
                    if let Some(r) = packet.as_ref()
                        && !seen.insert(r.num)
                    {
                        continue;
                    }
                    let Some(packet) = self.read(packet)? else { continue };
                    if matches!(*packet, Object::Stream(_)) {
                        named.push((name, packet));
                    }
                }
            }
            _ => {}
        }
        let streams: Vec<&Stream> = named
            .iter()
            .filter_map(|(_, o)| if let Object::Stream(s) = &**o { Some(s) } else { None })
            .collect();
        let decoded = self.cos().decode_streams(&streams);
        let mut out = Vec::new();
        for ((name, _), bytes) in named.into_iter().zip(decoded) {
            if let Some(bytes) = tolerant(bytes)? {
                out.push(XfaPacket { name, bytes });
            }
        }
        Ok(out)
    }
}

/// The `config` and `template` packets as trees, found by packet name or inside the one XDP
/// document.
struct Parts {
    config: Option<Vec<Node>>,
    template: Option<Vec<Node>>,
    xdp: Option<Vec<Node>>,
}

impl Parts {
    fn of(packets: &[XfaPacket]) -> Self {
        let find = |name: &str| packets.iter().find(|p| p.name == name).and_then(|p| parse(&p.bytes));
        let (config, template) = (find("config"), find("template"));
        // One stream holds the XDP: the packets are the children of its root.
        let xdp = if config.is_none() && template.is_none() {
            find(XDP_PACKET)
        } else {
            None
        };
        Self { config, template, xdp }
    }

    /// The elements of the packet `name`, or the children of the XDP root with that name.
    fn packet(&self, name: &str) -> Vec<&Element> {
        let own = if name == "config" { &self.config } else { &self.template };
        if let Some(tree) = own {
            return top(tree).collect();
        }
        let root = self.xdp.as_deref().and_then(|tree| top(tree).next());
        root.into_iter()
            .flat_map(Element::child_elements)
            .filter(|e| e.name.local == name)
            .collect()
    }

    /// Whether the `config` packet has `<dynamicRender>required</dynamicRender>`.
    fn asks_for_dynamic_render(&self) -> bool {
        descendants(self.packet("config"))
            .into_iter()
            .any(|e| e.name.local == "dynamicRender" && e.text().trim() == "required")
    }

    /// Whether the `template` packet has a `field` element.
    fn template_has_fields(&self) -> bool {
        descendants(self.packet("template"))
            .into_iter()
            .any(|e| e.name.local == "field")
    }
}

/// A packet's bytes as an XML tree; `None` when they aren't UTF-8 or don't parse.
fn parse(bytes: &[u8]) -> Option<Vec<Node>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    // XFA packets use prefixes their enclosing XDP declares; a bare packet doesn't declare them.
    xml::parse(text, |_| Some("urn:undeclared")).ok()
}

/// The elements at the top of a tree.
fn top(nodes: &[Node]) -> impl Iterator<Item = &Element> {
    nodes
        .iter()
        .filter_map(|n| if let Node::Element(e) = n { Some(e) } else { None })
}

/// Every element of the trees under `roots`, the roots included, in document order.
fn descendants(roots: Vec<&Element>) -> Vec<&Element> {
    let mut out = Vec::new();
    let mut stack: Vec<&Element> = roots.into_iter().rev().collect();
    while let Some(e) = stack.pop() {
        out.push(e);
        stack.extend(e.child_elements().collect::<Vec<_>>().into_iter().rev());
    }
    out
}
