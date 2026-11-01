//! [`RenderError`]: what a render backend can refuse or fail with, in the ADR 0026 contract.

use lamina_syntax::{Diagnostic, ErrorCode, ErrorKind, Location, Position};
use serde_json::{Map, Value, json};

/// Why a render backend refused a request or couldn't render.
///
/// Every variant has a stable [`code`](RenderError::code) from `fixtures/codes.json`: the new
/// ones under `renderErrors`, the rest reused from the groups that already define them
/// (`page-index-out-of-range`, `password-required`, `wrong-password`, `data-pending`, `source`,
/// `cancelled`, `invalid-argument`). Messages never contain text read from the document.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// The requested tile is bigger than [`RenderLimits`](crate::RenderLimits) allow
    /// (`render-too-large`). Checked before anything is allocated.
    #[error("a {width}×{height} pixel tile exceeds the render limit of {limit} {unit}")]
    TooLarge {
        /// Tile width in pixels.
        width: u64,
        /// Tile height in pixels.
        height: u64,
        /// The limit that was exceeded.
        limit: u64,
        /// `pixels` (per tile) or `pixels per side`.
        unit: &'static str,
    },
    /// The input is longer than [`RenderLimits::max_input_bytes`](crate::RenderLimits) for a
    /// backend that reads the whole file into memory (`render-too-large`).
    #[error("the input is {len} bytes, over the render limit of {limit} bytes")]
    InputTooLarge {
        /// The input's length.
        len: u64,
        /// The limit.
        limit: u64,
    },
    /// A request that can never be valid: a non-finite or non-positive scale, an empty tile, a
    /// tile outside the page (`invalid-argument`).
    #[error("invalid render request: {what}")]
    InvalidRequest {
        /// What is wrong (fixed text).
        what: &'static str,
    },
    /// The output buffer doesn't have exactly the bytes the tile needs (`invalid-argument`).
    #[error("the output buffer holds {actual} bytes but the tile needs {expected}")]
    BufferSize {
        /// The bytes the tile needs ([`TilePlan::byte_len`](crate::TilePlan::byte_len)).
        expected: usize,
        /// The bytes the caller passed.
        actual: usize,
    },
    /// The page index is past the last page (`page-index-out-of-range`, ADR 0027: 0-based).
    #[error("page index {index} is out of range for a document of {count} pages")]
    PageIndexOutOfRange {
        /// The index asked for.
        index: u32,
        /// The number of pages.
        count: u32,
    },
    /// The document is encrypted and no password was given (`password-required`).
    #[error("the document is encrypted and needs a password")]
    PasswordRequired,
    /// The password given doesn't open the document (`wrong-password`).
    #[error("the password does not open the document")]
    WrongPassword,
    /// The source hasn't got these bytes yet: fetch them and call again (`data-pending`).
    #[error("{} byte range(s) of the input are not available yet", ranges.len())]
    DataPending {
        /// The missing `(offset, length)` ranges.
        ranges: Vec<(u64, u64)>,
    },
    /// The source failed to read these bytes (`source`).
    #[error("bytes {offset}..+{len} of the input could not be read")]
    Source {
        /// First byte of the failed range.
        offset: u64,
        /// Length of the failed range.
        len: u64,
    },
    /// The engine couldn't open the document or render the page: a damaged or unsupported
    /// file, or an engine failure (`render-failed`).
    #[error("the {backend} backend could not {stage} the document")]
    Failed {
        /// `pdfium` or `hayro`.
        backend: &'static str,
        /// `open` or `render`.
        stage: &'static str,
    },
    /// The engine isn't available: the PDFium library couldn't be found or bound
    /// (`render-backend-unavailable`).
    #[error("the {backend} backend is not available")]
    Unavailable {
        /// `pdfium` or `hayro`.
        backend: &'static str,
    },
    /// A [`CancelToken`](crate::CancelToken) stopped the render (`cancelled`).
    #[error("the render was cancelled")]
    Cancelled,
}

impl RenderError {
    /// The codes this crate adds to `fixtures/codes.json` (under `renderErrors`). The other
    /// variants reuse codes defined by other groups.
    pub const CODES: &'static [&'static str] = &["render-too-large", "render-failed", "render-backend-unavailable"];

    /// The code as the generated [`ErrorCode`] (one per entry of `fixtures/codes.json`),
    /// from which [`code`](RenderError::code), [`kind`](RenderError::kind) and the
    /// [`diagnostic`](RenderError::diagnostic)'s hint and `docsUrl` are read.
    #[must_use]
    pub const fn error_code(&self) -> ErrorCode {
        match self {
            Self::TooLarge { .. } | Self::InputTooLarge { .. } => ErrorCode::RenderTooLarge,
            Self::InvalidRequest { .. } | Self::BufferSize { .. } => ErrorCode::InvalidArgument,
            Self::PageIndexOutOfRange { .. } => ErrorCode::PageIndexOutOfRange,
            Self::PasswordRequired => ErrorCode::PasswordRequired,
            Self::WrongPassword => ErrorCode::WrongPassword,
            Self::DataPending { .. } => ErrorCode::DataPending,
            Self::Source { .. } => ErrorCode::Source,
            Self::Failed { .. } => ErrorCode::RenderFailed,
            Self::Unavailable { .. } => ErrorCode::RenderBackendUnavailable,
            Self::Cancelled => ErrorCode::Cancelled,
        }
    }

    /// A stable kebab-case code for this error (ADR 0026 §1).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.error_code().as_str()
    }

    /// The kind of this error ([ADR 0026] §2), from the generated [`ErrorCode`] table.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.error_code().kind()
    }

    /// The [ADR 0026] §1 object: what a binding hands to another language. `data` holds the
    /// fields each code's `docs/errors` page lists; nothing in it is read from the document.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::error(self.error_code(), self.to_string())
            .with_location(self.location())
            .with_data(self.data())
    }

    /// The `data` of [`RenderError::diagnostic`].
    fn data(&self) -> Map<String, Value> {
        let value = match self {
            Self::TooLarge {
                width,
                height,
                limit,
                unit,
            } => json!({ "width": width, "height": height, "limit": limit, "unit": unit }),
            Self::InputTooLarge { len, limit } => json!({ "len": len, "limit": limit }),
            Self::InvalidRequest { what } => json!({ "argument": "request", "detail": what }),
            Self::BufferSize { .. } => json!({ "argument": "output", "detail": self.to_string() }),
            Self::PageIndexOutOfRange { index, count } => json!({ "index": index, "count": count }),
            Self::DataPending { ranges } => json!({ "ranges": ranges }),
            Self::Failed { backend, stage } => json!({ "backend": backend, "stage": stage }),
            Self::Unavailable { backend } => json!({ "backend": backend }),
            Self::PasswordRequired | Self::WrongPassword | Self::Source { .. } | Self::Cancelled => json!({}),
        };
        // Every arm above is an object literal.
        if let Value::Object(map) = value {
            map
        } else {
            Map::new()
        }
    }

    /// Where the error was found (ADR 0026 §1.1): the failed byte range for a source error;
    /// nothing else has a position in the document.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Source { offset, .. } => Location::from(Position::FileOffset(*offset)),
            Self::TooLarge { .. }
            | Self::InputTooLarge { .. }
            | Self::InvalidRequest { .. }
            | Self::BufferSize { .. }
            | Self::PageIndexOutOfRange { .. }
            | Self::PasswordRequired
            | Self::WrongPassword
            | Self::DataPending { .. }
            | Self::Failed { .. }
            | Self::Unavailable { .. }
            | Self::Cancelled => Location::NONE,
        }
    }
}

impl From<lamina_cos::SourceError> for RenderError {
    fn from(e: lamina_cos::SourceError) -> Self {
        match e {
            lamina_cos::SourceError::Pending { offset, len } => Self::DataPending {
                ranges: vec![(offset, len)],
            },
            lamina_cos::SourceError::Failed { offset, len } => Self::Source { offset, len },
            // SourceError is non_exhaustive: a future variant is a read that failed somewhere.
            _ => Self::Source { offset: 0, len: 0 },
        }
    }
}

#[cfg(test)]
mod tests {

    use super::RenderError;

    /// One sample of every variant.
    fn samples() -> Vec<RenderError> {
        vec![
            RenderError::TooLarge {
                width: 1,
                height: 1,
                limit: 0,
                unit: "pixels",
            },
            RenderError::InputTooLarge { len: 2, limit: 1 },
            RenderError::InvalidRequest { what: "x" },
            RenderError::BufferSize { expected: 4, actual: 0 },
            RenderError::PageIndexOutOfRange { index: 1, count: 1 },
            RenderError::PasswordRequired,
            RenderError::WrongPassword,
            RenderError::DataPending { ranges: vec![(0, 1)] },
            RenderError::Source { offset: 0, len: 1 },
            RenderError::Failed {
                backend: "hayro",
                stage: "open",
            },
            RenderError::Unavailable { backend: "pdfium" },
            RenderError::Cancelled,
        ]
    }

    /// Every variant's §1 object validates against the generated `lamina.error/1` schema.
    #[test]
    fn every_diagnostic_matches_the_error_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../lamina/schemas/error.schema.json")).unwrap();
        let id = schema["$id"].as_str().unwrap().to_owned();
        let mut schemas = boon::Schemas::new();
        let mut compiler = boon::Compiler::new();
        compiler.add_resource(&id, schema).unwrap();
        let index = compiler.compile(&id, &mut schemas).unwrap();
        for e in samples() {
            let value = e.diagnostic().to_json();
            if let Err(report) = schemas.validate(&value, index) {
                panic!("{value} does not match lamina.error/1:\n{report}");
            }
        }
    }

    #[test]
    fn every_variant_has_the_kind_codes_json_lists() {
        let codes: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/codes.json")).unwrap();
        for e in samples() {
            let listed = codes
                .as_object()
                .unwrap()
                .values()
                .filter_map(serde_json::Value::as_array)
                .flatten()
                .find(|c| c["code"] == e.code())
                .unwrap_or_else(|| panic!("{} is not in codes.json", e.code()));
            assert_eq!(listed["kind"].as_str().unwrap(), e.kind().as_str(), "{}", e.code());
        }
    }
}
