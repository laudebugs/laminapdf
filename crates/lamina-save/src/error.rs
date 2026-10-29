//! The error type of the save layer.

use std::fmt;
use std::sync::Arc;

use lamina_cos::{ByteSource, Source};

use crate::constraint::{RewriteReason, SignatureInfo, Violation};
use crate::sink::SinkError;

/// Everything that can go wrong while saving.
///
/// When it happened decides what was written and where the document stands:
///
/// * **Before writing** ([`SaveError::WouldInvalidateSignatures`],
///   [`SaveError::RequiresRewrite`], [`SaveError::EncryptedBase`], [`SaveError::Cos`],
///   [`SaveError::Ops`], and [`SinkError::NotEmpty`]): nothing reached the sink; the document
///   is unchanged. The planner's refusals ([ADR 0007] §3), the writers' own (a repaired base
///   for an incremental update, [`lamina_cos::Error::RepairedBase`]), input that isn't available yet
///   ([`lamina_cos::Error::DataPending`]: fetch and retry), limits and exhausted object numbers
///   all happen here.
/// * **While writing** ([`SaveError::Sink`]): the sink may hold a prefix of the output; the
///   document is unchanged (`Dirty`, overlay untouched), so the save can be retried
///   elsewhere. A source that loses bytes between the incremental writer's check and its copy
///   also fails here, as [`SaveError::Cos`] with a partial output.
/// * **After writing** ([`SaveError::NotRebased`]): the output is complete, but the document
///   could not be rebased onto it. It is `SavedNotRebased`, and the error carries the output
///   as a source, so the caller can retry [`lamina_ops::Document::rebase`].
///
/// Messages hold no document text (field names, strings): only counts and fixed words.
///
/// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SaveError {
    /// The save would invalidate signatures ([ADR 0007] §3): a full rewrite of a signed
    /// document, or any save of one whose pending changes or rewrite reasons conflict with its
    /// signatures. Carries the whole constraint, so the UI can name every signature; retry
    /// with [`SaveOptions::with_invalidate_signatures`](crate::SaveOptions::with_invalidate_signatures)
    /// only after asking the user.
    ///
    /// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
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
    },
    /// An incremental save was requested, but only a full rewrite is safe: save with
    /// [`SaveMode::Auto`](crate::SaveMode::Auto) or
    /// [`SaveMode::FullRewrite`](crate::SaveMode::FullRewrite).
    #[error("only a full rewrite is safe ({} reason(s))", reasons.len())]
    RequiresRewrite {
        /// Why.
        reasons: Vec<RewriteReason>,
    },
    /// The document is encrypted, and lamina can't encrypt on write yet (full-editor E11):
    /// it can only be saved as a plaintext full rewrite, with
    /// [`SaveOptions::with_remove_encryption`](crate::SaveOptions::with_remove_encryption).
    /// Every mode refuses without it, so encryption is never dropped silently.
    #[error("the document is encrypted; saving it needs remove_encryption")]
    EncryptedBase,
    /// Reading the document failed or a writer refused it (see the type's documentation).
    #[error(transparent)]
    Cos(lamina_cos::Error),
    /// The ops layer refused: a transaction is open, or the document is waiting for
    /// [`rebase`](lamina_ops::Document::rebase) after a save it could not read back.
    #[error(transparent)]
    Ops(lamina_ops::Error),
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
        error: lamina_ops::Error,
    },
}

impl SaveError {
    /// Every code [`SaveError::code`] can return besides those of [`lamina_cos::Error`] and
    /// [`lamina_ops::Error`] (listed in `fixtures/codes.json` under `saveErrors`).
    /// [`SaveError::EncryptedBase`] reports `encrypted-base`, which `errors` already lists.
    pub const CODES: &'static [&'static str] =
        &["sink", "not-rebased", "would-invalidate-signatures", "requires-rewrite"];

    /// A stable kebab-case code for this error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cos(e) => e.code(),
            Self::Ops(e) => e.code(),
            Self::Sink(_) => "sink",
            Self::NotRebased { .. } => "not-rebased",
            Self::WouldInvalidateSignatures { .. } => "would-invalidate-signatures",
            Self::RequiresRewrite { .. } => "requires-rewrite",
            // The same refusal as the COS layer's, with the same code (ADR 0003 §2).
            Self::EncryptedBase => "encrypted-base",
        }
    }
}

impl From<lamina_cos::Error> for SaveError {
    fn from(e: lamina_cos::Error) -> Self {
        Self::Cos(e)
    }
}

impl From<lamina_ops::Error> for SaveError {
    fn from(e: lamina_ops::Error) -> Self {
        match e {
            lamina_ops::Error::Cos(e) => Self::Cos(e),
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
#[derive(Clone)]
pub struct SavedOutput(Source);

impl SavedOutput {
    pub(crate) const fn new(source: Source) -> Self {
        Self(source)
    }

    /// The output, to pass to [`lamina_ops::Document::rebase`].
    #[must_use]
    pub fn source(&self) -> Source {
        self.0.clone()
    }

    /// The underlying byte source.
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

/// Shorthand for `Result<T, lamina_save::SaveError>`.
pub type Result<T> = core::result::Result<T, SaveError>;
