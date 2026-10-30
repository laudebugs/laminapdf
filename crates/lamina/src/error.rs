//! [`Error`]: the façade's one error type (ADR 0028 F9), wrapping every crate's error, with
//! the ADR 0026 accessors and serialisation.

use lamina_cos::{ErrorKind, Location};
use serde::{Serialize, Serializer};

use crate::codes;
use crate::data;
use crate::diagnostic::Diagnostic;

/// Everything that can go wrong in the façade: the errors of the core crates, flattened so
/// a COS error is always [`Error::Cos`] whichever layer raised it, plus the job codes of
/// [ADR 0028] §3 and §5.
///
/// Every error has a stable [`code`](Error::code) from `fixtures/codes.json`, a
/// [`kind`](Error::kind), a [`location`](Error::location), a [`hint`](Error::hint) and a
/// [`docs_url`](Error::docs_url), and serialises to the [ADR 0026] §1 object
/// ([`Error::diagnostic`]). `Display` (the `message`) never carries text read from the
/// document or given by the caller; such strings go only to `data`.
///
/// ```
/// let err = lamina::Document::open_bytes(b"not a pdf".to_vec(), lamina::OpenOptions::default()).unwrap_err();
/// assert_eq!((err.code(), err.kind().as_str()), ("not-pdf", "input"));
/// let json = serde_json::to_value(&err)?;
/// assert_eq!(json["docsUrl"], "https://github.com/laudebugs/lamina/blob/main/docs/errors/not-pdf.md");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
/// [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Reading the file structure (`lamina-cos`, `lamina-syntax`).
    #[error(transparent)]
    Cos(lamina_cos::Error),
    /// Ops, transactions and undo (`lamina-ops`).
    #[error(transparent)]
    Ops(lamina_ops::Error),
    /// Saving (`lamina-save`).
    #[error(transparent)]
    Save(lamina_save::SaveError),
    /// Fonts, shaping and layout (`lamina-text`).
    #[cfg(feature = "text")]
    #[error(transparent)]
    Text(lamina_text::Error),
    /// Building pages (`lamina-build`).
    #[cfg(feature = "text")]
    #[error(transparent)]
    Build(lamina_build::Error),
    /// A 0-based page index the document doesn't have (`page-index-out-of-range`).
    #[error("page index {index} is out of range: the document has {count} pages")]
    PageIndexOutOfRange {
        /// The index given.
        index: u32,
        /// The number of pages.
        count: u32,
    },
    /// A [`CancelToken`](crate::CancelToken) stopped the work (`cancelled`).
    #[error("the work was cancelled")]
    Cancelled,
    /// A job was stepped after it finished, failed or was cancelled (`job-finished`).
    #[error("the job has already finished")]
    JobFinished,
    /// The document changed, or another document was passed, since the job was created
    /// (`document-changed`).
    #[error("the document changed since the job was created")]
    DocumentChanged,
}

/// Shorthand for `Result<T, lamina::Error>`.
pub type Result<T, E = Error> = core::result::Result<T, E>;

impl Error {
    /// The codes only the façade raises (listed in `fixtures/codes.json` under
    /// `facadeErrors`; `page-index-out-of-range` is `lamina-build`'s code, reused).
    pub const CODES: &'static [&'static str] = &["cancelled", "job-finished", "document-changed"];

    /// The stable kebab-case code ([ADR 0026] §3).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cos(e) => e.code(),
            Self::Ops(e) => e.code(),
            Self::Save(e) => e.code(),
            #[cfg(feature = "text")]
            Self::Text(e) => e.code(),
            #[cfg(feature = "text")]
            Self::Build(e) => e.code(),
            Self::PageIndexOutOfRange { .. } => "page-index-out-of-range",
            Self::Cancelled => "cancelled",
            Self::JobFinished => "job-finished",
            Self::DocumentChanged => "document-changed",
        }
    }

    /// The kind ([ADR 0026] §2): what the caller can do about it.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Cos(e) => e.kind(),
            Self::Ops(e) => e.kind(),
            Self::Save(e) => e.kind(),
            #[cfg(feature = "text")]
            Self::Text(e) => e.kind(),
            #[cfg(feature = "text")]
            Self::Build(e) => e.kind(),
            Self::PageIndexOutOfRange { .. } => ErrorKind::Usage,
            Self::Cancelled => ErrorKind::Cancelled,
            Self::JobFinished | Self::DocumentChanged => ErrorKind::State,
        }
    }

    /// Where the error was found ([ADR 0026] §1.1); empty when nothing is known.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Cos(e) => e.location(),
            Self::Ops(e) => e.location(),
            Self::Save(e) => e.location(),
            #[cfg(feature = "text")]
            Self::Text(e) => e.location(),
            #[cfg(feature = "text")]
            Self::Build(e) => e.location(),
            Self::PageIndexOutOfRange { .. } | Self::Cancelled | Self::JobFinished | Self::DocumentChanged => {
                Location::NONE
            }
        }
    }

    /// The code's one-sentence hint from `fixtures/codes.json` ([ADR 0026] §5), with
    /// `{limit}` filled in for `limit-exceeded`. Fixed text: never document text.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn hint(&self) -> Option<String> {
        codes::hint(self.code(), self.limit_name())
    }

    /// The code's explain page (`docsUrl`, [ADR 0026] §6).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn docs_url(&self) -> String {
        codes::docs_url(self.code())
    }

    /// The [ADR 0026] §1 object as a plain record of strings and numbers: what
    /// `Serialize` writes, and what an edge maps to its own `LaminaError` (ADR 0028 F9).
    /// Document-derived strings (a filter name, a font name, the text of an op log) appear
    /// only in [`Diagnostic::data`].
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            code: self.code().to_owned(),
            kind: self.kind(),
            message: self.to_string(),
            hint: self.hint(),
            did_you_mean: Vec::new(),
            location: self.location(),
            data: data::of_error(self),
            docs_url: self.docs_url(),
        }
    }

    /// The output of a save whose rebase failed (`not-rebased`): the file was written in
    /// full. Rebase the document onto it with [`Document::rebase`](crate::Document::rebase),
    /// or keep it and reopen.
    #[must_use]
    pub fn saved_output(&self) -> Option<Vec<u8>> {
        let Self::Save(lamina_save::SaveError::NotRebased { saved, .. }) = self else {
            return None;
        };
        let source = saved.bytes();
        if let Some(bytes) = source.as_slice() {
            return Some(bytes.to_vec());
        }
        let len = usize::try_from(source.len()).ok()?;
        let mut bytes = vec![0; len];
        lamina_cos::read_exact(source.as_ref(), 0, &mut bytes).ok()?;
        Some(bytes)
    }

    /// The limit's name, for a `limit-exceeded` error.
    fn limit_name(&self) -> Option<&'static str> {
        match self {
            Self::Cos(lamina_cos::Error::LimitExceeded { limit, .. }) => Some(limit.name()),
            _ => None,
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error> {
        self.diagnostic().serialize(serializer)
    }
}

impl From<lamina_cos::Error> for Error {
    fn from(e: lamina_cos::Error) -> Self {
        Self::Cos(e)
    }
}

impl From<lamina_syntax::Error> for Error {
    fn from(e: lamina_syntax::Error) -> Self {
        Self::Cos(e.into())
    }
}

impl From<lamina_ops::Error> for Error {
    fn from(e: lamina_ops::Error) -> Self {
        match e {
            lamina_ops::Error::Cos(e) => Self::Cos(e),
            other => Self::Ops(other),
        }
    }
}

impl From<lamina_save::SaveError> for Error {
    fn from(e: lamina_save::SaveError) -> Self {
        match e {
            lamina_save::SaveError::Cos(e) => Self::Cos(e),
            lamina_save::SaveError::Ops(e) => e.into(),
            other => Self::Save(other),
        }
    }
}

#[cfg(feature = "text")]
impl From<lamina_text::Error> for Error {
    fn from(e: lamina_text::Error) -> Self {
        match e {
            lamina_text::Error::Cos(e) => Self::Cos(e),
            other => Self::Text(other),
        }
    }
}

#[cfg(feature = "text")]
impl From<lamina_build::Error> for Error {
    fn from(e: lamina_build::Error) -> Self {
        match e {
            lamina_build::Error::Cos(e) => Self::Cos(e),
            lamina_build::Error::Text(e) => e.into(),
            other => Self::Build(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_codes as codes;

    /// One value of each variant the façade defines itself (the wrapped ones are checked
    /// by their crates).
    fn samples() -> Vec<Error> {
        vec![
            Error::PageIndexOutOfRange { index: 3, count: 1 },
            Error::Cancelled,
            Error::JobFinished,
            Error::DocumentChanged,
        ]
    }

    #[test]
    fn codes_lists_the_facade_codes_in_order() {
        let own: Vec<_> = samples().iter().map(Error::code).skip(1).collect();
        assert_eq!(own, Error::CODES);
    }

    #[test]
    fn codes_matches_the_facade_group_of_codes_json() {
        let listed: Vec<String> = codes::codes_json()["facadeErrors"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| e["code"].as_str().map(str::to_owned))
            .collect();
        assert_eq!(listed, Error::CODES);
    }

    #[test]
    fn every_facade_variant_has_the_kind_codes_json_lists() {
        codes::assert_kinds_match(samples().iter().map(|e| (e.code(), e.kind().as_str())));
    }

    #[test]
    fn a_cos_error_from_any_layer_is_flattened() {
        let from_ops = Error::from(lamina_ops::Error::Cos(lamina_cos::Error::MissingRoot));
        let from_save = Error::from(lamina_save::SaveError::Ops(lamina_ops::Error::Cos(
            lamina_cos::Error::MissingRoot,
        )));
        assert_eq!(
            [from_ops, from_save],
            [const { Error::Cos(lamina_cos::Error::MissingRoot) }; 2]
        );
    }

    #[test]
    fn limit_hint_names_the_limit() {
        let e = Error::Cos(lamina_cos::Error::LimitExceeded {
            limit: lamina_cos::Limit::NestingDepth,
            at: None,
        });
        assert!(
            e.hint()
                .is_some_and(|h| h.contains("max_nesting_depth") && !h.contains('{'))
        );
    }

    #[test]
    fn every_facade_code_has_a_hint() {
        assert!(samples().iter().all(|e| e.hint().is_some()));
    }
}
