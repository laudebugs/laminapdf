//! Images, shapes and pages of other PDFs placed on pages as page content (E6 plan §2.1), from
//! boxes in page space (ADR 0027): [`Document::image_at`], [`Document::page_at`] and
//! [`Document::shape_on`] give the edit that adds one, [`Document::text_at`] the edit that adds
//! text, [`Document::added_objects`] lists them, and [`Document::update_added`] and
//! [`Document::delete_added`] give the edits that change or remove one, before or after a save.
//!
//! Unlike a stamp or a comment, an added object is part of the page: it prints, every reader
//! draws it, and it can't be hidden or flattened.
//!
//! Page marks go on many pages at once: [`Document::watermark`] and [`Document::background`], the
//! running text of [`Document::header_footer`], and the numbers of [`Document::bates`]. They are
//! listed by [`Document::page_marks`], and changed and removed by [`Document::update_mark`] and
//! [`Document::delete_mark`].

use laminapdf_model::{Matrix, PdfDate, RectPt};
use laminapdf_ops::{
    AddBackground, AddContentObject, AddText, AddWatermark, AddedObject, AnnotColor, ApplyBates, ContentId,
    ContentObjectSpec, DeleteContentObject, DeletePageMark, MarkContent, PageMark, PageMarkInfo, PathSpec,
    SetHeaderFooter, Slot, Stroke, TextSpan, UpdateContentObject, UpdatePageMark,
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

    /// The edit that adds `runs` to page `page` (0-based) as `id`, laid out in `area`, a box in
    /// page space: in 12-point black Helvetica, left-aligned, wrapped at the box's width. A line
    /// feed in a run starts a paragraph. Set the font and its fallbacks, the size, the colour, the
    /// alignment and the tagging on the op with its `with_` methods before applying it; a font
    /// given by its bytes is added with [`Document::add_blob`] and named as `AnnotFont::Font`.
    ///
    /// ```
    /// # use laminapdf::{ContentId, Document, Op, OpenOptions, PdfDate, Pt, RectPt, TextSpan};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let area = RectPt::from_xywh(Pt::new(72.0), Pt::new(72.0), Pt::new(216.0), Pt::new(48.0));
    /// let id = ContentId::new("note").unwrap();
    /// let runs = vec![TextSpan::plain("Reviewed twice")];
    /// let note = doc.text_at(0, id, area, runs, "D:20261118120000Z".parse::<PdfDate>()?)?;
    /// doc.apply(vec![Op::from(note)])?;
    /// assert_eq!(doc.added_objects(0)?[0].bounds, area);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). A character no font has, a font
    /// that forbids embedding or isn't a font, and a font in a module without text layout fail
    /// when the edit is applied (`content-glyph-missing`, `content-font-not-embeddable`,
    /// `content-font-invalid`, `appearance-needs-text`).
    ///
    /// Added in 0.7.0.
    pub fn text_at(
        &self,
        page: u32,
        id: ContentId,
        area: RectPt,
        runs: Vec<TextSpan>,
        modified: PdfDate,
    ) -> Result<AddText> {
        let (page_id, _) = self.page_frame(page)?;
        Ok(AddText::new(id, page_id, area, runs, modified))
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

    /// The edit that puts a watermark on `pages` (0-based) as `id`, drawn in front of each page's
    /// content: text, or an image added with [`Document::add_blob`], centred on the page. Set its
    /// opacity, its angle and whether a reader's layers panel can hide it on the op with its
    /// `with_` methods before applying it.
    ///
    /// ```
    /// # use laminapdf::{ContentId, Document, MarkContent, Op, OpenOptions, PdfDate};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let text = MarkContent::Text { text: "DRAFT".into(), fonts: vec![], size: 96.0, color: None };
    /// let id = ContentId::new("draft").unwrap();
    /// let mark = doc.watermark(&[0], id, text, "D:20261118120000Z".parse::<PdfDate>()?)?.with_angle(45.0);
    /// doc.apply(vec![Op::from(mark)])?;
    /// assert_eq!(doc.page_marks(0)?.len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). An ID in use on a page, text no
    /// font has a glyph for, bytes that aren't an image and the like fail when the edit is applied
    /// (`content-id-in-use`, `content-glyph-missing`, `content-object-invalid`).
    ///
    /// Added in 0.7.0.
    pub fn watermark(
        &self,
        pages: &[u32],
        id: ContentId,
        mark: MarkContent,
        modified: PdfDate,
    ) -> Result<AddWatermark> {
        Ok(AddWatermark::new(id, self.page_ids(pages)?, mark, modified))
    }

    /// The edit that puts a background on `pages` (0-based) as `id`, drawn behind each page's
    /// content: text centred on the page, or an image that covers it.
    ///
    /// # Errors
    /// As [`Document::watermark`].
    ///
    /// Added in 0.7.0.
    pub fn background(
        &self,
        pages: &[u32],
        id: ContentId,
        mark: MarkContent,
        modified: PdfDate,
    ) -> Result<AddBackground> {
        Ok(AddBackground::new(id, self.page_ids(pages)?, mark, modified))
    }

    /// The edit that puts running text on `pages` (0-based) as `id`: up to six slots, three
    /// across the top and three across the bottom, in the order of [`Slot::ALL`]. A slot's text
    /// has the fields `{page}`, `{pages}`, `{label}`, `{date}` and `{file}`, filled when the edit
    /// is applied; set what `{date}` and `{file}` stand for, the font and the margins on the op
    /// with its `with_` methods first.
    ///
    /// ```
    /// # use laminapdf::{ContentId, Document, Op, OpenOptions, PdfDate};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let mut slots: [Option<String>; 6] = Default::default();
    /// slots[4] = Some("Page {page} of {pages}".into());
    /// let id = ContentId::new("folio").unwrap();
    /// let folio = doc.header_footer(&[0], id, slots, "D:20261118120000Z".parse::<PdfDate>()?)?;
    /// doc.apply(vec![Op::from(folio)])?;
    /// assert_eq!(doc.page_marks(0)?[0].id.as_str(), "folio");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// As [`Document::watermark`]; a header with no text fails when applied (`invalid-op`).
    ///
    /// Added in 0.7.0.
    pub fn header_footer(
        &self,
        pages: &[u32],
        id: ContentId,
        slots: [Option<String>; 6],
        modified: PdfDate,
    ) -> Result<SetHeaderFooter> {
        Ok(SetHeaderFooter::new(id, self.page_ids(pages)?, slots, modified))
    }

    /// The edit that numbers `pages` (0-based) from `start` in `slot`, in the order given, as
    /// `id`: six digits, no prefix or suffix until the op's [`ApplyBates::with_text`] says so.
    /// Numbering many files in turn is one edit on each, each with its own `start`.
    ///
    /// ```
    /// # use laminapdf::{ContentId, Document, Op, OpenOptions, PdfDate, Slot};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let mut doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let id = ContentId::new("bates").unwrap();
    /// let numbers = doc.bates(&[0], id, 41, Slot::BottomRight, "D:20261118120000Z".parse::<PdfDate>()?)?;
    /// doc.apply(vec![Op::from(numbers.with_text("ACME-", 6, ""))])?;
    /// assert_eq!(doc.page_marks(0)?[0].id.as_str(), "bates");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// As [`Document::watermark`].
    ///
    /// Added in 0.7.0.
    pub fn bates(&self, pages: &[u32], id: ContentId, start: u64, slot: Slot, modified: PdfDate) -> Result<ApplyBates> {
        Ok(ApplyBates::new(id, self.page_ids(pages)?, start, slot, modified))
    }

    /// The page marks Lamina PDF added to page `page` (0-based), in the order they paint: those
    /// behind the page's content first. Each has its ID, what it is and whether a reader can hide
    /// it.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors.
    ///
    /// Added in 0.7.0.
    pub fn page_marks(&self, page: u32) -> Result<Vec<PageMarkInfo>> {
        let (page_id, _) = self.page_frame(page)?;
        Ok(laminapdf_ops::page_marks(self.ops(), &page_id)?)
    }

    /// The edit that replaces the mark `id` on `pages` (0-based) with `mark`, of the same kind.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors. A page without the mark fails when the edit is
    /// applied (`unknown-page-mark`).
    ///
    /// Added in 0.7.0.
    pub fn update_mark(
        &self,
        pages: &[u32],
        id: ContentId,
        mark: PageMark,
        modified: PdfDate,
    ) -> Result<UpdatePageMark> {
        Ok(UpdatePageMark::new(id, self.page_ids(pages)?, mark, modified))
    }

    /// The edit that takes the mark `id` off `pages` (0-based).
    ///
    /// # Errors
    /// As [`Document::update_mark`].
    ///
    /// Added in 0.7.0.
    pub fn delete_mark(&self, pages: &[u32], id: ContentId) -> Result<DeletePageMark> {
        Ok(DeletePageMark::new(id, self.page_ids(pages)?))
    }

    /// The IDs of the pages at `pages` (0-based).
    fn page_ids(&self, pages: &[u32]) -> Result<Vec<laminapdf_model::PageId>> {
        pages.iter().map(|page| Ok(self.page_frame(*page)?.0)).collect()
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
