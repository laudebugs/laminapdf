//! Artifacts (§14.8.2.2): content that isn't part of the document's logical content, written
//! as `/Artifact <</Type …>> BDC … EMC` and left out of the structure tree.

use pdf_writer::types::ArtifactType;
use pdf_writer::{Content, Name, Rect};

/// What kind of artifact (§14.8.2.2.2, Table 363 `/Type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ArtifactKind {
    /// Layout: rules, decorative lines, footnote separators.
    Layout,
    /// Page: cut marks, colour bars and other marks of the physical page.
    Page,
    /// Pagination: running heads, footers, page numbers.
    Pagination,
    /// Background: a background image or colour; its `/BBox` (required) is the page's crop
    /// box.
    Background,
}

impl ArtifactKind {
    /// Every artifact kind, in declaration order.
    pub const ALL: [Self; 4] = [
        Self::Layout,
        Self::Page,
        Self::Pagination,
        Self::Background,
    ];

    /// The kind named `name` (`"Layout"`, `"Page"`, `"Pagination"`, `"Background"`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Layout" => Some(Self::Layout),
            "Page" => Some(Self::Page),
            "Pagination" => Some(Self::Pagination),
            "Background" => Some(Self::Background),
            _ => None,
        }
    }

    const fn artifact_type(self) -> ArtifactType {
        match self {
            Self::Layout => ArtifactType::Layout,
            Self::Page => ArtifactType::Page,
            Self::Pagination => ArtifactType::Pagination,
            Self::Background => ArtifactType::Background,
        }
    }
}

/// `/Artifact <</Type …>> BDC`, with the `/BBox` (user space) a background artifact needs.
pub(crate) fn begin(content: &mut Content, kind: ArtifactKind, page_box: [f32; 4]) {
    let mut mark = content.begin_marked_content_with_properties(Name(b"Artifact"));
    let mut artifact = mark.properties().artifact();
    artifact.kind(kind.artifact_type());
    if kind == ArtifactKind::Background {
        let [x0, y0, x1, y1] = page_box;
        artifact.bounding_box(Rect::new(x0, y0, x1, y1));
    }
}
