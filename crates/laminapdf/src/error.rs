//! [`Error`]: the façade's one error type (ADR 0028 F9), wrapping every crate's error, with
//! the ADR 0026 accessors and serialisation.

use laminapdf_cos::{ErrorKind, Location};
use serde::{Serialize, Serializer};

use crate::Diagnostic;
use crate::ErrorCode;
use crate::codes;
use crate::data;

/// Everything that can go wrong in the façade: the errors of the core crates, opaque in
/// [`Error::Core`] and flattened so a COS error is the same whichever layer raised it, plus
/// the page index and the job codes of
/// [ADR 0028] §3 and §5.
///
/// Every error has a stable [`code`](Error::code) from `fixtures/codes.json`, a
/// [`kind`](Error::kind), a [`location`](Error::location), a [`hint`](Error::hint) and a
/// [`docs_url`](Error::docs_url), and serialises to the [ADR 0026] §1 object
/// ([`Error::diagnostic`]). `Display` (the `message`) never carries text read from the
/// document or given by the caller; such strings go only to `data`.
///
/// ```
/// let err = laminapdf::Document::open_bytes(b"not a pdf".to_vec(), laminapdf::OpenOptions::new()).unwrap_err();
/// assert_eq!((err.code(), err.kind().as_str()), ("not-pdf", "input"));
/// let json = serde_json::to_value(&err)?;
/// assert_eq!(json["docsUrl"], "https://docs.laminapdf.com/reference/codes/not-pdf/");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
/// [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error of one of the crates below the façade (reading, editing, saving, fonts and
    /// text, building). It is opaque ([ADR 0028] F9 amendment): read it through
    /// [`code`](Error::code), [`error_code`](Error::error_code), [`kind`](Error::kind),
    /// [`location`](Error::location), [`hint`](Error::hint) and [`diagnostic`](Error::diagnostic),
    /// which are stable, and match on [`ErrorCode`] rather than on the inner types, which
    /// aren't. Its message is the inner error's, and `Debug` shows the inner error.
    #[error(transparent)]
    Core(CoreError),
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
    /// An earlier revision asked for past the last ([`Document::open_revision`](crate::Document::open_revision),
    /// `revision-index-out-of-range`).
    #[error("revision index {index} is out of range: the file has {count} revisions")]
    RevisionIndexOutOfRange {
        /// The index given.
        index: u32,
        /// The number of revisions.
        count: u32,
    },
}

/// Shorthand for `Result<T, laminapdf::Error>`.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// An error raised by a crate below the façade, kept whole but opaque: its type is no part
/// of the façade's API, so those crates' error enums can change without breaking callers.
/// [`Error`]'s accessors read everything a caller needs from it.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error(transparent)]
pub struct CoreError(pub(crate) Inner);

/// The wrapped error, by crate. A COS error is always `Cos`, whichever layer raised it.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub(crate) enum Inner {
    #[error(transparent)]
    Cos(laminapdf_cos::Error),
    #[error(transparent)]
    Ops(laminapdf_ops::Error),
    #[error(transparent)]
    Save(laminapdf_save::SaveError),
    #[cfg(feature = "extract")]
    #[error(transparent)]
    Text(laminapdf_text::Error),
    #[cfg(feature = "build")]
    #[error(transparent)]
    Build(laminapdf_build::Error),
    /// An extraction error the façade has no variant of its own for.
    #[cfg(feature = "extract")]
    #[error(transparent)]
    Extract(laminapdf_extract::ExtractError),
}

impl Error {
    const fn core(inner: Inner) -> Self {
        Self::Core(CoreError(inner))
    }

    /// The COS error inside, if this is one (the façade's own policies read it).
    pub(crate) const fn as_cos(&self) -> Option<&laminapdf_cos::Error> {
        match self {
            Self::Core(CoreError(Inner::Cos(e))) => Some(e),
            _ => None,
        }
    }
}

impl Error {
    /// The codes only the façade raises (listed in `fixtures/codes.json` under
    /// `facadeErrors`; `page-index-out-of-range` is `laminapdf-build`'s code, reused).
    pub const CODES: &'static [&'static str] = &[
        "cancelled",
        "job-finished",
        "document-changed",
        "revision-index-out-of-range",
    ];

    /// The stable kebab-case code ([ADR 0026] §3).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.error_code().as_str()
    }

    /// The code as the generated [`ErrorCode`] (one per entry of `fixtures/codes.json`),
    /// from which [`code`](Error::code), [`kind`](Error::kind), [`hint`](Error::hint) and
    /// [`docs_url`](Error::docs_url) are read. A wrapped crate's code is looked up in the
    /// table; the build fails if any code a crate declares is missing from it.
    #[must_use]
    pub const fn error_code(&self) -> ErrorCode {
        match self {
            Self::Core(CoreError(inner)) => match inner {
                Inner::Cos(e) => codes::error_code(e.code()),
                Inner::Ops(e) => codes::error_code(e.code()),
                Inner::Save(e) => codes::error_code(e.code()),
                #[cfg(feature = "extract")]
                Inner::Text(e) => codes::error_code(e.code()),
                #[cfg(feature = "build")]
                Inner::Build(e) => codes::error_code(e.code()),
                #[cfg(feature = "extract")]
                Inner::Extract(e) => codes::error_code(e.code()),
            },
            Self::PageIndexOutOfRange { .. } => ErrorCode::PageIndexOutOfRange,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::JobFinished => ErrorCode::JobFinished,
            Self::DocumentChanged => ErrorCode::DocumentChanged,
            Self::RevisionIndexOutOfRange { .. } => ErrorCode::RevisionIndexOutOfRange,
        }
    }

    /// The kind ([ADR 0026] §2): what the caller can do about it, from the generated
    /// [`ErrorCode`] table.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.error_code().kind()
    }

    /// Where the error was found ([ADR 0026] §1.1); empty when nothing is known.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Core(CoreError(inner)) => match inner {
                Inner::Cos(e) => e.location(),
                Inner::Ops(e) => e.location(),
                Inner::Save(e) => e.location(),
                #[cfg(feature = "extract")]
                Inner::Text(e) => e.location(),
                #[cfg(feature = "build")]
                Inner::Build(e) => e.location(),
                #[cfg(feature = "extract")]
                Inner::Extract(e) => e.location(),
            },
            Self::PageIndexOutOfRange { .. }
            | Self::Cancelled
            | Self::JobFinished
            | Self::DocumentChanged
            | Self::RevisionIndexOutOfRange { .. } => Location::NONE,
        }
    }

    /// The code's one-sentence hint from `fixtures/codes.json` ([ADR 0026] §5), with
    /// `{limit}` filled in for `limit-exceeded`. Fixed text: never document text.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn hint(&self) -> Option<String> {
        codes::fill_hint(self.error_code().hint(), self.limit_name())
    }

    /// The code's explain page (`docsUrl`, [ADR 0026] §6).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn docs_url(&self) -> String {
        self.error_code().docs_url()
    }

    /// The [ADR 0026] §1 object as a plain record of strings and numbers: what
    /// `Serialize` writes, and what an edge maps to its own `LaminaPdfError` (ADR 0028 F9).
    /// Document-derived strings (a filter name, a font name, the text of an op log) appear
    /// only in [`Diagnostic::data`].
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::error(self.error_code(), self.to_string())
            .with_hint(self.hint())
            .with_did_you_mean(data::did_you_mean(self))
            .with_location(self.location())
            .with_data(data::of_error(self))
    }

    /// The output of a save whose rebase failed (`not-rebased`): the file was written in
    /// full. Rebase the document onto it with [`Document::rebase`](crate::Document::rebase),
    /// or keep it and reopen.
    #[must_use]
    pub fn saved_output(&self) -> Option<Vec<u8>> {
        let Self::Core(CoreError(Inner::Save(laminapdf_save::SaveError::NotRebased { saved, .. }))) = self else {
            return None;
        };
        let source = saved.bytes();
        if let Some(bytes) = source.as_slice() {
            return Some(bytes.to_vec());
        }
        let len = usize::try_from(source.len()).ok()?;
        let mut bytes = vec![0; len];
        laminapdf_cos::read_exact(source.as_ref(), 0, &mut bytes).ok()?;
        Some(bytes)
    }

    /// The byte ranges a `data-pending` error asks for, as `start..end`: supply them to the
    /// source (`laminapdf-source`'s `SparseSource::supply`) and call again ([ADR 0003]);
    /// `None` for every other error.
    ///
    /// [ADR 0003]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
    #[must_use]
    pub fn pending_ranges(&self) -> Option<Vec<core::ops::Range<u64>>> {
        let laminapdf_cos::Error::DataPending { ranges } = self.as_cos()? else {
            return None;
        };
        Some(
            ranges
                .iter()
                .map(|&(offset, len)| offset..offset.saturating_add(len))
                .collect(),
        )
    }

    /// The limit's name, for a `limit-exceeded` error.
    fn limit_name(&self) -> Option<&'static str> {
        match self.as_cos() {
            Some(laminapdf_cos::Error::LimitExceeded { limit, .. }) => Some(limit.name()),
            _ => None,
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error> {
        self.diagnostic().serialize(serializer)
    }
}

impl From<laminapdf_cos::Error> for Error {
    fn from(e: laminapdf_cos::Error) -> Self {
        Self::core(Inner::Cos(e))
    }
}

impl From<laminapdf_syntax::Error> for Error {
    fn from(e: laminapdf_syntax::Error) -> Self {
        Self::core(Inner::Cos(e.into()))
    }
}

impl From<laminapdf_ops::Error> for Error {
    fn from(e: laminapdf_ops::Error) -> Self {
        match e {
            laminapdf_ops::Error::Cos(e) => Self::core(Inner::Cos(e)),
            other => Self::core(Inner::Ops(other)),
        }
    }
}

impl From<laminapdf_save::SaveError> for Error {
    fn from(e: laminapdf_save::SaveError) -> Self {
        match e {
            laminapdf_save::SaveError::Cos(e) => Self::core(Inner::Cos(e)),
            laminapdf_save::SaveError::Ops(e) => e.into(),
            other => Self::core(Inner::Save(other)),
        }
    }
}

#[cfg(feature = "extract")]
impl From<laminapdf_text::Error> for Error {
    fn from(e: laminapdf_text::Error) -> Self {
        match e {
            laminapdf_text::Error::Cos(e) => Self::core(Inner::Cos(e)),
            other => Self::core(Inner::Text(other)),
        }
    }
}

#[cfg(feature = "extract")]
impl From<laminapdf_extract::ExtractError> for Error {
    fn from(e: laminapdf_extract::ExtractError) -> Self {
        use laminapdf_extract::ExtractError as E;
        match e {
            E::Cancelled => Self::Cancelled,
            E::JobFinished => Self::JobFinished,
            E::PageIndexOutOfRange { index, count } => Self::PageIndexOutOfRange {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                count: u32::try_from(count).unwrap_or(u32::MAX),
            },
            E::Document(e) => Self::core(Inner::Cos(e)),
            other => Self::core(Inner::Extract(other)),
        }
    }
}

#[cfg(feature = "build")]
impl From<laminapdf_build::Error> for Error {
    fn from(e: laminapdf_build::Error) -> Self {
        match e {
            laminapdf_build::Error::Cos(e) => Self::core(Inner::Cos(e)),
            #[cfg(feature = "text")]
            laminapdf_build::Error::Text(e) => e.into(),
            other => Self::core(Inner::Build(other)),
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
            Error::RevisionIndexOutOfRange { index: 2, count: 2 },
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
        let from_ops = Error::from(laminapdf_ops::Error::Cos(laminapdf_cos::Error::MissingRoot));
        let from_save = Error::from(laminapdf_save::SaveError::Ops(laminapdf_ops::Error::Cos(
            laminapdf_cos::Error::MissingRoot,
        )));
        assert_eq!(
            [from_ops, from_save],
            [const { Error::core(Inner::Cos(laminapdf_cos::Error::MissingRoot)) }; 2]
        );
    }

    #[test]
    fn limit_hint_names_the_limit() {
        let e = Error::from(laminapdf_cos::Error::LimitExceeded {
            limit: laminapdf_cos::Limit::NestingDepth,
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
