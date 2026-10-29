//! Errors and warnings of the font layer.

use lamina_cos::{ErrorKind, Location, Position};

use crate::font::Tag;
use crate::fstype::FsType;

/// A `Result` with this crate's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Everything that can go wrong while loading, using or embedding a font.
///
/// Messages carry only numbers and fixed text ([ADR 0026] §4): font names, characters, axis
/// tags and texts are in the variant's fields, never in its `Display`. Kinds and pages:
/// `docs/errors/<code>.md`.
///
/// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The bytes are not an OpenType/TrueType font or collection that can be read, or a table
    /// every font needs (`head`, `hhea`, `hmtx`, `maxp`) is missing or malformed.
    #[error("not a readable font (collection member {index})")]
    InvalidFont {
        /// The collection member that was asked for (0 for a single font).
        index: u32,
    },
    /// A collection member past the end of the collection.
    #[error("font collection member {index} does not exist (the collection has {count})")]
    CollectionIndex {
        /// The member that was asked for.
        index: u32,
        /// How many members the collection has (1 for a single font).
        count: u32,
    },
    /// An instance was asked for on an axis the font does not have (a static font has none).
    #[error("the font has no such variation axis")]
    UnknownAxis {
        /// The font's PostScript name.
        font: String,
        /// The axis that was asked for.
        tag: Tag,
    },
    /// A variation coordinate that is NaN or infinite.
    #[error("variation axis values must be finite numbers")]
    InvalidAxisValue {
        /// The axis.
        tag: Tag,
    },
    /// [`FontRegistry::finish_fonts`](crate::FontRegistry::finish_fonts) was given a document
    /// in which a font's reserved object is not a free reserved number: the registry reserved
    /// it in another document.
    #[error("object {object} is not a font object reserved in this document")]
    WrongDocument {
        /// The reserved object number.
        object: u32,
    },
    /// `OS/2.fsType` forbids embedding (restricted licence or bitmap embedding only), and the
    /// caller did not choose [`EmbedPolicy::Override`](crate::EmbedPolicy::Override).
    #[error("the font's OS/2 fsType ({fs_type}) forbids embedding")]
    FontNotEmbeddable {
        /// The font's PostScript name.
        font: String,
        /// The font's embedding permissions.
        fs_type: FsType,
    },
    /// The font has no glyph for a character: lamina never draws `.notdef` silently.
    #[error("the font has no glyph for U+{code:04X}", code = u32::from(*ch))]
    MissingGlyph {
        /// The character.
        ch: char,
        /// The font's PostScript name.
        font: String,
    },
    /// A glyph ID at or past the font's glyph count.
    #[error("glyph {gid} is out of range for the font ({count} glyphs)")]
    GlyphOutOfRange {
        /// The glyph ID.
        gid: u16,
        /// The font's glyph count.
        count: u16,
        /// The font's PostScript name.
        font: String,
    },
    /// A [`FontKey`](crate::FontKey) that this registry did not hand out.
    #[error("font key {key} is not registered here")]
    UnknownFont {
        /// The key's index.
        key: usize,
    },
    /// The subsetter could not build the font program.
    #[error("subsetting the font failed: {reason}")]
    Subset {
        /// The font's PostScript name.
        font: String,
        /// What the subsetter reported (its error's text: the `subsetter` crate's error type is
        /// not part of this API).
        reason: &'static str,
    },
    /// A standard 14 font ran out of codes: each Latin glyph outside `WinAnsiEncoding` that
    /// the document uses takes one of the codes `WinAnsiEncoding` leaves undefined, through
    /// `/Differences`, and there are only 37 of them (`limit`). The text needed more new codes
    /// than were `free`; none were assigned (all or nothing, like
    /// [`FontRegistry::encode`](crate::FontRegistry::encode)).
    #[error("the simple font has {free} of its {limit} /Differences codes left, fewer than the text needs")]
    EncodingFull {
        /// The font's PostScript name.
        font: String,
        /// How many codes `/Differences` can assign in this font.
        limit: usize,
        /// How many of them were still free.
        free: usize,
    },
    /// Writing the font objects into the document failed.
    #[error(transparent)]
    Cos(#[from] lamina_cos::Error),
}

impl Error {
    /// Every code [`Error::code`] can return besides those of [`lamina_cos::Error`] (listed in
    /// `fixtures/codes.json` under `textErrors`).
    pub const CODES: &'static [&'static str] = &[
        "invalid-font",
        "collection-index",
        "unknown-axis",
        "font-not-embeddable",
        "missing-glyph",
        "glyph-out-of-range",
        "unknown-font",
        "subset-failed",
        "invalid-axis-value",
        "wrong-document",
        "encoding-full",
    ];

    /// A stable kebab-case code for this error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidFont { .. } => "invalid-font",
            Self::CollectionIndex { .. } => "collection-index",
            Self::UnknownAxis { .. } => "unknown-axis",
            Self::InvalidAxisValue { .. } => "invalid-axis-value",
            Self::WrongDocument { .. } => "wrong-document",
            Self::FontNotEmbeddable { .. } => "font-not-embeddable",
            Self::MissingGlyph { .. } => "missing-glyph",
            Self::GlyphOutOfRange { .. } => "glyph-out-of-range",
            Self::UnknownFont { .. } => "unknown-font",
            Self::Subset { .. } => "subset-failed",
            Self::EncodingFull { .. } => "encoding-full",
            Self::Cos(e) => e.code(),
        }
    }

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`Error::code`] (a test checks every variant). COS errors report
    /// [`lamina_cos::Error::kind`].
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Cos(e) => e.kind(),
            Self::InvalidFont { .. } | Self::MissingGlyph { .. } => ErrorKind::Input,
            Self::CollectionIndex { .. }
            | Self::UnknownAxis { .. }
            | Self::InvalidAxisValue { .. }
            | Self::WrongDocument { .. }
            | Self::FontNotEmbeddable { .. }
            | Self::GlyphOutOfRange { .. }
            | Self::UnknownFont { .. } => ErrorKind::Usage,
            Self::Subset { .. } => ErrorKind::Unsupported,
            Self::EncodingFull { .. } => ErrorKind::Limit,
        }
    }

    /// Where the error was found (ADR 0026 §1.1): a COS error's position, or the reserved
    /// object of [`Error::WrongDocument`]; font errors have no position in a document.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Cos(e) => e.location(),
            Self::WrongDocument { object } => Location::from(Position::Object(*object)),
            Self::InvalidFont { .. }
            | Self::CollectionIndex { .. }
            | Self::UnknownAxis { .. }
            | Self::InvalidAxisValue { .. }
            | Self::FontNotEmbeddable { .. }
            | Self::MissingGlyph { .. }
            | Self::GlyphOutOfRange { .. }
            | Self::UnknownFont { .. }
            | Self::Subset { .. }
            | Self::EncodingFull { .. } => Location::NONE,
        }
    }
}

/// Something the font layer noticed and worked around; the output is still valid.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// One glyph was used for two different texts. A ToUnicode CMap maps a code to one string
    /// (ISO 32000-2 §9.10.3), so the first text is kept; the builder should mark the other
    /// occurrence with `/ActualText` (§14.9.4).
    ToUnicodeConflict {
        /// The font's PostScript name.
        font: String,
        /// The glyph, in the original font's numbering.
        gid: u16,
        /// The text the ToUnicode CMap maps it to.
        kept: String,
        /// The text that could not be mapped.
        dropped: String,
    },
    /// A font whose `fsType` forbids subsetting could not be embedded byte for byte: it is a
    /// variable-font instance or has CFF2 outlines, which PDF can't embed as they are, so it
    /// was rebuilt by the subsetter with every glyph kept (instanced, or converted to
    /// TrueType outlines; layout tables and `cmap` dropped).
    WholeFontRebuilt {
        /// The font's PostScript name.
        font: String,
    },
}

impl Warning {
    /// Every code [`Warning::code`] can return (listed in `fixtures/codes.json` under
    /// `textWarnings`).
    pub const CODES: &'static [&'static str] = &["tounicode-conflict", "whole-font-rebuilt"];

    /// A stable kebab-case code for this warning.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ToUnicodeConflict { .. } => "tounicode-conflict",
            Self::WholeFontRebuilt { .. } => "whole-font-rebuilt",
        }
    }

    /// The kind of this warning ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`Warning::code`] (a test checks every variant).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::ToUnicodeConflict { .. } | Self::WholeFontRebuilt { .. } => ErrorKind::Usage,
        }
    }

    /// Where the problem is (ADR 0026 §1.1): font warnings have no position in a document.
    #[must_use]
    pub const fn location(&self) -> Location {
        match self {
            Self::ToUnicodeConflict { .. } | Self::WholeFontRebuilt { .. } => Location::NONE,
        }
    }
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToUnicodeConflict { gid, .. } => {
                write!(
                    f,
                    "glyph {gid} stands for two different texts; ToUnicode keeps the first"
                )
            }
            Self::WholeFontRebuilt { .. } => {
                f.write_str("a font that forbids subsetting was rebuilt with every glyph kept")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_codes as codes;

    /// One value of every variant this crate adds: the matches in [`Error::kind`] and
    /// [`Warning::kind`] have no wildcard, so a new variant is a compile error there, and these
    /// lists are what the tests compare.
    fn errors() -> Vec<Error> {
        let tag = Tag::WEIGHT;
        let font = String::new();
        vec![
            Error::InvalidFont { index: 0 },
            Error::CollectionIndex { index: 1, count: 1 },
            Error::UnknownAxis {
                font: font.clone(),
                tag,
            },
            Error::FontNotEmbeddable {
                font: font.clone(),
                fs_type: FsType::new(2),
            },
            Error::MissingGlyph {
                ch: 'x',
                font: font.clone(),
            },
            Error::GlyphOutOfRange {
                gid: 9,
                count: 3,
                font: font.clone(),
            },
            Error::UnknownFont { key: 0 },
            Error::Subset { font, reason: "r" },
            Error::InvalidAxisValue { tag },
            Error::WrongDocument { object: 1 },
            Error::EncodingFull {
                font: String::new(),
                limit: 37,
                free: 0,
            },
        ]
    }

    fn warnings() -> Vec<Warning> {
        vec![
            Warning::ToUnicodeConflict {
                font: String::new(),
                gid: 1,
                kept: String::new(),
                dropped: String::new(),
            },
            Warning::WholeFontRebuilt { font: String::new() },
        ]
    }

    #[test]
    fn codes_lists_the_code_of_every_error_variant_in_order() {
        assert_eq!(codes::distinct(errors().iter().map(Error::code)), Error::CODES);
    }

    #[test]
    fn codes_lists_the_code_of_every_warning_variant_in_order() {
        assert_eq!(codes::distinct(warnings().iter().map(Warning::code)), Warning::CODES);
    }

    #[test]
    fn every_variant_has_the_kind_codes_json_lists() {
        let errors = errors();
        let warnings = warnings();
        codes::assert_kinds_match(
            errors
                .iter()
                .map(|e| (e.code(), e.kind().as_str()))
                .chain(warnings.iter().map(|w| (w.code(), w.kind().as_str()))),
        );
    }

    #[test]
    fn a_cos_error_keeps_its_kind() {
        assert_eq!(Error::Cos(lamina_cos::Error::WrongPassword).kind(), ErrorKind::Security);
    }

    #[test]
    fn wrong_document_is_located_at_the_reserved_object() {
        assert_eq!(Error::WrongDocument { object: 7 }.location().object, Some(7));
    }
}
