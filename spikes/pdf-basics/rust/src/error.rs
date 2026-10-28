//! The crate-wide error type.

/// Everything that can go wrong while parsing, reading or writing a PDF.
///
/// Every fallible public function returns [`Result`]; no input is allowed to panic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
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
    /// The input does not look like a PDF at all (no `%PDF-` header, no objects).
    #[error("not a PDF: {0}")]
    NotPdf(&'static str),
    /// No usable trailer with a `/Root` entry could be found, even after repair.
    #[error("no document catalog (/Root) found")]
    MissingRoot,
    /// A reference chain or page tree loops back on itself.
    #[error("reference cycle detected at object {0}")]
    ReferenceCycle(u32),
    /// An object had a different type than the structure requires.
    #[error("type error: expected {expected} for {context}")]
    Type {
        /// The expected PDF type (e.g. "dictionary").
        expected: &'static str,
        /// Where the value was found (e.g. "/Kids").
        context: &'static str,
    },
    /// The stream uses a filter this spike does not implement.
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
    /// An object stream or xref stream was malformed.
    #[error("invalid {kind}: {message}")]
    InvalidStructure {
        /// Which structure (e.g. "xref stream", "object stream").
        kind: &'static str,
        /// What was wrong with it.
        message: &'static str,
    },
    /// A writer object number was reserved but never given a value.
    #[error("object {0} was reserved but never set")]
    UnsetObject(u32),
    /// An incremental update was requested for a document whose xref had to be repaired.
    #[error("cannot append an incremental update to a document whose xref needed repair")]
    RepairedBase,
}

/// Shorthand for `Result<T, lamina_pdf_basics::Error>`.
pub type Result<T> = std::result::Result<T, Error>;
