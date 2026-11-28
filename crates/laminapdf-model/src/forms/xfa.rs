//! The form's XFA part (`/AcroForm /XFA`, ISO 32000-2 §12.7.9, deprecated in PDF 2.0): its
//! packets as bytes and whether the form is static or dynamic ([`Document::xfa`]).
//!
//! A static form has real pages and `AcroForm` fields beside its XFA part. A dynamic form has
//! placeholder pages, usually Acrobat's "Please wait..." page, and only Adobe's apps lay out
//! its template. Nothing here lays out XFA.

use laminapdf_cos::{Object, Result, tolerant};

use crate::document::Document;
use crate::values::text_of;
use crate::xml::{self, Element, Node};

/// The most packets read from one `/XFA` array.
const MAX_PACKETS: usize = 256;

/// The name given to the one packet of an `/XFA` stream, which holds the whole XDP document.
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
        if parts.config.as_deref().is_some_and(asks_for_dynamic_render) {
            signals.push(XfaSignal::DynamicRender);
        }
        if signals.is_empty() && parts.template.as_deref().is_some_and(has_fields) && self.form()?.fields.is_empty() {
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

    /// The packets the `/XFA` value holds.
    fn xfa_packets(&self, entry: &Object) -> Result<Vec<XfaPacket>> {
        let mut out = Vec::new();
        match entry {
            Object::Stream(s) => {
                if let Some(bytes) = tolerant(self.cos().decode_stream(s))? {
                    out.push(XfaPacket {
                        name: XDP_PACKET.to_owned(),
                        bytes,
                    });
                }
            }
            Object::Array(items) => {
                for pair in items.chunks(2).take(MAX_PACKETS) {
                    let [name, packet] = pair else { break };
                    let Some(name) = self.read(name)?.as_deref().and_then(text_of) else {
                        continue;
                    };
                    let Some(packet) = self.read(packet)? else { continue };
                    let Object::Stream(s) = &*packet else { continue };
                    if let Some(bytes) = tolerant(self.cos().decode_stream(s))? {
                        out.push(XfaPacket { name, bytes });
                    }
                }
            }
            _ => {}
        }
        Ok(out)
    }
}

/// The `config` and `template` packets as trees, found by packet name or inside the one XDP
/// document.
struct Parts {
    config: Option<Vec<Node>>,
    template: Option<Vec<Node>>,
}

impl Parts {
    fn of(packets: &[XfaPacket]) -> Self {
        let find = |name: &str| packets.iter().find(|p| p.name == name).and_then(|p| parse(&p.bytes));
        let (mut config, mut template) = (find("config"), find("template"));
        if config.is_none() && template.is_none() {
            // One stream holds the XDP: the packets are the children of its root.
            let xdp = packets
                .iter()
                .find(|p| p.name == XDP_PACKET)
                .and_then(|p| parse(&p.bytes));
            let root = xdp
                .iter()
                .flatten()
                .find_map(|n| if let Node::Element(e) = n { Some(e) } else { None });
            if let Some(root) = root {
                let child = |local: &str| {
                    root.child_elements()
                        .find(|e| e.name.local == local)
                        .map(|e| vec![Node::Element(e.clone())])
                };
                config = child("config");
                template = child("template");
            }
        }
        Self { config, template }
    }
}

/// A packet's bytes as an XML tree; `None` when they aren't UTF-8 or don't parse.
fn parse(bytes: &[u8]) -> Option<Vec<Node>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    // XFA packets use prefixes their enclosing XDP declares; a bare packet doesn't declare them.
    xml::parse(text, |_| Some("urn:undeclared")).ok()
}

/// Every element of `nodes`, in document order.
fn elements(nodes: &[Node]) -> Vec<&Element> {
    let mut out = Vec::new();
    let mut stack: Vec<&Node> = nodes.iter().rev().collect();
    while let Some(node) = stack.pop() {
        if let Node::Element(e) = node {
            out.push(e);
            stack.extend(e.children.iter().rev());
        }
    }
    out
}

/// Whether a `config` packet has `<dynamicRender>required</dynamicRender>`.
fn asks_for_dynamic_render(config: &[Node]) -> bool {
    elements(config)
        .into_iter()
        .any(|e| e.name.local == "dynamicRender" && e.text().trim() == "required")
}

/// Whether a `template` packet has a `field` element.
fn has_fields(template: &[Node]) -> bool {
    elements(template).into_iter().any(|e| e.name.local == "field")
}
