//! A page's annotations (ISO 32000-2 §12.5) as records in page space (ADR 0027): what a viewer's
//! annotation layer, an inspector and a comments panel read, without PDF objects. Each record
//! carries the annotation's ID, which [`UpdateAnnot`](crate::UpdateAnnot) and
//! [`DeleteAnnot`](crate::DeleteAnnot) take.

use laminapdf_model::{Annot, AnnotGeometry, PageFrame, PointPt, RectPt, ReplyType, UserSpacePoint};

use crate::document::Document;
use crate::error::{Error, Result};

/// One annotation on a page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PageAnnotation {
    /// Its ID (`a12.0`), as the annotation ops take it; a path ID for an annotation stored as a
    /// direct dictionary, which the ops refuse.
    pub id: String,
    /// Its position in the page's `/Annots`, as PDFium and pdf.js count them.
    pub index: u32,
    /// Its subtype as written (`Highlight`, `Text`, …).
    pub subtype: String,
    /// Its rectangle, in page space.
    pub rect: Option<RectPt>,
    /// `/Contents`, document text.
    pub contents: Option<String>,
    /// `/NM`, document text: a name unique on its page, only a hint.
    pub name: Option<String>,
    /// `/T`, the author.
    pub author: Option<String>,
    /// `/Subj`.
    pub subject: Option<String>,
    /// `/CreationDate` as stored.
    pub created: Option<String>,
    /// `/M` as stored.
    pub modified: Option<String>,
    /// `/F`, the flags (ISO 32000-2 Table 167).
    pub flags: u32,
    /// `/C`: no components (transparent), one (gray), three (RGB) or four (CMYK).
    pub color: Vec<f64>,
    /// `/CA`, the opacity.
    pub opacity: Option<f64>,
    /// The ID of the annotation this one replies to or is grouped with (`/IRT`).
    pub in_reply_to: Option<String>,
    /// Whether [`PageAnnotation::in_reply_to`] is a group (`/RT /Group`) rather than a reply.
    pub grouped: bool,
    /// The ID of its Popup.
    pub popup: Option<String>,
    /// For a Popup, the ID of the annotation it belongs to.
    pub parent: Option<String>,
    /// Whether it has a normal appearance stream.
    pub has_appearance: bool,
    /// Whether it has rich-text contents (`/RC`).
    pub has_rich_text: bool,
    /// The subtype's own geometry, in page space.
    pub geometry: AnnotationGeometry,
}

/// Where an annotation's own geometry puts it, in page space.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AnnotationGeometry {
    /// Nothing besides its rectangle.
    #[default]
    None,
    /// The text markup's areas, four corners each, in the order the file stores them.
    Quads(Vec<[PointPt; 4]>),
    /// Ink strokes, one point list each.
    Ink(Vec<Vec<PointPt>>),
    /// A polygon's or polyline's vertices.
    Vertices(Vec<PointPt>),
    /// A line's two ends.
    Line([PointPt; 2]),
    /// A callout's two or three points.
    Callout(Vec<PointPt>),
}

impl Document {
    /// The annotations on page `index`, in the order of its `/Annots`, every subtype included.
    ///
    /// ```
    /// # #[cfg(feature = "build")] {
    /// use laminapdf::{BuildOptions, Builder, DeleteAnnot, LinkTarget, Op, PageSize, Pt, RectPt};
    ///
    /// let mut b = Builder::new(BuildOptions::new().with_id(*b"annotations-0001"));
    /// let mut page = b.page(PageSize::A4);
    /// let area = RectPt::from_xywh(Pt::new(56.0), Pt::new(56.0), Pt::new(120.0), Pt::new(16.0));
    /// page.surface().link(area, LinkTarget::Uri("https://example.com/".into()), None);
    /// page.finish()?;
    /// let mut doc = b.finish()?;
    ///
    /// let annots = doc.page_annotations(0)?;
    /// assert_eq!((annots.len(), annots[0].subtype.as_str()), (1, "Link"));
    /// let id = annots[0].id.parse().map_err(|_| "an annotation ID")?;
    /// doc.apply(vec![Op::from(DeleteAnnot::new(id))])?;
    /// assert!(doc.page_annotations(0)?.is_empty());
    /// # }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the annotations, the page tree).
    ///
    /// Added in 0.2.0.
    pub fn page_annotations(&self, index: u32) -> Result<Vec<PageAnnotation>> {
        let pages = self.ops().pages()?;
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or(Error::PageIndexOutOfRange {
                index,
                count: u32::try_from(pages.len()).unwrap_or(u32::MAX),
            })?;
        let frame = page.frame()?;
        Ok(self
            .ops()
            .annotations(&page)?
            .into_iter()
            .map(|annot| record(annot, &frame))
            .collect())
    }
}

fn record(annot: Annot, frame: &PageFrame) -> PageAnnotation {
    let points = |list: Vec<UserSpacePoint>| list.into_iter().map(|p| frame.to_page(p)).collect();
    let geometry = match annot.geometry {
        AnnotGeometry::Quads(quads) => {
            AnnotationGeometry::Quads(quads.iter().map(|q| q.map(|p| frame.to_page(p))).collect())
        }
        AnnotGeometry::Ink(strokes) => AnnotationGeometry::Ink(strokes.into_iter().map(points).collect()),
        AnnotGeometry::Vertices(list) => AnnotationGeometry::Vertices(points(list)),
        AnnotGeometry::Line(ends) => AnnotationGeometry::Line(ends.map(|p| frame.to_page(p))),
        AnnotGeometry::Callout(list) => AnnotationGeometry::Callout(points(list)),
        _ => AnnotationGeometry::None,
    };
    PageAnnotation {
        id: annot.id.to_string(),
        index: u32::try_from(annot.index).unwrap_or(u32::MAX),
        subtype: String::from_utf8_lossy(annot.subtype.name()).into_owned(),
        rect: annot.rect.map(|r| frame.rect_to_page(&r)),
        contents: annot.contents,
        name: annot.name,
        author: annot.author,
        subject: annot.subject,
        created: annot.created,
        modified: annot.modified,
        flags: annot.flags.0,
        color: annot.color,
        opacity: annot.opacity,
        in_reply_to: annot.in_reply_to.map(|id| id.to_string()),
        grouped: annot.reply_type == ReplyType::Group,
        popup: annot.popup.map(|id| id.to_string()),
        parent: annot.parent.map(|id| id.to_string()),
        has_appearance: annot.has_appearance,
        has_rich_text: annot.has_rich_text,
        geometry,
    }
}
