//! Artifacts (§14.8.2.2): content that isn't part of the document's logical content, written
//! as `/Artifact <</Type …>> BDC … EMC` and left out of the structure tree. A pagination
//! artifact can name what it is (`/Subtype /Header`, `/Footer` or `/Watermark`) and the page
//! edges it is attached to (`/Attached [/Bottom]`), both PDF 1.7 (Table 363).

use pdf_writer::types::{ArtifactAttachment, ArtifactSubtype, ArtifactType};
use pdf_writer::{Content, Name, Rect};

/// What kind of artifact (§14.8.2.2.2, Table 363 `/Type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ArtifactKind {
    /// Layout: rules, decorative lines, footnote separators.
    Layout,
    /// Page: cut marks, colour bars and other marks of the physical page.
    Page,
    /// Pagination: running heads, footers, page numbers. [`Artifact::header`],
    /// [`Artifact::footer`] and [`Artifact::watermark`] also say which.
    Pagination,
    /// Background: a background image or colour; its `/BBox` (required) is the page's crop
    /// box.
    Background,
}

impl ArtifactKind {
    /// Every artifact kind, in declaration order.
    pub const ALL: [Self; 4] = [Self::Layout, Self::Page, Self::Pagination, Self::Background];

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

/// What a pagination artifact is (Table 363 `/Subtype`, PDF 1.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Pagination {
    Header,
    Footer,
    Watermark,
}

/// A page edge an artifact is attached to (Table 363 `/Attached`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PageEdge {
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
}

impl PageEdge {
    /// Every edge, in the order `/Attached` lists them.
    pub const ALL: [Self; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];

    const fn bit(self) -> u8 {
        1 << self as u8
    }

    const fn attachment(self) -> ArtifactAttachment {
        match self {
            Self::Top => ArtifactAttachment::Top,
            Self::Bottom => ArtifactAttachment::Bottom,
            Self::Left => ArtifactAttachment::Left,
            Self::Right => ArtifactAttachment::Right,
        }
    }
}

/// An artifact to draw with [`Surface::artifact`](crate::Surface::artifact): an
/// [`ArtifactKind`] (every one converts into an `Artifact`), or a pagination artifact that
/// says what it is.
///
/// ```
/// use laminapdf_build::{Artifact, ArtifactKind, PageEdge};
///
/// // `/Artifact <</Type /Pagination /Subtype /Footer /Attached [/Bottom]>>`
/// let footer = Artifact::footer();
/// // A running head in the left margin.
/// let side = Artifact::header().with_attached(&[PageEdge::Left]);
/// // `/Artifact <</Type /Layout>>`
/// let rule = Artifact::from(ArtifactKind::Layout);
/// # let _ = (footer, side, rule);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Artifact {
    kind: ArtifactKind,
    subtype: Option<Pagination>,
    /// The edges, one bit per [`PageEdge`].
    attached: u8,
}

impl Artifact {
    /// An artifact of `kind`, with no subtype or edges (the same as `Artifact::from(kind)`).
    #[must_use]
    pub const fn new(kind: ArtifactKind) -> Self {
        Self {
            kind,
            subtype: None,
            attached: 0,
        }
    }

    const fn pagination(subtype: Pagination, edge: Option<PageEdge>) -> Self {
        Self {
            kind: ArtifactKind::Pagination,
            subtype: Some(subtype),
            attached: match edge {
                Some(e) => e.bit(),
                None => 0,
            },
        }
    }

    /// A running head: `/Subtype /Header`, attached to the top edge.
    #[must_use]
    pub const fn header() -> Self {
        Self::pagination(Pagination::Header, Some(PageEdge::Top))
    }

    /// A running footer: `/Subtype /Footer`, attached to the bottom edge.
    #[must_use]
    pub const fn footer() -> Self {
        Self::pagination(Pagination::Footer, Some(PageEdge::Bottom))
    }

    /// A watermark: `/Subtype /Watermark`, attached to no edge.
    #[must_use]
    pub const fn watermark() -> Self {
        Self::pagination(Pagination::Watermark, None)
    }

    /// The same artifact attached to `edges` instead (none: no `/Attached`). Only pagination
    /// artifacts write the edges.
    #[must_use]
    pub fn with_attached(mut self, edges: &[PageEdge]) -> Self {
        self.attached = edges.iter().fold(0, |bits, e| bits | e.bit());
        self
    }

    /// Its kind (`/Type`).
    #[must_use]
    pub const fn kind(self) -> ArtifactKind {
        self.kind
    }

    /// Whether it writes an entry of PDF 1.7 (`/Subtype` or `/Attached`).
    pub(crate) const fn needs_pdf_1_7(self) -> bool {
        matches!(self.kind, ArtifactKind::Pagination) && (self.subtype.is_some() || self.attached != 0)
    }
}

impl From<ArtifactKind> for Artifact {
    fn from(kind: ArtifactKind) -> Self {
        Self::new(kind)
    }
}

/// `/Artifact <</Type …>> BDC`, with the `/BBox` (user space) a background artifact needs and
/// a pagination artifact's `/Subtype` and `/Attached`.
pub(crate) fn begin(content: &mut Content, artifact: Artifact, page_box: [f32; 4]) {
    let mut mark = content.begin_marked_content_with_properties(Name(b"Artifact"));
    let mut properties = mark.properties().artifact();
    properties.kind(artifact.kind.artifact_type());
    if artifact.kind == ArtifactKind::Background {
        let [x0, y0, x1, y1] = page_box;
        properties.bounding_box(Rect::new(x0, y0, x1, y1));
    }
    if artifact.kind == ArtifactKind::Pagination {
        if let Some(subtype) = artifact.subtype {
            properties.subtype(match subtype {
                Pagination::Header => ArtifactSubtype::Header,
                Pagination::Footer => ArtifactSubtype::Footer,
                Pagination::Watermark => ArtifactSubtype::Watermark,
            });
        }
        if artifact.attached != 0 {
            properties.attached(
                PageEdge::ALL
                    .into_iter()
                    .filter(|e| artifact.attached & e.bit() != 0)
                    .map(PageEdge::attachment),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(artifact: Artifact) -> String {
        let mut content = Content::new();
        begin(&mut content, artifact, [0.0, 0.0, 10.0, 10.0]);
        content.end_marked_content();
        String::from_utf8(content.finish().into_vec()).unwrap_or_default()
    }

    /// The match is exhaustive, so a new variant fails to build here first: give it the next
    /// index and add it to `ALL` there, which this test then checks.
    #[test]
    fn all_should_list_every_artifact_kind_once_in_declaration_order() {
        let index = |v: ArtifactKind| match v {
            ArtifactKind::Layout => 0,
            ArtifactKind::Page => 1,
            ArtifactKind::Pagination => 2,
            ArtifactKind::Background => 3,
        };
        assert!(ArtifactKind::ALL.iter().enumerate().all(|(i, v)| index(*v) == i));
    }

    #[test]
    fn all_should_list_every_edge_once_in_declaration_order() {
        let index = |v: PageEdge| match v {
            PageEdge::Top => 0,
            PageEdge::Bottom => 1,
            PageEdge::Left => 2,
            PageEdge::Right => 3,
        };
        assert!(PageEdge::ALL.iter().enumerate().all(|(i, v)| index(*v) == i));
    }

    #[test]
    fn a_footer_is_a_pagination_artifact_attached_to_the_bottom() {
        assert_eq!(
            written(Artifact::footer()),
            "/Artifact <<\n  /Type /Pagination\n  /Subtype /Footer\n  /Attached [/Bottom]\n>> BDC\nEMC"
        );
    }

    #[test]
    fn a_header_is_attached_to_the_top() {
        assert!(written(Artifact::header()).contains("/Subtype /Header\n  /Attached [/Top]"));
    }

    #[test]
    fn a_watermark_is_attached_to_no_edge() {
        let w = written(Artifact::watermark());
        assert!(w.contains("/Subtype /Watermark") && !w.contains("/Attached"));
    }

    #[test]
    fn edges_are_written_in_a_fixed_order_once_each() {
        let a = Artifact::header().with_attached(&[PageEdge::Right, PageEdge::Top, PageEdge::Right]);
        assert!(written(a).contains("/Attached [/Top /Right]"));
    }

    #[test]
    fn the_bare_kind_writes_only_its_type() {
        assert_eq!(
            written(ArtifactKind::Pagination.into()),
            "/Artifact <<\n  /Type /Pagination\n>> BDC\nEMC"
        );
    }

    #[test]
    fn only_pagination_artifacts_write_edges() {
        let a = Artifact::from(ArtifactKind::Layout).with_attached(&[PageEdge::Top]);
        assert_eq!((written(a).contains("/Attached"), a.needs_pdf_1_7()), (false, false));
    }
}
