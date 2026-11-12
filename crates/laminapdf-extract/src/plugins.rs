//! Models as plug-ins ([ADR 0030] §6): the [`LayoutModel`] and [`OcrEngine`] traits, what they
//! read ([`PageRaster`]) and return ([`LayoutRegion`], [`OcrWord`]), and the request/provide
//! types ([`ModelRequest`], [`ModelResult`]) for hosts whose models are asynchronous.
//!
//! The core never renders and never runs a model. A caller either hands the extraction a model
//! and a [`PageRenderer`] ([`ModelSlot::Plugin`]), and the job calls them between pages, or
//! names the model it will run itself ([`ModelSlot::Host`]): the job then stops at each page
//! that needs it with [`ExtractJob::request`](crate::ExtractJob::request) set, and goes on once
//! [`ExtractJob::provide`](crate::ExtractJob::provide) hands it the result. Both paths go
//! through the same steps, so they give the same output for the same results.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use std::fmt;
use std::sync::Arc;

use laminapdf_model::{Pt, RectPt};
use laminapdf_syntax::CancelToken;

/// A rendered page, as a model reads it: straight (not premultiplied) RGBA, 8 bits per channel,
/// rows top to bottom, `width_px × 4` bytes per row, no padding.
///
/// Added in 0.2.0.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub struct PageRaster {
    /// Pixels across.
    pub width_px: u32,
    /// Pixels down.
    pub height_px: u32,
    /// Pixels per inch: 72 pixels per inch is one pixel per point.
    pub dpi: u32,
    /// `width_px × height_px × 4` bytes.
    pub pixels: Vec<u8>,
    /// The part of the page the image shows, in page space (ADR 0027): normally the whole
    /// displayed page, `(0, 0)` to its width and height.
    pub page_box: RectPt,
}

impl fmt::Debug for PageRaster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageRaster")
            .field("width_px", &self.width_px)
            .field("height_px", &self.height_px)
            .field("dpi", &self.dpi)
            .field("pixels", &format_args!("{} bytes", self.pixels.len()))
            .field("page_box", &self.page_box)
            .finish()
    }
}

impl PageRaster {
    /// A raster of `pixels` (straight RGBA rows) showing `page_box` at `dpi`.
    ///
    /// # Errors
    /// [`ModelError::BadRaster`] when `pixels` isn't `width_px × height_px × 4` bytes, a side
    /// is 0, `dpi` is 0, or `page_box` isn't finite.
    ///
    /// Added in 0.2.0.
    pub fn new(width_px: u32, height_px: u32, dpi: u32, pixels: Vec<u8>, page_box: RectPt) -> Result<Self, ModelError> {
        let expected = u64::from(width_px) * u64::from(height_px) * 4;
        if width_px == 0 || height_px == 0 || dpi == 0 || !page_box.is_finite() {
            return Err(ModelError::BadRaster);
        }
        if u64::try_from(pixels.len()).ok() != Some(expected) {
            return Err(ModelError::BadRaster);
        }
        Ok(Self {
            width_px,
            height_px,
            dpi,
            pixels,
            page_box,
        })
    }

    /// A box in the image's pixels (`x0, y0, x1, y1`, top-left origin) in page space, the
    /// coordinates [`LayoutRegion`] and [`OcrWord`] take.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn to_page(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
        let sx = self.page_box.width().get() / f64::from(self.width_px.max(1));
        let sy = self.page_box.height().get() / f64::from(self.height_px.max(1));
        let (left, top) = (self.page_box.x0.get(), self.page_box.y0.get());
        RectPt::new(
            Pt::new(left + x0 * sx),
            Pt::new(top + y0 * sy),
            Pt::new(left + x1 * sx),
            Pt::new(top + y1 * sy),
        )
    }
}

/// What a layout model says a region of the page is. Models' own label sets map onto these;
/// labels with no match here are [`RegionLabel::Other`].
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RegionLabel {
    /// Running text.
    Text,
    /// The document's title.
    Title,
    /// A section heading.
    SectionHeader,
    /// One item of a list.
    ListItem,
    /// A table.
    Table,
    /// A picture or figure.
    Picture,
    /// A caption.
    Caption,
    /// A footnote.
    Footnote,
    /// A formula.
    Formula,
    /// A running header.
    PageHeader,
    /// A running footer.
    PageFooter,
    /// Anything else (code, forms, an index…).
    Other,
}

/// A region a [`LayoutModel`] found.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LayoutRegion {
    /// What it is.
    pub label: RegionLabel,
    /// Where, in page space ([`PageRaster::to_page`] converts from pixels).
    pub bbox: RectPt,
    /// The model's confidence, 0 to 1.
    pub score: f64,
}

impl LayoutRegion {
    /// A region.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(label: RegionLabel, bbox: RectPt, score: f64) -> Self {
        Self { label, bbox, score }
    }
}

/// A word an [`OcrEngine`] read.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct OcrWord {
    /// Its text.
    pub text: String,
    /// Where, in page space.
    pub bbox: RectPt,
    /// The engine's confidence, 0 to 1.
    pub confidence: f64,
}

impl OcrWord {
    /// A word.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(text: impl Into<String>, bbox: RectPt, confidence: f64) -> Self {
        Self {
            text: text.into(),
            bbox,
            confidence,
        }
    }
}

/// Which model made a result, as the output's `generator.models` records it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ModelInfo {
    /// Its name, e.g. `docling-layout-heron`.
    pub name: String,
    /// Its version or revision.
    pub version: String,
    /// The SHA-256 of its weights, in lowercase hex.
    pub sha256: String,
    /// The SPDX licence of its weights.
    pub licence: String,
}

impl ModelInfo {
    /// A model's description.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        sha256: impl Into<String>,
        licence: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            sha256: sha256.into(),
            licence: licence.into(),
        }
    }
}

/// Why a model, or the renderer feeding it, gave no result.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ModelError {
    /// It saw the cancel token and stopped.
    #[error("the model was cancelled")]
    Cancelled,
    /// A raster's size, resolution or box is wrong.
    #[error("the page raster's size, resolution or box is invalid")]
    BadRaster,
    /// It failed; `reason` is the plug-in's own message, which never holds document text.
    #[error("the model failed: {reason}")]
    Failed {
        /// What went wrong.
        reason: String,
    },
}

/// A model that finds a page's regions: text, headings, tables, pictures, furniture.
///
/// Added in 0.2.0.
pub trait LayoutModel: Send + Sync {
    /// Which model this is.
    ///
    /// Added in 0.2.0.
    fn info(&self) -> ModelInfo;

    /// The regions on `page`, in page space. Check `cancel` between expensive stages.
    ///
    /// # Errors
    /// [`ModelError::Cancelled`] once `cancel` is cancelled; [`ModelError::Failed`] otherwise.
    ///
    /// Added in 0.2.0.
    fn detect(&self, page: &PageRaster, cancel: &CancelToken) -> Result<Vec<LayoutRegion>, ModelError>;
}

/// An engine that reads the words on a page image.
///
/// Added in 0.2.0.
pub trait OcrEngine: Send + Sync {
    /// Which engine this is.
    ///
    /// Added in 0.2.0.
    fn info(&self) -> ModelInfo;

    /// The words on `page`, in page space, reading in `langs` (BCP 47 tags; empty for the
    /// engine's default).
    ///
    /// # Errors
    /// [`ModelError::Cancelled`] once `cancel` is cancelled; [`ModelError::Failed`] otherwise.
    ///
    /// Added in 0.2.0.
    fn recognise(&self, page: &PageRaster, langs: &[String], cancel: &CancelToken) -> Result<Vec<OcrWord>, ModelError>;
}

/// Renders pages for the models a job runs itself. The core renders nothing; the caller
/// supplies this (from `laminapdf-render`, a platform API, a cache…).
///
/// Added in 0.2.0.
pub trait PageRenderer: Send + Sync {
    /// Page `index` (0-based, ADR 0027) at `dpi`, the whole displayed page.
    ///
    /// # Errors
    /// [`ModelError::Cancelled`] once `cancel` is cancelled; [`ModelError::Failed`] otherwise.
    ///
    /// Added in 0.2.0.
    fn render(&self, index: usize, dpi: u32, cancel: &CancelToken) -> Result<PageRaster, ModelError>;
}

/// Where a model's results come from.
///
/// Added in 0.2.0.
#[non_exhaustive]
pub enum ModelSlot<M: ?Sized> {
    /// The job runs this model itself, on rasters from [`Models::renderer`].
    Plugin(Arc<M>),
    /// The host runs the model this names: the job asks for each result
    /// ([`ExtractJob::request`](crate::ExtractJob::request)) and waits for it.
    Host(ModelInfo),
}

impl<M: ?Sized> Clone for ModelSlot<M> {
    fn clone(&self) -> Self {
        match self {
            Self::Plugin(model) => Self::Plugin(Arc::clone(model)),
            Self::Host(info) => Self::Host(info.clone()),
        }
    }
}

/// Two slots are equal when they hold the same model object, or name the same model.
impl<M: ?Sized> PartialEq for ModelSlot<M> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Plugin(a), Self::Plugin(b)) => std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b)),
            (Self::Host(a), Self::Host(b)) => a == b,
            (Self::Plugin(_), Self::Host(_)) | (Self::Host(_), Self::Plugin(_)) => false,
        }
    }
}

impl<M: ?Sized> Eq for ModelSlot<M> {}

impl fmt::Debug for ModelSlot<dyn LayoutModel> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plugin(model) => f.debug_tuple("Plugin").field(&model.info()).finish(),
            Self::Host(info) => f.debug_tuple("Host").field(info).finish(),
        }
    }
}

impl fmt::Debug for ModelSlot<dyn OcrEngine> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plugin(model) => f.debug_tuple("Plugin").field(&model.info()).finish(),
            Self::Host(info) => f.debug_tuple("Host").field(info).finish(),
        }
    }
}

impl ModelSlot<dyn LayoutModel> {
    /// The model it names.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn info(&self) -> ModelInfo {
        match self {
            Self::Plugin(model) => model.info(),
            Self::Host(info) => info.clone(),
        }
    }
}

impl ModelSlot<dyn OcrEngine> {
    /// The engine it names.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn info(&self) -> ModelInfo {
        match self {
            Self::Plugin(model) => model.info(),
            Self::Host(info) => info.clone(),
        }
    }
}

/// The models an extraction uses: none by default, so extraction is rules only.
///
/// Added in 0.2.0.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct Models {
    /// Finds every page's regions.
    pub layout: Option<ModelSlot<dyn LayoutModel>>,
    /// Reads the pages that have no visible text.
    pub ocr: Option<ModelSlot<dyn OcrEngine>>,
    /// Renders pages for [`ModelSlot::Plugin`] models; without one, their results are asked of
    /// the host as for [`ModelSlot::Host`].
    pub renderer: Option<Arc<dyn PageRenderer>>,
    /// The resolution pages are rendered at for the models; 0 means [`Models::DEFAULT_DPI`].
    pub dpi: u32,
    /// The languages OCR reads (BCP 47 tags); empty for the engine's default.
    pub langs: Vec<String>,
}

impl Models {
    /// The resolution models get unless told otherwise: enough for body text.
    ///
    /// Added in 0.2.0.
    pub const DEFAULT_DPI: u32 = 150;

    /// No models.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// With a layout model.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_layout(mut self, slot: ModelSlot<dyn LayoutModel>) -> Self {
        self.layout = Some(slot);
        self
    }

    /// With an OCR engine.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_ocr(mut self, slot: ModelSlot<dyn OcrEngine>) -> Self {
        self.ocr = Some(slot);
        self
    }

    /// With a renderer for the plug-in models.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_renderer(mut self, renderer: Arc<dyn PageRenderer>) -> Self {
        self.renderer = Some(renderer);
        self
    }

    /// Whether any model is set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.layout.is_some() || self.ocr.is_some()
    }

    /// The resolution rasters are asked for at.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn dpi(&self) -> u32 {
        if self.dpi == 0 { Self::DEFAULT_DPI } else { self.dpi }
    }

    /// Every model set, layout first, as `generator.models` lists them.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn infos(&self) -> Vec<ModelInfo> {
        let layout = self.layout.as_ref().map(ModelSlot::<dyn LayoutModel>::info);
        let ocr = self.ocr.as_ref().map(ModelSlot::<dyn OcrEngine>::info);
        layout.into_iter().chain(ocr).collect()
    }
}

impl PartialEq for Models {
    fn eq(&self, other: &Self) -> bool {
        let renderer = match (&self.renderer, &other.renderer) {
            (Some(a), Some(b)) => std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b)),
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
        };
        renderer
            && self.layout == other.layout
            && self.ocr == other.ocr
            && self.dpi == other.dpi
            && self.langs == other.langs
    }
}

impl Eq for Models {}

impl fmt::Debug for Models {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Models")
            .field("layout", &self.layout)
            .field("ocr", &self.ocr)
            .field("renderer", &self.renderer.as_ref().map(|_| "PageRenderer"))
            .field("dpi", &self.dpi)
            .field("langs", &self.langs)
            .finish()
    }
}

/// A result an extraction job waits for ([`ExtractJob::request`](crate::ExtractJob::request)).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelRequest {
    /// The layout model's regions on page `page`, rendered at `dpi`.
    Layout {
        /// The page index.
        page: usize,
        /// The resolution to render at.
        dpi: u32,
    },
    /// The OCR engine's words on page `page`, rendered at `dpi`, in `langs`.
    Ocr {
        /// The page index.
        page: usize,
        /// The resolution to render at.
        dpi: u32,
        /// The languages to read.
        langs: Vec<String>,
    },
}

impl ModelRequest {
    /// The page it is about.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn page(&self) -> usize {
        match self {
            Self::Layout { page, .. } | Self::Ocr { page, .. } => *page,
        }
    }
}

/// A result handed to a waiting job ([`ExtractJob::provide`](crate::ExtractJob::provide)).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ModelResult {
    /// For [`ModelRequest::Layout`].
    Layout(Vec<LayoutRegion>),
    /// For [`ModelRequest::Ocr`].
    Ocr(Vec<OcrWord>),
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{ModelError, PageRaster};

    fn page(w: f64, h: f64) -> RectPt {
        RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(w), Pt::new(h))
    }

    #[test]
    fn a_raster_checks_its_size() {
        assert_eq!(
            PageRaster::new(2, 2, 72, vec![0; 15], page(2.0, 2.0)),
            Err(ModelError::BadRaster)
        );
        assert_eq!(
            PageRaster::new(0, 2, 72, Vec::new(), page(2.0, 2.0)),
            Err(ModelError::BadRaster)
        );
        assert!(PageRaster::new(2, 2, 72, vec![0; 16], page(2.0, 2.0)).is_ok());
    }

    #[test]
    fn pixels_map_to_page_space() -> Result<(), ModelError> {
        // 612 × 792 pt at 144 dpi: two pixels per point.
        let raster = PageRaster::new(1224, 1584, 144, vec![0; 1224 * 1584 * 4], page(612.0, 792.0))?;
        let rect = raster.to_page(100.0, 200.0, 300.0, 400.0);
        assert_eq!(
            rect,
            RectPt::new(Pt::new(50.0), Pt::new(100.0), Pt::new(150.0), Pt::new(200.0))
        );
        Ok(())
    }
}
