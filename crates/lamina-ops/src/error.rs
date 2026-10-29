//! The error type of the ops layer.

use lamina_model::PageId;

/// Everything that can go wrong while applying ops, running transactions or reading op logs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error from the COS or model layer (reading the document, allocating a number).
    #[error(transparent)]
    Cos(lamina_cos::Error),
    /// A transaction was started (or undo/redo called) while one is open (ADR 0006 §2). Use
    /// [`Tx::savepoint`](crate::Tx::savepoint) to try something inside a transaction.
    #[error("a transaction is already open")]
    NestedTransaction,
    /// [`Document::apply`](crate::Document::apply), `commit` or `rollback` without an open
    /// transaction.
    #[error("no transaction is open")]
    NoTransaction,
    /// The op names a page the document does not have.
    #[error("no page {0}")]
    UnknownPage(PageId),
    /// The page is a direct dictionary inside its parent; promoting it to an indirect object
    /// (ADR 0005 §5) arrives with the page-structure ops.
    #[error("page {0} is a direct dictionary, which ops cannot edit yet")]
    DirectPage(PageId),
    /// An op log with a schema version this reader does not know (ADR 0006 §6).
    #[error("unsupported op log version {0}")]
    UnsupportedVersion(u64),
    /// An op log that does not parse: an unknown op or field, a bad value, or not JSON.
    #[error("invalid op log: {0}")]
    InvalidLog(String),
}

impl From<lamina_cos::Error> for Error {
    fn from(e: lamina_cos::Error) -> Self {
        Self::Cos(e)
    }
}

impl Error {
    /// Every code [`Error::code`] can return besides those of [`lamina_cos::Error`] (listed in
    /// `fixtures/codes.json` under `opErrors`).
    pub const CODES: &'static [&'static str] = &[
        "nested-transaction",
        "no-transaction",
        "unknown-page",
        "direct-page",
        "unsupported-version",
        "invalid-op-log",
    ];

    /// A stable kebab-case code for this error; COS errors report [`lamina_cos::Error::code`].
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Cos(e) => e.code(),
            Self::NestedTransaction => "nested-transaction",
            Self::NoTransaction => "no-transaction",
            Self::UnknownPage(_) => "unknown-page",
            Self::DirectPage(_) => "direct-page",
            Self::UnsupportedVersion(_) => "unsupported-version",
            Self::InvalidLog(_) => "invalid-op-log",
        }
    }
}

/// Shorthand for `Result<T, lamina_ops::Error>`.
pub type Result<T> = core::result::Result<T, Error>;

/// `Ok(Some(v))`, or `Ok(None)` for a COS error an op may treat as "unreadable": every one
/// except those that [must propagate](lamina_cos::Error::must_propagate), which end the op.
pub(crate) fn tolerant<T>(result: lamina_cos::Result<T>) -> Result<Option<T>> {
    lamina_cos::tolerant(result).map_err(Error::Cos)
}
