//! `OS/2.fsType`: the embedding permissions a font declares (OpenType `OS/2` table; research
//! 03 §6.3). They are advisory metadata, not the licence, so the caller can override them.

use std::fmt;

/// A font's `OS/2.fsType` bits. A font without an `OS/2` table has `0` (installable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FsType(u16);

impl FsType {
    /// Restricted licence embedding: the font must not be embedded.
    const RESTRICTED: u16 = 0x0002;
    /// Preview & print embedding.
    const PREVIEW_PRINT: u16 = 0x0004;
    /// Editable embedding.
    const EDITABLE: u16 = 0x0008;
    /// No subsetting: embed the whole font.
    const NO_SUBSETTING: u16 = 0x0100;
    /// Bitmap embedding only: outlines must not be embedded.
    const BITMAP_ONLY: u16 = 0x0200;

    /// Wraps the raw bits.
    #[must_use]
    pub const fn new(bits: u16) -> Self {
        Self(bits)
    }

    /// The raw bits.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Restricted licence embedding. The OpenType spec says the least restrictive of the
    /// usage bits (0–3) wins when several are set, so a font that also allows preview & print
    /// or editable embedding is not restricted.
    #[must_use]
    pub const fn is_restricted(self) -> bool {
        self.0 & Self::RESTRICTED != 0 && self.0 & (Self::PREVIEW_PRINT | Self::EDITABLE) == 0
    }

    /// Only bitmaps may be embedded; lamina embeds outlines, so this forbids embedding too.
    #[must_use]
    pub const fn is_bitmap_only(self) -> bool {
        self.0 & Self::BITMAP_ONLY != 0
    }

    /// The font must be embedded whole (all glyphs, original glyph IDs).
    #[must_use]
    pub const fn forbids_subsetting(self) -> bool {
        self.0 & Self::NO_SUBSETTING != 0
    }

    /// Whether the font may be embedded at all.
    #[must_use]
    pub const fn allows_embedding(self) -> bool {
        !self.is_restricted() && !self.is_bitmap_only()
    }
}

impl fmt::Display for FsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:04X}", self.0)
    }
}

/// Whether to honour a font's `fsType` embedding restrictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum EmbedPolicy {
    /// Refuse fonts whose `fsType` forbids embedding (restricted licence, bitmap only) with
    /// [`Error::FontNotEmbeddable`](crate::Error::FontNotEmbeddable). The default.
    #[default]
    Respect,
    /// Embed anyway: the caller states that its licence allows it. "No subsetting" is still
    /// honoured (the font is embedded whole).
    Override,
}

#[cfg(test)]
mod tests {
    use super::FsType;

    #[test]
    fn installable_allows_embedding() {
        assert!(FsType::new(0).allows_embedding());
    }

    #[test]
    fn restricted_forbids_embedding() {
        assert!(!FsType::new(0x0002).allows_embedding());
    }

    #[test]
    fn restricted_with_editable_is_not_restricted() {
        assert!(FsType::new(0x000A).allows_embedding());
    }

    #[test]
    fn bitmap_only_forbids_embedding() {
        assert!(!FsType::new(0x0200).allows_embedding());
    }

    #[test]
    fn no_subsetting_is_reported() {
        assert!(FsType::new(0x0108).forbids_subsetting());
    }

    #[test]
    fn displays_as_hex() {
        assert_eq!(FsType::new(0x0102).to_string(), "0x0102");
    }
}
