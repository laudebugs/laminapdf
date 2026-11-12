//! [`ExtractError`]: what stops an extraction ([ADR 0026] codes; the façade wraps it).
//!
//! [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md

use laminapdf_syntax::{ErrorKind, Location};

/// What stops an extraction. Errors end the job ([ADR 0028] §3): there is no partial result.
///
/// [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
///
/// Added in 0.2.0.
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
    /// A layout model, OCR engine or page renderer failed.
    #[error("a model failed: {reason}")]
    ModelFailed {
        /// The plug-in's message (never document text).
        reason: String,
    },
    /// [`extract`](crate::extract) was given a model whose results only the host can give: a
    /// host model, or a plug-in without a renderer.
    #[error("a model's results must come from the host: step an extraction job and provide them")]
    ModelNeedsHost,
    /// [`ExtractJob::provide`](crate::ExtractJob::provide) was given a result the job didn't
    /// ask for: none was asked for, or it was the other model's.
    #[error("the job asked for no such model result")]
    ModelResultUnexpected,
    /// Reading the document failed (its own code: `data-pending`, `strict`, a limit…).
    #[error(transparent)]
    Document(#[from] laminapdf_cos::Error),
}

impl From<crate::plugins::ModelError> for ExtractError {
    fn from(e: crate::plugins::ModelError) -> Self {
        match e {
            crate::plugins::ModelError::Cancelled => Self::Cancelled,
            crate::plugins::ModelError::Failed { reason } => Self::ModelFailed { reason },
            other => Self::ModelFailed {
                reason: other.to_string(),
            },
        }
    }
}

impl ExtractError {
    /// The error's code in `fixtures/codes.json`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::JobFinished => "job-finished",
            Self::PageIndexOutOfRange { .. } => "page-index-out-of-range",
            Self::ModelFailed { .. } => "model-failed",
            Self::ModelResultUnexpected => "model-result-unexpected",
            Self::ModelNeedsHost => "model-needs-host",
            Self::Document(e) => e.code(),
        }
    }

    /// The error's kind (ADR 0026 §2).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Cancelled => ErrorKind::Cancelled,
            Self::JobFinished => ErrorKind::State,
            Self::PageIndexOutOfRange { .. } | Self::ModelResultUnexpected | Self::ModelNeedsHost => ErrorKind::Usage,
            Self::ModelFailed { .. } => ErrorKind::Internal,
            Self::Document(e) => e.kind(),
        }
    }

    /// Where the error is: the document's location for a reading error, none otherwise.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Document(e) => e.location(),
            Self::Cancelled
            | Self::JobFinished
            | Self::PageIndexOutOfRange { .. }
            | Self::ModelFailed { .. }
            | Self::ModelResultUnexpected
            | Self::ModelNeedsHost => Location::NONE,
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
            ExtractError::ModelFailed { reason: String::new() },
            ExtractError::ModelResultUnexpected,
            ExtractError::ModelNeedsHost,
        ];
        assert_kinds_match(own.iter().map(|e| (e.code(), e.kind().as_str())));
    }
}
