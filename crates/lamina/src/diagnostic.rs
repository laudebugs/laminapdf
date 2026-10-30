//! [`Diagnostic`]: the ADR 0026 §1 object as a plain record.

use lamina_cos::{ErrorKind, Location};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

/// One error or warning as the [ADR 0026] §1 object: what [`Error`](crate::Error) and
/// [`Warning`](crate::Warning) serialise to, as a record of strings and numbers an edge
/// maps one-to-one (the `UniFFI` `Diagnostic` record, the TS `LaminaError` fields; ADR 0028
/// F9). Serialises with camelCase keys, optional keys omitted (never `null`).
///
/// `message`, `hint` and `did_you_mean` never carry document or caller text ([ADR 0026] §4);
/// `data` is the only place such strings appear, under the keys each code's explain page
/// marks *untrusted*.
///
/// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Diagnostic {
    /// The kebab-case code from `fixtures/codes.json`.
    pub code: String,
    /// One of the nine kinds.
    pub kind: ErrorKind,
    /// One lowercase sentence (the Rust `Display`).
    pub message: String,
    /// One actionable sentence for this code.
    pub hint: Option<String>,
    /// Candidates from lamina's own vocabulary (errors of kind `usage` only); empty today.
    pub did_you_mean: Vec<String>,
    /// Where: file offset, object, stream offset, page index, `LaminaId`.
    pub location: Location,
    /// Code-specific fields, documented on the code's explain page.
    pub data: serde_json::Map<String, serde_json::Value>,
    /// The code's explain page.
    pub docs_url: String,
}

impl Diagnostic {
    /// The record as a JSON value (the §1 object).
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

impl Serialize for Diagnostic {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("code", &self.code)?;
        map.serialize_entry("kind", self.kind.as_str())?;
        map.serialize_entry("message", &self.message)?;
        if let Some(hint) = &self.hint {
            map.serialize_entry("hint", hint)?;
        }
        if !self.did_you_mean.is_empty() {
            map.serialize_entry("didYouMean", &self.did_you_mean)?;
        }
        if !self.location.is_empty() {
            map.serialize_entry("location", &LocationJson(&self.location))?;
        }
        if !self.data.is_empty() {
            map.serialize_entry("data", &self.data)?;
        }
        map.serialize_entry("docsUrl", &self.docs_url)?;
        map.end()
    }
}

/// `location` with each field present only when set (ADR 0026 §1.1).
struct LocationJson<'a>(&'a Location);

impl Serialize for LocationJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let l = self.0;
        let mut map = serializer.serialize_map(None)?;
        if let Some(offset) = l.offset {
            map.serialize_entry("offset", &offset)?;
        }
        if let Some(object) = l.object {
            map.serialize_entry("object", &object)?;
        }
        if let Some(offset) = l.stream_offset {
            map.serialize_entry("streamOffset", &offset)?;
        }
        if let Some(index) = l.page_index {
            map.serialize_entry("pageIndex", &index)?;
        }
        if let Some(id) = &l.id {
            map.serialize_entry("id", id)?;
        }
        map.end()
    }
}
