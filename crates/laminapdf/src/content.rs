//! The objects a page draws, for an editor to select and change (ADR 0022):
//! [`Document::content_objects`], [`Document::hit_test`] and [`Document::hit_test_rect`].
//! [`EditContent`](crate::EditContent) changes them.

use laminapdf_content::{ContentObjectKind, ContentOps, ContentOwner, NotEditable};
use laminapdf_model::{PointPt, RectPt};
use laminapdf_ops::{BlobId, ObjectRef};

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
    /// Keeps `bytes` for the session (a replacement image's JPEG or PNG) and returns their ID,
    /// which [`ContentEdit::ReplaceImage`](crate::ContentEdit::ReplaceImage) names them by: the
    /// SHA-256 of the bytes, so the same bytes get the same ID on every device and are kept
    /// once. Adding bytes changes nothing to save or undo.
    ///
    /// Added in 0.5.0.
    pub fn add_blob(&mut self, bytes: Vec<u8>) -> BlobId {
        self.inner.add_blob(bytes)
    }

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
        #[cfg(feature = "extract")]
        laminapdf_text::TextExtractor::new().page_with(doc, &page, &mut recorder)?;
        #[cfg(not(feature = "extract"))]
        laminapdf_content::interpret_page(doc, &page, &mut recorder)?;
        Ok(ir)
    }
}

fn names(ir: &ContentOps, ids: Vec<laminapdf_content::ContentObjectId>) -> Vec<ObjectRef> {
    ids.into_iter()
        .filter_map(|id| ir.reference(id))
        .map(|name| ObjectRef::from(&name))
        .collect()
}
