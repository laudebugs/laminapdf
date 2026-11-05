//! [`ExtractError`]: what stops an extraction ([ADR 0026] codes; the façade wraps it).
//!
//! [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md

use laminapdf_syntax::{ErrorKind, Location};

/// What stops an extraction. Errors end the job ([ADR 0028] §3): there is no partial result.
///
/// [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractError {
    /// The job's [`CancelToken`](laminapdf_syntax::CancelToken) was cancelled.
    #[error("the extraction was cancelled")]
    Cancelled,
    /// The job was stepped after it had finished, failed or been cancelled.
    #[error("the extraction job has already finished")]
    JobFinished,
    /// A page asked for is past the document's last page.
    #[error("page index {index} is out of range: the document has {count} pages")]
    PageIndexOutOfRange {
        /// The index asked for.
        index: usize,
        /// The document's page count.
        count: usize,
    },
    /// Reading the document failed (its own code: `data-pending`, `strict`, a limit…).
    #[error(transparent)]
    Document(#[from] laminapdf_cos::Error),
}

impl ExtractError {
    /// The error's code in `fixtures/codes.json`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::JobFinished => "job-finished",
            Self::PageIndexOutOfRange { .. } => "page-index-out-of-range",
            Self::Document(e) => e.code(),
        }
    }

    /// The error's kind (ADR 0026 §2).
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Cancelled => ErrorKind::Cancelled,
            Self::JobFinished => ErrorKind::State,
            Self::PageIndexOutOfRange { .. } => ErrorKind::Usage,
            Self::Document(e) => e.kind(),
        }
    }

    /// Where the error is: the document's location for a reading error, none otherwise.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Document(e) => e.location(),
            Self::Cancelled | Self::JobFinished | Self::PageIndexOutOfRange { .. } => Location::NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_test_fixtures::codes::assert_kinds_match;

    use super::ExtractError;

    #[test]
    fn every_code_and_kind_is_the_one_codes_json_lists() {
        let own = [
            ExtractError::Cancelled,
            ExtractError::JobFinished,
            ExtractError::PageIndexOutOfRange { index: 3, count: 1 },
        ];
        assert_kinds_match(own.iter().map(|e| (e.code(), e.kind().as_str())));
    }
}
