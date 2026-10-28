//! The error type of the syntax layer.

use crate::limits::Limit;

/// Everything that can go wrong while lexing, parsing or decoding PDF syntax.
///
/// Every fallible public function returns [`Result`]; no input is allowed to panic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The input ended before a complete token or object was read.
    #[error("unexpected end of input at byte {offset}")]
    UnexpectedEof {
        /// Byte offset where more input was expected.
        offset: usize,
    },
    /// A token or structure did not match what the grammar requires.
    #[error("syntax error at byte {offset}: {message}")]
    Syntax {
        /// Byte offset of the offending token.
        offset: usize,
        /// Human-readable description of what was expected.
        message: &'static str,
    },
    /// A hard [`Limits`](crate::Limits) value was exceeded (ADR 0003 §3).
    #[error("{} exceeded{}", limit.name(), at.map(|at| format!(" at {at}")).unwrap_or_default())]
    LimitExceeded {
        /// Which limit.
        limit: Limit,
        /// Where it was hit, when known: a byte offset in the parsed buffer for syntax limits
        /// (nesting, string length), an object number for the reader's limits (reference
        /// chains); `None` for decoding budgets.
        at: Option<u64>,
    },
    /// An object had a different type than the structure requires.
    #[error("type error: expected {expected} for {context}")]
    Type {
        /// The expected PDF type (e.g. "name").
        expected: &'static str,
        /// Where the value was found (e.g. "/Filter").
        context: &'static str,
    },
    /// The stream uses a filter that is not implemented.
    #[error("unsupported filter /{0}")]
    UnsupportedFilter(String),
    /// Compressed data was corrupt.
    #[error("corrupt compressed data: {0}")]
    Decompress(&'static str),
    /// Filter parameters (e.g. a PNG predictor) were malformed.
    #[error("invalid {kind}: {message}")]
    InvalidStructure {
        /// Which structure (e.g. "PNG predictor").
        kind: &'static str,
        /// What was wrong with it.
        message: &'static str,
    },
}

impl Error {
    /// Every code [`Error::code`] can return.
    pub const CODES: &'static [&'static str] = &[
        "unexpected-eof",
        "syntax",
        "limit-exceeded",
        "type",
        "unsupported-filter",
        "decompress",
        "invalid-structure",
    ];

    /// A stable kebab-case code for this error, part of the public API in Rust, WASM and
    /// TypeScript (listed in `fixtures/codes.json`).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnexpectedEof { .. } => "unexpected-eof",
            Self::Syntax { .. } => "syntax",
            Self::LimitExceeded { .. } => "limit-exceeded",
            Self::Type { .. } => "type",
            Self::UnsupportedFilter(_) => "unsupported-filter",
            Self::Decompress(_) => "decompress",
            Self::InvalidStructure { .. } => "invalid-structure",
        }
    }

    /// An [`Error::LimitExceeded`] for `limit` at `at`.
    pub(crate) fn limit(limit: Limit, at: Option<usize>) -> Self {
        Self::LimitExceeded {
            limit,
            at: at.and_then(|at| u64::try_from(at).ok()),
        }
    }
}

/// Shorthand for `Result<T, lamina_syntax::Error>`.
pub type Result<T> = core::result::Result<T, Error>;
