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

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use laminapdf_render::{
    ByteSource, CancelToken, RenderBackend, RenderDocument, RenderError, RenderLimits, RenderOpenOptions, RenderPage,
    RenderRequest, Rotation, SourceError, TilePlan, merge_ranges,
};
use pdfium_render::prelude::{
    PdfBitmap, PdfBitmapFormat, PdfColor, PdfDocument, PdfMatrix, PdfPage, PdfPageRenderRotation, PdfRenderConfig,
    Pdfium, PdfiumError, PdfiumInternalError,
};

const NAME: &str = "pdfium";

/// The pinned PDFium build (ADR 0004 §3): part of every golden's key.
pub const PDFIUM_BUILD: &str = "chromium/7881";

/// The environment variable naming the folder that holds the PDFium library.
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

    /// Binds the library in the folder named by `LAMINA_PDF_PDFIUM_DIR` (what
    /// `cargo xtask fetch-pdfium` prints).
    ///
    /// # Errors
    /// `render-backend-unavailable` when the variable is unset or the library can't be loaded.
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
            let int = |v: u32| i32::try_from(v).map_err(|_| failed("render"));
            let background = request
                .background
                .map_or(PdfColor::new(0, 0, 0, 0), |c| PdfColor::new(c.r, c.g, c.b, c.a));
            let config = PdfRenderConfig::new()
                .render_annotations(request.flags.annotations)
                .use_print_quality(request.flags.print)
                .use_grayscale_rendering(request.flags.grayscale)
                .set_reverse_byte_order(true)
                .clear_before_rendering(true)
                .set_clear_color(background);
            // Widgets are annotations: drawn only when annotations are (as hayro does).
            let config = if request.flags.annotations && request.flags.form_widgets {
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
                let m = plan.page_to_tile(&info, request.rotation);
                #[expect(clippy::cast_possible_truncation, reason = "PDFium's matrix is f32")]
                let matrix = PdfMatrix::new(
                    m[0] as f32,
                    m[1] as f32,
                    m[2] as f32,
                    m[3] as f32,
                    m[4] as f32,
                    m[5] as f32,
                );
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
                .map_err(|_| failed("render"))?;
            drop(bitmap);
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
