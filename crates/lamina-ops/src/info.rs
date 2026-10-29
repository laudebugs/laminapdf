//! `SetInfo`: edit the document information dictionary (ISO 32000-2 §14.3.3).

use lamina_cos::{Dict, Object, PdfString};
use lamina_model::{PageId, PdfDate};
use lamina_syntax::text::encode_text_string;

use crate::error::Result;
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

    #[expect(
        clippy::option_option,
        reason = "InfoPatch's set / clear / leave fields, as the todo and ADR 0006 specify"
    )]
    fn fields(&self) -> [(&'static str, &Option<Option<String>>); 6] {
        [
            ("Title", &self.title),
            ("Author", &self.author),
            ("Subject", &self.subject),
            ("Keywords", &self.keywords),
            ("Creator", &self.creator),
            ("Producer", &self.producer),
        ]
    }
}

/// Edits the Info dictionary: applies `patch` and, when `at` is given, sets `/ModDate` to it.
/// The date is part of the op, never read from a clock, so the same op gives the same bytes
/// everywhere (ADR 0006 §1).
///
/// An Info dictionary written directly in the trailer, or missing, becomes a new indirect
/// object that the trailer's `/Info` points to (the first time the op changes something).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
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

    pub(crate) fn apply_to(&self, doc: &mut lamina_model::Document) -> Result<Vec<PageId>> {
        let mut cos = doc.cos_mut().edit();
        let mut trailer = cos.trailer().clone();
        let (target, current) = match trailer.get("Info") {
            Some(Object::Ref(r)) => match cos.get(*r)?.as_deref() {
                Some(Object::Dict(d)) => (Some(*r), d.clone()),
                // Missing, null or not a dictionary: start a new one.
                _ => (None, Dict::new()),
            },
            Some(Object::Dict(d)) => (None, d.clone()),
            _ => (None, Dict::new()),
        };
        let patched = self.patched(&current);
        if patched == current {
            return Ok(Vec::new());
        }
        if let Some(r) = target {
            cos.set(r, Object::Dict(patched))?;
        } else {
            let r = cos.create(Object::Dict(patched))?;
            trailer.insert("Info", r);
            cos.set_trailer(trailer);
        }
        Ok(Vec::new())
    }

    fn patched(&self, current: &Dict) -> Dict {
        let mut dict = current.clone();
        for (key, field) in self.patch.fields() {
            if let Some(value) = field {
                match value {
                    None => {
                        dict.remove(key);
                    }
                    Some(text) => dict.insert(key, PdfString::literal(encode_text_string(text))),
                }
            }
        }
        if let Some(at) = self.at {
            dict.insert("ModDate", PdfString::literal(at.to_string()));
        }
        dict
    }
}
