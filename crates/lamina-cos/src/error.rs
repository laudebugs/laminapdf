//! The error type of the file-structure layer.

/// Everything that can go wrong while reading or writing a PDF file.
///
/// Every fallible public function returns [`Result`]; no input is allowed to panic. Errors from
/// the syntax layer (lexing, parsing, decoding streams) arrive as [`Error::Syntax`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error from the syntax layer (`lamina-syntax`), which ships in lockstep with this crate.
    #[error(transparent)]
    Syntax(#[from] lamina_syntax::Error),
    /// The input does not look like a PDF at all (empty, or no `n g obj` anywhere).
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
    /// An object stream, xref section or compact output was malformed or out of range.
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

/// Shorthand for `Result<T, lamina_cos::Error>`.
pub type Result<T> = core::result::Result<T, Error>;
