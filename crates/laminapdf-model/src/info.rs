//! The document information dictionary (ISO 32000-2 §14.3.3).

use std::sync::Arc;

use laminapdf_cos::{Object, Result, Warning};

use crate::date::PdfDate;
use crate::document::{Document, object_number};

/// A date entry of the Info dictionary: the decoded string as stored, and the date when it
/// parses.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InfoDate {
    /// The text string as stored (decoded to Unicode).
    pub raw: String,
    /// The parsed date; `None` when `raw` is not a valid PDF date ([`Warning::InvalidDate`]).
    pub parsed: Option<PdfDate>,
}

/// The trailer's `/Info` dictionary: title, author and the other metadata entries, read
/// lazily. A field that is absent or not a string is `None`.
///
/// PDF 2.0 deprecates most of these in favour of XMP metadata (§14.3.2); reading XMP is a
/// later round.
#[derive(Debug, Clone)]
pub struct Info<'d> {
    doc: &'d Document,
    dict: Arc<Object>,
    object: Option<u32>,
}

impl<'d> Info<'d> {
    pub(crate) fn new(doc: &'d Document) -> Result<Option<Self>> {
        let Some(info) = doc.trailer().get("Info") else {
            return Ok(None);
        };
        let object = object_number(info);
        let dict = doc.read(info)?;
        match dict {
            Some(dict) if matches!(*dict, Object::Dict(_)) => Ok(Some(Self { doc, dict, object })),
            // A missing object is `null` (§7.3.10): no Info, nothing to report.
            Some(dict) if matches!(*dict, Object::Null) => Ok(None),
            _ => {
                doc.warn(Warning::InvalidEntry { object, key: "Info" })?;
                Ok(None)
            }
        }
    }

    /// The text string at `key`, decoded (§7.9.2.2); `None` when absent or not a string.
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::DataPending`] / [`laminapdf_cos::Error::Source`].
    pub fn text(&self, key: &str) -> Result<Option<String>> {
        match self.dict.as_dict() {
            Some(d) => self.doc.text_entry(d, key),
            None => Ok(None),
        }
    }

    /// `/Title`.
    ///
    /// # Errors
    /// As [`Info::text`].
    pub fn title(&self) -> Result<Option<String>> {
        self.text("Title")
    }

    /// `/Author`.
    ///
    /// # Errors
    /// As [`Info::text`].
    pub fn author(&self) -> Result<Option<String>> {
        self.text("Author")
    }

    /// `/Subject`.
    ///
    /// # Errors
    /// As [`Info::text`].
    pub fn subject(&self) -> Result<Option<String>> {
        self.text("Subject")
    }

    /// `/Keywords`.
    ///
    /// # Errors
    /// As [`Info::text`].
    pub fn keywords(&self) -> Result<Option<String>> {
        self.text("Keywords")
    }

    /// `/Creator`: the application that created the original document.
    ///
    /// # Errors
    /// As [`Info::text`].
    pub fn creator(&self) -> Result<Option<String>> {
        self.text("Creator")
    }

    /// `/Producer`: the application that converted it to PDF.
    ///
    /// # Errors
    /// As [`Info::text`].
    pub fn producer(&self) -> Result<Option<String>> {
        self.text("Producer")
    }

    /// `/CreationDate`.
    ///
    /// # Errors
    /// As [`Info::date`].
    pub fn creation_date(&self) -> Result<Option<InfoDate>> {
        self.date("CreationDate")
    }

    /// `/ModDate`.
    ///
    /// # Errors
    /// As [`Info::date`].
    pub fn mod_date(&self) -> Result<Option<InfoDate>> {
        self.date("ModDate")
    }

    /// The date at `key`: the raw string, and its parsed form when valid. An invalid date
    /// keeps the raw string and raises [`Warning::InvalidDate`].
    ///
    /// # Errors
    /// [`laminapdf_cos::Error::Strict`] in strict mode for an invalid date; as [`Info::text`].
    pub fn date(&self, key: &str) -> Result<Option<InfoDate>> {
        let Some(raw) = self.text(key)? else {
            return Ok(None);
        };
        let parsed = raw.parse::<PdfDate>().ok();
        if parsed.is_none() {
            // Warnings are deduplicated by entry too, so a bad /CreationDate does not hide a
            // bad /ModDate.
            let key = match key {
                "CreationDate" => "CreationDate",
                "ModDate" => "ModDate",
                _ => "other",
            };
            self.doc.warn(Warning::InvalidDate {
                object: self.object,
                key,
            })?;
        }
        Ok(Some(InfoDate { raw, parsed }))
    }
}
