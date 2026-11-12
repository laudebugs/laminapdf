//! The error type of the save layer.

use std::fmt;
use std::sync::Arc;

use laminapdf_cos::{ByteSource, ErrorKind, Location, Source};

use crate::constraint::{RewriteReason, SignatureInfo, Violation};
use crate::sink::SinkError;

/// Everything that can go wrong while saving.
///
/// When it happened decides what was written and where the document stands:
///
/// * **Before writing** ([`SaveError::WouldInvalidateSignatures`],
///   [`SaveError::RequiresRewrite`], [`SaveError::EncryptedBase`], [`SaveError::Cos`],
///   [`SaveError::Ops`], and [`SinkError::NotEmpty`], code `sink-not-empty`): nothing reached
///   the sink; the document is unchanged. The planner's refusals ([ADR 0007] §3), the writers'
///   own (a repaired base for an incremental update, [`laminapdf_cos::Error::RepairedBase`]), input that isn't available yet
///   ([`laminapdf_cos::Error::DataPending`]: fetch and retry), limits and exhausted object numbers
///   all happen here.
/// * **While writing** ([`SaveError::Sink`]): the sink may hold a prefix of the output; the
///   document is unchanged (`Dirty`, overlay untouched), so the save can be retried
///   elsewhere. A source that loses bytes between the incremental writer's check and its copy
///   also fails here, as [`SaveError::Cos`] with a partial output.
/// * **After writing** ([`SaveError::NotRebased`]): the output is complete, but the document
///   could not be rebased onto it. It is `SavedNotRebased`, and the error carries the output
///   as a source, so the caller can retry [`laminapdf_ops::Document::rebase`].
///
/// Messages hold no document text (field names, strings): only counts and fixed words.
///
/// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SaveError {
    /// The save would invalidate signatures ([ADR 0007] §3): a full rewrite of a signed
    /// document, or any save of one whose pending changes or rewrite reasons conflict with its
    /// signatures. Carries the whole constraint, so the UI can name every signature; retry
    /// with [`SaveOptions::with_invalidate_signatures`](crate::SaveOptions::with_invalidate_signatures)
    /// only after asking the user.
    ///
    /// [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md
    #[error(
        "the save would invalidate {} signature(s) ({} rewrite reason(s), {} violation(s))",
        signatures.len(),
        rewrite_reasons.len(),
        violations.len()
    )]
    WouldInvalidateSignatures {
        /// Every signature of the document.
        signatures: Vec<SignatureInfo>,
        /// Why only a full rewrite is safe, if anything says so.
        rewrite_reasons: Vec<RewriteReason>,
        /// Pending changes the signatures' restrictions forbid.
        violations: Vec<Violation>,
        /// Part of the form couldn't be read: there may be signatures beyond `signatures`.
        signatures_unknown: bool,
    },
    /// An incremental save was requested, but only a full rewrite is safe: save with
    /// [`SaveMode::Auto`](crate::SaveMode::Auto) or
    /// [`SaveMode::FullRewrite`](crate::SaveMode::FullRewrite).
    #[error("only a full rewrite is safe ({} reason(s))", reasons.len())]
    RequiresRewrite {
        /// Why.
        reasons: Vec<RewriteReason>,
    },
    /// The document is encrypted, and Lamina PDF can't encrypt on write yet (full-editor E11):
    /// it can only be saved as a plaintext full rewrite, with
    /// [`SaveOptions::with_remove_encryption`](crate::SaveOptions::with_remove_encryption).
    /// Every mode refuses without it, so encryption is never dropped silently.
    #[error("the document is encrypted; saving it needs remove_encryption")]
    EncryptedBase,
    /// Reading the document failed or a writer refused it (see the type's documentation).
    #[error(transparent)]
    Cos(laminapdf_cos::Error),
    /// The ops layer refused: a transaction is open, or the document is waiting for
    /// [`rebase`](laminapdf_ops::Document::rebase) after a save it could not read back.
    #[error(transparent)]
    Ops(laminapdf_ops::Error),
    /// The sink failed to take the bytes, held bytes already, or could not give them back.
    #[error(transparent)]
    Sink(SinkError),
    /// The output was written, but rebasing onto it failed with `error`; retry with
    /// `doc.rebase(saved.source())`.
    #[error("saved, but the rebase onto the output failed: {error}")]
    NotRebased {
        /// The complete output.
        saved: SavedOutput,
        /// Why the rebase failed.
        error: laminapdf_ops::Error,
    },
}

impl SaveError {
    /// Every code [`SaveError::code`] can return besides those of [`laminapdf_cos::Error`] and
    /// [`laminapdf_ops::Error`] (listed in `fixtures/codes.json` under `saveErrors`).
    /// [`SaveError::EncryptedBase`] reports `encrypted-base`, which `errors` already lists.
    ///
    /// Added in 0.2.0.
    pub const CODES: &'static [&'static str] = &[
        "sink",
        "not-rebased",
        "sink-not-empty",
        "would-invalidate-signatures",
        "requires-rewrite",
    ];

    /// A stable kebab-case code for this error.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cos(e) => e.code(),
            Self::Ops(e) => e.code(),
            Self::Sink(SinkError::NotEmpty { .. }) => "sink-not-empty",
            Self::Sink(SinkError::Io { .. } | SinkError::Failed(_)) => "sink",
            Self::NotRebased { .. } => "not-rebased",
            Self::WouldInvalidateSignatures { .. } => "would-invalidate-signatures",
            Self::RequiresRewrite { .. } => "requires-rewrite",
            // The same refusal as the COS layer's, with the same code (ADR 0003 §2).
            Self::EncryptedBase => "encrypted-base",
        }
    }

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`SaveError::code`] (a test checks every variant). Wrapped errors report their own.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Cos(e) => e.kind(),
            Self::Ops(e) => e.kind(),
            Self::Sink(SinkError::NotEmpty { .. }) => ErrorKind::Usage,
            Self::Sink(SinkError::Io { .. } | SinkError::Failed(_)) => ErrorKind::Io,
            Self::NotRebased { .. } | Self::RequiresRewrite { .. } => ErrorKind::State,
            Self::WouldInvalidateSignatures { .. } => ErrorKind::Security,
            Self::EncryptedBase => ErrorKind::Unsupported,
        }
    }

    /// Where the error was found in the document's input (ADR 0026 §1.1): the wrapped
    /// error's location; empty for sink failures and for [`SaveError::NotRebased`], whose
    /// `error` is about the newly written output, not the input (its own `location()` gives
    /// positions in that output).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Cos(e) => e.location(),
            Self::Ops(e) => e.location(),
            Self::Sink(_)
            | Self::NotRebased { .. }
            | Self::WouldInvalidateSignatures { .. }
            | Self::RequiresRewrite { .. }
            | Self::EncryptedBase => Location::NONE,
        }
    }
}

impl From<laminapdf_cos::Error> for SaveError {
    fn from(e: laminapdf_cos::Error) -> Self {
        Self::Cos(e)
    }
}

impl From<laminapdf_ops::Error> for SaveError {
    fn from(e: laminapdf_ops::Error) -> Self {
        match e {
            laminapdf_ops::Error::Cos(e) => Self::Cos(e),
            other => Self::Ops(other),
        }
    }
}

impl From<SinkError> for SaveError {
    fn from(e: SinkError) -> Self {
        Self::Sink(e)
    }
}

/// A save's complete output, handed back when the rebase onto it failed
/// ([`SaveError::NotRebased`]). Two are equal when they share the same source.
///
/// Added in 0.2.0.
#[derive(Clone)]
pub struct SavedOutput(Source);

impl SavedOutput {
    pub(crate) const fn new(source: Source) -> Self {
        Self(source)
    }

    /// The output, to pass to [`laminapdf_ops::Document::rebase`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn source(&self) -> Source {
        self.0.clone()
    }

    /// The underlying byte source.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn bytes(&self) -> &Arc<dyn ByteSource> {
        self.0.get()
    }
}

impl PartialEq for SavedOutput {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Arc::as_ptr(self.0.get()), Arc::as_ptr(other.0.get()))
    }
}

impl Eq for SavedOutput {}

impl fmt::Debug for SavedOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SavedOutput").field(&self.0.get().len()).finish()
    }
}

/// Shorthand for `Result<T, laminapdf_save::SaveError>`.
///
/// Added in 0.2.0.
pub type Result<T> = core::result::Result<T, SaveError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_codes as codes;

    fn saved() -> SavedOutput {
        SavedOutput::new(Source::from(Vec::new()))
    }

    /// One value of every variant this crate adds (each sink failure): the matches in
    /// [`SaveError::code`] and [`SaveError::kind`] have no wildcard, so a new variant is a
    /// compile error there, and this list is what the tests compare.
    fn samples() -> Vec<SaveError> {
        vec![
            SaveError::Sink(SinkError::Io {
                kind: std::io::ErrorKind::Other,
                message: String::new(),
            }),
            SaveError::Sink(SinkError::Failed(String::new())),
            SaveError::NotRebased {
                saved: saved(),
                error: laminapdf_ops::Error::NoPendingSave,
            },
            SaveError::Sink(SinkError::NotEmpty { held: 1 }),
            SaveError::WouldInvalidateSignatures {
                signatures: Vec::new(),
                rewrite_reasons: Vec::new(),
                violations: Vec::new(),
                signatures_unknown: false,
            },
            SaveError::RequiresRewrite { reasons: Vec::new() },
        ]
    }

    #[test]
    fn the_encrypted_base_refusal_has_the_cos_code_and_kind() {
        let e = SaveError::EncryptedBase;
        codes::assert_kinds_match([(e.code(), e.kind().as_str())]);
    }

    #[test]
    fn codes_lists_the_code_of_every_variant_in_order() {
        assert_eq!(codes::distinct(samples().iter().map(SaveError::code)), SaveError::CODES);
    }

    #[test]
    fn every_variant_has_the_kind_codes_json_lists() {
        codes::assert_kinds_match(samples().iter().map(|e| (e.code(), e.kind().as_str())));
    }

    #[test]
    fn not_rebased_reports_no_input_location_for_a_position_in_the_output() {
        let error = laminapdf_ops::Error::Cos(laminapdf_cos::Error::ReferenceCycle(7));
        let e = SaveError::NotRebased { saved: saved(), error };
        assert_eq!(e.location(), Location::NONE);
    }
}
