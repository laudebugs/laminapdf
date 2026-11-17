//! [`AfRelationship`]: how an associated file relates to what it is attached to.

/// How an attached file relates to the document, page or annotation it is associated with
/// (`/AFRelationship`, ISO 32000-2 §14.13.2, Table 43).
///
/// ```
/// use laminapdf_model::AfRelationship;
///
/// assert_eq!(AfRelationship::Alternative.as_str(), "Alternative");
/// assert_eq!(AfRelationship::from_name(b"Data"), Some(AfRelationship::Data));
/// assert_eq!(AfRelationship::from_name(b"Mystery"), None);
/// assert_eq!(AfRelationship::EncryptedPayload.camel_name(), "encryptedPayload");
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AfRelationship {
    /// The original the content was made from (a word-processor file).
    Source,
    /// Data the content was derived from (a chart's CSV).
    Data,
    /// Another representation of the same content (an e-invoice's XML).
    Alternative,
    /// A supplement to the content.
    Supplement,
    /// An encrypted payload document (ISO 32000-2 §7.6.7).
    ///
    /// Added in 0.6.0.
    EncryptedPayload,
    /// Data of a form's fields.
    ///
    /// Added in 0.6.0.
    FormData,
    /// A schema of other data.
    ///
    /// Added in 0.6.0.
    Schema,
    /// None of these (the default).
    #[default]
    Unspecified,
}

impl AfRelationship {
    /// Every relationship, in the order of ISO 32000-2's Table 43.
    ///
    /// Added in 0.6.0.
    pub const ALL: [Self; 8] = [
        Self::Source,
        Self::Data,
        Self::Alternative,
        Self::Supplement,
        Self::EncryptedPayload,
        Self::FormData,
        Self::Schema,
        Self::Unspecified,
    ];

    /// The name written as `/AFRelationship`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Data => "Data",
            Self::Alternative => "Alternative",
            Self::Supplement => "Supplement",
            Self::EncryptedPayload => "EncryptedPayload",
            Self::FormData => "FormData",
            Self::Schema => "Schema",
            Self::Unspecified => "Unspecified",
        }
    }

    /// The relationship's name in camelCase (`encryptedPayload`), as the attachment ops and the
    /// bindings spell it: the one list both read.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn camel_name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Data => "data",
            Self::Alternative => "alternative",
            Self::Supplement => "supplement",
            Self::EncryptedPayload => "encryptedPayload",
            Self::FormData => "formData",
            Self::Schema => "schema",
            Self::Unspecified => "unspecified",
        }
    }

    /// The relationship an `/AFRelationship` name stands for; `None` for a name the standard
    /// doesn't define (ISO 32000-2 allows second-class names, which readers treat as
    /// unspecified).
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn from_name(name: &[u8]) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.as_str().as_bytes() == name)
    }
}
