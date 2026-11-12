//! `SetInfo`: edit the document information dictionary (ISO 32000-2 §14.3.3).
#![expect(
    deprecated,
    reason = "SetInfo is defined and dispatched here; it stays, deprecated, so v1 op logs still apply"
)]

use laminapdf_model::PdfDate;
use laminapdf_model::xmp::{MappedField, Value};

use crate::error::Result;
use crate::metadata::{Policy, write};
use crate::op::Effect;
use crate::wire;

/// Changes to the Info dictionary's text entries. For each field, `None` leaves the entry
/// alone, `Some(None)` removes it and `Some(Some(text))` sets it (encoded as
/// PDFDocEncoding when it can be, else UTF-16BE, §7.9.2.2).
///
/// On the wire an absent field is left alone and `null` clears: `{ "title": "Report",
/// "author": null }`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct InfoPatch {
    /// `/Title`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub title: Option<Option<String>>,
    /// `/Author`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub author: Option<Option<String>>,
    /// `/Subject`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub subject: Option<Option<String>>,
    /// `/Keywords`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub keywords: Option<Option<String>>,
    /// `/Creator`: the application that created the original document.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub creator: Option<Option<String>>,
    /// `/Producer`: the application that converted it to PDF.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub producer: Option<Option<String>>,
}

impl InfoPatch {
    /// A patch that changes nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `/Title` to `title`, or removes it for `None`.
    #[must_use]
    pub fn title(mut self, title: Option<&str>) -> Self {
        self.title = Some(title.map(str::to_owned));
        self
    }

    /// Sets `/Author` to `author`, or removes it for `None`.
    #[must_use]
    pub fn author(mut self, author: Option<&str>) -> Self {
        self.author = Some(author.map(str::to_owned));
        self
    }

    /// Sets `/Subject` to `subject`, or removes it for `None`.
    #[must_use]
    pub fn subject(mut self, subject: Option<&str>) -> Self {
        self.subject = Some(subject.map(str::to_owned));
        self
    }

    /// Sets `/Keywords` to `keywords`, or removes it for `None`.
    #[must_use]
    pub fn keywords(mut self, keywords: Option<&str>) -> Self {
        self.keywords = Some(keywords.map(str::to_owned));
        self
    }

    /// Sets `/Creator` to `creator`, or removes it for `None`.
    #[must_use]
    pub fn creator(mut self, creator: Option<&str>) -> Self {
        self.creator = Some(creator.map(str::to_owned));
        self
    }

    /// Sets `/Producer` to `producer`, or removes it for `None`.
    #[must_use]
    pub fn producer(mut self, producer: Option<&str>) -> Self {
        self.producer = Some(producer.map(str::to_owned));
        self
    }
}

/// Edits the Info dictionary: applies `patch` and, when `at` is given, sets `/ModDate` to it.
///
/// **Deprecated:** use [`SetMetadata`](crate::SetMetadata), which writes authors as a list and
/// the dates, and edits files whose metadata is in XMP alone. `SetInfo` stays, with its v1 wire
/// form (`setInfo`) and meaning, so op logs already written still apply (ADR 0006 §6). It now
/// goes through the same writer, so a document's XMP packet, when it has one, changes with it.
/// It adds a packet only to a PDF 2.0 file, where the fields other than the dates belong in XMP.
/// The date is part of the op, never read from a clock, so the same op gives the same bytes
/// everywhere (ADR 0006 §1).
///
/// An Info dictionary written directly in the trailer, or missing, becomes a new indirect
/// object that the trailer's `/Info` points to (the first time the op changes something).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
#[deprecated(
    since = "0.2.0",
    note = "use SetMetadata, which writes Info and XMP together; SetInfo stays so op logs already written still apply"
)]
pub struct SetInfo {
    /// The entries to set or clear.
    #[serde(default)]
    pub patch: InfoPatch,
    /// The modification date to record as `/ModDate`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub at: Option<PdfDate>,
}

impl SetInfo {
    /// The op that applies `patch`, recording `at` as `/ModDate` when given.
    #[must_use]
    pub const fn new(patch: InfoPatch, at: Option<PdfDate>) -> Self {
        Self { patch, at }
    }

    /// The op that applies `patch` and leaves `/ModDate` as it is.
    ///
    /// ```
    /// use laminapdf_ops::{InfoPatch, SetInfo};
    ///
    /// let op = SetInfo::patch(InfoPatch::new().title(Some("Final report")));
    /// assert_eq!(op.at, None);
    /// ```
    #[must_use]
    pub const fn patch(patch: InfoPatch) -> Self {
        Self { patch, at: None }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let text =
            |value: &Option<Option<String>>| value.as_ref().map(|v| v.as_deref().map(|s| Value::Text(s.to_owned())));
        let p = &self.patch;
        let mut changes: Vec<(MappedField, Option<Value>)> = [
            (MappedField::Title, text(&p.title)),
            (
                MappedField::Authors,
                p.author
                    .as_ref()
                    .map(|v| v.as_ref().map(|a| Value::List(vec![a.clone()]))),
            ),
            (MappedField::Subject, text(&p.subject)),
            (MappedField::Keywords, text(&p.keywords)),
            (MappedField::Creator, text(&p.creator)),
            (MappedField::Producer, text(&p.producer)),
        ]
        .into_iter()
        .filter_map(|(field, value)| value.map(|v| (field, v)))
        .collect();
        if let Some(at) = self.at {
            changes.push((MappedField::Modified, Some(Value::Date(at))));
        }
        write(doc, &changes, Policy::info())
    }
}
