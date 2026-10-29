//! The error type of the save layer.

use crate::sink::SinkError;

/// Everything that can go wrong while saving.
///
/// A save that fails before writing (every refusal, and input that isn't available) writes
/// nothing to the sink; one that fails while writing ([`SaveError::Sink`]) may have written a
/// prefix. Either way the document keeps its overlay and stays `Dirty`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SaveError {
    /// Reading the document failed: input not available yet
    /// ([`lamina_cos::Error::DataPending`]: fetch and retry), a source failure, a strict-mode
    /// refusal, a hard limit, or object numbers running out.
    #[error(transparent)]
    Cos(lamina_cos::Error),
    /// The ops layer refused: a transaction is open, or the document is waiting for
    /// [`rebase`](lamina_ops::Document::rebase) after a save it could not read back.
    #[error(transparent)]
    Ops(lamina_ops::Error),
    /// The sink failed to take the bytes (or to give them back for rebasing).
    #[error(transparent)]
    Sink(SinkError),
    /// An incremental save of a document whose xref had to be repaired: there is no valid
    /// section for `/Prev` to point at. A full rewrite works.
    #[error("cannot append an incremental update to a document whose xref needed repair")]
    RepairedBase,
    /// A save of an encrypted document. Encryption on write is full-editor E11 and removing
    /// encryption is B3c's explicit option, so both writers refuse before writing anything
    /// (ADR 0007 §2).
    #[error("cannot save an encrypted document yet")]
    EncryptedBase,
}

impl SaveError {
    /// Every code [`SaveError::code`] can return besides those of [`lamina_cos::Error`] and
    /// [`lamina_ops::Error`] (listed in `fixtures/codes.json` under `saveErrors`;
    /// `repaired-base` and `encrypted-base` are COS codes, with the same meaning).
    pub const CODES: &'static [&'static str] = &["sink"];

    /// A stable kebab-case code for this error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cos(e) => e.code(),
            Self::Ops(e) => e.code(),
            Self::Sink(_) => "sink",
            Self::RepairedBase => "repaired-base",
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

/// Shorthand for `Result<T, lamina_save::SaveError>`.
pub type Result<T> = core::result::Result<T, SaveError>;
