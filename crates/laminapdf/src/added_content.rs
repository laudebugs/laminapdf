//! Images, shapes and pages of other PDFs placed on pages as page content (E6 plan §2.1), from
//! boxes in page space (ADR 0027): [`Document::image_at`], [`Document::page_at`] and
//! [`Document::shape_on`] give the edit that adds one, [`Document::added_objects`] lists them, and
//! [`Document::update_added`] and [`Document::delete_added`] give the edits that change or remove
//! one, before or after a save.
//!
//! Unlike a stamp or a comment, an added object is part of the page: it prints, every reader
//! draws it, and it can't be hidden or flattened.

use laminapdf_model::{Matrix, PdfDate, RectPt};
use laminapdf_ops::{
    AddContentObject, AddedObject, AnnotColor, ContentId, ContentObjectSpec, DeleteContentObject, PathSpec, Stroke,
    UpdateContentObject,
};

use crate::document::Document;
use crate::error::Result;

impl Document {
    /// The edit that places the JPEG or PNG image `bytes` on page `page` (0-based) as `id`,
    /// stretched to `area`, a box in page space. The bytes are kept for the session as
    /// [`Document::add_blob`] keeps them; set its opacity, layer or alternate text on the op
    /// before applying it.
    ///
    /// ```
    /// # use laminapdf::{ContentId, Document, Op, OpenOptions, PdfDate, Pt, RectPt};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// # let logo = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/images/photo-rgb.jpg"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let area = RectPt::from_xywh(Pt::new(72.0), Pt::new(72.0), Pt::new(144.0), Pt::new(96.0));
    /// let id = ContentId::new("photo").unwrap();
    /// let place = doc.image_at(0, id, area, logo, "D:20261118120000Z".parse::<PdfDate>()?)?;
    /// doc.apply(vec![Op::from(place)])?;
    /// assert_eq!(doc.added_objects(0)?[0].bounds, area);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). Bytes that aren't a JPEG or
    /// PNG image fail when the edit is applied (`content-object-invalid`).
    ///
    /// Added in 0.7.0.
    pub fn image_at(
        &mut self,
        page: u32,
        id: ContentId,
        area: RectPt,
        bytes: Vec<u8>,
        modified: PdfDate,
    ) -> Result<AddContentObject> {
        let (page_id, _) = self.page_frame(page)?;
        let object = ContentObjectSpec::Image {
            blob: self.add_blob(bytes),
            crop: None,
        };
        Ok(AddContentObject::new(id, page_id, object, unit_to(area), modified))
    }

    /// The edit that places page `source_page` (0-based) of the PDF `pdf` on page `page` as
    /// `id`, stretched to `area`: a logo or a seal kept as vectors and text. The page is drawn as
    /// it shows, without its annotations. The bytes are kept for the session as
    /// [`Document::add_blob`] keeps them.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). A PDF that can't be opened or
    /// has no such page fails when the edit is applied (`content-object-invalid`).
    ///
    /// Added in 0.7.0.
    pub fn page_at(
        &mut self,
        page: u32,
        id: ContentId,
        area: RectPt,
        pdf: Vec<u8>,
        source_page: u32,
        modified: PdfDate,
    ) -> Result<AddContentObject> {
        let (page_id, _) = self.page_frame(page)?;
        let object = ContentObjectSpec::Form {
            blob: self.add_blob(pdf),
            page: source_page,
        };
        Ok(AddContentObject::new(id, page_id, object, unit_to(area), modified))
    }

    /// The edit that draws `path` on page `page` as `id`, its coordinates in page space, filled
    /// with `fill`, stroked with `stroke`, or both.
    ///
    /// ```
    /// # use laminapdf::{AnnotColor, ContentId, Document, Op, OpenOptions, PathSpec, PdfDate, Pt, RectPt, Stroke};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let rect = RectPt::from_xywh(Pt::new(72.0), Pt::new(72.0), Pt::new(200.0), Pt::new(100.0));
    /// let path = PathSpec::Rect { rect, radius: 8.0 };
    /// let stroke = Stroke::new(AnnotColor::Rgb(0.8, 0.1, 0.1), 2.0);
    /// let id = ContentId::new("box").unwrap();
    /// let draw = doc.shape_on(0, id, path, None, Some(stroke), "D:20261118120000Z".parse::<PdfDate>()?)?;
    /// doc.apply(vec![Op::from(draw)])?;
    /// assert_eq!(doc.added_objects(0)?.len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). A shape with neither a fill nor
    /// a stroke, or out of range, fails when the edit is applied (`invalid-op`).
    ///
    /// Added in 0.7.0.
    pub fn shape_on(
        &self,
        page: u32,
        id: ContentId,
        path: PathSpec,
        fill: Option<AnnotColor>,
        stroke: Option<Stroke>,
        modified: PdfDate,
    ) -> Result<AddContentObject> {
        let (page_id, _) = self.page_frame(page)?;
        let object = ContentObjectSpec::Shape { path, fill, stroke };
        let identity = Matrix::IDENTITY;
        Ok(AddContentObject::new(id, page_id, object, identity, modified))
    }

    /// The objects Lamina PDF added to page `page` (0-based), in the order they paint: those
    /// behind the page's content first. Each has its ID, what it is, where it is and its box in
    /// page space.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors.
    ///
    /// Added in 0.7.0.
    pub fn added_objects(&self, page: u32) -> Result<Vec<AddedObject>> {
        let (page_id, _) = self.page_frame(page)?;
        Ok(laminapdf_ops::added_objects(self.ops(), &page_id)?)
    }

    /// The edit that changes the object `id` added to page `page` (0-based), dated `modified`:
    /// set what changes on it with its `with_` methods before applying it.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). An ID the page draws no added
    /// object with fails when the edit is applied (`unknown-content-object`).
    ///
    /// Added in 0.7.0.
    pub fn update_added(&self, page: u32, id: ContentId, modified: PdfDate) -> Result<UpdateContentObject> {
        let (page_id, _) = self.page_frame(page)?;
        Ok(UpdateContentObject::new(id, page_id, modified))
    }

    /// The edit that takes the object `id` added to page `page` (0-based) off the page.
    ///
    /// # Errors
    /// As [`Document::update_added`].
    ///
    /// Added in 0.7.0.
    pub fn delete_added(&self, page: u32, id: ContentId) -> Result<DeleteContentObject> {
        let (page_id, _) = self.page_frame(page)?;
        Ok(DeleteContentObject::new(id, page_id))
    }
}

/// The matrix that stretches the unit square, an image's or a page's space, to `area`.
fn unit_to(area: RectPt) -> Matrix {
    Matrix::new([
        area.width().get(),
        0.0,
        0.0,
        area.height().get(),
        area.x0.get(),
        area.y0.get(),
    ])
}
