//! [`Diagnostic`]: the ADR 0026 §1 object as a plain record, and its one serialisation.
//!
//! Every crate that hands an error to another language builds this record (the façade for
//! its errors and warnings, the WASM bindings for theirs, `lamina-render` for render errors),
//! so the object has one shape and one serialiser.

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::generated::codes::{ErrorCode, WarningCode};
use crate::{ErrorKind, Location};

/// One error or warning as the [ADR 0026] §1 object: what the façade's `Error` and `Warning`
/// serialise to, what the WASM bindings throw, as a record of strings and numbers an edge
/// maps one-to-one (the `UniFFI` `Diagnostic` record, the TS `LaminaError` fields; ADR 0028
/// F9). Serialises with camelCase keys, optional keys omitted (never `null`).
///
/// Built with [`Diagnostic::error`] or [`Diagnostic::warning`], which read the kind, hint and
/// `docsUrl` from the generated table, then the `with_` methods.
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
    pub data: Map<String, Value>,
    /// The code's explain page.
    pub docs_url: String,
}

impl Diagnostic {
    /// The error `code` with `message`: its kind, hint and `docsUrl` from the generated
    /// table; no location, data or candidates. `message` is one lowercase sentence that
    /// never quotes the document or the caller ([ADR 0026] §4).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::of(code.as_str(), code.kind(), message.into(), code.hint(), code.docs_url())
    }

    /// The warning `code`: its kind, hint and `docsUrl` from the generated table, and its
    /// fixed summary as the message.
    #[must_use]
    pub fn warning(code: WarningCode) -> Self {
        Self::of(
            code.as_str(),
            code.kind(),
            code.summary().to_owned(),
            code.hint(),
            code.docs_url(),
        )
    }

    fn of(code: &str, kind: ErrorKind, message: String, hint: &str, docs_url: String) -> Self {
        Self {
            code: code.to_owned(),
            kind,
            message,
            hint: Some(hint.to_owned()),
            did_you_mean: Vec::new(),
            location: Location::NONE,
            data: Map::new(),
            docs_url,
        }
    }

    /// With `hint` in place of the code's (a hint with its parameter filled in).
    #[must_use]
    pub fn with_hint(mut self, hint: Option<String>) -> Self {
        self.hint = hint;
        self
    }

    /// With the candidates for `didYouMean` (errors of kind `usage` only).
    #[must_use]
    pub fn with_did_you_mean(mut self, did_you_mean: Vec<String>) -> Self {
        self.did_you_mean = did_you_mean;
        self
    }

    /// With where the problem was found.
    #[must_use]
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = location;
        self
    }

    /// With the code-specific fields its explain page lists.
    #[must_use]
    pub fn with_data(mut self, data: Map<String, Value>) -> Self {
        self.data = data;
        self
    }

    /// The record as a JSON value (the §1 object).
    #[must_use]
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
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
