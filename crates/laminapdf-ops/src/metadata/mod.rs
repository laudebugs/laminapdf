//! The metadata ops: [`SetMetadata`], [`RemoveMetadata`] and the save-time
//! [`UpdateSaveMetadata`], which write the Info dictionary (ISO 32000-2 §14.3.3) and the XMP
//! packet (§14.3.2) together from one value, through the one Info ↔ XMP table
//! ([`laminapdf_model::xmp::MAPPINGS`]); the read view [`DocumentMetadata`] that says which side
//! won and where they disagree; and the [`ConformanceWarning`]s an edit can raise in a PDF/A or
//! PDF/UA file.
//!
//! The rules every metadata op follows:
//!
//! * Both sides get the same value, at the same precision and time zone. Authors are a list:
//!   `dc:creator` holds them as an `rdf:Seq`, Info's `/Author` joined with `; `. Characters
//!   XML can't hold (C0 controls but tab, line feed and carriage return) are left out of both,
//!   as the builder does.
//! * A file without a packet gets a minimal one ([`Packet::new_minimal`]) the first time an
//!   edit sets a value; a file with one keeps every property Lamina PDF doesn't map, in place.
//!   A packet that can't be read ([`PacketState::Malformed`]) is replaced only when the op says
//!   so ([`SetMetadata::replace_malformed`]); otherwise the op fails with `malformed-xmp`.
//! * In a PDF 2.0 file, where ISO 32000-2 §14.3.3 deprecates every Info entry but the two
//!   dates, an edit writes the other fields to XMP only and leaves Info's old entries as they
//!   were (a removal removes them from both).
//! * The XMP stream is written unfiltered, as PDF/A requires of the metadata stream.
//!
//! [`Packet::new_minimal`]: laminapdf_model::xmp::Packet::new_minimal
//! [`PacketState::Malformed`]: laminapdf_model::xmp::PacketState::Malformed

mod conformance;
mod read;
mod write;

use laminapdf_model::PdfDate;
use laminapdf_model::xmp::{MappedField, Value, is_xml_char};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::op::Effect;
use crate::wire;

pub use conformance::{ConformanceWarning, conformance_warnings};
pub use read::{DocumentMetadata, MetadataSource, PacketStatus, read_metadata};
pub(crate) use write::{Policy, write};

/// A metadata field Lamina PDF keeps in step in the Info dictionary and the XMP packet: what
/// [`RemoveMetadata`] removes and [`DocumentMetadata::disagreements`] names.
///
/// | Key | Info | XMP |
/// | --- | --- | --- |
/// | `title` | `/Title` | `dc:title` |
/// | `authors` | `/Author` (joined with `; `) | `dc:creator` |
/// | `subject` | `/Subject` | `dc:description` |
/// | `keywords` | `/Keywords` | `pdf:Keywords` |
/// | `creator` | `/Creator` | `xmp:CreatorTool` |
/// | `producer` | `/Producer` | `pdf:Producer` |
/// | `created` | `/CreationDate` | `xmp:CreateDate` |
/// | `modified` | `/ModDate` | `xmp:ModifyDate` |
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum MetadataKey {
    /// The title.
    Title,
    /// The authors.
    Authors,
    /// The subject (a description).
    Subject,
    /// The keywords.
    Keywords,
    /// The application the content came from.
    Creator,
    /// The application that wrote the PDF.
    Producer,
    /// The creation date.
    Created,
    /// The modification date.
    Modified,
}

impl MetadataKey {
    /// Every key, in the table's order.
    pub const ALL: [Self; 8] = [
        Self::Title,
        Self::Authors,
        Self::Subject,
        Self::Keywords,
        Self::Creator,
        Self::Producer,
        Self::Created,
        Self::Modified,
    ];

    /// The key's row of the shared Info ↔ XMP table.
    #[must_use]
    pub const fn field(self) -> MappedField {
        match self {
            Self::Title => MappedField::Title,
            Self::Authors => MappedField::Authors,
            Self::Subject => MappedField::Subject,
            Self::Keywords => MappedField::Keywords,
            Self::Creator => MappedField::Creator,
            Self::Producer => MappedField::Producer,
            Self::Created => MappedField::Created,
            Self::Modified => MappedField::Modified,
        }
    }

    /// The coalescing key for edits of this field: its XMP property's local name (`title`,
    /// `creator`, `CreatorTool`, …), one per field. Consecutive
    /// [`transact_coalescing`](crate::Document::transact_coalescing) edits with it merge into
    /// one undo step (typing into one field of a properties dialog, ADR 0006 §4).
    #[must_use]
    pub const fn coalescing_key(self) -> &'static str {
        self.field().mapping().name
    }
}

/// Changes to the document's metadata. For each field, `None` leaves it alone, `Some(None)`
/// removes it and `Some(Some(value))` sets it, in Info and XMP alike.
///
/// On the wire an absent field is left alone and `null` removes: `{ "title": "Report",
/// "authors": ["Ada Lovelace", "Charles Babbage"], "keywords": null }`; dates are PDF date
/// strings (`"D:20261110093000+01'00'"`).
///
/// ```
/// use laminapdf_ops::MetadataPatch;
///
/// let patch = MetadataPatch::new().title(Some("Quarterly figures")).authors(&["Ada Lovelace", "Charles Babbage"]);
/// assert_eq!(patch.authors, Some(Some(vec!["Ada Lovelace".to_owned(), "Charles Babbage".to_owned()])));
/// assert_eq!(serde_json::to_string(&patch)?, r#"{"title":"Quarterly figures","authors":["Ada Lovelace","Charles Babbage"]}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct MetadataPatch {
    /// `/Title`, `dc:title`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub title: Option<Option<String>>,
    /// `/Author` (joined with `; `), `dc:creator` (in order). An empty list removes them.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string[] | null"))]
    pub authors: Option<Option<Vec<String>>>,
    /// `/Subject`, `dc:description`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub subject: Option<Option<String>>,
    /// `/Keywords`, `pdf:Keywords`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub keywords: Option<Option<String>>,
    /// `/Creator`, `xmp:CreatorTool`: the application the content came from.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub creator: Option<Option<String>>,
    /// `/Producer`, `pdf:Producer`: the application that wrote the PDF.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub producer: Option<Option<String>>,
    /// `/CreationDate`, `xmp:CreateDate`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::double_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub created: Option<Option<PdfDate>>,
    /// `/ModDate`, `xmp:ModifyDate`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::double_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub modified: Option<Option<PdfDate>>,
}

impl MetadataPatch {
    /// A patch that changes nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the title, or removes it for `None`.
    #[must_use]
    pub fn title(mut self, title: Option<&str>) -> Self {
        self.title = Some(title.map(str::to_owned));
        self
    }

    /// Sets the authors, in order; an empty list removes them.
    #[must_use]
    pub fn authors<S: AsRef<str>>(mut self, authors: &[S]) -> Self {
        let authors: Vec<String> = authors.iter().map(|a| a.as_ref().to_owned()).collect();
        self.authors = Some((!authors.is_empty()).then_some(authors));
        self
    }

    /// Sets the subject, or removes it for `None`.
    #[must_use]
    pub fn subject(mut self, subject: Option<&str>) -> Self {
        self.subject = Some(subject.map(str::to_owned));
        self
    }

    /// Sets the keywords, or removes them for `None`.
    #[must_use]
    pub fn keywords(mut self, keywords: Option<&str>) -> Self {
        self.keywords = Some(keywords.map(str::to_owned));
        self
    }

    /// Sets the creator tool, or removes it for `None`.
    #[must_use]
    pub fn creator(mut self, creator: Option<&str>) -> Self {
        self.creator = Some(creator.map(str::to_owned));
        self
    }

    /// Sets the producer, or removes it for `None`.
    #[must_use]
    pub fn producer(mut self, producer: Option<&str>) -> Self {
        self.producer = Some(producer.map(str::to_owned));
        self
    }

    /// Sets the creation date, or removes it for `None`.
    #[must_use]
    pub const fn created(mut self, created: Option<PdfDate>) -> Self {
        self.created = Some(created);
        self
    }

    /// Sets the modification date, or removes it for `None`.
    #[must_use]
    pub const fn modified(mut self, modified: Option<PdfDate>) -> Self {
        self.modified = Some(modified);
        self
    }

    /// The patch as the writer's changes, in the table's order: text with the characters XML
    /// can't hold left out, authors without empty entries.
    fn changes(&self) -> Vec<(MappedField, Option<Value>)> {
        let text = |key: MetadataKey, value: &Option<Option<String>>| {
            value
                .as_ref()
                .map(|v| (key.field(), v.as_deref().map(|s| Value::Text(xml_text(s)))))
        };
        let authors = self.authors.as_ref().map(|v| {
            let items: Vec<String> = v
                .iter()
                .flatten()
                .map(|a| xml_text(a))
                .filter(|a| !a.is_empty())
                .collect();
            (MappedField::Authors, (!items.is_empty()).then_some(Value::List(items)))
        });
        let date = |key: MetadataKey, value: Option<Option<PdfDate>>| value.map(|v| (key.field(), v.map(Value::Date)));
        [
            text(MetadataKey::Title, &self.title),
            authors,
            text(MetadataKey::Subject, &self.subject),
            text(MetadataKey::Keywords, &self.keywords),
            text(MetadataKey::Creator, &self.creator),
            text(MetadataKey::Producer, &self.producer),
            date(MetadataKey::Created, self.created),
            date(MetadataKey::Modified, self.modified),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// `s` without the characters XML can't hold.
fn xml_text(s: &str) -> String {
    s.chars().filter(|&c| is_xml_char(c)).collect()
}

/// Edits the document's metadata: applies `patch` to the Info dictionary and the XMP packet
/// together and, when `at` is given, records it as the modification date (`/ModDate`,
/// `xmp:ModifyDate`) and as `xmp:MetadataDate`, unless the patch sets the modification date
/// itself. The date is part of the op, never read from a clock (ADR 0006 §1).
///
/// ```
/// use laminapdf_ops::{MetadataPatch, Op, SetMetadata};
///
/// let op = Op::from(SetMetadata::new(MetadataPatch::new().title(Some("Minutes")), Some("D:20261110093000Z".parse()?)));
/// assert_eq!(
///     serde_json::to_string(&op)?,
///     r#"{"op":"setMetadata","args":{"patch":{"title":"Minutes"},"at":"D:20261110093000Z"}}"#
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetMetadata {
    /// The fields to set or remove.
    #[serde(default)]
    pub patch: MetadataPatch,
    /// The modification date the edit writes.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub at: Option<PdfDate>,
    /// Replace an XMP packet that can't be read with a new one, losing whatever it held.
    /// Without it, such a packet makes the op fail with `malformed-xmp`, so nothing is lost
    /// without the user saying so.
    #[serde(default, skip_serializing_if = "wire::is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub replace_malformed: bool,
}

impl SetMetadata {
    /// The op that applies `patch`, recording `at` as the modification date when given.
    #[must_use]
    pub const fn new(patch: MetadataPatch, at: Option<PdfDate>) -> Self {
        Self {
            patch,
            at,
            replace_malformed: false,
        }
    }

    /// The same op, replacing an XMP packet that can't be read
    /// ([`SetMetadata::replace_malformed`]).
    #[must_use]
    pub const fn replacing_malformed(mut self) -> Self {
        self.replace_malformed = true;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let mut changes = self.patch.changes();
        if let Some(at) = self.at {
            if self.patch.modified.is_none() {
                changes.push((MappedField::Modified, Some(Value::Date(at))));
            }
            changes.push((MappedField::MetadataDate, Some(Value::Date(at))));
        }
        write(doc, &changes, Policy::edit(self.replace_malformed))
    }
}

/// Removes metadata fields from the Info dictionary and the XMP packet; the packet's other
/// properties stay (J.10).
///
/// The values stay in the file's earlier revisions until a full rewrite: an incremental save
/// appends the change and keeps the original bytes. When it removed anything, the op's result
/// says so ([`Applied::still_in_earlier_revisions`](crate::Applied::still_in_earlier_revisions)),
/// so the caller can offer a full rewrite (on a signed file, the override that removes the
/// signatures).
///
/// ```
/// use laminapdf_ops::{MetadataKey, Op, RemoveMetadata};
///
/// let op = Op::from(RemoveMetadata::new(vec![MetadataKey::Authors, MetadataKey::Creator]));
/// assert_eq!(serde_json::to_string(&op)?, r#"{"op":"removeMetadata","args":{"keys":["authors","creator"]}}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct RemoveMetadata {
    /// The fields to remove.
    pub keys: Vec<MetadataKey>,
    /// As [`SetMetadata::replace_malformed`].
    #[serde(default, skip_serializing_if = "wire::is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub replace_malformed: bool,
}

impl RemoveMetadata {
    /// The op that removes `keys`.
    #[must_use]
    pub const fn new(keys: Vec<MetadataKey>) -> Self {
        Self {
            keys,
            replace_malformed: false,
        }
    }

    /// The same op, replacing an XMP packet that can't be read
    /// ([`SetMetadata::replace_malformed`]).
    #[must_use]
    pub const fn replacing_malformed(mut self) -> Self {
        self.replace_malformed = true;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let mut changes: Vec<(MappedField, Option<Value>)> = Vec::new();
        for key in &self.keys {
            if !changes.iter().any(|(f, _)| *f == key.field()) {
                changes.push((key.field(), None));
            }
        }
        write(doc, &changes, Policy::remove(self.replace_malformed))
    }
}

/// The save-time metadata ([E1 plan §2]): `xmp:MetadataDate`, `xmp:ModifyDate` and Info's
/// `/ModDate` set to `metadata_date` (when the shell gave one) and `xmpMM:InstanceID` to
/// `instance_id`.
///
/// Only the save planner makes it, from the shell's `SaveOptions`, and runs it through the
/// same classification as every other op (class `Metadata`): a save whose constraint it would
/// break leaves it out. It never goes on the undo history and never crosses the wire: it can't
/// be serialised or read from an op log.
///
/// It edits only what exists: a file without an XMP packet doesn't get one for the stamp, and a
/// packet that can't be read is left alone.
///
/// [E1 plan §2]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e1-editing-foundation.md#2-the-operations
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UpdateSaveMetadata {
    /// When the save happened, as the shell's clock read it; `None` leaves the dates alone.
    pub metadata_date: Option<PdfDate>,
    /// The new `xmpMM:InstanceID` (`uuid:…`).
    pub instance_id: String,
}

impl UpdateSaveMetadata {
    /// The stamp of a save at `metadata_date` with `instance_id`.
    #[must_use]
    pub const fn new(metadata_date: Option<PdfDate>, instance_id: String) -> Self {
        Self {
            metadata_date,
            instance_id,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let mut changes = Vec::new();
        if let Some(date) = self.metadata_date {
            changes.push((MappedField::Modified, Some(Value::Date(date))));
            changes.push((MappedField::MetadataDate, Some(Value::Date(date))));
        }
        changes.push((MappedField::InstanceId, Some(Value::Text(xml_text(&self.instance_id)))));
        write(doc, &changes, Policy::stamp())
    }
}

#[cfg(test)]
mod tests;
