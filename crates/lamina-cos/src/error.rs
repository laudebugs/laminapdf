//! The error type of the file-structure layer.

use lamina_syntax::Limit;

use crate::reader::Warning;
use crate::source::SourceError;

/// Everything that can go wrong while reading or writing a PDF file.
///
/// Every fallible public function returns [`Result`]; no input is allowed to panic. Errors from
/// the syntax layer (lexing, parsing, decoding streams) arrive as [`Error::Syntax`], except
/// [`lamina_syntax::Error::LimitExceeded`], which becomes [`Error::LimitExceeded`] so that every
/// limit is matched in one place.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error from the syntax layer (`lamina-syntax`), which ships in lockstep with this crate.
    #[error(transparent)]
    Syntax(lamina_syntax::Error),
    /// A hard [`Limits`](lamina_syntax::Limits) value was exceeded (ADR 0003 §3).
    #[error("{} exceeded{}", limit.name(), at.map(|at| format!(" at {at}")).unwrap_or_default())]
    LimitExceeded {
        /// Which limit.
        limit: Limit,
        /// Where it was hit, when known: a byte offset for syntax limits (nesting, string
        /// length), an object number for reference chains; `None` for decoding budgets.
        at: Option<u64>,
    },
    /// Strict mode ([`OpenOptions::with_strict`](crate::OpenOptions::with_strict)) refused
    /// something the tolerant reader would have repaired with this warning.
    #[error("strict mode: {}", warning.code())]
    Strict {
        /// The warning the tolerant reader would have raised.
        warning: Warning,
    },
    /// The [`ByteSource`](crate::ByteSource) has not fetched these `(offset, length)` ranges
    /// yet: fetch them and retry the call (ADR 0005 §2).
    #[error("input bytes not available yet: {ranges:?}")]
    DataPending {
        /// The missing `(offset, length)` ranges.
        ranges: Vec<(u64, u64)>,
    },
    /// The [`ByteSource`](crate::ByteSource) failed to read.
    #[error(transparent)]
    Source(SourceError),
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
    /// An incremental update was requested for an encrypted document. Its new objects would
    /// have to be encrypted too, which is not implemented yet (decryption is Round B2,
    /// encryption on save is full-editor E11).
    #[error("cannot append an incremental update to an encrypted document")]
    EncryptedBase,
}

impl From<lamina_syntax::Error> for Error {
    fn from(e: lamina_syntax::Error) -> Self {
        match e {
            lamina_syntax::Error::LimitExceeded { limit, at } => Self::LimitExceeded { limit, at },
            other => Self::Syntax(other),
        }
    }
}

impl From<SourceError> for Error {
    fn from(e: SourceError) -> Self {
        match e {
            SourceError::Pending { offset, len } => Self::DataPending {
                ranges: vec![(offset, len)],
            },
            other => Self::Source(other),
        }
    }
}

impl Error {
    /// Every code [`Error::code`] can return: this crate's own codes and those of
    /// [`lamina_syntax::Error`] (listed together in `fixtures/codes.json`).
    pub const CODES: &'static [&'static str] = &[
        "not-pdf",
        "missing-root",
        "reference-cycle",
        "limit-exceeded",
        "strict",
        "data-pending",
        "source",
        "type",
        "invalid-structure",
        "unset-object",
        "repaired-base",
        "encrypted-base",
        "unexpected-eof",
        "syntax",
        "unsupported-filter",
        "decompress",
    ];

    /// A stable kebab-case code for this error, part of the public API in Rust, WASM and
    /// TypeScript. Syntax-layer errors report [`lamina_syntax::Error::code`].
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Syntax(e) => e.code(),
            Self::LimitExceeded { .. } => "limit-exceeded",
            Self::Strict { .. } => "strict",
            Self::DataPending { .. } => "data-pending",
            Self::Source(_) => "source",
            Self::NotPdf(_) => "not-pdf",
            Self::MissingRoot => "missing-root",
            Self::ReferenceCycle(_) => "reference-cycle",
            Self::Type { .. } => "type",
            Self::InvalidStructure { .. } => "invalid-structure",
            Self::UnsetObject(_) => "unset-object",
            Self::RepairedBase => "repaired-base",
            Self::EncryptedBase => "encrypted-base",
        }
    }

    /// An [`Error::LimitExceeded`] for `limit` at object `num`.
    pub(crate) fn limit_at_object(limit: Limit, num: u32) -> Self {
        Self::LimitExceeded {
            limit,
            at: Some(u64::from(num)),
        }
    }
}

/// Shorthand for `Result<T, lamina_cos::Error>`.
pub type Result<T> = core::result::Result<T, Error>;
