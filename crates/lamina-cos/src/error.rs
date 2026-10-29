//! The error type of the file-structure layer.

use lamina_syntax::{ErrorKind, Limit, Location, Position};

use crate::reader::Warning;
use crate::source::SourceError;

/// Everything that can go wrong while reading or writing a PDF file.
///
/// Every fallible public function returns [`Result`]; no input is allowed to panic. Errors from
/// the syntax layer (lexing, parsing, decoding streams) arrive as [`Error::Syntax`], except
/// [`lamina_syntax::Error::LimitExceeded`], which becomes [`Error::LimitExceeded`] so that every
/// limit is matched in one place. Their positions are file offsets, object numbers or offsets
/// in an object stream's decoded data, tagged by the reader ([`Position`]).
///
/// `Display` formats only fixed text and numbers, never names or bytes read from the input
/// (ADR 0026 §4): those stay in the variant's fields.
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
        /// Where it was hit, when known: a file offset (or an offset in an object stream's
        /// decoded data) for syntax limits (nesting, string length) and the read window, an
        /// object for reference chains; `None` for decoding budgets.
        at: Option<Position>,
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
    /// An incremental update was requested for a document whose xref had to be repaired
    /// (there is no valid section for `/Prev` to point at; a full rewrite works).
    #[error("cannot append an incremental update to a document whose xref needed repair")]
    RepairedBase,
    /// A save of an encrypted document was requested: an incremental update's new objects
    /// would have to be encrypted too, and a full rewrite would have to re-encrypt or remove
    /// encryption, neither of which exists yet (encryption on save is full-editor E11,
    /// removal B3c; reading decrypts). `lamina-save`'s writers refuse it before writing.
    #[error("cannot save an encrypted document yet")]
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
    /// combination, or an unknown crypt filter method (e.g. ISO/TS 32003 AES-GCM). `filter`
    /// is read from the file, so it is not formatted (ADR 0026 §4).
    #[error("unsupported security handler (V {version:?}, R {revision:?})")]
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

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`Error::code`] (a test checks every variant). Syntax-layer errors report
    /// [`lamina_syntax::Error::kind`].
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Syntax(e) => e.kind(),
            Self::Strict { .. }
            | Self::NotPdf(_)
            | Self::MissingRoot
            | Self::ReferenceCycle(_)
            | Self::Type { .. }
            | Self::InvalidStructure { .. } => ErrorKind::Input,
            Self::LimitExceeded { .. } => ErrorKind::Limit,
            Self::DataPending { .. } | Self::RepairedBase => ErrorKind::State,
            Self::Source(_) => ErrorKind::Io,
            Self::UnsetObject(_) => ErrorKind::Internal,
            Self::EncryptedBase | Self::UnsupportedSecurityHandler { .. } => ErrorKind::Unsupported,
            Self::PasswordRequired | Self::WrongPassword => ErrorKind::Security,
        }
    }

    /// Where the error was found (ADR 0026 §1.1); empty when there is no position.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Syntax(e) => e.location(),
            Self::LimitExceeded { at, .. } => Location::from(*at),
            Self::Strict { warning } => warning.location(),
            Self::Source(SourceError::Failed { offset, .. }) => Location::from(Position::FileOffset(*offset)),
            Self::ReferenceCycle(object) | Self::UnsetObject(object) => Location::object(Some(*object)),
            _ => Location::NONE,
        }
    }

    /// This error with its position passed through `f` (see
    /// [`lamina_syntax::Error::map_position`]).
    #[must_use]
    pub(crate) fn map_position(self, f: impl FnOnce(Position) -> Position) -> Self {
        match self {
            Self::Syntax(e) => Self::Syntax(e.map_position(f)),
            Self::LimitExceeded { limit, at } => Self::LimitExceeded { limit, at: at.map(f) },
            other => other,
        }
    }

    /// An [`Error::LimitExceeded`] for `limit` at object `num`.
    pub(crate) const fn limit_at_object(limit: Limit, num: u32) -> Self {
        Self::LimitExceeded {
            limit,
            at: Some(Position::Object(num)),
        }
    }
}

/// Shorthand for `Result<T, lamina_cos::Error>`.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_codes as codes;

    /// One value of every variant (each syntax-layer one wrapped): the match in
    /// [`Error::kind`] has no wildcard, so a new variant is a compile error there, and this
    /// list is what the tests compare.
    fn samples() -> Vec<Error> {
        let at = Position::FileOffset(0);
        let syntax = [
            lamina_syntax::Error::UnexpectedEof { at },
            lamina_syntax::Error::Syntax { at, message: "m" },
            lamina_syntax::Error::Type {
                expected: "name",
                context: "/Filter",
            },
            lamina_syntax::Error::UnsupportedFilter(String::new()),
            lamina_syntax::Error::Decompress("m"),
            lamina_syntax::Error::InvalidStructure {
                kind: "k",
                message: "m",
            },
            lamina_syntax::Error::UnexpectedEof { at },
        ];
        let mut all = vec![
            Error::NotPdf("m"),
            Error::MissingRoot,
            Error::ReferenceCycle(1),
            Error::LimitExceeded {
                limit: Limit::ResolveDepth,
                at: None,
            },
            Error::Strict {
                warning: Warning::XrefRepaired,
            },
            Error::DataPending { ranges: Vec::new() },
            Error::Source(SourceError::Failed { offset: 0, len: 1 }),
            Error::Type {
                expected: "dictionary",
                context: "/Kids",
            },
            Error::InvalidStructure {
                kind: "k",
                message: "m",
            },
            Error::UnsetObject(1),
            Error::RepairedBase,
            Error::EncryptedBase,
            Error::PasswordRequired,
            Error::WrongPassword,
            Error::UnsupportedSecurityHandler {
                filter: String::new(),
                version: None,
                revision: None,
            },
        ];
        all.extend(syntax.into_iter().map(Error::Syntax));
        all
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
