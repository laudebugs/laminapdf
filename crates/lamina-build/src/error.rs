//! Errors of the builder.

use lamina_cos::{ErrorKind, Location};

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
/// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
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
    /// A valid image using a feature PDF or lamina does not support (an arithmetic-coded or
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
    /// Reading or writing the document's objects failed.
    #[error(transparent)]
    Cos(#[from] lamina_cos::Error),
}

impl Error {
    /// Every code [`Error::code`] can return besides those of [`lamina_cos::Error`] (listed in
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
            Self::InvalidImage { .. } | Self::InvalidIccProfile { .. } => ErrorKind::Input,
            Self::UnsupportedImage { .. } => ErrorKind::Unsupported,
            Self::ImageTooLarge { .. } => ErrorKind::Limit,
            Self::ImageDataLength { .. }
            | Self::ColorComponents { .. }
            | Self::InvalidColor { .. }
            | Self::InvalidGeometry { .. } => ErrorKind::Usage,
        }
    }

    /// Where the error was found (ADR 0026 §1.1): a COS error's position; the builder's own
    /// errors are about the caller's inputs, which have no position in a document.
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Cos(e) => e.location(),
            Self::InvalidImage { .. }
            | Self::UnsupportedImage { .. }
            | Self::ImageTooLarge { .. }
            | Self::ImageDataLength { .. }
            | Self::InvalidIccProfile { .. }
            | Self::ColorComponents { .. }
            | Self::InvalidColor { .. }
            | Self::InvalidGeometry { .. } => Location::NONE,
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
    fn a_cos_error_keeps_its_kind_and_code() {
        let e = Error::from(lamina_cos::Error::RepairedBase);
        assert_eq!((e.code(), e.kind()), ("repaired-base", ErrorKind::State));
    }

    #[test]
    fn the_message_never_contains_the_colour_text() {
        let e = Error::InvalidColor {
            text: "IgnorePreviousInstructions".into(),
        };
        assert!(!e.to_string().contains("Ignore"));
    }
}
