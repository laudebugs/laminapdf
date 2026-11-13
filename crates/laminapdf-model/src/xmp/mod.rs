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
//! `xmp-invalid` for it and reports [`PacketState::Malformed`] ([`Document::xmp`]). Two kinds of
//! damage other readers read past are read the same way and reported ([`Repairs`], the
//! document's `xmp-repaired`): bytes that aren't UTF-8 in the `<?xpacket … ?>` header, and a
//! standard prefix used without its declaration.
//!
//! The typed getters and setters cover the fields of [`MAPPINGS`], the one table that also
//! drives the builder's packet. [`Packet::write`] gives the packet back:
//!
//! * unchanged, the bytes it was parsed from;
//! * changed, the same bytes with only the changed properties' text replaced, removed or
//!   added to a description (declaring a namespace when the packet hasn't bound it, an
//!   undeclared standard prefix included), a repaired header's bad bytes written as `?`, and the
//!   padding before `<?xpacket end … ?>` shortened or lengthened so the packet keeps its
//!   length when it fits (else 2 KB of fresh padding);
//! * when that can't be done in place (a packet in UTF-16, or one whose trailer says
//!   `end="r"`, which ISO 16684-1 says not to edit in place), rewritten canonically with every
//!   property kept, which [`Written::canonical`] reports so the caller can say so.
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

mod canonical;
mod custom;
mod extension;
mod mapping;
mod rdf;
mod tree;
mod value;
mod write;

use std::collections::BTreeMap;

pub use custom::PropertyError;
pub use extension::{ExtensionProperty, PDFA_EXTENSION, PDFA_PROPERTY, PDFA_SCHEMA, is_predefined_in_pdfa};
pub use mapping::{
    ADOBE_META, DC, MAPPINGS, MappedField, Mapping, PDF, PDFA_ID, PDFUA_ID, RDF, STANDARD_PREFIXES, ValueKind, XML,
    XMP, XMP_MM,
};
pub use rdf::{ArrayItem, ArrayKind, RdfProperty, RdfValue};
pub use value::{LangText, ValueError, XmpReal, XmpValue};
pub use write::{escape, is_xml_char, property_element};

use crate::date::PdfDate;
use rdf::Rdf;
use tree::Node;

/// A mapped field's value.
///
/// Added in 0.2.0.
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

/// Why a packet couldn't be read. Offsets count bytes of the packet as given (of its UTF-16
/// bytes for a UTF-16 packet).
///
/// Added in 0.2.0.
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
    /// A start tag with more than 256 attributes, or more than 256 namespace declarations in
    /// scope at once: far beyond any real packet, and a way to make reading it slow.
    #[error("the packet declares too many attributes or namespaces at byte {offset}")]
    TooLarge {
        /// Where the element that goes over starts.
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
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn offset(&self) -> Option<u64> {
        match *self {
            Self::Dtd { offset }
            | Self::UndefinedEntity { offset }
            | Self::UnboundPrefix { offset }
            | Self::TooDeep { offset }
            | Self::TooLarge { offset }
            | Self::Syntax { offset } => Some(offset),
            Self::Encoding | Self::NoRdf => None,
        }
    }

    /// The error with its offset passed through `f`.
    fn map_offset(self, f: impl Fn(u64) -> u64) -> Self {
        match self {
            Self::Dtd { offset } => Self::Dtd { offset: f(offset) },
            Self::UndefinedEntity { offset } => Self::UndefinedEntity { offset: f(offset) },
            Self::UnboundPrefix { offset } => Self::UnboundPrefix { offset: f(offset) },
            Self::TooDeep { offset } => Self::TooDeep { offset: f(offset) },
            Self::TooLarge { offset } => Self::TooLarge { offset: f(offset) },
            Self::Syntax { offset } => Self::Syntax { offset: f(offset) },
            other @ (Self::Encoding | Self::NoRdf) => other,
        }
    }

    /// A short stable name for the problem, the `reason` of the `xmp-invalid` warning:
    /// `encoding`, `dtd`, `entity`, `prefix`, `depth`, `limit`, `syntax` or `no-rdf`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::Encoding => "encoding",
            Self::Dtd { .. } => "dtd",
            Self::UndefinedEntity { .. } => "entity",
            Self::UnboundPrefix { .. } => "prefix",
            Self::TooDeep { .. } => "depth",
            Self::TooLarge { .. } => "limit",
            Self::Syntax { .. } => "syntax",
            Self::NoRdf => "no-rdf",
        }
    }
}

/// A value of the wrong shape for its field, such as a list for the title.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the value's shape doesn't match the field's")]
#[non_exhaustive]
pub struct KindMismatch {
    /// The field.
    pub field: MappedField,
}

/// What the document's `/Metadata` holds ([`Document::xmp`](crate::Document::xmp)).
///
/// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn packet(&self) -> Option<&Packet> {
        match self {
            Self::Parsed(p) => Some(p),
            Self::Absent | Self::Malformed { .. } => None,
        }
    }

    /// Whether the document has a packet that couldn't be read.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_malformed(&self) -> bool {
        matches!(self, Self::Malformed { .. })
    }
}

/// A packet as [`Packet::write`] gives it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Written {
    /// The packet, UTF-8.
    pub bytes: Vec<u8>,
    /// Whether the packet was rewritten whole because it couldn't be edited in place (UTF-16,
    /// or read-only by its `end="r"` trailer); its comments and layout are gone, its properties
    /// kept. The metadata ops pass this on, so a save can say the packet was rewritten.
    pub canonical: bool,
    /// The properties it declared in the packet's PDF/A extension schemas
    /// ([`Packet::set_property`], [`Packet::declare_extensions`]).
    pub declared: Vec<ExtensionProperty>,
}

/// A parsed XMP packet and the changes made to it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone)]
pub struct Packet {
    original: Vec<u8>,
    /// The packet as UTF-8: `original` itself, decoded from UTF-16, or with each byte of its
    /// header that isn't UTF-8 read as `?` ([`Repairs::header_encoding`]).
    text: String,
    /// Whether `original` is UTF-16, which only a canonical rewrite can edit.
    utf16: bool,
    top: Vec<Node>,
    rdf: Rdf,
    /// The changes made since parsing, boxed so a [`PacketState`] stays small.
    edits: Box<Edits>,
    repairs: Repairs,
}

/// A packet's changes since it was parsed.
#[derive(Debug, Clone, Default)]
struct Edits {
    changes: BTreeMap<MappedField, Option<Value>>,
    /// Changes to properties outside [`MAPPINGS`], by namespace and name.
    custom: BTreeMap<(String, String), custom::Custom>,
    /// Declare every property a PDF/A packet uses, not only those changed.
    declare_all: bool,
}

/// Damage a packet was read through, which other readers read past too
/// ([`Packet::repairs`]); the document raises `xmp-repaired` for each.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Repairs {
    /// Bytes that aren't UTF-8 in the leading `<?xpacket … ?>` header, whose content XML doesn't
    /// read (some writers mangle its byte-order mark): each was read as `?`, so offsets and the
    /// packet's length are those of the bytes given.
    pub header_encoding: bool,
    /// One of XMP's standard prefixes (`dc`, `xmp`, `pdf`, `xmpMM`, `pdfaid`, `pdfuaid`, and the
    /// older `xap` and `xapMM`) used without its declaration, read in its standard namespace.
    pub undeclared_prefix: bool,
}

impl Packet {
    /// Reads a packet: UTF-16 when it starts with a UTF-16 byte-order mark or with `<` beside a
    /// zero byte (XML 1.0 Appendix F), else UTF-8.
    ///
    /// # Errors
    /// A [`ParseError`] when the bytes aren't such a packet.
    ///
    /// Added in 0.2.0.
    pub fn parse(bytes: &[u8]) -> Result<Self, ParseError> {
        let utf16 = utf16(bytes)?;
        let (text, header_encoding) = match &utf16 {
            Some(decoded) => (decoded.clone(), false),
            None => utf8(bytes)?,
        };
        let back = |offset: u64| match utf16 {
            Some(_) => utf16_offset(bytes, &text, offset),
            None => offset,
        };
        let (top, undeclared_prefix) = tree::parse(&text).map_err(|e| e.map_offset(back))?;
        let rdf = rdf::read(&text, &top).map_err(|e| e.map_offset(back))?;
        Ok(Self {
            original: bytes.to_vec(),
            text,
            utf16: utf16.is_some(),
            top,
            rdf,
            edits: Box::default(),
            repairs: Repairs {
                header_encoding,
                undeclared_prefix,
            },
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
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new_minimal() -> Self {
        let text = write::minimal();
        // Our own text, which a unit test parses. Were it not to, the packet would still write
        // that text back, since it has no changes; edits would be rewritten canonically.
        Self::parse(text.as_bytes()).unwrap_or_else(|_| Self {
            original: text.clone().into_bytes(),
            text,
            utf16: false,
            top: Vec::new(),
            rdf: Rdf::default(),
            edits: Box::default(),
            repairs: Repairs::default(),
        })
    }

    /// The damage the packet was read through ([`Repairs`]); none for a well-formed one.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn repairs(&self) -> Repairs {
        self.repairs
    }

    /// The bytes the packet was parsed from.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn original(&self) -> &[u8] {
        &self.original
    }

    /// The packet's XML as parsed, in UTF-8 (a UTF-16 packet decoded).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn xml(&self) -> &str {
        &self.text
    }

    /// Whether a setter changed a value since the packet was parsed.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_changed(&self) -> bool {
        !self.edits.changes.is_empty() || !self.edits.custom.is_empty() || self.edits.declare_all
    }

    /// Every property of the packet as parsed (before changes), in document order: attribute
    /// forms first within each description.
    ///
    /// Added in 0.2.0.
    pub fn properties(&self) -> impl Iterator<Item = &RdfProperty> {
        self.rdf.properties.iter().map(|p| &p.property)
    }

    /// The first value of the property `name` in `namespace`, as parsed.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn property(&self, namespace: &str, name: &str) -> Option<&RdfValue> {
        self.properties()
            .find(|p| p.namespace == namespace && p.name == name)
            .map(|p| &p.value)
    }

    /// A mapped field's value, with the changes made so far. Language alternatives read the
    /// `x-default` entry, else the first; a date that isn't ISO 8601 reads as `None`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn get(&self, field: MappedField) -> Option<Value> {
        match self.edits.changes.get(&field) {
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
    ///
    /// Added in 0.2.0.
    pub fn set(&mut self, field: MappedField, value: Option<Value>) -> Result<(), KindMismatch> {
        if value.as_ref().is_some_and(|v| !v.fits(field.mapping().kind)) {
            return Err(KindMismatch { field });
        }
        self.store(field, value);
        Ok(())
    }

    /// Records `value` (of the field's shape) as the field's new value, or forgets the change
    /// when it is the value the packet was parsed with.
    fn store(&mut self, field: MappedField, value: Option<Value>) {
        let value = match value {
            Some(Value::List(items)) if items.is_empty() => None,
            other => other,
        };
        if value == self.parsed(field) {
            self.edits.changes.remove(&field);
        } else {
            self.edits.changes.insert(field, value);
        }
    }

    /// The packet with its changes: the parsed bytes when nothing changed, otherwise edited
    /// in place or, when it can't be, rewritten canonically (see the module docs). In a packet
    /// that claims PDF/A, a property outside the schemas PDF/A predefines is declared in its
    /// extension schemas ([`Written::declared`]).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn write(&self) -> Written {
        if !self.is_changed() {
            return Written {
                bytes: self.original.clone(),
                canonical: false,
                declared: Vec::new(),
            };
        }
        let mapped = self.edits.changes.iter().map(|(field, value)| {
            let m = field.mapping();
            write::Change {
                namespace: m.namespace,
                name: m.name,
                prefix: m.prefix,
                value: value.as_ref().map(|v| write::Body::Mapped(m, v)),
            }
        });
        let custom = self.edits.custom.iter().map(|((namespace, name), c)| write::Change {
            namespace,
            name,
            prefix: &c.prefix,
            value: c.value.as_ref().map(write::Body::Custom),
        });
        let changes: Vec<write::Change<'_>> = mapped.chain(custom).collect();
        let declare = match (self.pdfa_part(), self.edits.declare_all) {
            (None, _) => write::Declare::Nothing,
            (Some(_), false) => write::Declare::Changed,
            (Some(_), true) => write::Declare::All,
        };
        if !self.utf16
            && let Some((text, declared)) = write::in_place(&self.text, &self.top, &self.rdf, &changes, declare)
        {
            return Written {
                bytes: text.into_bytes(),
                canonical: false,
                declared,
            };
        }
        let (text, declared) = canonical::canonical(&self.text, &self.rdf, &changes, declare);
        Written {
            bytes: text.into_bytes(),
            canonical: true,
            declared,
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

    /// `dc:title`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn title(&self) -> Option<String> {
        self.text(MappedField::Title)
    }

    /// Sets `dc:title` (its `x-default` entry); `None` removes it.
    ///
    /// The other languages' entries are kept. One that read the same as the old `x-default`
    /// entry is the document's own language written twice, as XMP toolkits do, and takes the
    /// new value too.
    ///
    /// Added in 0.2.0.
    pub fn set_title(&mut self, title: Option<&str>) {
        self.store(MappedField::Title, title.map(|s| Value::Text(s.to_owned())));
    }

    /// `dc:creator`: the authors, in order (empty without any).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn authors(&self) -> Vec<String> {
        match self.get(MappedField::Authors) {
            Some(Value::List(items)) => items,
            _ => Vec::new(),
        }
    }

    /// Sets `dc:creator`; an empty list removes it.
    ///
    /// Added in 0.2.0.
    pub fn set_authors<S: AsRef<str>>(&mut self, authors: &[S]) {
        let items = authors.iter().map(|a| a.as_ref().to_owned()).collect();
        self.store(MappedField::Authors, Some(Value::List(items)));
    }

    /// `dc:description`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn subject(&self) -> Option<String> {
        self.text(MappedField::Subject)
    }

    /// Sets `dc:description` (its `x-default` entry); `None` removes it.
    ///
    /// The other languages' entries are kept. One that read the same as the old `x-default`
    /// entry is the document's own language written twice, as XMP toolkits do, and takes the
    /// new value too.
    ///
    /// Added in 0.2.0.
    pub fn set_subject(&mut self, subject: Option<&str>) {
        self.store(MappedField::Subject, subject.map(|s| Value::Text(s.to_owned())));
    }

    /// `pdf:Keywords`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn keywords(&self) -> Option<String> {
        self.text(MappedField::Keywords)
    }

    /// Sets `pdf:Keywords`; `None` removes it.
    ///
    /// Added in 0.2.0.
    pub fn set_keywords(&mut self, keywords: Option<&str>) {
        self.store(MappedField::Keywords, keywords.map(|s| Value::Text(s.to_owned())));
    }

    /// `xmp:CreatorTool`: the application the content came from (Info's `/Creator`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn creator_tool(&self) -> Option<String> {
        self.text(MappedField::Creator)
    }

    /// Sets `xmp:CreatorTool`; `None` removes it.
    ///
    /// Added in 0.2.0.
    pub fn set_creator_tool(&mut self, tool: Option<&str>) {
        self.store(MappedField::Creator, tool.map(|s| Value::Text(s.to_owned())));
    }

    /// `pdf:Producer`: the application that wrote the PDF.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn producer(&self) -> Option<String> {
        self.text(MappedField::Producer)
    }

    /// Sets `pdf:Producer`; `None` removes it.
    ///
    /// Added in 0.2.0.
    pub fn set_producer(&mut self, producer: Option<&str>) {
        self.store(MappedField::Producer, producer.map(|s| Value::Text(s.to_owned())));
    }

    /// `xmp:CreateDate`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn create_date(&self) -> Option<PdfDate> {
        self.date(MappedField::Created)
    }

    /// Sets `xmp:CreateDate`; `None` removes it.
    ///
    /// Added in 0.2.0.
    pub fn set_create_date(&mut self, date: Option<PdfDate>) {
        self.store(MappedField::Created, date.map(Value::Date));
    }

    /// `xmp:ModifyDate`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn modify_date(&self) -> Option<PdfDate> {
        self.date(MappedField::Modified)
    }

    /// Sets `xmp:ModifyDate`; `None` removes it.
    ///
    /// Added in 0.2.0.
    pub fn set_modify_date(&mut self, date: Option<PdfDate>) {
        self.store(MappedField::Modified, date.map(Value::Date));
    }

    /// `xmp:MetadataDate`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn metadata_date(&self) -> Option<PdfDate> {
        self.date(MappedField::MetadataDate)
    }

    /// Sets `xmp:MetadataDate`; `None` removes it.
    ///
    /// Added in 0.2.0.
    pub fn set_metadata_date(&mut self, date: Option<PdfDate>) {
        self.store(MappedField::MetadataDate, date.map(Value::Date));
    }

    /// `xmpMM:DocumentID`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn document_id(&self) -> Option<String> {
        self.text(MappedField::DocumentId)
    }

    /// `xmpMM:InstanceID`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn instance_id(&self) -> Option<String> {
        self.text(MappedField::InstanceId)
    }

    /// `pdfaid:part`: the part of ISO 19005 (PDF/A) the file claims.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn pdfa_part(&self) -> Option<u8> {
        self.simple(PDFA_ID, "part")?.trim().parse().ok()
    }

    /// `pdfaid:conformance`: the PDF/A conformance level the file claims (`A`, `B` or `U`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn pdfa_conformance(&self) -> Option<String> {
        self.simple(PDFA_ID, "conformance").map(|s| s.trim().to_owned())
    }

    /// `pdfuaid:part`: the part of ISO 14289 (PDF/UA) the file claims.
    ///
    /// Added in 0.2.0.
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

/// The packet as UTF-8 text, and whether its header was repaired. Bytes that aren't UTF-8 are
/// refused, except inside the leading `<?xpacket … ?>` header, whose content XML doesn't read:
/// some writers mangle its byte-order mark (`begin="\xEF\xBB?"`), and each such byte is read as
/// `?`, one byte for one, so offsets and lengths stay those of the bytes given.
fn utf8(bytes: &[u8]) -> Result<(String, bool), ParseError> {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok((text.to_owned(), false));
    }
    let header_end = bytes
        .starts_with(b"<?xpacket")
        .then(|| bytes.windows(2).position(|w| w == b"?>"))
        .flatten()
        .ok_or(ParseError::Encoding)?;
    let (header, rest) = bytes.split_at(header_end);
    let rest = std::str::from_utf8(rest).map_err(|_| ParseError::Encoding)?;
    let mut header = header.to_vec();
    while let Err(e) = std::str::from_utf8(&header) {
        let bad = e.error_len().unwrap_or(header.len() - e.valid_up_to());
        for b in header.iter_mut().skip(e.valid_up_to()).take(bad) {
            *b = b'?';
        }
    }
    let header = String::from_utf8(header).map_err(|_| ParseError::Encoding)?;
    Ok((format!("{header}{rest}"), true))
}

/// The packet decoded, when it is UTF-16: by its byte-order mark, or by a first `<` with a
/// zero byte beside it; `None` for any other packet.
fn utf16(bytes: &[u8]) -> Result<Option<String>, ParseError> {
    let (big_endian, body) = match bytes {
        [0xFE, 0xFF, rest @ ..] => (true, rest),
        [0xFF, 0xFE, rest @ ..] => (false, rest),
        [0, b'<', ..] => (true, bytes),
        [b'<', 0, ..] => (false, bytes),
        _ => return Ok(None),
    };
    let (pairs, rest) = body.as_chunks::<2>();
    if !rest.is_empty() {
        return Err(ParseError::Encoding);
    }
    let units = pairs.iter().map(|&pair| {
        if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    });
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map(Some)
        .map_err(|_| ParseError::Encoding)
}

/// The byte offset in a UTF-16 packet's `bytes` of `offset` in its decoded `text`.
fn utf16_offset(bytes: &[u8], text: &str, offset: u64) -> u64 {
    let bom = if matches!(bytes, [0xFE, 0xFF, ..] | [0xFF, 0xFE, ..]) {
        2
    } else {
        0
    };
    let end = usize::try_from(offset).unwrap_or(usize::MAX).min(text.len());
    let units: usize = text
        .char_indices()
        .take_while(|(i, _)| *i < end)
        .map(|(_, c)| c.len_utf16())
        .sum();
    bom + 2 * units as u64
}

#[cfg(test)]
mod tests;
