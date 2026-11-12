//! Setting and removing any property of a packet, beyond the fields of
//! [`MAPPINGS`](super::MAPPINGS), and declaring a PDF/A packet's properties.

use super::Packet;
use super::extension::{PDFA_EXTENSION, PDFA_PROPERTY, PDFA_SCHEMA};
use super::mapping::{ADOBE_META, MAPPINGS, MappedField, RDF, XML};
use super::tree::is_name;
use super::value::{ValueError, XmpValue};
use super::write::is_xml_char;

/// A change to a property outside [`MAPPINGS`].
#[derive(Debug, Clone)]
pub(super) struct Custom {
    /// The prefix to write the namespace with when the packet binds none.
    pub(super) prefix: String,
    /// The new value; `None` removes the property.
    pub(super) value: Option<XmpValue>,
}

/// Why [`Packet::set_property`] refused a property.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PropertyError {
    /// An empty namespace, one holding characters XML can't, or one of the namespaces the
    /// packet's own structure uses (RDF, `xml:`, `x:` and PDF/A's extension schemas).
    #[error("the namespace can't hold properties")]
    Namespace,
    /// A prefix that isn't an XML name without a colon, or that starts with `xml`.
    #[error("the prefix isn't a valid XML prefix")]
    Prefix,
    /// A property name that isn't an XML name without a colon.
    #[error("the property name isn't a valid XML name")]
    Name,
    /// A property the Info dictionary mirrors, which [`Packet::set`] writes.
    #[error("the property is a mapped field")]
    Mapped(MappedField),
    /// A value that can't be written.
    #[error(transparent)]
    Value(#[from] ValueError),
}

/// Namespaces whose properties are the packet's structure, not metadata.
const RESERVED: [&str; 6] = [RDF, XML, ADOBE_META, PDFA_EXTENSION, PDFA_SCHEMA, PDFA_PROPERTY];

/// Whether `s` is a prefix or local name: an XML name with no colon.
fn is_ncname(s: &str) -> bool {
    is_name(s) && !s.contains(':')
}

/// Checks that `name` in `namespace` is a property [`Packet::set_property`] may write.
fn check_property(namespace: &str, name: &str) -> Result<(), PropertyError> {
    if namespace.is_empty() || RESERVED.contains(&namespace) || !namespace.chars().all(is_xml_char) {
        return Err(PropertyError::Namespace);
    }
    if !is_ncname(name) {
        return Err(PropertyError::Name);
    }
    match MAPPINGS.iter().find(|m| m.namespace == namespace && m.name == name) {
        Some(m) => Err(PropertyError::Mapped(m.field)),
        None => Ok(()),
    }
}

impl Packet {
    /// Sets the property `name` in `namespace` to `value`, or removes it for `None`. A new
    /// property goes to the description that declares its namespace, else the first; `prefix`
    /// names the namespace when the packet binds no prefix to it (another free one when
    /// `prefix` is taken).
    ///
    /// Struct-valued properties can only be written through a whole packet.
    ///
    /// ```
    /// use laminapdf_model::xmp::{Packet, XmpValue};
    ///
    /// let mut packet = Packet::new_minimal();
    /// packet.set_property("http://example.com/ns/", "acme", "batch", Some(XmpValue::Integer(42)))?;
    /// let text = String::from_utf8(packet.write().bytes)?;
    /// assert!(text.contains(r#"xmlns:acme="http://example.com/ns/""#) && text.contains("<acme:batch>42</acme:batch>"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// A [`PropertyError`] when the namespace, prefix, name or value can't be written, or the
    /// property is one of [`MAPPINGS`].
    ///
    /// Added in 0.2.0.
    pub fn set_property(
        &mut self,
        namespace: &str,
        prefix: &str,
        name: &str,
        value: Option<XmpValue>,
    ) -> Result<(), PropertyError> {
        check_property(namespace, name)?;
        if !is_ncname(prefix) || prefix.to_ascii_lowercase().starts_with("xml") {
            return Err(PropertyError::Prefix);
        }
        if let Some(v) = &value {
            v.check()?;
        }
        self.edits.custom.insert(
            (namespace.to_owned(), name.to_owned()),
            Custom {
                prefix: prefix.to_owned(),
                value,
            },
        );
        Ok(())
    }

    /// Removes the property `name` in `namespace` (every copy of it the packet holds).
    ///
    /// # Errors
    /// A [`PropertyError`] when the namespace or name can't hold a property, or the property is
    /// one of [`MAPPINGS`], which [`Packet::set`] removes.
    ///
    /// Added in 0.2.0.
    pub fn remove_property(&mut self, namespace: &str, name: &str) -> Result<(), PropertyError> {
        check_property(namespace, name)?;
        self.edits.custom.insert(
            (namespace.to_owned(), name.to_owned()),
            Custom {
                prefix: String::new(),
                value: None,
            },
        );
        Ok(())
    }

    /// Declares, in a packet that claims PDF/A, every property it uses that PDF/A doesn't
    /// predefine and its extension schemas don't declare yet, so the next
    /// [`write`](Packet::write) adds them ([`Written::declared`](super::Written::declared)). A
    /// structure or a qualified value is left undeclared: its type can't be told from the
    /// packet. It does nothing to a packet that doesn't claim PDF/A.
    ///
    /// Added in 0.2.0.
    pub fn declare_extensions(&mut self) {
        self.edits.declare_all = self.pdfa_part().is_some();
    }
}
