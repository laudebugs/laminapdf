//! Errors of the builder.

use laminapdf_cos::{ErrorKind, Location};

use crate::profile::Profile;
use crate::tag::ContentKind;

/// A `Result` with this crate's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The image formats the builder reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    /// JPEG (embedded as it is, `DCTDecode`).
    Jpeg,
    /// PNG (decoded).
    Png,
}

/// Everything that can go wrong while building.
///
/// Messages carry only numbers and fixed text ([ADR 0026] §4): text the caller passed (a colour
/// string) is in the variant's fields, never in its `Display`. Kinds and pages:
/// `docs/errors/<code>.md`.
///
/// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The image bytes are not a readable image of the format.
    #[error("not a readable {format:?} image: {reason}")]
    InvalidImage {
        /// The format the bytes were read as.
        format: ImageFormat,
        /// What is wrong (fixed text).
        reason: &'static str,
    },
    /// A valid image using a feature PDF or Lamina PDF does not support (an arithmetic-coded or
    /// lossless JPEG, a 12-bit JPEG, an unusual number of components).
    #[error("{format:?} image uses an unsupported feature: {feature}")]
    UnsupportedImage {
        /// The image's format.
        format: ImageFormat,
        /// The feature (fixed text).
        feature: &'static str,
    },
    /// Decoding the image would need more memory than the builder allows.
    #[error("the image is too large to decode (more than {limit} bytes)")]
    ImageTooLarge {
        /// The limit, in decoded bytes.
        limit: usize,
    },
    /// Raw samples whose length is not `width × height × channels`.
    #[error("image data is {actual} bytes, expected {expected}")]
    ImageDataLength {
        /// The length the dimensions call for.
        expected: usize,
        /// The length given.
        actual: usize,
    },
    /// ICC profile bytes that are not a usable ICC profile.
    #[error("not a usable ICC profile: {reason}")]
    InvalidIccProfile {
        /// What is wrong (fixed text).
        reason: &'static str,
    },
    /// A colour with the wrong number of components for its colour space.
    #[error("the colour space has {expected} components, {actual} were given")]
    ColorComponents {
        /// The colour space's number of components (`/N`).
        expected: usize,
        /// How many were given.
        actual: usize,
    },
    /// A colour string that is not `#rgb` or `#rrggbb`.
    #[error("not a colour of the form #rgb or #rrggbb")]
    InvalidColor {
        /// The string as given (**untrusted**: caller text).
        text: String,
    },
    /// A page size, box, `/UserUnit` or other geometry that is not finite and positive where
    /// it must be.
    #[error("invalid page geometry: {what}")]
    InvalidGeometry {
        /// What is invalid (fixed text).
        what: &'static str,
    },
    /// A text box's lines don't all fit its height, and its options asked for an error
    /// rather than clipping ([`Overflow::Error`](crate::Overflow::Error)).
    #[error("the text needs {needed} lines, the box holds {fits}")]
    TextOverflow {
        /// The lines that fit.
        fits: usize,
        /// The lines the text needs.
        needed: usize,
    },
    /// A link, outline item or destination names a page index the document doesn't have.
    #[error("page index {index} is out of range: the document has {count} pages")]
    PageIndexOutOfRange {
        /// The 0-based index given.
        index: u32,
        /// The number of pages.
        count: usize,
    },
    /// A link or outline item names a destination that [`Builder::destination`](crate::Builder::destination)
    /// never defined.
    #[error("no named destination with this name was defined")]
    UnknownDestination {
        /// The name (**untrusted**: caller text).
        name: String,
    },
    /// Page label ranges that don't start at page 0, aren't in increasing order, start past
    /// the last page or number from 0.
    #[error("invalid page label ranges: {what}")]
    InvalidPageLabels {
        /// What is wrong (fixed text).
        what: &'static str,
    },
    /// [`Surface::resume`](crate::Surface::resume) or
    /// [`Builder::close_element`](crate::Builder::close_element) named an element that isn't
    /// open: closed already, opened by another builder, inside an artifact or on an appended
    /// page, or on a page whose [`PageBuilder::finish`](crate::PageBuilder::finish) failed.
    #[error("the structure element resumed or closed isn't open")]
    TagElementNotOpen,
    /// [`Builder::finish`](crate::Builder::finish) was called with elements opened by
    /// [`Surface::open_element`](crate::Surface::open_element) and never closed.
    #[error("{count} structure element(s) opened across pages were never closed")]
    TagElementUnclosed {
        /// How many are open.
        count: usize,
    },
    /// The PDF version asked for is one the profile doesn't allow (PDF/A-2, PDF/A-3 and
    /// PDF/UA-1 are based on PDF 1.7, so PDF 2.0 is refused).
    #[error("{} documents can't be PDF 2.0", .profile.as_str())]
    ProfileVersionConflict {
        /// The profile.
        profile: Profile,
    },
    /// PDF/UA-1 needs a title (`dc:title` in the XMP packet, ISO 14289-1 §7.1), and
    /// [`Builder::metadata`](crate::Builder::metadata) gave none.
    #[error("the document has no title, which PDF/UA-1 requires")]
    MissingTitle,
    /// PDF/UA-1 needs the document's natural language ([`Builder::lang`](crate::Builder::lang)).
    #[error("the document has no language, which PDF/UA-1 requires")]
    MissingLang,
    /// A `Figure` or `Formula` without alternate text (`/Alt` or `/ActualText`) under PDF/UA-1.
    #[error("a figure or formula on page {page} has no alternate text, which PDF/UA-1 requires")]
    MissingAltText {
        /// The page's index (0-based).
        page: u32,
    },
    /// Content drawn outside every structure element and artifact under PDF/UA-1.
    #[error("page {page} has {count} {content:?} drawing(s) outside every structure element and artifact")]
    UntaggedContent {
        /// The page's index (0-based).
        page: u32,
        /// What was drawn.
        content: ContentKind,
        /// How many times.
        count: usize,
    },
    /// A heading level skipped under PDF/UA-1: the first numbered heading isn't `H1`, or a
    /// heading is more than one level below the one before it (an `H3` right after an `H1`).
    #[error("an H{level} on page {page} follows an H{previous}: heading levels can't be skipped under PDF/UA-1")]
    HeadingLevelSkipped {
        /// The page's index (0-based).
        page: u32,
        /// The heading's level.
        level: u32,
        /// The level of the heading before it (0: none before it).
        previous: u32,
    },
    /// An unnumbered `H` heading misused under PDF/UA-1: a second `H` in the same parent, or
    /// `H` in a document that also uses numbered headings (`H1`, `H2`, …).
    #[error(
        "an H heading on page {page} shares its parent with another H, or the document also uses numbered headings"
    )]
    UnnumberedHeadingMisused {
        /// The page's index (0-based).
        page: u32,
    },
    /// A table or list element in a place ISO 32000-1 Tables 336 and 337 don't allow, under
    /// PDF/UA-1 (a `TD` outside a `TR`, a `P` directly in a `Table` or an `L`).
    #[error("a structure element on page {page} is inside a parent its type doesn't allow")]
    InvalidStructureNesting {
        /// The page's index (0-based).
        page: u32,
        /// The element's standard type.
        kind: String,
        /// Its parent's standard type (`Document` at the top).
        parent: String,
    },
    /// A link annotation without a `Link` structure element under PDF/UA-1 (drawn inside an
    /// artifact).
    #[error("{count} link(s) on page {page} have no Link structure element, which PDF/UA-1 requires")]
    LinkWithoutStructure {
        /// The page's index (0-based).
        page: u32,
        /// How many links.
        count: usize,
    },
    /// A link annotation without `/Contents` (its alternate description) under PDF/UA-1.
    #[error("a link on page {page} has no description (/Contents), which PDF/UA-1 requires")]
    LinkWithoutContents {
        /// The page's index (0-based).
        page: u32,
    },
    /// A standard 14 font (never embedded) used under a profile that needs every font
    /// embedded.
    #[error("a standard 14 font is not embedded, which {} requires", .profile.as_str())]
    FontNotEmbedded {
        /// The profile.
        profile: Profile,
        /// The font's PostScript name.
        font: String,
    },
    /// A glyph drawn without a Unicode value (it stands for no text of its own) under a
    /// profile that needs every glyph mapped to Unicode.
    #[error("glyph {gid} is drawn without a Unicode value, which {} requires", .profile.as_str())]
    GlyphWithoutUnicode {
        /// The profile.
        profile: Profile,
        /// The font's PostScript name (**untrusted**: read from the font).
        font: String,
        /// The glyph's ID in the font.
        gid: u16,
    },
    /// A `DeviceCMYK` colour or image under PDF/A: the output intent is sRGB, so CMYK
    /// device colour has no defined meaning.
    #[error("page {page} uses DeviceCMYK, which the sRGB output intent doesn't cover")]
    DeviceColorWithoutOutputIntent {
        /// The page's index (0-based).
        page: u32,
    },
    /// An attached file under PDF/A-2, which only allows embedded files that are themselves
    /// PDF/A (Lamina PDF can't check that): use PDF/A-3b.
    #[error("{} doesn't allow attached files; PDF/A-3b does", .profile.as_str())]
    EmbeddedFileForbidden {
        /// The profile.
        profile: Profile,
    },
    /// An attached file without a MIME type under PDF/A, which needs one for every embedded
    /// file (`/Subtype`).
    #[error("an attached file has no MIME type, which {} requires", .profile.as_str())]
    EmbeddedFileWithoutMimeType {
        /// The profile.
        profile: Profile,
        /// The file's name (**untrusted**: caller text).
        name: String,
    },
    /// A line, figure, table row or footnote of a flow is taller than the page's body: no page
    /// can hold it.
    #[error("a flow {what} is taller than the page's body")]
    FlowContentTooLarge {
        /// What is too tall: `line`, `figure`, `table row` or `footnote` (fixed text).
        what: &'static str,
    },
    /// A flow with a contents page was laid out four times and its page numbers still
    /// changed. A contents page keeps the same room for every number, so the second layout
    /// settles them: this is a guard against a fault in the layout.
    #[error("a flow's contents page did not settle in four layouts")]
    FlowLayoutUnstable,
    /// Shaping, laying out or embedding text failed (`missing-glyph`, `invalid-feature`, …).
    #[cfg(feature = "text")]
    #[error(transparent)]
    Text(#[from] laminapdf_text::Error),
    /// Reading or writing the document's objects failed.
    #[error(transparent)]
    Cos(#[from] laminapdf_cos::Error),
}

impl Error {
    /// Every code [`Error::code`] can return besides those of [`laminapdf_cos::Error`] (listed in
    /// `fixtures/codes.json` under `buildErrors`).
    pub const CODES: &'static [&'static str] = &[
        "invalid-image",
        "unsupported-image",
        "image-too-large",
        "image-data-length",
        "invalid-icc-profile",
        "color-components",
        "invalid-color",
        "invalid-geometry",
        "text-overflow",
        "page-index-out-of-range",
        "unknown-destination",
        "invalid-page-labels",
        "tag-element-not-open",
        "tag-element-unclosed",
        "profile-version-conflict",
        "missing-title",
        "missing-lang",
        "missing-alt-text",
        "untagged-content",
        "heading-level-skipped",
        "unnumbered-heading-misused",
        "invalid-structure-nesting",
        "link-without-structure",
        "link-without-contents",
        "font-not-embedded",
        "glyph-without-unicode",
        "device-color-without-output-intent",
        "embedded-file-forbidden",
        "embedded-file-without-mime-type",
        "flow-content-too-large",
        "flow-layout-unstable",
    ];

    /// A stable kebab-case code for this error.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidImage { .. } => "invalid-image",
            Self::UnsupportedImage { .. } => "unsupported-image",
            Self::ImageTooLarge { .. } => "image-too-large",
            Self::ImageDataLength { .. } => "image-data-length",
            Self::InvalidIccProfile { .. } => "invalid-icc-profile",
            Self::ColorComponents { .. } => "color-components",
            Self::InvalidColor { .. } => "invalid-color",
            Self::InvalidGeometry { .. } => "invalid-geometry",
            Self::TextOverflow { .. } => "text-overflow",
            Self::PageIndexOutOfRange { .. } => "page-index-out-of-range",
            Self::UnknownDestination { .. } => "unknown-destination",
            Self::InvalidPageLabels { .. } => "invalid-page-labels",
            Self::TagElementNotOpen => "tag-element-not-open",
            Self::TagElementUnclosed { .. } => "tag-element-unclosed",
            Self::ProfileVersionConflict { .. } => "profile-version-conflict",
            Self::MissingTitle => "missing-title",
            Self::MissingLang => "missing-lang",
            Self::MissingAltText { .. } => "missing-alt-text",
            Self::UntaggedContent { .. } => "untagged-content",
            Self::HeadingLevelSkipped { .. } => "heading-level-skipped",
            Self::UnnumberedHeadingMisused { .. } => "unnumbered-heading-misused",
            Self::InvalidStructureNesting { .. } => "invalid-structure-nesting",
            Self::LinkWithoutStructure { .. } => "link-without-structure",
            Self::LinkWithoutContents { .. } => "link-without-contents",
            Self::FontNotEmbedded { .. } => "font-not-embedded",
            Self::GlyphWithoutUnicode { .. } => "glyph-without-unicode",
            Self::DeviceColorWithoutOutputIntent { .. } => "device-color-without-output-intent",
            Self::EmbeddedFileForbidden { .. } => "embedded-file-forbidden",
            Self::EmbeddedFileWithoutMimeType { .. } => "embedded-file-without-mime-type",
            Self::FlowContentTooLarge { .. } => "flow-content-too-large",
            Self::FlowLayoutUnstable => "flow-layout-unstable",
            #[cfg(feature = "text")]
            Self::Text(e) => e.code(),
            Self::Cos(e) => e.code(),
        }
    }

    /// The kind of this error ([ADR 0026] §2), as `fixtures/codes.json` lists it for
    /// [`Error::code`] (a test checks every variant). COS errors report
    /// [`laminapdf_cos::Error::kind`].
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Cos(e) => e.kind(),
            #[cfg(feature = "text")]
            Self::Text(e) => e.kind(),
            Self::InvalidImage { .. } | Self::InvalidIccProfile { .. } => ErrorKind::Input,
            Self::UnsupportedImage { .. } => ErrorKind::Unsupported,
            Self::ImageTooLarge { .. } => ErrorKind::Limit,
            Self::ImageDataLength { .. }
            | Self::ColorComponents { .. }
            | Self::InvalidColor { .. }
            | Self::InvalidGeometry { .. }
            | Self::TextOverflow { .. }
            | Self::PageIndexOutOfRange { .. }
            | Self::UnknownDestination { .. }
            | Self::InvalidPageLabels { .. }
            | Self::TagElementNotOpen
            | Self::TagElementUnclosed { .. }
            | Self::ProfileVersionConflict { .. }
            | Self::MissingTitle
            | Self::MissingLang
            | Self::MissingAltText { .. }
            | Self::UntaggedContent { .. }
            | Self::HeadingLevelSkipped { .. }
            | Self::UnnumberedHeadingMisused { .. }
            | Self::InvalidStructureNesting { .. }
            | Self::LinkWithoutStructure { .. }
            | Self::LinkWithoutContents { .. }
            | Self::FontNotEmbedded { .. }
            | Self::GlyphWithoutUnicode { .. }
            | Self::DeviceColorWithoutOutputIntent { .. }
            | Self::EmbeddedFileForbidden { .. }
            | Self::EmbeddedFileWithoutMimeType { .. }
            | Self::FlowContentTooLarge { .. } => ErrorKind::Usage,
            Self::FlowLayoutUnstable => ErrorKind::Internal,
        }
    }

    /// Where the error was found (ADR 0026 §1.1): a COS or text error's position; the page of
    /// a profile rule broken on one page; no position for the builder's other errors, which
    /// are about the caller's inputs.
    #[must_use]
    pub fn location(&self) -> Location {
        match *self {
            Self::Cos(ref e) => e.location(),
            #[cfg(feature = "text")]
            Self::Text(ref e) => e.location(),
            Self::MissingAltText { page }
            | Self::UntaggedContent { page, .. }
            | Self::HeadingLevelSkipped { page, .. }
            | Self::UnnumberedHeadingMisused { page }
            | Self::InvalidStructureNesting { page, .. }
            | Self::LinkWithoutStructure { page, .. }
            | Self::LinkWithoutContents { page }
            | Self::DeviceColorWithoutOutputIntent { page } => Location {
                page_index: Some(u64::from(page)),
                ..Location::NONE
            },
            Self::InvalidImage { .. }
            | Self::UnsupportedImage { .. }
            | Self::ImageTooLarge { .. }
            | Self::ImageDataLength { .. }
            | Self::InvalidIccProfile { .. }
            | Self::ColorComponents { .. }
            | Self::InvalidColor { .. }
            | Self::InvalidGeometry { .. }
            | Self::TextOverflow { .. }
            | Self::PageIndexOutOfRange { .. }
            | Self::UnknownDestination { .. }
            | Self::InvalidPageLabels { .. }
            | Self::TagElementNotOpen
            | Self::TagElementUnclosed { .. }
            | Self::ProfileVersionConflict { .. }
            | Self::MissingTitle
            | Self::MissingLang
            | Self::FontNotEmbedded { .. }
            | Self::GlyphWithoutUnicode { .. }
            | Self::EmbeddedFileForbidden { .. }
            | Self::EmbeddedFileWithoutMimeType { .. }
            | Self::FlowContentTooLarge { .. }
            | Self::FlowLayoutUnstable => Location::NONE,
        }
    }
}

/// Something the builder worked around; the output is still valid (listed in
/// `fixtures/codes.json` under `buildWarnings`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum Warning {
    /// Blocks kept together (a `Keep` group, or a heading and what it keeps with) are taller
    /// than a page, so the flow split them.
    #[error("blocks kept together are taller than a page and were split")]
    FlowKeepBroken {
        /// The page they start on (0-based, within the flow).
        page: u32,
    },
    /// Markdown held raw HTML, which is left out.
    #[error("raw HTML in the Markdown was left out")]
    MarkdownHtmlIgnored {
        /// Where it starts: a byte offset into the Markdown.
        offset: usize,
    },
    /// Markdown named an image the caller didn't supply (a remote one is never fetched), so
    /// it was left out.
    #[error("an image the Markdown names wasn't supplied and was left out")]
    MarkdownImageMissing {
        /// Where the image starts: a byte offset into the Markdown.
        offset: usize,
    },
}

impl Warning {
    /// Every code [`Warning::code`] can return.
    pub const CODES: &'static [&'static str] = &["flow-keep-broken", "markdown-html-ignored", "markdown-image-missing"];

    /// A stable kebab-case code for this warning.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::FlowKeepBroken { .. } => "flow-keep-broken",
            Self::MarkdownHtmlIgnored { .. } => "markdown-html-ignored",
            Self::MarkdownImageMissing { .. } => "markdown-image-missing",
        }
    }

    /// Its kind ([ADR 0026] §2).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::FlowKeepBroken { .. } | Self::MarkdownImageMissing { .. } => ErrorKind::Usage,
            Self::MarkdownHtmlIgnored { .. } => ErrorKind::Unsupported,
        }
    }

    /// Where it happened: the page, for a flow's warning; a Markdown warning's byte offset is
    /// in its data instead (an offset into the Markdown, not the PDF).
    #[must_use]
    pub fn location(&self) -> Location {
        match *self {
            Self::FlowKeepBroken { page } => Location {
                page_index: Some(u64::from(page)),
                ..Location::NONE
            },
            Self::MarkdownHtmlIgnored { .. } | Self::MarkdownImageMissing { .. } => Location::NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_codes as codes;

    /// One value of every variant this crate adds: the match in [`Error::kind`] has no
    /// wildcard, so a new variant is a compile error there, and this list is what the tests
    /// compare.
    fn errors() -> Vec<Error> {
        vec![
            Error::InvalidImage {
                format: ImageFormat::Png,
                reason: "r",
            },
            Error::UnsupportedImage {
                format: ImageFormat::Jpeg,
                feature: "f",
            },
            Error::ImageTooLarge { limit: 1 },
            Error::ImageDataLength { expected: 4, actual: 3 },
            Error::InvalidIccProfile { reason: "r" },
            Error::ColorComponents { expected: 3, actual: 1 },
            Error::InvalidColor { text: String::new() },
            Error::InvalidGeometry { what: "w" },
            Error::TextOverflow { fits: 1, needed: 2 },
            Error::PageIndexOutOfRange { index: 3, count: 1 },
            Error::UnknownDestination { name: String::new() },
            Error::InvalidPageLabels { what: "w" },
            Error::TagElementNotOpen,
            Error::TagElementUnclosed { count: 1 },
            Error::ProfileVersionConflict {
                profile: Profile::PdfA2b,
            },
            Error::MissingTitle,
            Error::MissingLang,
            Error::MissingAltText { page: 0 },
            Error::UntaggedContent {
                page: 0,
                content: ContentKind::Path,
                count: 1,
            },
            Error::HeadingLevelSkipped {
                page: 0,
                level: 3,
                previous: 1,
            },
            Error::UnnumberedHeadingMisused { page: 0 },
            Error::InvalidStructureNesting {
                page: 0,
                kind: String::new(),
                parent: String::new(),
            },
            Error::LinkWithoutStructure { page: 0, count: 1 },
            Error::LinkWithoutContents { page: 0 },
            Error::FontNotEmbedded {
                profile: Profile::PdfUa1,
                font: String::new(),
            },
            Error::GlyphWithoutUnicode {
                profile: Profile::PdfA2u,
                font: String::new(),
                gid: 1,
            },
            Error::DeviceColorWithoutOutputIntent { page: 0 },
            Error::EmbeddedFileForbidden {
                profile: Profile::PdfA2b,
            },
            Error::EmbeddedFileWithoutMimeType {
                profile: Profile::PdfA3b,
                name: String::new(),
            },
            Error::FlowContentTooLarge { what: "line" },
            Error::FlowLayoutUnstable,
        ]
    }

    #[test]
    fn codes_lists_the_code_of_every_variant_in_order() {
        assert_eq!(codes::distinct(errors().iter().map(Error::code)), Error::CODES);
    }

    #[test]
    fn every_variant_has_the_kind_codes_json_lists() {
        codes::assert_kinds_match(errors().iter().map(|e| (e.code(), e.kind().as_str())));
    }

    #[test]
    fn an_error_found_on_a_page_names_the_page_index() {
        let e = Error::LinkWithoutContents { page: 2 };
        assert_eq!(e.location().page_index, Some(2));
    }

    #[test]
    fn every_error_with_a_page_has_its_page_index() {
        let pages: Vec<Option<u64>> = errors()
            .iter()
            .filter(|e| e.to_string().contains("page 0"))
            .map(|e| e.location().page_index)
            .collect();
        assert!(pages.len() == 8 && pages.iter().all(|p| *p == Some(0)), "{pages:?}");
    }

    #[test]
    fn the_message_never_contains_the_attachment_name() {
        let e = Error::EmbeddedFileWithoutMimeType {
            profile: Profile::PdfA3b,
            name: "IgnorePreviousInstructions".into(),
        };
        assert!(!e.to_string().contains("Ignore"));
    }

    #[test]
    fn a_cos_error_keeps_its_kind_and_code() {
        let e = Error::from(laminapdf_cos::Error::RepairedBase);
        assert_eq!((e.code(), e.kind()), ("repaired-base", ErrorKind::State));
    }

    #[cfg(feature = "text")]
    #[test]
    fn a_text_error_keeps_its_kind_and_code() {
        let e = Error::from(laminapdf_text::Error::InvalidFeature { feature: String::new() });
        assert_eq!((e.code(), e.kind()), ("invalid-feature", ErrorKind::Usage));
    }

    #[test]
    fn the_message_never_contains_the_destination_name() {
        let e = Error::UnknownDestination {
            name: "IgnorePreviousInstructions".into(),
        };
        assert!(!e.to_string().contains("Ignore"));
    }

    #[test]
    fn the_message_never_contains_the_font_name() {
        let e = Error::GlyphWithoutUnicode {
            profile: Profile::PdfA2u,
            font: "IgnorePreviousInstructions".into(),
            gid: 3,
        };
        assert!(!e.to_string().contains("Ignore"));
    }

    #[test]
    fn the_message_never_contains_the_colour_text() {
        let e = Error::InvalidColor {
            text: "IgnorePreviousInstructions".into(),
        };
        assert!(!e.to_string().contains("Ignore"));
    }
}
