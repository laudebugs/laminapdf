//! The XMP ops: [`SetXmpProperty`], [`RemoveXmpProperty`] and [`ReplaceXmpPacket`], which edit
//! any property of the packet, or the whole packet, beyond the fields [`SetMetadata`] covers.
//!
//! A property the Info dictionary mirrors (one of [`MAPPINGS`]) goes through the same writer as
//! [`SetMetadata`], so Info stays in step. The PDF/A and PDF/UA identification (`pdfaid:`,
//! `pdfuaid:`) is refused, in a property op or a replaced packet: conversion or a check makes
//! such a claim, never an edit. In a file that claims PDF/A, a property outside the schemas
//! PDF/A predefines is declared in the packet's extension schemas (ISO 19005-2 §6.6.2.3).
//!
//! [`SetMetadata`]: crate::SetMetadata
//! [`MAPPINGS`]: laminapdf_model::xmp::MAPPINGS

use laminapdf_model::xmp::{MappedField, Mapping, PDFA_ID, PDFUA_ID, Value, ValueKind, XmpValue};
use serde::{Deserialize, Serialize};

use super::write::{Policy, edit_packet, replace_packet, write};
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

/// Sets any XMP property: text, a URI, a number, a boolean, a date, a language alternative,
/// or an ordered or unordered array of simple values. A file without a packet gets one.
///
/// `prefix` names the namespace when the packet binds no prefix to it. A property the Info
/// dictionary mirrors (`dc:title`, `pdf:Producer`, …) is set in both, as
/// [`SetMetadata`](crate::SetMetadata) sets it, and must have its shape: text, or a language
/// alternative of the `x-default` entry alone, for `dc:title` and `dc:description`; an array of
/// text for `dc:creator`; a date for the dates.
///
/// ```
/// use laminapdf_ops::{Op, SetXmpProperty};
/// use laminapdf_model::xmp::XmpValue;
///
/// let op = Op::from(SetXmpProperty::new("http://example.com/ns/", "acme", "batch", XmpValue::Integer(42)));
/// assert_eq!(
///     serde_json::to_string(&op)?,
///     r#"{"op":"setXmpProperty","args":{"namespace":"http://example.com/ns/","prefix":"acme","name":"batch","value":{"type":"integer","value":42}}}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetXmpProperty {
    /// The property's namespace URI.
    pub namespace: String,
    /// The prefix to write the namespace with when the packet binds none.
    pub prefix: String,
    /// The property's local name.
    pub name: String,
    /// The new value.
    #[serde(with = "wire::xmp_value")]
    #[cfg_attr(feature = "tsify", tsify(type = "XmpValue"))]
    pub value: XmpValue,
    /// As [`SetMetadata::replace_malformed`](crate::SetMetadata::replace_malformed).
    #[serde(default, skip_serializing_if = "wire::is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub replace_malformed: bool,
}

impl SetXmpProperty {
    /// The op that sets `name` in `namespace` to `value`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn new(
        namespace: impl Into<String>,
        prefix: impl Into<String>,
        name: impl Into<String>,
        value: XmpValue,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            prefix: prefix.into(),
            name: name.into(),
            value,
            replace_malformed: false,
        }
    }

    /// The same op, replacing an XMP packet that can't be read
    /// ([`SetMetadata::replace_malformed`](crate::SetMetadata::replace_malformed)).
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn replacing_malformed(mut self) -> Self {
        self.replace_malformed = true;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        refuse_claims(&self.namespace)?;
        let policy = Policy::edit(self.replace_malformed);
        if let Some(m) = mapped(&self.namespace, &self.name) {
            return write(doc, &[(m.field, Some(mapped_value(m, &self.value)?))], policy);
        }
        edit_packet(doc, true, policy, |packet| {
            packet.set_property(&self.namespace, &self.prefix, &self.name, Some(self.value.clone()))?;
            Ok(false)
        })
    }
}

/// Removes an XMP property, every copy of it in the packet; a property the Info dictionary
/// mirrors is removed from both. The value stays in the file's earlier revisions until a full
/// rewrite ([`Applied::still_in_earlier_revisions`](crate::Applied::still_in_earlier_revisions)).
///
/// ```
/// use laminapdf_ops::{Op, RemoveXmpProperty};
///
/// let op = Op::from(RemoveXmpProperty::new("http://example.com/ns/", "batch"));
/// assert_eq!(
///     serde_json::to_string(&op)?,
///     r#"{"op":"removeXmpProperty","args":{"namespace":"http://example.com/ns/","name":"batch"}}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct RemoveXmpProperty {
    /// The property's namespace URI.
    pub namespace: String,
    /// The property's local name.
    pub name: String,
    /// As [`SetMetadata::replace_malformed`](crate::SetMetadata::replace_malformed).
    #[serde(default, skip_serializing_if = "wire::is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub replace_malformed: bool,
}

impl RemoveXmpProperty {
    /// The op that removes `name` in `namespace`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            replace_malformed: false,
        }
    }

    /// The same op, replacing an XMP packet that can't be read
    /// ([`SetMetadata::replace_malformed`](crate::SetMetadata::replace_malformed)).
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn replacing_malformed(mut self) -> Self {
        self.replace_malformed = true;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        refuse_claims(&self.namespace)?;
        let policy = Policy::remove(self.replace_malformed);
        if let Some(m) = mapped(&self.namespace, &self.name) {
            return write(doc, &[(m.field, None)], policy);
        }
        edit_packet(doc, false, policy, |packet| {
            let held = packet.property(&self.namespace, &self.name).is_some();
            packet.remove_property(&self.namespace, &self.name)?;
            Ok(held)
        })
    }
}

/// Replaces the whole XMP packet with `xml`, for what the property ops can't write
/// (structures, qualifiers, comments). The packet is parsed before it applies and refused if it
/// doesn't parse (`invalid-xmp-packet`) or changes the PDF/A or PDF/UA identification
/// (`xmp-claim-read-only`). Info's mirrored entries follow the new packet; in a file that
/// claims PDF/A, the extension schema entries its properties lack are added and reported
/// ([`Applied::declared`](crate::Applied::declared)). The old packet stays in the file's
/// earlier revisions until a full rewrite.
///
/// ```
/// use laminapdf_ops::{Op, ReplaceXmpPacket};
///
/// let op = Op::from(ReplaceXmpPacket::new("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">…</x:xmpmeta>"));
/// assert!(serde_json::to_string(&op)?.starts_with(r#"{"op":"replaceXmpPacket","args":{"xml":"<x:xmpmeta"#));
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct ReplaceXmpPacket {
    /// The new packet's XML, with or without its `<?xpacket … ?>` wrapper.
    pub xml: String,
}

impl ReplaceXmpPacket {
    /// The op that replaces the packet with `xml`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn new(xml: impl Into<String>) -> Self {
        Self { xml: xml.into() }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        replace_packet(doc, &self.xml)
    }
}

/// Refuses an edit of the PDF/A or PDF/UA identification.
fn refuse_claims(namespace: &str) -> Result<()> {
    if namespace == PDFA_ID || namespace == PDFUA_ID {
        return Err(Error::XmpClaimReadOnly);
    }
    Ok(())
}

/// The row of [`MAPPINGS`] for the property, when the Info dictionary mirrors it.
fn mapped(namespace: &str, name: &str) -> Option<&'static Mapping> {
    MappedField::from_property(namespace, name).map(MappedField::mapping)
}

/// `value` as the metadata writer takes it for `mapping`'s field: `shape` when it has the
/// wrong shape.
fn mapped_value(mapping: &Mapping, value: &XmpValue) -> Result<Value> {
    value.check().map_err(laminapdf_model::xmp::PropertyError::from)?;
    let shape = Error::InvalidXmpProperty { reason: "shape" };
    match (mapping.kind, value) {
        (ValueKind::Text | ValueKind::LangAlt, XmpValue::Text(s)) => Ok(Value::Text(s.clone())),
        (ValueKind::LangAlt, XmpValue::LangAlt(entries)) => match entries.as_slice() {
            [only] if only.lang.eq_ignore_ascii_case("x-default") => Ok(Value::Text(only.text.clone())),
            _ => Err(shape),
        },
        (ValueKind::Seq, XmpValue::Seq(items)) => items
            .iter()
            .map(|i| match i {
                XmpValue::Text(s) => Ok(s.clone()),
                _ => Err(Error::InvalidXmpProperty { reason: "shape" }),
            })
            .collect::<Result<Vec<_>>>()
            .map(Value::List),
        (ValueKind::Date, XmpValue::Date(d)) => Ok(Value::Date(*d)),
        _ => Err(shape),
    }
}

#[cfg(test)]
mod tests;
