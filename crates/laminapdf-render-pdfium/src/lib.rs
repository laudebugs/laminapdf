//! The PDFium adapter of `laminapdf-render` ([ADR 0004] §2, §3, §5): native PDFium through
//! `pdfium-render`, bound **at runtime** to the pinned `bblanchon/pdfium-binaries`
//! `chromium/7881` build, so building and linting never need the library.
//!
//! - **Input:** PDFium reads Lamina PDF's [`ByteSource`] through its custom file access
//!   (`FPDF_LoadCustomDocument`), never a second copy. A read the source answers `Pending` for
//!   is remembered, and the call fails with `data-pending` and those ranges (a render that hit
//!   one is discarded: PDFium can't report a failed read mid-page).
//! - **Threads:** PDFium isn't thread-safe across documents. `pdfium-render`'s `thread_safe`
//!   feature serialises each FFI call, and this crate holds one process-wide lock for every
//!   `open`, `page` and `render`, so two documents on two threads render one after the other.
//! - **Tiles:** the page is laid out at its whole pixel size and shifted so only the tile lands
//!   in a tile-sized bitmap that wraps the caller's buffer (no copy, no full-page allocation).
//! - **Cancellation:** checked before and after the engine call. `pdfium-render` doesn't expose
//!   PDFium's progressive rendering, so a running render finishes (ADR 0004 §1 allows this).
//!
//! [ADR 0004]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0004-render-backend.md

mod fonts;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use laminapdf_render::{
    Annotated, ByteSource, CancelToken, Kept, Matrix, Quad, RenderBackend, RenderDocument, RenderError, RenderLimits,
    RenderOpenOptions, RenderPage, RenderRequest, Rotation, SourceError, TilePlan, merge_ranges, mostly_changed,
    recolour_tile,
};
use pdfium_render::prelude::{
    PdfBitmap, PdfBitmapFormat, PdfColor, PdfDocument, PdfMatrix, PdfPage, PdfPageAnnotationCommon as _, PdfPageObject,
    PdfPageObjectCommon as _, PdfPageObjectType, PdfPageObjectsCommon as _, PdfPageRenderRotation, PdfQuadPoints,
    PdfRenderConfig, Pdfium, PdfiumError, PdfiumInternalError,
};

pub use fonts::{FontCharset, FontQuery, SystemFont, SystemFontSource};

const NAME: &str = "pdfium";

/// The pinned PDFium build (ADR 0004 §3): part of every golden's key.
///
/// Added in 0.2.0.
pub const PDFIUM_BUILD: &str = "chromium/7881";

/// The environment variable naming the folder that holds the PDFium library.
///
/// Added in 0.2.0.
pub const PDFIUM_DIR_ENV: &str = "LAMINA_PDF_PDFIUM_DIR";

/// The bindings, created once per process: every document borrows them for `'static`.
static PDFIUM: OnceLock<Pdfium> = OnceLock::new();

/// Serialises every engine call of every document (PDFium's global state is shared).
static ENGINE: Mutex<()> = Mutex::new(());

fn engine() -> MutexGuard<'static, ()> {
    // A panic while holding the lock leaves nothing half-written on our side.
    ENGINE.lock().unwrap_or_else(PoisonError::into_inner)
}

const fn unavailable() -> RenderError {
    RenderError::Unavailable { backend: NAME }
}

const fn failed(stage: &'static str) -> RenderError {
    RenderError::Failed { backend: NAME, stage }
}

/// Native PDFium.
///
/// Added in 0.2.0.
#[derive(Clone, Copy)]
pub struct PdfiumBackend {
    pdfium: &'static Pdfium,
}

impl std::fmt::Debug for PdfiumBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfiumBackend").field("build", &PDFIUM_BUILD).finish()
    }
}

impl PdfiumBackend {
    /// Binds the PDFium library in `dir` (`libpdfium.dylib`, `libpdfium.so` or `pdfium.dll`).
    /// The first successful bind wins for the whole process; later calls return it whatever
    /// folder they name.
    ///
    /// # Errors
    /// `render-backend-unavailable` when the library can't be loaded.
    ///
    /// Added in 0.2.0.
    pub fn bind(dir: &Path) -> Result<Self, RenderError> {
        if let Some(pdfium) = PDFIUM.get() {
            return Ok(Self { pdfium });
        }
        let _guard = engine();
        if let Some(pdfium) = PDFIUM.get() {
            return Ok(Self { pdfium });
        }
        let bindings =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir)).map_err(|_| unavailable())?;
        let pdfium = PDFIUM.get_or_init(|| Pdfium::new(bindings));
        Ok(Self { pdfium })
    }

    /// [`PdfiumBackend::bind`], with PDFium's system fonts coming from `fonts` instead of its own
    /// scan of the font folders: for a process that can't open them. Only the first bind of the
    /// process sets the fonts.
    ///
    /// # Errors
    /// `render-backend-unavailable` when the library can't be loaded.
    ///
    /// Added in 0.2.0.
    pub fn bind_with_fonts(dir: &Path, fonts: Box<dyn SystemFontSource>) -> Result<Self, RenderError> {
        if let Some(pdfium) = PDFIUM.get() {
            return Ok(Self { pdfium });
        }
        let _guard = engine();
        if let Some(pdfium) = PDFIUM.get() {
            return Ok(Self { pdfium });
        }
        let bindings =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir)).map_err(|_| unavailable())?;
        let mut pdfium = Pdfium::new(bindings);
        pdfium.set_custom_font_provider(Box::new(fonts::Provider(fonts)));
        let pdfium = PDFIUM.get_or_init(|| pdfium);
        Ok(Self { pdfium })
    }

    /// Binds the library in the folder named by `LAMINA_PDF_PDFIUM_DIR` (what
    /// `cargo xtask fetch-pdfium` prints).
    ///
    /// # Errors
    /// `render-backend-unavailable` when the variable is unset or the library can't be loaded.
    ///
    /// Added in 0.2.0.
    pub fn from_env() -> Result<Self, RenderError> {
        let dir = std::env::var_os(PDFIUM_DIR_ENV).ok_or_else(unavailable)?;
        Self::bind(Path::new(&dir))
    }
}

impl RenderBackend for PdfiumBackend {
    fn name(&self) -> &'static str {
        NAME
    }

    fn version(&self) -> &'static str {
        PDFIUM_BUILD
    }

    fn open(
        &self,
        source: Arc<dyn ByteSource>,
        options: &RenderOpenOptions,
    ) -> Result<Box<dyn RenderDocument>, RenderError> {
        let pending = Arc::new(Mutex::new(Vec::new()));
        let reader = SourceReader {
            source,
            position: 0,
            pending: Arc::clone(&pending),
        };
        let _guard = engine();
        let doc = self
            .pdfium
            .load_pdf_from_reader(reader, options.password())
            .map_err(|e| {
                take_pending(&pending).unwrap_or_else(|| match e {
                    PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
                        if options.password().is_some() {
                            RenderError::WrongPassword
                        } else {
                            RenderError::PasswordRequired
                        }
                    }
                    _ => failed("open"),
                })
            })?;
        // PDFium rebuilds a damaged structure by scanning, so it may open after refused reads,
        // with the wrong pages: any refused read makes the open data-pending.
        let count = u32::try_from(doc.pages().len()).unwrap_or(0);
        if let Some(e) = take_pending(&pending) {
            drop(doc);
            return Err(e);
        }
        Ok(Box::new(PdfiumDocument {
            doc: Some(doc),
            count,
            pending,
            limits: *options.limits(),
            pages: RefCell::new(BTreeMap::new()),
            whole: RefCell::default(),
            alpha: RefCell::default(),
        }))
    }
}

/// A document open in PDFium. `doc` is only `None` while being dropped.
struct PdfiumDocument {
    doc: Option<PdfDocument<'static>>,
    count: u32,
    pending: Arc<Mutex<Vec<Refusal>>>,
    limits: RenderLimits,
    /// Page geometry already read: `plan` then `render` loads the page once. The document never
    /// changes, so entries never go stale. A `RefCell`: the document is `Send + !Sync`.
    pages: RefCell<BTreeMap<u32, RenderPage>>,
    /// Per page, which annotations keep their whole rectangle when recoloured
    /// ([`mostly_changed`], decided once on a small drawing of the page).
    whole: RefCell<BTreeMap<u32, Vec<bool>>>,
    /// Whether an image has transparent samples, by page and the image's place in the page's
    /// objects: decoding it once is enough.
    alpha: RefCell<BTreeMap<(u32, usize), bool>>,
}

impl PdfiumDocument {
    fn index(&self, index: u32) -> Result<i32, RenderError> {
        if index < self.count {
            i32::try_from(index).map_err(|_| self.out_of_range(index))
        } else {
            Err(self.out_of_range(index))
        }
    }

    const fn out_of_range(&self, index: u32) -> RenderError {
        RenderError::PageIndexOutOfRange {
            index,
            count: self.count,
        }
    }

    fn doc(&self) -> Result<&PdfDocument<'static>, RenderError> {
        self.doc.as_ref().ok_or_else(|| failed("render"))
    }

    /// Runs `f` under the engine lock with the refusals cleared; any read the source refused
    /// during `f` turns the outcome into `source` or `data-pending`, success or not (PDFium falls back to
    /// defaults, such as a 612 × 792 page, when it can't read inherited values).
    fn with_engine<T>(&self, f: impl FnOnce() -> Result<T, RenderError>) -> Result<T, RenderError> {
        let _guard = engine();
        clear_pending(&self.pending);
        let outcome = f();
        match take_pending(&self.pending) {
            Some(pending) => Err(pending),
            None => outcome,
        }
    }

    /// Which of `page`'s annotations keep their whole rectangle, from a drawing of the whole
    /// page at most [`DECISION_SIDE`] pixels on a side; cached per page.
    fn whole_rects(
        &self,
        page: &PdfPage<'_>,
        request: &RenderRequest,
        info: &RenderPage,
    ) -> Result<Vec<bool>, RenderError> {
        if let Some(flags) = self.whole.borrow().get(&request.page) {
            return Ok(flags.clone());
        }
        let longest = info.width.max(info.height).max(1.0);
        let small = RenderRequest::page(request.page, DECISION_SIDE / longest)
            .with_flags(request.flags)
            .with_background(request.background);
        let plan = TilePlan::new(info, &small, &self.limits)?;
        let mut without = vec![0; plan.byte_len()];
        let mut annotated = vec![0; plan.byte_len()];
        draw(page, &small, &plan, info, false, &mut without)?;
        draw(page, &small, &plan, info, true, &mut annotated)?;
        let rects = annotation_rects(page, &user_to_tile(page, &plan, info, Rotation::None));
        let flags = mostly_changed(&rects, &without, &annotated, plan.tile.width);
        self.whole.borrow_mut().insert(request.page, flags.clone());
        Ok(flags)
    }

    fn load(&self, index: u32) -> Result<PdfPage<'_>, RenderError> {
        let at = self.index(index)?;
        self.doc()?.pages().get(at).map_err(|_| failed("render"))
    }
}

impl Drop for PdfiumDocument {
    fn drop(&mut self) {
        // Closing the document is engine work too: under the same lock as every other call.
        let _guard = engine();
        drop(self.doc.take());
    }
}

/// A loaded page's displayed size and `/Rotate`.
fn geometry(page: &PdfPage<'_>) -> RenderPage {
    RenderPage {
        width: page.width().value,
        height: page.height().value,
        rotate: match page.rotation() {
            Ok(PdfPageRenderRotation::Degrees90) => Rotation::Clockwise90,
            Ok(PdfPageRenderRotation::Degrees180) => Rotation::Half,
            Ok(PdfPageRenderRotation::Degrees270) => Rotation::Clockwise270,
            Ok(PdfPageRenderRotation::None) | Err(_) => Rotation::None,
        },
    }
}

impl RenderDocument for PdfiumDocument {
    fn page_count(&self) -> u32 {
        self.count
    }

    fn page(&self, index: u32) -> Result<RenderPage, RenderError> {
        if let Some(page) = self.pages.borrow().get(&index) {
            return Ok(*page);
        }
        let page = self.with_engine(|| self.load(index).map(|page| geometry(&page)))?;
        self.pages.borrow_mut().insert(index, page);
        Ok(page)
    }

    fn limits(&self) -> &RenderLimits {
        &self.limits
    }

    fn render(&self, request: &RenderRequest, out: &mut [u8], cancel: &CancelToken) -> Result<TilePlan, RenderError> {
        let (plan, info) = self.with_engine(|| {
            // One page load for the geometry and the drawing.
            let page = self.load(request.page)?;
            let info = geometry(&page);
            let plan = TilePlan::new(&info, request, &self.limits)?;
            plan.check_buffer(out)?;
            if cancel.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            if request.appearance.recolours() {
                // Recoloured: the content alone, then (when the page has any) the annotations
                // drawn again over it, whose pixels keep their colours; images keep theirs too.
                draw(&page, request, &plan, &info, false, out)?;
                let user_to_tile = user_to_tile(&page, &plan, &info, request.rotation);
                let rects = if request.flags.annotations {
                    annotation_rects(&page, &user_to_tile)
                } else {
                    Vec::new()
                };
                let (with, whole) = if rects.is_empty() {
                    (None, Vec::new())
                } else {
                    let mut with = vec![0; out.len()];
                    draw(&page, request, &plan, &info, true, &mut with)?;
                    let flags = self.whole_rects(&page, request, &info)?;
                    let whole = rects.iter().zip(flags).filter(|(_, w)| *w).map(|(q, _)| *q).collect();
                    (Some(with), whole)
                };
                let mut images = Images {
                    doc: self.doc()?,
                    page: request.page,
                    tile: (plan.tile.width, plan.tile.height),
                    alpha: &self.alpha,
                    seen: 0,
                    opaque: Vec::new(),
                    see_through: Vec::new(),
                };
                for object in page.objects().iter() {
                    images.add(&object, &user_to_tile);
                }
                let kept = Kept {
                    images: &images.opaque,
                    see_through: &images.see_through,
                    background: request.background,
                    annotated: with.as_deref().map(|pixels| Annotated { pixels, whole: &whole }),
                };
                recolour_tile(request.appearance, plan.tile.width, out, &kept);
            } else {
                draw(&page, request, &plan, &info, request.flags.annotations, out)?;
            }
            if cancel.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            Ok((plan, info))
        })?;
        // Only geometry read without a refused read is cached (with_engine succeeded).
        self.pages.borrow_mut().insert(request.page, info);
        Ok(plan)
    }
}

/// Draws the request's tile of `page` into `out`, with or without annotations (and widgets).
fn draw(
    page: &PdfPage<'_>,
    request: &RenderRequest,
    plan: &TilePlan,
    info: &RenderPage,
    annotations: bool,
    out: &mut [u8],
) -> Result<(), RenderError> {
    let int = |v: u32| i32::try_from(v).map_err(|_| failed("render"));
    let background = request
        .background
        .map_or(PdfColor::new(0, 0, 0, 0), |c| PdfColor::new(c.r, c.g, c.b, c.a));
    let config = PdfRenderConfig::new()
        .render_annotations(annotations)
        .use_print_quality(request.flags.print)
        .use_grayscale_rendering(request.flags.grayscale)
        .set_reverse_byte_order(true)
        .clear_before_rendering(true)
        .set_clear_color(background);
    // Widgets are annotations: drawn only when annotations are (as hayro does).
    let config = if annotations && request.flags.form_widgets {
        // pdfium-render's form path: FPDF_RenderPageBitmap then FPDF_FFLDraw at the whole
        // page's pixel size, shifted so the tile lands in the tile-sized bitmap.
        config
            .render_form_data(true)
            .set_fixed_size(int(plan.page_width)?, int(plan.page_height)?)
            .rotate(rotation(request.rotation), false)
            .set_origin(-int(plan.tile.x)?, -int(plan.tile.y)?)
    } else {
        // Without widgets pdfium-render uses FPDF_RenderPageBitmapWithMatrix, which ignores
        // the origin and the fixed size: give it the whole page-to-tile matrix (from
        // PDFium's displayed page space, the same space hayro draws from) and the tile's
        // own size, with no rotation or scale of its own.
        let m = plan.page_to_tile(info, request.rotation);
        #[expect(clippy::cast_possible_truncation, reason = "PDFium's matrix is f32")]
        let matrix = PdfMatrix::new(m.a as f32, m.b as f32, m.c as f32, m.d as f32, m.e as f32, m.f as f32);
        config
            .render_form_data(false)
            .set_fixed_size(int(plan.tile.width)?, int(plan.tile.height)?)
            .apply_matrix(matrix)
            .map_err(|_| failed("render"))?
    };
    let mut bitmap = PdfBitmap::from_bytes(
        int(plan.tile.width)?,
        int(plan.tile.height)?,
        PdfBitmapFormat::BGRA,
        out,
    )
    .map_err(|_| failed("render"))?;
    page.render_into_bitmap_with_config(&mut bitmap, &config)
        .map_err(|_| failed("render"))
}

/// From PDFium's user space (its crop box, y up, `/Rotate` not applied) to the tile's pixels:
/// into the displayed page space `page_to_tile` starts from, then `page_to_tile`.
fn user_to_tile(page: &PdfPage<'_>, plan: &TilePlan, info: &RenderPage, view: Rotation) -> Matrix {
    let crop = page
        .boundaries()
        .crop()
        .or_else(|_| page.boundaries().media())
        .map(|b| b.bounds);
    let (x0, y0, x1, y1) = crop.map_or((0.0, 0.0, 0.0, 0.0), |r| {
        (
            f64::from(r.left().value),
            f64::from(r.bottom().value),
            f64::from(r.right().value),
            f64::from(r.top().value),
        )
    });
    // The displayed page has its origin top left, y down, after the page's own /Rotate.
    let to_page = Matrix::new(match info.rotate {
        Rotation::None => [1.0, 0.0, 0.0, -1.0, -x0, y1],
        Rotation::Clockwise90 => [0.0, 1.0, 1.0, 0.0, -y0, -x0],
        Rotation::Half => [-1.0, 0.0, 0.0, 1.0, x1, -y0],
        Rotation::Clockwise270 => [0.0, -1.0, -1.0, 0.0, y1, x1],
    });
    to_page.then(&plan.page_to_tile(info, view))
}

/// `quad` (user space) in tile pixels.
fn quad_in_tile(quad: &PdfQuadPoints, to_tile: &Matrix) -> Quad {
    let [xx, xy, yx, yy, dx, dy] = to_tile.to_array();
    let at = |px: f32, py: f32| {
        let (px, py) = (f64::from(px), f64::from(py));
        [xx * px + yx * py + dx, xy * px + yy * py + dy]
    };
    Quad([
        at(quad.x1().value, quad.y1().value),
        at(quad.x2().value, quad.y2().value),
        at(quad.x3().value, quad.y3().value),
        at(quad.x4().value, quad.y4().value),
    ])
}

/// Where each annotation's rectangle lands in the tile.
fn annotation_rects(page: &PdfPage<'_>, to_tile: &Matrix) -> Vec<Quad> {
    page.annotations()
        .iter()
        .filter_map(|annotation| annotation.bounds().ok())
        .map(|rect| quad_in_tile(&PdfQuadPoints::from_rect(&rect), to_tile))
        .collect()
}

/// The longest side of the page drawing [`mostly_changed`] decides on.
const DECISION_SIDE: f32 = 256.0;

/// The images of a page that touch the tile, in tile pixels, sorted by whether the page shows
/// through them.
struct Images<'d, 'a> {
    doc: &'d PdfDocument<'a>,
    page: u32,
    /// The tile's width and height.
    tile: (u32, u32),
    /// The document's alpha cache.
    alpha: &'d RefCell<BTreeMap<(u32, usize), bool>>,
    /// Images visited so far: each one's place in the page, for the cache.
    seen: usize,
    opaque: Vec<Quad>,
    see_through: Vec<Quad>,
}

impl Images<'_, '_> {
    /// Adds `object` if it is an image in the tile, or the images of a form (whose children's
    /// bounds are in the form's space, so its matrix is applied first).
    fn add(&mut self, object: &PdfPageObject<'_>, to_tile: &Matrix) {
        match object.object_type() {
            PdfPageObjectType::Image => {
                let place = self.seen;
                self.seen += 1;
                let Ok(bounds) = object.bounds() else { return };
                let quad = quad_in_tile(&bounds, to_tile);
                if !quad.touches(self.tile.0, self.tile.1) {
                    return;
                }
                if object.has_transparency() || self.has_alpha(object, place) {
                    self.see_through.push(quad);
                } else {
                    self.opaque.push(quad);
                }
            }
            PdfPageObjectType::XObjectForm => {
                if let Some(form) = object.as_x_object_form_object() {
                    let inner = form.matrix().map_or(*to_tile, |m| {
                        let form_matrix = Matrix::new([
                            f64::from(m.a()),
                            f64::from(m.b()),
                            f64::from(m.c()),
                            f64::from(m.d()),
                            f64::from(m.e()),
                            f64::from(m.f()),
                        ]);
                        form_matrix.then(to_tile)
                    });
                    for child in form.iter() {
                        self.add(&child, &inner);
                    }
                }
            }
            _ => {}
        }
    }

    /// [`has_alpha`], once per image.
    fn has_alpha(&self, object: &PdfPageObject<'_>, place: usize) -> bool {
        let key = (self.page, place);
        if let Some(&known) = self.alpha.borrow().get(&key) {
            return known;
        }
        let found = has_alpha(object, self.doc);
        self.alpha.borrow_mut().insert(key, found);
        found
    }
}

/// Images up to this many pixels are decoded to see whether they have transparent samples.
const ALPHA_PROBE_PIXELS: u64 = 1 << 20;

/// Whether an image's own `/SMask` or `/Mask` leaves some samples transparent, which
/// `has_transparency` (the drawing state's blend, alpha and soft mask) doesn't say. Larger images
/// are taken as opaque: decoding them costs more than a tile, and they are nearly always photos.
fn has_alpha(object: &PdfPageObject<'_>, doc: &PdfDocument<'_>) -> bool {
    let Some(image) = object.as_image_object() else {
        return false;
    };
    let size = match (image.width(), image.height()) {
        (Ok(w), Ok(h)) => u64::from(w.unsigned_abs()) * u64::from(h.unsigned_abs()),
        _ => return false,
    };
    if size == 0 || size > ALPHA_PROBE_PIXELS {
        return false;
    }
    image.get_processed_bitmap(doc).is_ok_and(|bitmap| {
        matches!(bitmap.format(), Ok(PdfBitmapFormat::BGRA))
            && bitmap
                .as_raw_bytes()
                .as_chunks::<4>()
                .0
                .iter()
                .any(|[_, _, _, a]| *a < 255)
    })
}

const fn rotation(r: Rotation) -> PdfPageRenderRotation {
    match r {
        Rotation::None => PdfPageRenderRotation::None,
        Rotation::Clockwise90 => PdfPageRenderRotation::Degrees90,
        Rotation::Half => PdfPageRenderRotation::Degrees180,
        Rotation::Clockwise270 => PdfPageRenderRotation::Degrees270,
    }
}

fn lock_pending(pending: &Mutex<Vec<Refusal>>) -> MutexGuard<'_, Vec<Refusal>> {
    pending.lock().unwrap_or_else(PoisonError::into_inner)
}

fn clear_pending(pending: &Mutex<Vec<Refusal>>) {
    lock_pending(pending).clear();
}

/// A read the source refused: not there yet, or failed.
#[derive(Debug, Clone, Copy)]
enum Refusal {
    Pending(u64, u64),
    Failed(u64, u64),
}

/// The error for the reads refused during a call, if any: a failed read is `source` (the
/// first one); otherwise every pending range, merged, is `data-pending`.
fn take_pending(pending: &Mutex<Vec<Refusal>>) -> Option<RenderError> {
    let refusals = std::mem::take(&mut *lock_pending(pending));
    if let Some((offset, len)) = refusals.iter().find_map(|r| match *r {
        Refusal::Failed(o, l) => Some((o, l)),
        Refusal::Pending(..) => None,
    }) {
        return Some(RenderError::Source { offset, len });
    }
    let ranges: Vec<(u64, u64)> = refusals
        .iter()
        .filter_map(|r| match *r {
            Refusal::Pending(o, l) => Some((o, l)),
            Refusal::Failed(..) => None,
        })
        .collect();
    (!ranges.is_empty()).then(|| RenderError::DataPending {
        ranges: merge_ranges(ranges),
    })
}

/// Lamina PDF's [`ByteSource`] as the `Read + Seek` PDFium's custom file access reads through.
struct SourceReader {
    source: Arc<dyn ByteSource>,
    position: u64,
    pending: Arc<Mutex<Vec<Refusal>>>,
}

impl Read for SourceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.source.len() {
            return Ok(0);
        }
        // pdfium-render treats any non-zero read as the whole block: fill `buf` (a source may
        // return short reads, ByteSource docs) up to the end of the input.
        let mut done = 0;
        while let Some(rest) = buf.get_mut(done..).filter(|r| !r.is_empty()) {
            let at = self.position + done as u64;
            if at >= self.source.len() {
                break;
            }
            match self.source.read_at(at, rest) {
                Ok(0) => break,
                Ok(n) => done += n,
                Err(e) => {
                    self.position = at;
                    return Err(self.refused(&e, at));
                }
            }
        }
        self.position += done as u64;
        Ok(done)
    }
}

impl SourceReader {
    /// Records a pending range and turns the source error into an I/O error for PDFium.
    fn refused(&self, e: &SourceError, at: u64) -> io::Error {
        let refusal = match *e {
            SourceError::Pending { offset, len } => Refusal::Pending(offset, len),
            SourceError::Failed { offset, len } => Refusal::Failed(offset, len),
            // SourceError is non_exhaustive: an unknown refusal is a failed read here.
            _ => Refusal::Failed(at, 0),
        };
        lock_pending(&self.pending).push(refusal);
        io::Error::other("source refused a read")
    }
}

impl Seek for SourceReader {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let len = self.source.len();
        let target = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => len.checked_add_signed(d),
            SeekFrom::Current(d) => self.position.checked_add_signed(d),
        };
        let target = target.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek out of range"))?;
        self.position = target;
        Ok(target)
    }
}
