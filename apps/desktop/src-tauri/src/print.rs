//! File ▸ Print (ADR 0008 §9): the OS's print dialog, then every page rendered by this window's
//! sandboxed engine at print resolution and drawn into the OS's print job by
//! `laminapdf-desktop-os`. The main process never parses the PDF and never hands it to the OS:
//! it only moves checked pixels from the engine to the printer, one band at a time.
//!
//! On macOS and Windows the dialog runs on the main thread and the pages spool on a worker thread,
//! so the webview shows the progress ([`PROGRESS_EVENT`]) and its Cancel calls [`cancel_print`].
//! Linux prints from the main thread (GTK's rule), so the webview waits while a page is drawn, and
//! GTK shows its own progress window with Cancel.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use laminapdf_desktop_os::print::{Band, PixelOrder, PrintError, PrintJob, PrintOutcome, PrintPage, PrintSource};
use laminapdf_desktop_protocol::{DocId, ReqId, TileFormat};
use serde::Serialize;
use tauri::Emitter as _;

use crate::DesktopError;
use crate::engine::{BandQuery, Engines};

/// The resolution pages print at: the cap the desktop-print spike measured. Higher would only enlarge the
/// spool for no gain on most printers; the engine lowers it for a page too wide to render at it.
pub(crate) const PRINT_DPI: u16 = 300;

/// The event a window gets as each page is drawn: `{ done, total }`.
pub(crate) const PROGRESS_EVENT: &str = "print://progress";

/// How a print ended, for the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Printed {
    /// The pages went to the printer.
    Printed,
    /// The user closed the dialog or stopped the job.
    Cancelled,
}

/// How far a print has got.
#[derive(Debug, Clone, Copy, Serialize)]
struct Progress {
    done: usize,
    total: usize,
}

/// One window's running print: whether it was cancelled, and the band request in flight.
#[derive(Debug, Default)]
pub(crate) struct Cancel {
    cancelled: AtomicBool,
    running: Mutex<Option<ReqId>>,
}

impl Cancel {
    fn running(&self) -> MutexGuard<'_, Option<ReqId>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The prints running, one per window at most.
#[derive(Debug, Default)]
pub(crate) struct Printing(Mutex<BTreeMap<String, Arc<Cancel>>>);

impl Printing {
    fn jobs(&self) -> MutexGuard<'_, BTreeMap<String, Arc<Cancel>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts `window`'s print, refused while one runs there. The print runs until the returned
    /// guard drops.
    fn start(self: &Arc<Self>, window: &str) -> Result<Running, DesktopError> {
        let mut jobs = self.jobs();
        if jobs.contains_key(window) {
            return Err(DesktopError::AlreadyPrinting);
        }
        let cancel = Arc::new(Cancel::default());
        jobs.insert(window.to_owned(), Arc::clone(&cancel));
        Ok(Running {
            printing: Arc::clone(self),
            window: window.to_owned(),
            cancel,
        })
    }

    fn finish(&self, window: &str) {
        self.jobs().remove(window);
    }

    /// Cancels `window`'s print: no band after the one in flight, which the engine drops.
    pub(crate) fn cancel(&self, window: &str, engines: &Engines) {
        let Some(cancel) = self.jobs().get(window).cloned() else {
            return;
        };
        cancel.cancelled.store(true, Ordering::SeqCst);
        if let Some(req) = *cancel.running() {
            engines.cancel_request(window, req);
        }
    }
}

/// A window's document as a [`PrintSource`]: its pages' sizes, and its bands from the engine.
struct EngineSource {
    engines: Arc<Engines>,
    window: tauri::WebviewWindow,
    doc: DocId,
    pages: Vec<PrintPage>,
    cancel: Arc<Cancel>,
    done: usize,
    /// The pages the user chose, once the OS says (every page until then).
    total: usize,
}

impl PrintSource for EngineSource {
    fn start(&mut self, pages: usize) {
        self.total = pages;
        // The window's progress shows from the start, at 0 of the pages chosen.
        let progress = Progress { done: 0, total: pages };
        let _ = self.window.emit_to(self.window.label(), PROGRESS_EVENT, progress);
    }

    fn pages(&self) -> &[PrintPage] {
        &self.pages
    }

    fn page_bands(
        &mut self,
        page: usize,
        order: PixelOrder,
        draw: &mut dyn FnMut(&Band<'_>) -> Result<(), PrintError>,
    ) -> Result<(), PrintError> {
        let page_index = u32::try_from(page).map_err(|_| PrintError::PAGE_OUT_OF_RANGE)?;
        let format = match order {
            PixelOrder::Rgba => TileFormat::Rgba8,
            PixelOrder::Bgra => TileFormat::Bgra8,
        };
        let label = self.window.label().to_owned();
        let mut band = 0;
        loop {
            if self.cancel.cancelled.load(Ordering::SeqCst) {
                return Err(PrintError::Cancelled);
            }
            let query = BandQuery {
                doc: self.doc,
                page: page_index,
                dpi: PRINT_DPI,
                band,
                format,
            };
            let cancel = Arc::clone(&self.cancel);
            let got = self.engines.print_band(&label, query, &mut |req| {
                *cancel.running() = Some(req);
            });
            *self.cancel.running() = None;
            let got = got.map_err(|error| match error {
                DesktopError::Engine { kind: "cancelled", .. } => PrintError::Cancelled,
                other => PrintError::Source { kind: other.kind() },
            })?;
            draw(&Band {
                page_width: got.width,
                page_height: got.height,
                y: got.y,
                rows: got.rows,
                pixels: &got.pixels,
            })?;
            band += 1;
            if band >= got.bands {
                break;
            }
        }
        self.done += 1;
        let progress = Progress {
            done: self.done,
            total: self.total,
        };
        // The webview shows it on macOS and Windows; on Linux GTK's own window does.
        let _ = self.window.emit_to(self.window.label(), PROGRESS_EVENT, progress);
        Ok(())
    }
}

/// A window's print, from its start: finishes it when dropped, so a print whose command future was
/// dropped doesn't leave the window "printing already".
struct Running {
    printing: Arc<Printing>,
    window: String,
    cancel: Arc<Cancel>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.printing.finish(&self.window);
    }
}

/// Prints `doc` of `window`: the OS's dialog, then the pages. `job.output` prints to that PDF
/// with no dialog instead (the e2e hook).
pub(crate) async fn print(
    window: tauri::WebviewWindow,
    engines: Arc<Engines>,
    printing: Arc<Printing>,
    doc: DocId,
    job: PrintJob,
) -> Result<Printed, DesktopError> {
    // Frees the window's print however this ends, a dropped command included.
    let running = printing.start(window.label())?;
    let cancel = Arc::clone(&running.cancel);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let pages: Vec<PrintPage> = engines
            .page_info(window.label(), doc)?
            .iter()
            .map(|page| PrintPage {
                width: f64::from(page.width),
                height: f64::from(page.height),
            })
            .collect();
        let source = EngineSource {
            engines,
            window: window.clone(),
            doc,
            total: pages.len(),
            pages,
            cancel,
            done: 0,
        };
        run_os_print(&window, &job, source)
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))
    .and_then(|result| result);
    match result? {
        PrintOutcome::Printed { .. } => Ok(Printed::Printed),
        PrintOutcome::Cancelled => Ok(Printed::Cancelled),
    }
}

fn print_error(error: PrintError) -> DesktopError {
    match error {
        PrintError::Cancelled => DesktopError::Engine {
            kind: "cancelled",
            message: "the print was cancelled".into(),
        },
        PrintError::Source { kind } => DesktopError::Engine {
            kind,
            message: "a page couldn't be drawn for printing".into(),
        },
        other => DesktopError::Engine {
            kind: "print-refused",
            message: other.to_string(),
        },
    }
}

/// Runs `f` on the main thread and waits for its answer, from a thread that isn't the main one.
#[cfg(any(target_os = "macos", target_os = "linux", windows))]
fn on_main<T: Send + 'static>(
    window: &tauri::WebviewWindow,
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, DesktopError> {
    let (tx, rx) = std::sync::mpsc::channel();
    window.run_on_main_thread(move || {
        let _ = tx.send(f());
    })?;
    rx.recv()
        .map_err(|_| DesktopError::Internal("the main thread dropped the print".into()))
}

#[cfg(target_os = "linux")]
fn run_os_print(
    window: &tauri::WebviewWindow,
    job: &PrintJob,
    source: EngineSource,
) -> Result<PrintOutcome, DesktopError> {
    let (owner, job) = (window.clone(), job.clone());
    on_main(window, move || {
        let gtk = owner.gtk_window().ok();
        laminapdf_desktop_os::print::print(gtk.as_ref(), &job, Box::new(source))
    })?
    .map_err(print_error)
}

/// macOS and Windows: the dialog on the main thread, then the pages spooled on this one.
#[cfg(any(target_os = "macos", windows))]
fn run_os_print(
    window: &tauri::WebviewWindow,
    job: &PrintJob,
    mut source: EngineSource,
) -> Result<PrintOutcome, DesktopError> {
    use laminapdf_desktop_os::print::{choose, spool};
    let count = source.pages().len();
    let asked = job.clone();
    #[cfg(windows)]
    let owner = window.hwnd()?.0 as isize;
    #[cfg(windows)]
    let chosen = on_main(window, move || choose(owner, &asked, count))?;
    #[cfg(target_os = "macos")]
    let chosen = on_main(window, move || choose(&asked, count))?;
    let Some(chosen) = chosen.map_err(print_error)? else {
        return Ok(PrintOutcome::Cancelled);
    };
    spool(chosen, job, &mut source).map_err(print_error)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn run_os_print(_: &tauri::WebviewWindow, _: &PrintJob, _: EngineSource) -> Result<PrintOutcome, DesktopError> {
    Err(print_error(PrintError::refused("printing on this OS")))
}

/// Prints this window's document `doc` (File ▸ Print…, ⌘P): the OS's print dialog, then its
/// pages. `current_page` (0-based) is the page shown, for the dialog's "Current page".
#[tauri::command]
pub(crate) async fn print_document(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    printing: tauri::State<'_, Arc<Printing>>,
    doc: u32,
    current_page: Option<u32>,
) -> Result<Printed, DesktopError> {
    let title = engines
        .path_in(window.label(), DocId(doc))
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .ok_or(DesktopError::UnknownDocument)?;
    let job = PrintJob::new(title, current_page.and_then(|page| usize::try_from(page).ok()));
    print(
        window,
        engines.inner().clone(),
        printing.inner().clone(),
        DocId(doc),
        job,
    )
    .await
}

/// Stops this window's print: no more pages are drawn, and the job ends cancelled.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window and state by value"
)]
pub(crate) fn cancel_print(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    printing: tauri::State<'_, Arc<Printing>>,
) {
    printing.cancel(window.label(), &engines);
}

/// Prints `doc` to the PDF file `path` with no dialog, through the same pipeline, then reads the
/// file back in the engine and returns its pages' sizes in points (end-to-end tests only,
/// feature `e2e`).
#[cfg(feature = "e2e")]
#[tauri::command]
pub(crate) async fn e2e_print_to_file(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    printing: tauri::State<'_, Arc<Printing>>,
    doc: u32,
    path: String,
) -> Result<Vec<(f32, f32)>, DesktopError> {
    let path = std::path::PathBuf::from(path);
    let mut job = PrintJob::new("e2e".into(), None);
    job.output = Some(path.clone());
    let engines = engines.inner().clone();
    let label = window.label().to_owned();
    print(window, Arc::clone(&engines), printing.inner().clone(), DocId(doc), job).await?;
    tauri::async_runtime::spawn_blocking(move || {
        // The engine reads the printed file: the main process parses no PDF, even its own output.
        let printed = engines.open(&label, &path)?;
        let sizes = printed.pages.iter().map(|page| (page.width, page.height)).collect();
        engines.close(&label, DocId(printed.doc))?;
        Ok(sizes)
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))?
}
