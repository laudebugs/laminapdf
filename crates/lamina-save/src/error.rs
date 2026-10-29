//! The error type of the save layer.

use std::fmt;
use std::sync::Arc;

use lamina_cos::{ByteSource, ErrorKind, Location, Source};

use crate::sink::SinkError;

/// Everything that can go wrong while saving.
///
/// When it happened decides what was written and where the document stands:
///
/// * **Before writing** ([`SaveError::Cos`], [`SaveError::Ops`], and
///   [`SinkError::NotEmpty`], code `sink-not-empty`): nothing reached the sink; the document is unchanged.
///   Refusals (a repaired or encrypted base: [`lamina_cos::Error::RepairedBase`],
///   [`lamina_cos::Error::EncryptedBase`]), input that isn't available yet
///   ([`lamina_cos::Error::DataPending`]: fetch and retry), limits and exhausted object numbers
///   all happen here.
/// * **While writing** ([`SaveError::Sink`]): the sink may hold a prefix of the output; the
///   document is unchanged (`Dirty`, overlay untouched), so the save can be retried
///   elsewhere. A source that loses bytes between the incremental writer's check and its copy
///   also fails here, as [`SaveError::Cos`] with a partial output.
/// * **After writing** ([`SaveError::NotRebased`]): the output is complete, but the document
///   could not be rebased onto it. It is `SavedNotRebased`, and the error carries the output
///   as a source, so the caller can retry [`lamina_ops::Document::rebase`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SaveError {
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
    pub const CODES: &'static [&'static str] = &["sink", "not-rebased", "sink-not-empty"];

    /// A stable kebab-case code for this error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cos(e) => e.code(),
            Self::Ops(e) => e.code(),
            Self::Sink(SinkError::NotEmpty { .. }) => "sink-not-empty",
            Self::Sink(SinkError::Io { .. } | SinkError::Failed(_)) => "sink",
            Self::NotRebased { .. } => "not-rebased",
        }
    }

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`SaveError::code`] (a test checks every variant). Wrapped errors report their own.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Cos(e) => e.kind(),
            Self::Ops(e) => e.kind(),
            Self::Sink(SinkError::NotEmpty { .. }) => ErrorKind::Usage,
            Self::Sink(SinkError::Io { .. } | SinkError::Failed(_)) => ErrorKind::Io,
            Self::NotRebased { .. } => ErrorKind::State,
        }
    }

    /// Where the error was found in the document's input (ADR 0026 §1.1): the wrapped
    /// error's location; empty for sink failures and for [`SaveError::NotRebased`], whose
    /// `error` is about the newly written output, not the input (its own `location()` gives
    /// positions in that output).
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Cos(e) => e.location(),
            Self::Ops(e) => e.location(),
            Self::Sink(_) | Self::NotRebased { .. } => Location::NONE,
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
                error: lamina_ops::Error::NoPendingSave,
            },
            SaveError::Sink(SinkError::NotEmpty { held: 1 }),
        ]
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
        let error = lamina_ops::Error::Cos(lamina_cos::Error::ReferenceCycle(7));
        let e = SaveError::NotRebased { saved: saved(), error };
        assert_eq!(e.location(), Location::NONE);
    }
}
