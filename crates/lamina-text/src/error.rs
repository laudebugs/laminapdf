//! Errors and warnings of the font layer.

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
    /// `/Differences`, and there are only 37 of them (`limit`).
    #[error("the simple font has no free code for another glyph ({limit} are in use)")]
    EncodingFull {
        /// The font's PostScript name.
        font: String,
        /// How many codes `/Differences` can assign.
        limit: usize,
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
