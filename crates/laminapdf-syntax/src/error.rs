//! The error type of the syntax layer.

use crate::diagnostic::{ErrorKind, Location, Position};
use crate::limits::Limit;

/// Everything that can go wrong while lexing, parsing or decoding PDF syntax.
///
/// Every fallible public function returns [`Result`]; no input is allowed to panic.
///
/// Positions are offsets in the buffer that was parsed, reported as
/// [`Position::FileOffset`]: a caller that parsed something other than the file re-tags them
/// with [`Error::map_position`] (`laminapdf-cos` does, for windows and object streams).
/// `Display` formats only fixed text and numbers, never bytes or names read from the input
/// (ADR 0026 §4).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The input ended before a complete token or object was read.
    #[error("unexpected end of input at {at}")]
    UnexpectedEof {
        /// Where more input was expected.
        at: Position,
    },
    /// A token or structure did not match what the grammar requires.
    #[error("syntax error at {at}: {message}")]
    Syntax {
        /// Where the offending token is.
        at: Position,
        /// Human-readable description of what was expected.
        message: &'static str,
    },
    /// A hard [`Limits`](crate::Limits) value was exceeded (ADR 0003 §3).
    #[error("{} exceeded{}", limit.name(), at.map(|at| format!(" at {at}")).unwrap_or_default())]
    LimitExceeded {
        /// Which limit.
        limit: Limit,
        /// Where it was hit, when known: an offset for syntax limits (nesting, string length),
        /// an object for the reader's limits (reference chains); `None` for decoding budgets.
        at: Option<Position>,
    },
    /// An object had a different type than the structure requires.
    #[error("type error: expected {expected} for {context}")]
    Type {
        /// The expected PDF type (e.g. "name").
        expected: &'static str,
        /// Where the value was found (e.g. "/Filter").
        context: &'static str,
    },
    /// The stream uses a filter that is not implemented. The name is read from the file, so
    /// it is kept here but never formatted (ADR 0026 §4).
    #[error("unsupported filter")]
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

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`Error::code`] (a test checks every variant).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::UnexpectedEof { .. }
            | Self::Syntax { .. }
            | Self::Type { .. }
            | Self::Decompress(_)
            | Self::InvalidStructure { .. } => ErrorKind::Input,
            Self::LimitExceeded { .. } => ErrorKind::Limit,
            Self::UnsupportedFilter(_) => ErrorKind::Unsupported,
        }
    }

    /// Where the error was found (ADR 0026 §1.1); empty when there is no position.
    #[must_use]
    pub fn location(&self) -> Location {
        Location::from(self.position())
    }

    /// The tagged position this error carries, if any.
    #[must_use]
    pub const fn position(&self) -> Option<Position> {
        match *self {
            Self::UnexpectedEof { at } | Self::Syntax { at, .. } => Some(at),
            Self::LimitExceeded { at, .. } => at,
            Self::Type { .. } | Self::UnsupportedFilter(_) | Self::Decompress(_) | Self::InvalidStructure { .. } => {
                None
            }
        }
    }

    /// This error with its position passed through `f`: how a caller re-tags an offset in
    /// the buffer it parsed ([`Position::shifted`] for a window of the file,
    /// [`Position::in_stream`] for a stream's decoded data).
    #[must_use]
    pub fn map_position(self, f: impl FnOnce(Position) -> Position) -> Self {
        match self {
            Self::UnexpectedEof { at } => Self::UnexpectedEof { at: f(at) },
            Self::Syntax { at, message } => Self::Syntax { at: f(at), message },
            Self::LimitExceeded { limit, at } => Self::LimitExceeded { limit, at: at.map(f) },
            other => other,
        }
    }

    /// An [`Error::LimitExceeded`] for `limit` at buffer offset `at`.
    pub(crate) fn limit(limit: Limit, at: Option<usize>) -> Self {
        Self::LimitExceeded {
            limit,
            at: at.map(Position::offset),
        }
    }
}

/// Shorthand for `Result<T, laminapdf_syntax::Error>`.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_codes as codes;

    /// One value of every variant: the match in [`Error::kind`] has no wildcard, so a new
    /// variant is a compile error there, and this list is what the tests compare.
    fn samples() -> Vec<Error> {
        let at = Position::FileOffset(0);
        vec![
            Error::UnexpectedEof { at },
            Error::Syntax { at, message: "m" },
            Error::LimitExceeded {
                limit: Limit::NestingDepth,
                at: None,
            },
            Error::Type {
                expected: "name",
                context: "/Filter",
            },
            Error::UnsupportedFilter(String::new()),
            Error::Decompress("m"),
            Error::InvalidStructure {
                kind: "k",
                message: "m",
            },
        ]
    }

    #[test]
    fn codes_lists_the_code_of_every_variant_in_order() {
        assert_eq!(codes::distinct(samples().iter().map(Error::code)), Error::CODES);
    }

    #[test]
    fn every_variant_has_the_kind_codes_json_lists() {
        codes::assert_kinds_match(samples().iter().map(|e| (e.code(), e.kind().as_str())));
    }
}
