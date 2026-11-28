//! The objects a page draws, for an editor to select and change (ADR 0022):
//! [`Document::content_objects`], [`Document::hit_test`] and [`Document::hit_test_rect`].
//! [`EditContent`](crate::EditContent) changes them.

use laminapdf_content::{ContentObjectKind, ContentOps, ContentOwner, NotEditable};
use laminapdf_model::{PointPt, RectPt};
use laminapdf_ops::{ContentTextRun, EditText, ObjectRef, TextEditReport};

use crate::{Document, Error, Result};

/// One object a page draws, in the order the page paints them (later ones over earlier ones).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PageObject {
    /// Its lasting name, which [`ContentEdit`](crate::ContentEdit)s take.
    pub object: ObjectRef,
    /// What it draws.
    pub kind: ContentObjectKind,
    /// Its bounding box in page space ([ADR 0027]), before clipping; `None` when it draws
    /// nothing.
    ///
    /// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
    pub bounds: Option<RectPt>,
    /// Why it can't be moved, deleted or reordered, or `None` when it can: an editor shows it
    /// read-only, with this reason.
    pub read_only: Option<NotEditable>,
}

impl Document {
    /// The objects page `index` (0-based) draws, in painting order: paths, text objects,
    /// images, forms and shadings, each with its name for an edit, its box and whether it can
    /// be moved. With the `extract` feature, a text object's box is as wide as its glyphs;
    /// without it, a text object has the height of its font but no width.
    ///
    /// ```
    /// # use laminapdf::{Document, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/examples/hello.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// for object in doc.content_objects(0)? {
    ///     println!("{:?} at {:?}", object.kind, object.bounds);
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; errors reading the page's content, and limits it exceeds.
    ///
    /// Added in 0.5.0.
    pub fn content_objects(&self, index: u32) -> Result<Vec<PageObject>> {
        let ir = self.content_ir(index)?;
        Ok(ir
            .objects()
            .iter()
            .filter_map(|object| {
                let name = ir.reference(object.id())?;
                Some(PageObject {
                    object: ObjectRef::from(&name),
                    kind: object.kind(),
                    bounds: object.bounds(),
                    read_only: ir.edit_refusal(object.id()),
                })
            })
            .collect())
    }

    /// The objects of page `index` whose box holds `at`, or comes within 2 points of it,
    /// topmost first.
    ///
    /// # Errors
    /// As [`Document::content_objects`].
    ///
    /// Added in 0.5.0.
    pub fn hit_test(&self, index: u32, at: PointPt) -> Result<Vec<ObjectRef>> {
        let ir = self.content_ir(index)?;
        Ok(names(&ir, ir.hit_test(at)))
    }

    /// The objects of page `index` whose box meets `rect`, topmost first.
    ///
    /// # Errors
    /// As [`Document::content_objects`].
    ///
    /// Added in 0.5.0.
    pub fn hit_test_rect(&self, index: u32, rect: &RectPt) -> Result<Vec<ObjectRef>> {
        let ir = self.content_ir(index)?;
        Ok(names(&ir, ir.hit_test_rect(rect)))
    }

    /// The runs of text page `index` (0-based) shows, in the order the page draws them: each
    /// with its name for an [`EditText`](crate::EditText), its text, box, font, line and, when
    /// it can't be edited, why. A run is the glyphs one show operator draws in one font and size
    /// on one baseline (and one marked-content sequence on a tagged page). Text inside a form
    /// isn't listed.
    ///
    /// # Errors
    /// As [`Document::content_objects`]; errors reading the page's fonts.
    ///
    /// Added in 0.5.0.
    pub fn text_runs(&self, index: u32) -> Result<Vec<ContentTextRun>> {
        let doc = self.ops();
        let page = {
            let pages = doc.pages()?;
            usize::try_from(index)
                .ok()
                .and_then(|i| pages.get(i))
                .map(|p| p.id().clone())
                .ok_or(Error::PageIndexOutOfRange {
                    index,
                    count: u32::try_from(pages.len()).unwrap_or(u32::MAX),
                })?
        };
        Ok(laminapdf_ops::text_runs(doc, &page)?)
    }

    /// What [`EditText`] would do, without doing it: the characters the run's font can't draw,
    /// the font each would be drawn in and how much wider the line would grow. An editor shows
    /// this before it commits the edit, so a font is never changed silently. The fonts the op
    /// names (`fonts`) that are given by their bytes must have been added with
    /// [`Document::add_blob`].
    ///
    /// # Errors
    /// As applying the op: `unknown-page`, `content-object-stale`, `content-not-editable`,
    /// `missing-glyph`, `unknown-blob`.
    ///
    /// Added in 0.7.0.
    pub fn plan_edit_text(&self, op: &EditText) -> Result<TextEditReport> {
        let doc = self.ops();
        Ok(laminapdf_ops::plan_edit_text(doc, op, doc.blobs())?)
    }

    /// Page `index`'s content IR, each object's state and box recorded.
    fn content_ir(&self, index: u32) -> Result<ContentOps> {
        let doc = self.ops();
        let pages = doc.pages()?;
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or(Error::PageIndexOutOfRange {
                index,
                count: u32::try_from(pages.len()).unwrap_or(u32::MAX),
            })?;
        let mut ir = ContentOps::read_page(doc, page.cos(), ContentOwner::Page(page.id().clone()))?;
        let mut recorder = ir.recorder(page.frame()?);
        // The extractor measures each text show's glyphs and passes the page's events on, so
        // text objects get their width.
        laminapdf_text::TextExtractor::new().page_with(doc, &page, &mut recorder)?;
        Ok(ir)
    }
}

fn names(ir: &ContentOps, ids: Vec<laminapdf_content::ContentObjectId>) -> Vec<ObjectRef> {
    ids.into_iter()
        .filter_map(|id| ir.reference(id))
        .map(|name| ObjectRef::from(&name))
        .collect()
}
