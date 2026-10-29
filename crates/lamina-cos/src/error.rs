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
        /// Where it was hit, when known: a byte offset in the buffer being parsed for syntax
        /// limits (nesting, string length; for content streams and the saved-state nesting of
        /// `lamina-content`, an offset in the decoded content), an object number for reference
        /// chains and Form XObject depth (the form that went too deep); `None` for budgets
        /// (decoded bytes, content operators). ADR 0026 will tag which one it is.
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
    ///
    /// The ranges are those the source answered [`SourceError::Pending`] for, sorted and with
    /// overlapping or adjacent ones merged: the windows, stream data or small xref-entry
    /// probes the call needs next, up to 1 024 per round. They cover the whole file only when
    /// the call needs all of it: repair, and the incremental writer, which copies the input.
    /// Any call that reads input can return this, not only
    /// [`Document::open`](crate::Document::open): `resolve`, `pages`, `decode_stream`,
    /// `page_content` and the incremental writer too. A failed call caches no object (the
    /// windows it did read stay in the source's window cache), so the document stays usable
    /// and the retry gives the result an in-memory source would have.
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
    /// have to be encrypted too, which is not implemented yet (encryption on save is
    /// full-editor E11; reading decrypts).
    #[error("cannot append an incremental update to an encrypted document")]
    EncryptedBase,
    /// The document is encrypted with a non-empty user password and
    /// [`OpenOptions::with_password`](crate::OpenOptions::with_password) gave none.
    #[error("the document needs a password")]
    PasswordRequired,
    /// The given password is neither the document's user nor its owner password.
    #[error("wrong password")]
    WrongPassword,
    /// The document is encrypted with a security handler lamina can't decrypt: not
    /// `/Standard` (e.g. the public-key handler `/Adobe.PubSec`), an unknown `/V`/`/R`
    /// combination, or an unknown crypt filter method (e.g. ISO/TS 32003 AES-GCM).
    #[error("unsupported security handler /{filter} (V {version:?}, R {revision:?})")]
    UnsupportedSecurityHandler {
        /// `/Filter` of the encryption dictionary (or `Standard /CFM /…` for an unknown crypt
        /// filter method).
        filter: String,
        /// `/V`, when present.
        version: Option<i64>,
        /// `/R`, when present.
        revision: Option<i64>,
    },
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
        "password-required",
        "wrong-password",
        "unsupported-security-handler",
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
            Self::PasswordRequired => "password-required",
            Self::WrongPassword => "wrong-password",
            Self::UnsupportedSecurityHandler { .. } => "unsupported-security-handler",
        }
    }

    /// Whether no tolerant path may swallow this error (ADR 0003): a strict-mode refusal (the
    /// caller asked for it) or input that is missing or unreadable ([`Error::DataPending`],
    /// [`Error::Source`]: the call has to be retried or fail). Every other error makes a
    /// tolerant reader treat the item as unreadable.
    #[must_use]
    pub const fn must_propagate(&self) -> bool {
        matches!(self, Self::Strict { .. } | Self::DataPending { .. } | Self::Source(_))
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

/// `result` read tolerantly (ADR 0003): `Ok(Some(v))`, `Ok(None)` when the item is
/// unreadable, or the error when it [must propagate](Error::must_propagate).
///
/// # Errors
/// The errors of `result` for which [`Error::must_propagate`] is true.
pub fn tolerant<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.must_propagate() => Err(e),
        Err(_) => Ok(None),
    }
}
