//! The parts of the error and diagnostic contract ([ADR 0026]) that every layer shares: the
//! nine [`ErrorKind`]s and tagged positions ([`Position`], [`Location`]).
//!
//! Codes form one namespace across crates, listed once in `fixtures/codes.json`, from which
//! `cargo xtask gen codes` generates [`ErrorCode`](crate::ErrorCode) and
//! [`WarningCode`](crate::WarningCode). Each crate's `kind()` is an exhaustive match over its own
//! variants, checked against that file by the crate's tests.
//!
//! [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md

use core::fmt;

/// What kind of failure a code is: what the caller can do about it (ADR 0026 §2).
///
/// A closed set: bindings and the CLI match on it exhaustively, so adding a kind amends
/// ADR 0026 and is a breaking change. That is why, unlike the error enums, it is not
/// `#[non_exhaustive]`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// The input (PDF bytes, an op log, JSON) is malformed beyond what tolerance repairs.
    Input,
    /// The host failed to supply or take bytes (a `ByteSource` read, a sink write).
    Io,
    /// Valid input that Lamina PDF doesn't implement.
    Unsupported,
    /// A `Limits` value or another resource bound was hit.
    Limit,
    /// Encryption, permissions or signatures block the call, or look tampered with.
    Security,
    /// The caller misused the API: an unknown option, an argument out of range, an unknown ID.
    Usage,
    /// The call isn't valid in the object's current state; it may succeed after a change.
    State,
    /// A cancel token stopped the work.
    Cancelled,
    /// A bug in Lamina PDF or a broken invariant.
    Internal,
}

impl ErrorKind {
    /// Whether an error of this kind says only that one part of the input is malformed or uses
    /// something unsupported (`input`, `unsupported`): what a pass over many parts, such as
    /// every page or every revision, skips. Any other kind (a password, a limit, data not yet
    /// fetched, cancellation) stops the pass, since the part it couldn't read might matter.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_skippable(self) -> bool {
        matches!(self, Self::Input | Self::Unsupported)
    }

    /// Every kind, in ADR 0026 §2 order.
    ///
    /// Added in 0.2.0.
    pub const ALL: [Self; 9] = [
        Self::Input,
        Self::Io,
        Self::Unsupported,
        Self::Limit,
        Self::Security,
        Self::Usage,
        Self::State,
        Self::Cancelled,
        Self::Internal,
    ];

    /// The kind's name as it appears in `codes.json` and in serialized diagnostics.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Io => "io",
            Self::Unsupported => "unsupported",
            Self::Limit => "limit",
            Self::Security => "security",
            Self::Usage => "usage",
            Self::State => "state",
            Self::Cancelled => "cancelled",
            Self::Internal => "internal",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where an error was found, tagged so that a number has one meaning (ADR 0026 §1.1).
///
/// The syntax layer only sees the buffer it was given, and reports positions in it as
/// [`Position::FileOffset`]. A layer that hands it something other than the whole file
/// re-tags them when it wraps the error: [`Position::shifted`] for a window of the file,
/// [`Position::in_stream`] for an object stream's decoded data. `laminapdf-cos` does both.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Position {
    /// A byte offset in the input file.
    FileOffset(u64),
    /// The indirect object with this number (e.g. where a reference chain got too long).
    Object(u32),
    /// A byte offset in the decoded data of stream `object` (an object stream's contents).
    InStream {
        /// The stream's object number.
        object: u32,
        /// The byte offset in its decoded data.
        offset: u64,
    },
}

impl Position {
    /// A [`Position::FileOffset`] from a buffer offset (saturating; offsets fit in `u64`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn offset(offset: usize) -> Self {
        Self::FileOffset(u64::try_from(offset).unwrap_or(u64::MAX))
    }

    /// The position moved by `base` bytes when it is a file offset: for an offset in a
    /// window of the file that starts at `base`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn shifted(self, base: u64) -> Self {
        match self {
            Self::FileOffset(offset) => Self::FileOffset(offset.saturating_add(base)),
            other => other,
        }
    }

    /// The position re-tagged as an offset in the decoded data of stream `object`, when it is
    /// a file offset: for a parse of that data.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn in_stream(self, object: u32) -> Self {
        match self {
            Self::FileOffset(offset) => Self::InStream { object, offset },
            other => other,
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::FileOffset(offset) => write!(f, "byte {offset}"),
            Self::Object(object) => write!(f, "object {object}"),
            Self::InStream { object, offset } => write!(f, "byte {offset} of object {object}'s stream data"),
        }
    }
}

/// The `location` of a serialized diagnostic (ADR 0026 §1.1): each field names one kind of
/// position, and all of them are lamina-made values, never document text.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    /// Byte offset in the input file.
    pub offset: Option<u64>,
    /// Object number of the indirect object the problem is in or at.
    pub object: Option<u32>,
    /// Byte offset in the decoded data of `object` (only with `object`).
    pub stream_offset: Option<u64>,
    /// 0-based page index.
    pub page_index: Option<u64>,
    /// A `LaminaPdfId` string, e.g. `p12.0`.
    pub id: Option<String>,
}

impl Location {
    /// No location.
    ///
    /// Added in 0.2.0.
    pub const NONE: Self = Self {
        offset: None,
        object: None,
        stream_offset: None,
        page_index: None,
        id: None,
    };

    /// A location naming only `object`, when there is one.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn object(object: Option<u32>) -> Self {
        Self { object, ..Self::NONE }
    }

    /// Whether no field is set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.offset.is_none()
            && self.object.is_none()
            && self.stream_offset.is_none()
            && self.page_index.is_none()
            && self.id.is_none()
    }
}

impl Default for Location {
    fn default() -> Self {
        Self::NONE
    }
}

impl From<Position> for Location {
    fn from(position: Position) -> Self {
        match position {
            Position::FileOffset(offset) => Self {
                offset: Some(offset),
                ..Self::NONE
            },
            Position::Object(object) => Self::object(Some(object)),
            Position::InStream { object, offset } => Self {
                object: Some(object),
                stream_offset: Some(offset),
                ..Self::NONE
            },
        }
    }
}

impl From<Option<Position>> for Location {
    fn from(position: Option<Position>) -> Self {
        position.map_or(Self::NONE, Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_names_round_trip_through_the_table_order() {
        let names: Vec<_> = ErrorKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            names,
            [
                "input",
                "io",
                "unsupported",
                "limit",
                "security",
                "usage",
                "state",
                "cancelled",
                "internal"
            ]
        );
    }

    #[test]
    fn shifted_moves_only_file_offsets() {
        let moved = [Position::FileOffset(3), Position::Object(3)].map(|p| p.shifted(10));
        assert_eq!(moved, [Position::FileOffset(13), Position::Object(3)]);
    }

    #[test]
    fn in_stream_retags_a_file_offset() {
        assert_eq!(
            Position::FileOffset(4).in_stream(7),
            Position::InStream { object: 7, offset: 4 }
        );
    }

    #[test]
    fn location_from_in_stream_names_the_object_and_the_stream_offset() {
        let location = Location::from(Position::InStream { object: 7, offset: 4 });
        assert_eq!(
            (location.object, location.stream_offset, location.offset),
            (Some(7), Some(4), None)
        );
    }
}
