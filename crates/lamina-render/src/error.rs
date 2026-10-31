//! [`RenderError`]: what a render backend can refuse or fail with, in the ADR 0026 contract.

use lamina_syntax::{ErrorKind, Location, Position};

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

    /// A stable kebab-case code for this error (ADR 0026 §1).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } | Self::InputTooLarge { .. } => "render-too-large",
            Self::InvalidRequest { .. } | Self::BufferSize { .. } => "invalid-argument",
            Self::PageIndexOutOfRange { .. } => "page-index-out-of-range",
            Self::PasswordRequired => "password-required",
            Self::WrongPassword => "wrong-password",
            Self::DataPending { .. } => "data-pending",
            Self::Source { .. } => "source",
            Self::Failed { .. } => "render-failed",
            Self::Unavailable { .. } => "render-backend-unavailable",
            Self::Cancelled => "cancelled",
        }
    }

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`RenderError::code`] (a unit test checks every variant).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::TooLarge { .. } | Self::InputTooLarge { .. } => ErrorKind::Limit,
            Self::InvalidRequest { .. } | Self::BufferSize { .. } | Self::PageIndexOutOfRange { .. } => {
                ErrorKind::Usage
            }
            Self::PasswordRequired | Self::WrongPassword => ErrorKind::Security,
            Self::DataPending { .. } => ErrorKind::State,
            Self::Source { .. } | Self::Unavailable { .. } => ErrorKind::Io,
            Self::Failed { .. } => ErrorKind::Input,
            Self::Cancelled => ErrorKind::Cancelled,
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
