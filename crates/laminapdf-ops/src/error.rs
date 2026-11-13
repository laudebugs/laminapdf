//! The error type of the ops layer.

use core::fmt;

use laminapdf_cos::{ErrorKind, Location};
use laminapdf_model::{OutlineItemId, PageId};

/// Everything that can go wrong while applying ops, running transactions or reading op logs.
///
/// `Display` formats only fixed text, numbers and object IDs (`p12.0`), never text from the
/// input or the op log (ADR 0026 §4). A path ID (`p5.0/Kids/3`) carries dictionary keys, so
/// it stays in the variant's field and is not printed.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error from the COS or model layer (reading the document, allocating a number).
    #[error(transparent)]
    Cos(laminapdf_cos::Error),
    /// A transaction was started (or undo/redo called) while one is open (ADR 0006 §2). Use
    /// [`Tx::savepoint`](crate::Tx::savepoint) to try something inside a transaction.
    #[error("a transaction is already open")]
    NestedTransaction,
    /// [`Document::apply`](crate::Document::apply), `commit` or `rollback` without an open
    /// transaction.
    #[error("no transaction is open")]
    NoTransaction,
    /// The op names a page the document does not have.
    #[error("no page {}", Shown(.0))]
    UnknownPage(PageId),
    /// The page is a direct dictionary inside its parent; promoting it to an indirect object
    /// (ADR 0005 §5) arrives with the page-structure ops.
    #[error("page {} is a direct dictionary, which ops cannot edit yet", Shown(.0))]
    DirectPage(PageId),
    /// An op log with a schema version this reader does not know (ADR 0006 §6).
    #[error("unsupported op log version {0}")]
    UnsupportedVersion(u64),
    /// An op log that does not parse: an unknown op or field, a bad value, or not JSON. The
    /// parser's message can quote the log, so it is kept here but never formatted
    /// (ADR 0026 §4).
    #[error("invalid op log")]
    InvalidLog(String),
    /// The document was saved to a sink the core can't read back and waits for
    /// [`Document::rebase`](crate::Document::rebase): ops, undo, redo and saves are refused
    /// until then (ADR 0005 §3).
    #[error("the document was saved but not rebased yet")]
    SavedNotRebased,
    /// [`Document::rebase`](crate::Document::rebase) without a save waiting for it: other
    /// bytes would not match the undo history, and unsaved changes would be lost.
    #[error("no save is waiting to be rebased onto")]
    NoPendingSave,
    /// A metadata edit would have to replace the document's XMP packet, which can't be read
    /// (`xmp-invalid`), losing whatever it holds. The edit is refused unless it says to
    /// replace the packet ([`SetMetadata::replace_malformed`](crate::SetMetadata::replace_malformed)).
    #[error("the XMP packet can't be read, and the edit would replace it")]
    MalformedXmp,
    /// An op, or a call, names a source document the session doesn't hold: add it with
    /// [`Document::add_source`](crate::Document::add_source) first (a restarted document worker re-adds
    /// its sources before it replays the log).
    #[error("no source document with this ID was added to the session")]
    UnknownSource(crate::SourceId),
    /// The op names a bookmark the outline does not have.
    #[error("the outline has no item {}", ShownItem(.0))]
    UnknownOutlineItem(OutlineItemId),
    /// The bookmark is a direct dictionary inside another item; the outline ops edit it once
    /// another outline op has rewritten the outline, which gives it an object of its own.
    #[error("outline item {} is a direct dictionary, which the outline ops can't name", ShownItem(.0))]
    DirectOutlineItem(OutlineItemId),
    /// A position names a bookmark that is not a child of the parent the op names.
    #[error("outline item {} is not a child of the given parent", ShownItem(.0))]
    NotASibling(OutlineItemId),
    /// A move would put a bookmark inside itself or one of its descendants.
    #[error("outline item {} can't move inside itself", ShownItem(.0))]
    OutlineIntoItself(OutlineItemId),
    /// The op names a destination the file does not define. The name is kept here but never
    /// formatted.
    #[error("no named destination has this name")]
    UnknownNamedDestination(String),
    /// A rename to a name another destination already has. The name is kept here but never
    /// formatted.
    #[error("another named destination already has this name")]
    NamedDestinationExists(String),
    /// A destination name is empty.
    #[error("a destination name can't be empty")]
    EmptyDestinationName,
    /// A web address that isn't an absolute `http`, `https` or `mailto` address in ASCII.
    #[error("only absolute http, https and mailto addresses are allowed")]
    LinkUriRefused,
    /// A destination's view has a number that isn't finite.
    #[error("a destination's coordinates and zoom must be finite numbers")]
    ViewNotFinite,
}

/// An outline item ID as a message may show it, as [`Shown`] does pages.
struct ShownItem<'a>(&'a OutlineItemId);

impl fmt::Display for ShownItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_lamina_id().obj_ref() {
            Some(_) => self.0.fmt(f),
            None => f.write_str("(a path ID)"),
        }
    }
}

/// A page ID as a message may show it: an object ID in full (lamina-made: a kind and two
/// numbers), a path ID without its keys.
struct Shown<'a>(&'a PageId);

impl fmt::Display for Shown<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_lamina_id().obj_ref() {
            Some(_) => self.0.fmt(f),
            None => f.write_str("(a path ID)"),
        }
    }
}

impl From<laminapdf_cos::Error> for Error {
    fn from(e: laminapdf_cos::Error) -> Self {
        Self::Cos(e)
    }
}

impl Error {
    /// Every code [`Error::code`] can return besides those of [`laminapdf_cos::Error`] (listed in
    /// `fixtures/codes.json` under `opErrors`).
    ///
    /// Added in 0.2.0.
    pub const CODES: &'static [&'static str] = &[
        "nested-transaction",
        "no-transaction",
        "unknown-page",
        "direct-page",
        "unsupported-version",
        "invalid-op-log",
        "saved-not-rebased",
        "no-pending-save",
        "malformed-xmp",
        "unknown-source",
        "unknown-outline-item",
        "direct-outline-item",
        "outline-not-a-sibling",
        "outline-move-into-itself",
        "unknown-named-destination",
        "named-destination-exists",
        "empty-destination-name",
        "link-uri-refused",
        "view-not-finite",
    ];

    /// A stable kebab-case code for this error; COS errors report [`laminapdf_cos::Error::code`].
    ///
    /// Added in 0.2.0.
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
            Self::SavedNotRebased => "saved-not-rebased",
            Self::NoPendingSave => "no-pending-save",
            Self::MalformedXmp => "malformed-xmp",
            Self::UnknownSource(_) => "unknown-source",
            Self::UnknownOutlineItem(_) => "unknown-outline-item",
            Self::DirectOutlineItem(_) => "direct-outline-item",
            Self::NotASibling(_) => "outline-not-a-sibling",
            Self::OutlineIntoItself(_) => "outline-move-into-itself",
            Self::UnknownNamedDestination(_) => "unknown-named-destination",
            Self::NamedDestinationExists(_) => "named-destination-exists",
            Self::EmptyDestinationName => "empty-destination-name",
            Self::LinkUriRefused => "link-uri-refused",
            Self::ViewNotFinite => "view-not-finite",
        }
    }

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`Error::code`] (a test checks every variant). COS errors report
    /// [`laminapdf_cos::Error::kind`].
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Cos(e) => e.kind(),
            Self::NestedTransaction | Self::NoTransaction | Self::SavedNotRebased | Self::NoPendingSave => {
                ErrorKind::State
            }
            Self::UnknownPage(_)
            | Self::UnknownSource(_)
            | Self::UnknownOutlineItem(_)
            | Self::NotASibling(_)
            | Self::OutlineIntoItself(_)
            | Self::UnknownNamedDestination(_)
            | Self::NamedDestinationExists(_)
            | Self::EmptyDestinationName
            | Self::LinkUriRefused
            | Self::ViewNotFinite => ErrorKind::Usage,
            Self::DirectPage(_) | Self::UnsupportedVersion(_) | Self::DirectOutlineItem(_) => ErrorKind::Unsupported,
            Self::InvalidLog(_) | Self::MalformedXmp => ErrorKind::Input,
        }
    }

    /// Where the error was found (ADR 0026 §1.1): a COS error's position, or the page's
    /// object and ID when it is an object ID (a path ID's keys are not lamina-made, so a path
    /// ID gives no location).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Cos(e) => e.location(),
            Self::UnknownPage(page) | Self::DirectPage(page) => match page.as_lamina_id().obj_ref() {
                Some(r) => Location {
                    object: Some(r.num),
                    id: Some(page.to_string()),
                    ..Location::NONE
                },
                None => Location::NONE,
            },
            Self::UnknownOutlineItem(item)
            | Self::DirectOutlineItem(item)
            | Self::NotASibling(item)
            | Self::OutlineIntoItself(item) => match item.as_lamina_id().obj_ref() {
                Some(r) => Location {
                    object: Some(r.num),
                    id: Some(item.to_string()),
                    ..Location::NONE
                },
                None => Location::NONE,
            },
            Self::UnknownNamedDestination(_)
            | Self::NamedDestinationExists(_)
            | Self::EmptyDestinationName
            | Self::LinkUriRefused
            | Self::ViewNotFinite
            | Self::NestedTransaction
            | Self::NoTransaction
            | Self::UnsupportedVersion(_)
            | Self::InvalidLog(_)
            | Self::SavedNotRebased
            | Self::NoPendingSave
            | Self::MalformedXmp
            | Self::UnknownSource(_) => Location::NONE,
        }
    }
}

/// Shorthand for `Result<T, laminapdf_ops::Error>`.
///
/// Added in 0.2.0.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_codes as codes;

    /// One value of every variant: the match in [`Error::kind`] has no wildcard, so a new
    /// variant is a compile error there, and this list is what the tests compare.
    fn samples() -> Vec<Error> {
        let page: PageId = "p3.0".parse().unwrap();
        let item: OutlineItemId = "o4.0".parse().unwrap();
        vec![
            Error::NestedTransaction,
            Error::NoTransaction,
            Error::UnknownPage(page.clone()),
            Error::DirectPage(page),
            Error::UnsupportedVersion(9),
            Error::InvalidLog(String::new()),
            Error::SavedNotRebased,
            Error::NoPendingSave,
            Error::MalformedXmp,
            Error::UnknownSource(crate::SourceId::from_bytes([0; 32])),
            Error::UnknownOutlineItem(item.clone()),
            Error::DirectOutlineItem(item.clone()),
            Error::NotASibling(item.clone()),
            Error::OutlineIntoItself(item),
            Error::UnknownNamedDestination(String::new()),
            Error::NamedDestinationExists(String::new()),
            Error::EmptyDestinationName,
            Error::LinkUriRefused,
            Error::ViewNotFinite,
        ]
    }

    #[test]
    fn codes_lists_the_code_of_every_variant_in_order() {
        assert_eq!(codes::distinct(samples().iter().map(Error::code)), Error::CODES);
    }

    #[test]
    fn every_variant_has_the_kind_codes_json_lists() {
        codes::assert_kinds_match(samples().iter().map(|e| (e.code(), e.kind().as_str())));
    }

    #[test]
    fn a_cos_error_keeps_its_kind() {
        assert_eq!(
            Error::Cos(laminapdf_cos::Error::WrongPassword).kind(),
            ErrorKind::Security
        );
    }
}
