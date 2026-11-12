//! The error type of the file-structure layer.

use laminapdf_syntax::{ErrorKind, Limit, Location, Position};

use crate::reader::Warning;
use crate::source::SourceError;

/// Everything that can go wrong while reading or writing a PDF file.
///
/// Every fallible public function returns [`Result`]; no input is allowed to panic. Errors from
/// the syntax layer (lexing, parsing, decoding streams) arrive as [`Error::Syntax`], except
/// [`laminapdf_syntax::Error::LimitExceeded`], which becomes [`Error::LimitExceeded`] so that every
/// limit is matched in one place. Their positions are file offsets, object numbers or offsets
/// in an object stream's decoded data, tagged by the reader ([`Position`]).
///
/// `Display` formats only fixed text and numbers, never names or bytes read from the input
/// (ADR 0026 §4): those stay in the variant's fields.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error from the syntax layer (`laminapdf-syntax`), which ships in lockstep with this crate.
    #[error(transparent)]
    Syntax(laminapdf_syntax::Error),
    /// A hard [`Limits`](laminapdf_syntax::Limits) value was exceeded (ADR 0003 §3).
    #[error("{} exceeded{}", limit.name(), at.map(|at| format!(" at {at}")).unwrap_or_default())]
    LimitExceeded {
        /// Which limit.
        limit: Limit,
        /// Where it was hit, when known: a file offset (or an offset in an object stream's or
        /// a content stream's decoded data, `laminapdf-content`'s saved-state nesting included)
        /// for syntax limits (nesting, string length) and the read window; an object for
        /// reference chains and Form XObject depth (the form that went too deep); `None` for
        /// budgets (decoded bytes, content operators).
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
    /// the encryption. Reading decrypts, but nothing encrypts on save yet, so `laminapdf-save`'s
    /// writers refuse such a save before writing, unless it removes the encryption.
    #[error("cannot save an encrypted document yet")]
    EncryptedBase,
    /// The document is encrypted with a non-empty user password and
    /// [`OpenOptions::with_password`](crate::OpenOptions::with_password) gave none.
    #[error("the document needs a password")]
    PasswordRequired,
    /// The given password is neither the document's user nor its owner password.
    #[error("wrong password")]
    WrongPassword,
    /// The document is encrypted with a security handler Lamina PDF can't decrypt: not
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

impl From<laminapdf_syntax::Error> for Error {
    fn from(e: laminapdf_syntax::Error) -> Self {
        match e {
            laminapdf_syntax::Error::LimitExceeded { limit, at } => Self::LimitExceeded { limit, at },
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
    /// [`laminapdf_syntax::Error`] (listed together in `fixtures/codes.json`).
    ///
    /// Added in 0.2.0.
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
    /// TypeScript. Syntax-layer errors report [`laminapdf_syntax::Error::code`].
    ///
    /// Added in 0.2.0.
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
    /// [`laminapdf_syntax::Error::kind`].
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    ///
    /// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
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
    /// [`laminapdf_syntax::Error::map_position`]).
    #[must_use]
    pub(crate) fn map_position(self, f: impl FnOnce(Position) -> Position) -> Self {
        match self {
            Self::Syntax(e) => Self::Syntax(e.map_position(f)),
            Self::LimitExceeded { limit, at } => Self::LimitExceeded { limit, at: at.map(f) },
            other => other,
        }
    }

    /// Whether no tolerant path may swallow this error (ADR 0003): a strict-mode refusal (the
    /// caller asked for it) or input that is missing or unreadable ([`Error::DataPending`],
    /// [`Error::Source`]: the call has to be retried or fail). Every other error makes a
    /// tolerant reader treat the item as unreadable.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn must_propagate(&self) -> bool {
        matches!(self, Self::Strict { .. } | Self::DataPending { .. } | Self::Source(_))
    }

    /// Whether this error ends the page-tree walk instead of skipping one node: a cycle
    /// (ADR 0003 §1) or an error that [must propagate](Self::must_propagate). Hard limits are
    /// not among them: the walk is one of ADR 0003's tolerant paths, so it skips the node and
    /// raises `page-tree-node-skipped` (§3).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn ends_page_walk(&self) -> bool {
        matches!(self, Self::ReferenceCycle(_)) || self.must_propagate()
    }

    /// Whether this is an encryption error no repair can get around: the password is wrong or
    /// missing, or the security handler is one Lamina PDF can't decrypt. Opening a file
    /// returns these instead of repairing its cross-reference table.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_security_refusal(&self) -> bool {
        matches!(
            self,
            Self::PasswordRequired | Self::WrongPassword | Self::UnsupportedSecurityHandler { .. }
        )
    }

    /// Whether this is [`Limits::max_read_window`](crate::Limits::max_read_window) being
    /// hit, which tolerant paths treat like an item that does not parse.
    pub(crate) const fn is_read_window_limit(&self) -> bool {
        matches!(
            self,
            Self::LimitExceeded {
                limit: Limit::ReadWindow,
                ..
            }
        )
    }

    /// An [`Error::LimitExceeded`] for `limit` at object `num`.
    pub(crate) const fn limit_at_object(limit: Limit, num: u32) -> Self {
        Self::LimitExceeded {
            limit,
            at: Some(Position::Object(num)),
        }
    }
}

/// Shorthand for `Result<T, laminapdf_cos::Error>`.
///
/// Added in 0.2.0.
pub type Result<T> = core::result::Result<T, Error>;

// The policies below decide which errors a read swallows. Each one is defined here and only
// here: `check maintainability`'s `error-policy` check rejects a `matches!` or an error
// or-pattern on `LimitExceeded` or `DataPending` anywhere else.

/// `result` read on one of ADR 0003's tolerant paths: `Ok(Some(v))`, `Ok(None)` when the item
/// is unreadable for any reason, hard limits included, or the error when it
/// [must propagate](Error::must_propagate). The tolerant paths are the page-tree walk's nodes,
/// the xref loader falling back to repair, and an op reading an entry it can do without.
///
/// # Errors
/// The errors of `result` for which [`Error::must_propagate`] is true.
///
/// Added in 0.2.0.
pub fn tolerant<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.must_propagate() => Err(e),
        Err(_) => Ok(None),
    }
}

/// `result` read outside ADR 0003's tolerant paths by a reader that falls back to "absent":
/// `Ok(Some(v))`, or `Ok(None)` for an object that is malformed. Reference cycles, hard limits
/// ([`Error::LimitExceeded`]) and errors that [must propagate](Error::must_propagate) are
/// returned, as ADR 0003 §3 requires outside tolerant paths. Such readers include an inherited
/// page attribute, `/Count`, and a font's entries during text extraction.
///
/// # Errors
/// [`Error::ReferenceCycle`], [`Error::LimitExceeded`] and the errors of `result` for which
/// [`Error::must_propagate`] is true.
///
/// Added in 0.2.0.
pub fn absent_if_malformed<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.must_propagate() || matches!(e, Error::ReferenceCycle(_) | Error::LimitExceeded { .. }) => Err(e),
        Err(_) => Ok(None),
    }
}

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
            laminapdf_syntax::Error::UnexpectedEof { at },
            laminapdf_syntax::Error::Syntax { at, message: "m" },
            laminapdf_syntax::Error::Type {
                expected: "name",
                context: "/Filter",
            },
            laminapdf_syntax::Error::UnsupportedFilter(String::new()),
            laminapdf_syntax::Error::Decompress("m"),
            laminapdf_syntax::Error::InvalidStructure {
                kind: "k",
                message: "m",
            },
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
