//! The two traits every renderer implements (ADR 0004 §1).

use std::sync::Arc;

use lamina_cos::ByteSource;
use lamina_syntax::CancelToken;

use crate::error::RenderError;
use crate::request::{RenderLimits, RenderOpenOptions, RenderPage, RenderRequest, TilePlan};

/// A rendering engine: opens documents. Object-safe, so shells hold `Box<dyn RenderBackend>`.
///
/// A renderer only produces pixels: text, glyph quads, structure, annotation and form
/// geometry, links and outlines always come from lamina's model, never from the renderer
/// (ADR 0004 §1).
pub trait RenderBackend: Send {
    /// `"pdfium"` or `"hayro"`; with [`RenderBackend::version`], part of every golden's key.
    fn name(&self) -> &'static str;

    /// The engine's version (a crate version or a PDFium build tag).
    fn version(&self) -> &'static str;

    /// Opens a document from `source` (the same source lamina reads, never a second copy).
    ///
    /// # Errors
    /// `password-required`, `wrong-password`, `data-pending` (with the ranges to fetch),
    /// `source`, `render-failed` (the engine can't open the file),
    /// `render-backend-unavailable`.
    fn open(
        &self,
        source: Arc<dyn ByteSource>,
        options: &RenderOpenOptions,
    ) -> Result<Box<dyn RenderDocument>, RenderError>;
}

/// One open document. `Send` but not `Sync`: one call at a time on a document. A backend
/// whose engine isn't thread-safe (PDFium) also serialises calls across its documents.
pub trait RenderDocument: Send {
    /// The number of pages.
    fn page_count(&self) -> u32;

    /// Page `index`'s displayed size and `/Rotate` (ADR 0027: 0-based).
    ///
    /// # Errors
    /// `page-index-out-of-range`, `data-pending`, `render-failed`.
    fn page(&self, index: u32) -> Result<RenderPage, RenderError>;

    /// The limits this document was opened with.
    fn limits(&self) -> &RenderLimits;

    /// Validates `request` for this document: the tile geometry and the bytes it needs.
    ///
    /// # Errors
    /// As [`RenderDocument::page`], plus [`TilePlan::new`]'s.
    fn plan(&self, request: &RenderRequest) -> Result<TilePlan, RenderError> {
        TilePlan::new(&self.page(request.page)?, request, self.limits())
    }

    /// Renders `request` into `out`, which must hold exactly [`TilePlan::byte_len`] bytes, in
    /// straight-alpha RGBA8. Checks `cancel` before and after the engine call (a running
    /// single-threaded render can't see a cancel, ADR 0004 §1).
    ///
    /// # Errors
    /// As [`RenderDocument::plan`], plus `invalid-argument` for a wrong buffer size,
    /// `render-failed` and `cancelled`.
    fn render(&self, request: &RenderRequest, out: &mut [u8], cancel: &CancelToken) -> Result<TilePlan, RenderError>;
}
