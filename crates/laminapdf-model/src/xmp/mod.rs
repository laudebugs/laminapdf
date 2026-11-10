//! The XMP metadata packet (ISO 16684-1; ISO 32000-2 §14.3.2; research 04 §5.2): a tolerant
//! reader, typed access to the properties the Info dictionary mirrors, and a writer that
//! changes only those properties.
//!
//! [`Packet::parse`] reads a packet's RDF/XML: the `<?xpacket … ?>` wrapper (optional),
//! `x:xmpmeta` (optional), `rdf:RDF` and its `rdf:Description` elements, properties in
//! attribute and element form, `rdf:parseType="Resource"` structures, `rdf:Alt`, `rdf:Seq`
//! and `rdf:Bag` with `xml:lang`, namespaces by URI whatever their prefix. It refuses a DTD,
//! so no entity expands (XXE, billion laughs), and decodes only XML's predefined and numeric
//! character references. A packet it can't read is a [`ParseError`]; the document raises
//! `xmp-invalid` for it and reports [`PacketState::Malformed`] ([`Document::xmp`]).
//!
//! The typed getters and setters cover the fields of [`MAPPINGS`], the one table that also
//! drives the builder's packet. [`Packet::write`] gives the packet back:
//!
//! * unchanged, the bytes it was parsed from;
//! * changed, the same bytes with only the changed properties' text replaced, removed or
//!   added to a description (declaring a namespace when the packet hasn't bound it), and the
//!   padding before `<?xpacket end … ?>` shortened or lengthened so the packet keeps its
//!   length when it fits (else 2 KB of fresh padding);
//! * when that can't be done in place (a packet not in UTF-8), rewritten canonically with every
//!   property kept, which [`Written::canonical`] reports.
//!
//! ```
//! use laminapdf_model::xmp::Packet;
//!
//! let mut packet = Packet::parse(br#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
//!  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
//!   <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"
//!       xmlns:pdf="http://ns.adobe.com/pdf/1.3/" pdf:Producer="Some writer">
//!    <dc:title><rdf:Alt><rdf:li xml:lang="x-default">Old</rdf:li></rdf:Alt></dc:title>
//!   </rdf:Description>
//!  </rdf:RDF>
//! </x:xmpmeta>"#)?;
//! assert_eq!((packet.title().as_deref(), packet.producer().as_deref()), (Some("Old"), Some("Some writer")));
//!
//! packet.set_title(Some("New"));
//! let written = packet.write();
//! assert!(!written.canonical);
//! assert_eq!(Packet::parse(&written.bytes)?.title().as_deref(), Some("New"));
//! # Ok::<(), laminapdf_model::xmp::ParseError>(())
//! ```
//!
//! [`Document::xmp`]: crate::Document::xmp

mod mapping;
mod rdf;
mod tree;
mod write;

use std::collections::BTreeMap;

pub use mapping::{
    ADOBE_META, DC, MAPPINGS, MappedField, Mapping, PDF, PDFA_ID, PDFUA_ID, RDF, STANDARD_PREFIXES, ValueKind, XML,
    XMP, XMP_MM,
};
pub use rdf::{ArrayItem, ArrayKind, RdfProperty, RdfValue};
pub use write::{escape, is_xml_char, property_element};

use crate::date::PdfDate;
use rdf::Rdf;
use tree::Node;

/// A mapped field's value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Value {
    /// Text: a [`ValueKind::Text`] property or the `x-default` entry of a
    /// [`ValueKind::LangAlt`] one.
    Text(String),
    /// The items of a [`ValueKind::Seq`] property, in order.
    List(Vec<String>),
    /// A [`ValueKind::Date`] property.
    Date(PdfDate),
}

impl Value {
    const fn fits(&self, kind: ValueKind) -> bool {
        matches!(
            (kind, self),
            (ValueKind::LangAlt | ValueKind::Text, Self::Text(_))
                | (ValueKind::Seq, Self::List(_))
                | (ValueKind::Date, Self::Date(_))
        )
    }
}

/// Why a packet couldn't be read. Positions are byte offsets into the packet as UTF-8 (as
/// decoded from UTF-16 for such a packet).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The packet is neither UTF-8 nor UTF-16.
    #[error("the packet is neither UTF-8 nor UTF-16")]
    Encoding,
    /// The packet declares a DTD, which XMP doesn't allow and which could define entities.
    #[error("the packet declares a DTD at byte {offset}")]
    Dtd {
        /// Where the `<!DOCTYPE` starts.
        offset: u64,
    },
    /// A reference to an entity other than XML's five predefined ones.
    #[error("the packet refers to an undefined entity at byte {offset}")]
    UndefinedEntity {
        /// Where the reference starts.
        offset: u64,
    },
    /// A prefix no `xmlns` declaration binds.
    #[error("the packet uses an undeclared prefix at byte {offset}")]
    UnboundPrefix {
        /// Where the element starts.
        offset: u64,
    },
    /// Elements nest deeper than 64 levels.
    #[error("the packet nests elements too deeply at byte {offset}")]
    TooDeep {
        /// Where the element that is too deep starts.
        offset: u64,
    },
    /// The packet is not well-formed XML.
    #[error("the packet is not well-formed XML near byte {offset}")]
    Syntax {
        /// About where reading stopped.
        offset: u64,
    },
    /// Well-formed XML without an `rdf:RDF` element.
    #[error("the packet has no rdf:RDF element")]
    NoRdf,
}

impl ParseError {
    /// Where in the packet the problem is, when there is one place.
    #[must_use]
    pub const fn offset(&self) -> Option<u64> {
        match *self {
            Self::Dtd { offset }
            | Self::UndefinedEntity { offset }
            | Self::UnboundPrefix { offset }
            | Self::TooDeep { offset }
            | Self::Syntax { offset } => Some(offset),
            Self::Encoding | Self::NoRdf => None,
        }
    }

    /// A short stable name for the problem, the `reason` of the `xmp-invalid` warning:
    /// `encoding`, `dtd`, `entity`, `prefix`, `depth`, `syntax` or `no-rdf`.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::Encoding => "encoding",
            Self::Dtd { .. } => "dtd",
            Self::UndefinedEntity { .. } => "entity",
            Self::UnboundPrefix { .. } => "prefix",
            Self::TooDeep { .. } => "depth",
            Self::Syntax { .. } => "syntax",
            Self::NoRdf => "no-rdf",
        }
    }
}

/// A value of the wrong shape for its field, such as a list for the title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the value's shape doesn't match the field's")]
#[non_exhaustive]
pub struct KindMismatch {
    /// The field.
    pub field: MappedField,
}

/// What the document's `/Metadata` holds ([`Document::xmp`](crate::Document::xmp)).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PacketState {
    /// No packet (or one whose stream can't be decoded).
    Absent,
    /// A packet that parsed.
    Parsed(Packet),
    /// A packet that didn't parse: read as absent. Replacing it loses what it held, so an
    /// edit asks first.
    Malformed {
        /// The packet's decoded bytes.
        bytes: Vec<u8>,
        /// Why it didn't parse.
        error: ParseError,
    },
}

impl PacketState {
    /// The packet, when one parsed.
    #[must_use]
    pub const fn packet(&self) -> Option<&Packet> {
        match self {
            Self::Parsed(p) => Some(p),
            Self::Absent | Self::Malformed { .. } => None,
        }
    }

    /// Whether the document has a packet that couldn't be read.
    #[must_use]
    pub const fn is_malformed(&self) -> bool {
        matches!(self, Self::Malformed { .. })
    }
}

/// A packet as [`Packet::write`] gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Written {
    /// The packet, UTF-8.
    pub bytes: Vec<u8>,
    /// Whether the packet was rewritten whole because it couldn't be edited in place; its
    /// comments and layout are gone, its properties kept.
    pub canonical: bool,
}

/// A parsed XMP packet and the changes made to it.
#[derive(Debug, Clone)]
pub struct Packet {
    original: Vec<u8>,
    /// The packet as UTF-8: `original` itself, or decoded from UTF-16.
    text: String,
    utf8: bool,
    top: Vec<Node>,
    rdf: Rdf,
    changes: BTreeMap<MappedField, Option<Value>>,
}

impl Packet {
    /// Reads a packet: UTF-8 or UTF-16 (by its byte-order mark or its first `<`).
    ///
    /// # Errors
    /// A [`ParseError`] when the bytes aren't such a packet.
    pub fn parse(bytes: &[u8]) -> Result<Self, ParseError> {
        let (text, utf8) = match std::str::from_utf8(bytes) {
            Ok(s) => (s.to_owned(), true),
            Err(_) => (utf16(bytes).ok_or(ParseError::Encoding)?, false),
        };
        let top = tree::parse(&text)?;
        let rdf = rdf::read(&text, &top)?;
        Ok(Self {
            original: bytes.to_vec(),
            text,
            utf8,
            top,
            rdf,
            changes: BTreeMap::new(),
        })
    }

    /// A packet for a file without one: the `xpacket` wrapper, `x:xmpmeta`, one
    /// `rdf:Description` declaring `dc`, `xmp`, `pdf` and `xmpMM` with their ISO 16684-1
    /// prefixes, `dc:format` `application/pdf`, and 2 KB of padding.
    ///
    /// ```
    /// use laminapdf_model::xmp::Packet;
    ///
    /// let mut packet = Packet::new_minimal();
    /// packet.set_title(Some("Minutes"));
    /// let bytes = packet.write().bytes;
    /// assert!(bytes.starts_with("<?xpacket begin=\"\u{FEFF}\"".as_bytes()));
    /// ```
    #[must_use]
    pub fn new_minimal() -> Self {
        let text = write::minimal();
        // Our own text always parses; were it not to, an empty packet still writes one.
        Self::parse(text.as_bytes()).unwrap_or_else(|_| Self {
            original: text.clone().into_bytes(),
            text,
            utf8: true,
            top: Vec::new(),
            rdf: Rdf::default(),
            changes: BTreeMap::new(),
        })
    }

    /// The bytes the packet was parsed from.
    #[must_use]
    pub fn original(&self) -> &[u8] {
        &self.original
    }

    /// Whether a setter changed a value since the packet was parsed.
    #[must_use]
    pub fn is_changed(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Every property of the packet as parsed (before changes), in document order: attribute
    /// forms first within each description.
    pub fn properties(&self) -> impl Iterator<Item = &RdfProperty> {
        self.rdf.properties.iter().map(|p| &p.property)
    }

    /// The first value of the property `name` in `namespace`, as parsed.
    #[must_use]
    pub fn property(&self, namespace: &str, name: &str) -> Option<&RdfValue> {
        self.properties()
            .find(|p| p.namespace == namespace && p.name == name)
            .map(|p| &p.value)
    }

    /// A mapped field's value, with the changes made so far. Language alternatives read the
    /// `x-default` entry, else the first; a date that isn't ISO 8601 reads as `None`.
    #[must_use]
    pub fn get(&self, field: MappedField) -> Option<Value> {
        match self.changes.get(&field) {
            Some(changed) => changed.clone(),
            None => self.parsed(field),
        }
    }

    fn parsed(&self, field: MappedField) -> Option<Value> {
        let mapping = field.mapping();
        let value = self.property(mapping.namespace, mapping.name)?;
        match mapping.kind {
            ValueKind::LangAlt | ValueKind::Text => text_of(value).map(Value::Text),
            ValueKind::Seq => {
                let items: Vec<String> = match value {
                    RdfValue::Text(s) => vec![s.clone()],
                    RdfValue::Array { items, .. } => items
                        .iter()
                        .filter_map(|i| match &i.value {
                            RdfValue::Text(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect(),
                    RdfValue::Other(_) => Vec::new(),
                };
                (!items.is_empty()).then_some(Value::List(items))
            }
            ValueKind::Date => text_of(value)
                .and_then(|s| PdfDate::from_iso8601(&s).ok())
                .map(Value::Date),
        }
    }

    /// Sets a mapped field (`None` removes it). Setting the value it was parsed with undoes
    /// the change, so the packet writes back as it was.
    ///
    /// # Errors
    /// [`KindMismatch`] when `value`'s shape isn't the field's [`ValueKind`].
    pub fn set(&mut self, field: MappedField, value: Option<Value>) -> Result<(), KindMismatch> {
        let value = match value {
            Some(Value::List(items)) if items.is_empty() => None,
            other => other,
        };
        if value.as_ref().is_some_and(|v| !v.fits(field.mapping().kind)) {
            return Err(KindMismatch { field });
        }
        if value == self.parsed(field) {
            self.changes.remove(&field);
        } else {
            self.changes.insert(field, value);
        }
        Ok(())
    }

    /// The packet with its changes: the parsed bytes when nothing changed, otherwise edited
    /// in place or, when it can't be, rewritten canonically (see the module docs).
    #[must_use]
    pub fn write(&self) -> Written {
        if self.changes.is_empty() {
            return Written {
                bytes: self.original.clone(),
                canonical: false,
            };
        }
        let changes: Vec<write::Change<'_>> = self
            .changes
            .iter()
            .map(|(field, value)| (field.mapping(), value.as_ref()))
            .collect();
        if self.utf8
            && let Some(text) = write::in_place(&self.text, &self.top, &self.rdf, &changes)
        {
            return Written {
                bytes: text.into_bytes(),
                canonical: false,
            };
        }
        Written {
            bytes: write::canonical(&self.text, &self.rdf, &changes).into_bytes(),
            canonical: true,
        }
    }

    fn text(&self, field: MappedField) -> Option<String> {
        match self.get(field)? {
            Value::Text(s) => Some(s),
            Value::List(_) | Value::Date(_) => None,
        }
    }

    fn date(&self, field: MappedField) -> Option<PdfDate> {
        match self.get(field)? {
            Value::Date(d) => Some(d),
            Value::Text(_) | Value::List(_) => None,
        }
    }

    fn set_known(&mut self, field: MappedField, value: Option<Value>) {
        // The typed setters pass the field's own shape, which `set` accepts.
        let _ = self.set(field, value);
    }

    /// `dc:title`.
    #[must_use]
    pub fn title(&self) -> Option<String> {
        self.text(MappedField::Title)
    }

    /// Sets `dc:title` (its `x-default` entry); `None` removes it.
    pub fn set_title(&mut self, title: Option<&str>) {
        self.set_known(MappedField::Title, title.map(|s| Value::Text(s.to_owned())));
    }

    /// `dc:creator`: the authors, in order (empty without any).
    #[must_use]
    pub fn authors(&self) -> Vec<String> {
        match self.get(MappedField::Authors) {
            Some(Value::List(items)) => items,
            _ => Vec::new(),
        }
    }

    /// Sets `dc:creator`; an empty list removes it.
    pub fn set_authors<S: AsRef<str>>(&mut self, authors: &[S]) {
        let items = authors.iter().map(|a| a.as_ref().to_owned()).collect();
        self.set_known(MappedField::Authors, Some(Value::List(items)));
    }

    /// `dc:description`.
    #[must_use]
    pub fn subject(&self) -> Option<String> {
        self.text(MappedField::Subject)
    }

    /// Sets `dc:description` (its `x-default` entry); `None` removes it.
    pub fn set_subject(&mut self, subject: Option<&str>) {
        self.set_known(MappedField::Subject, subject.map(|s| Value::Text(s.to_owned())));
    }

    /// `pdf:Keywords`.
    #[must_use]
    pub fn keywords(&self) -> Option<String> {
        self.text(MappedField::Keywords)
    }

    /// Sets `pdf:Keywords`; `None` removes it.
    pub fn set_keywords(&mut self, keywords: Option<&str>) {
        self.set_known(MappedField::Keywords, keywords.map(|s| Value::Text(s.to_owned())));
    }

    /// `xmp:CreatorTool`: the application the content came from (Info's `/Creator`).
    #[must_use]
    pub fn creator_tool(&self) -> Option<String> {
        self.text(MappedField::Creator)
    }

    /// Sets `xmp:CreatorTool`; `None` removes it.
    pub fn set_creator_tool(&mut self, tool: Option<&str>) {
        self.set_known(MappedField::Creator, tool.map(|s| Value::Text(s.to_owned())));
    }

    /// `pdf:Producer`: the application that wrote the PDF.
    #[must_use]
    pub fn producer(&self) -> Option<String> {
        self.text(MappedField::Producer)
    }

    /// Sets `pdf:Producer`; `None` removes it.
    pub fn set_producer(&mut self, producer: Option<&str>) {
        self.set_known(MappedField::Producer, producer.map(|s| Value::Text(s.to_owned())));
    }

    /// `xmp:CreateDate`.
    #[must_use]
    pub fn create_date(&self) -> Option<PdfDate> {
        self.date(MappedField::Created)
    }

    /// Sets `xmp:CreateDate`; `None` removes it.
    pub fn set_create_date(&mut self, date: Option<PdfDate>) {
        self.set_known(MappedField::Created, date.map(Value::Date));
    }

    /// `xmp:ModifyDate`.
    #[must_use]
    pub fn modify_date(&self) -> Option<PdfDate> {
        self.date(MappedField::Modified)
    }

    /// Sets `xmp:ModifyDate`; `None` removes it.
    pub fn set_modify_date(&mut self, date: Option<PdfDate>) {
        self.set_known(MappedField::Modified, date.map(Value::Date));
    }

    /// `xmp:MetadataDate`.
    #[must_use]
    pub fn metadata_date(&self) -> Option<PdfDate> {
        self.date(MappedField::MetadataDate)
    }

    /// Sets `xmp:MetadataDate`; `None` removes it.
    pub fn set_metadata_date(&mut self, date: Option<PdfDate>) {
        self.set_known(MappedField::MetadataDate, date.map(Value::Date));
    }

    /// `xmpMM:DocumentID`.
    #[must_use]
    pub fn document_id(&self) -> Option<String> {
        self.text(MappedField::DocumentId)
    }

    /// `xmpMM:InstanceID`.
    #[must_use]
    pub fn instance_id(&self) -> Option<String> {
        self.text(MappedField::InstanceId)
    }

    /// `pdfaid:part`: the part of ISO 19005 (PDF/A) the file claims.
    #[must_use]
    pub fn pdfa_part(&self) -> Option<u8> {
        self.simple(PDFA_ID, "part")?.trim().parse().ok()
    }

    /// `pdfaid:conformance`: the PDF/A conformance level the file claims (`A`, `B` or `U`).
    #[must_use]
    pub fn pdfa_conformance(&self) -> Option<String> {
        self.simple(PDFA_ID, "conformance").map(|s| s.trim().to_owned())
    }

    /// `pdfuaid:part`: the part of ISO 14289 (PDF/UA) the file claims.
    #[must_use]
    pub fn pdfua_part(&self) -> Option<u8> {
        self.simple(PDFUA_ID, "part")?.trim().parse().ok()
    }

    fn simple(&self, namespace: &str, name: &str) -> Option<String> {
        text_of(self.property(namespace, name)?)
    }
}

/// A simple value, or the `x-default` (else the first) entry of an array.
fn text_of(value: &RdfValue) -> Option<String> {
    match value {
        RdfValue::Text(s) => Some(s.clone()),
        RdfValue::Array { items, .. } => {
            let default = items
                .iter()
                .find(|i| i.lang.as_deref().is_some_and(|l| l.eq_ignore_ascii_case("x-default")));
            match &default.or_else(|| items.first())?.value {
                RdfValue::Text(s) => Some(s.clone()),
                _ => None,
            }
        }
        RdfValue::Other(_) => None,
    }
}

/// UTF-16 text by its byte-order mark, or by a first `<` with a zero byte beside it.
fn utf16(bytes: &[u8]) -> Option<String> {
    let (big_endian, body) = match bytes {
        [0xFE, 0xFF, rest @ ..] => (true, rest),
        [0xFF, 0xFE, rest @ ..] => (false, rest),
        [0, b'<', ..] => (true, bytes),
        [b'<', 0, ..] => (false, bytes),
        _ => return None,
    };
    let units = body.as_chunks::<2>().0.iter().map(|&pair| {
        if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    });
    char::decode_utf16(units).collect::<Result<String, _>>().ok()
}

#[cfg(test)]
mod tests;
