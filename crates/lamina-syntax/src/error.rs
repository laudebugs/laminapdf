//! The error type of the syntax layer.

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
    /// Arrays/dictionaries were nested deeper than the configured limit.
    #[error("nesting deeper than {limit} levels at byte {offset}")]
    TooDeep {
        /// The nesting limit that was exceeded.
        limit: usize,
        /// Byte offset where the limit was exceeded.
        offset: usize,
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
    /// A stream decompressed to more than the configured limit.
    #[error("decompressed stream exceeds the {limit}-byte limit")]
    DecompressLimit {
        /// The limit in bytes.
        limit: usize,
    },
    /// Filter parameters (e.g. a PNG predictor) were malformed.
    #[error("invalid {kind}: {message}")]
    InvalidStructure {
        /// Which structure (e.g. "PNG predictor").
        kind: &'static str,
        /// What was wrong with it.
        message: &'static str,
    },
}

/// Shorthand for `Result<T, lamina_syntax::Error>`.
pub type Result<T> = core::result::Result<T, Error>;
