//! Linux: GTK 3 (which WebKitGTK already loads) for paths B and A, CUPS for path C.
//!
//! - B: `gtk::PrintOperation` with a page setup per page and a `draw-page` handler that paints a
//!   PDFium raster through cairo. Without a dialog it exports to PDF (`set_export_filename`).
//! - C: the original bytes to CUPS with `lp` (CUPS renders PDF itself), after the GTK print
//!   dialog when there is one. Without a dialog the job goes to the `cups-pdf` printer
//!   (`LAMINA_SPIKE_CUPS_PRINTER`, default `PDF`), whose output folder is
//!   `LAMINA_SPIKE_CUPS_OUT` (default `~/PDF`).
//! - A: WebKitGTK's `PrintOperation`: `print()` to GTK's "Print to File" printer without a
//!   dialog, `run_dialog()` with one.
//!
//! GTK must be called on the main thread: each call hops there and waits for the result.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{PageOrientation, PageSetup, PaperSize, PrintOperation, PrintOperationAction, PrintOperationResult, Unit};
use tauri::{AppHandle, Manager};
use webkit2gtk::PrintOperationExt as _;

use crate::{Fixture, pdf};

const RASTER_DPI: f32 = 300.0;

fn on_main<T: Send + 'static>(
    handle: &AppHandle,
    f: impl FnOnce(Sender<Result<T, String>>) + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = channel();
    handle.run_on_main_thread(move || f(tx)).map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

fn main_window(handle: &AppHandle) -> Result<gtk::ApplicationWindow, String> {
    handle
        .get_webview_window("main")
        .ok_or("no main window")?
        .gtk_window()
        .map_err(|e| e.to_string())
}

pub fn print_raster(handle: &AppHandle, fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let (bytes, sizes, out) = (fixture.bytes.clone(), fixture.sizes.clone(), out.map(Path::to_path_buf));
    let h = handle.clone();
    on_main(handle, move |tx| {
        let result = (|| {
            // GTK objects aren't Send: look the window up on the main thread.
            let window = main_window(&h)?;
            let doc = Rc::new(pdf::open(&bytes)?);
            let op = PrintOperation::new();
            op.set_n_pages(sizes.len() as i32);
            op.set_use_full_page(true);
            op.set_unit(Unit::Points);
            let setups = sizes.clone();
            op.connect_request_page_setup(move |_, _, page, setup: &PageSetup| {
                if let Some(s) = setups.get(page as usize) {
                    let (w, h) = (f64::from(s.w), f64::from(s.h));
                    // A custom paper of exactly the page's size, upright.
                    setup.set_paper_size(&PaperSize::new_custom("page", "page", w, h, Unit::Points));
                    setup.set_orientation(PageOrientation::Portrait);
                    setup.set_top_margin(0.0, Unit::Points);
                    setup.set_bottom_margin(0.0, Unit::Points);
                    setup.set_left_margin(0.0, Unit::Points);
                    setup.set_right_margin(0.0, Unit::Points);
                }
            });
            let draw_doc = Rc::clone(&doc);
            op.connect_draw_page(move |_, ctx, page| {
                let Ok(raster) = pdf::render(&draw_doc, page as u16, RASTER_DPI) else { return };
                let Some(cr) = ctx.cairo_context() else { return };
                let stride = (raster.width * 4) as i32;
                let Ok(surface) = gtk::cairo::ImageSurface::create_for_data(
                    raster.bgra(),
                    gtk::cairo::Format::ARgb32,
                    raster.width as i32,
                    raster.height as i32,
                    stride,
                ) else {
                    return;
                };
                // Points per pixel: the raster covers the page exactly.
                let scale = 72.0 / f64::from(RASTER_DPI);
                cr.save().ok();
                cr.scale(scale, scale);
                let _ = cr.set_source_surface(&surface, 0.0, 0.0);
                let _ = cr.paint();
                cr.restore().ok();
            });
            // Without a dialog, a whole document goes to PDF through GTK's Export, which skips
            // the print settings (it wrote all 50 pages for a 3-7 range). A page range goes
            // through GTK's print pipeline to the CUPS PDF printer instead, as a dialog print
            // to a real printer would; the job collects that printer's output. (Printing to
            // GTK's "Print to File" without a dialog wrote nothing.)
            let mut cups_out = None;
            let action = match (&out, crate::page_range()) {
                (Some(path), Some((first, last))) => {
                    let settings = gtk::PrintSettings::new();
                    settings.set_printer(&std::env::var("LAMINA_SPIKE_CUPS_PRINTER").unwrap_or_else(|_| "PDF".into()));
                    settings.set_print_pages(gtk::PrintPages::Ranges);
                    // GTK's ranges are 0-based and inclusive.
                    settings.set_page_ranges(&[gtk::PageRange::new(first as i32 - 1, last as i32 - 1)]);
                    op.set_print_settings(Some(&settings));
                    cups_out = Some(path.clone());
                    PrintOperationAction::Print
                }
                (Some(path), None) => {
                    op.set_export_filename(path);
                    PrintOperationAction::Export
                }
                (None, _) => PrintOperationAction::PrintDialog,
            };
            let since = std::time::SystemTime::now();
            match op.run(action, Some(&window)) {
                Ok(PrintOperationResult::Apply) => match cups_out {
                    Some(path) => collect_cups_output(&cups_dir(), since, &path),
                    None => Ok(()),
                },
                Ok(other) => Err(format!("print operation ended with {other:?}")),
                Err(e) => Err(e.to_string()),
            }
        })();
        let _ = tx.send(result);
    })
}

/// Where the `cups-pdf` printer writes (`LAMINA_SPIKE_CUPS_OUT`, default `~/PDF`).
fn cups_dir() -> PathBuf {
    std::env::var_os("LAMINA_SPIKE_CUPS_OUT").map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("PDF"),
        PathBuf::from,
    )
}

/// Waits for a new PDF in `dir` (newer than `since`) and moves it to `out`.
fn collect_cups_output(dir: &Path, since: std::time::SystemTime, out: &Path) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(300);
    while Instant::now() < deadline {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.filter_map(Result::ok) {
                let p = e.path();
                let fresh = e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t > since);
                if fresh && p.extension().is_some_and(|x| x == "pdf") {
                    // cups-pdf writes then closes; let the size settle before moving it.
                    crate::wait_for_file(&p, Duration::from_secs(120))?;
                    std::fs::rename(&p, out).or_else(|_| std::fs::copy(&p, out).map(|_| ())).map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(format!("no cups-pdf output in {}", dir.display()))
}

fn lp(printer: &str, title: &str, file: &Path, ranges: Option<&str>) -> Result<(), String> {
    let mut cmd = Command::new("lp");
    cmd.args(["-d", printer, "-t", title]);
    if let Some(r) = ranges {
        cmd.args(["-o", &format!("page-ranges={r}")]);
    }
    let status = cmd.arg(file).status().map_err(|e| format!("lp: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("lp exited with {status}")) }
}

pub fn print_vector(handle: &AppHandle, fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let title = format!("lamina-{}", fixture.name);
    match out {
        Some(out) => {
            let printer = std::env::var("LAMINA_SPIKE_CUPS_PRINTER").unwrap_or_else(|_| "PDF".into());
            let since = std::time::SystemTime::now();
            lp(&printer, &title, &fixture.path, None)?;
            collect_cups_output(&cups_dir(), since, out)
        }
        None => {
            // The GTK dialog picks the printer and pages; CUPS prints the original bytes.
            let h = handle.clone();
            let pages = fixture.sizes.len() as i32;
            let chosen = on_main(handle, move |tx| {
                let window = match main_window(&h) {
                    Ok(w) => w,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                // GTK's own dialog; once the user presses Print, `begin-print` reads the chosen
                // printer and page ranges and cancels GTK's job: CUPS prints the bytes instead.
                let op = PrintOperation::new();
                op.set_n_pages(pages);
                let picked = Rc::new(std::cell::RefCell::new(None));
                let store = Rc::clone(&picked);
                op.connect_begin_print(move |op, _| {
                    if let Some(settings) = op.print_settings() {
                        let printer = settings.printer().map(|p| p.to_string());
                        let ranges = settings.get(gtk::PRINT_SETTINGS_PAGE_RANGES).map(|r| r.to_string());
                        *store.borrow_mut() = printer.map(|p| (p, ranges));
                    }
                    op.cancel();
                });
                let _ = op.run(PrintOperationAction::PrintDialog, Some(&window));
                let result = picked.borrow_mut().take().ok_or_else(|| "the print dialog was cancelled".to_owned());
                let _ = tx.send(result);
            })?;
            lp(&chosen.0, &title, &fixture.path, chosen.1.as_deref())
        }
    }
}

pub fn print_webview(handle: &AppHandle, _fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let window = handle.get_webview_window("main").ok_or("no main window")?;
    let out = out.map(Path::to_path_buf);
    let (tx, rx) = channel::<Result<(), String>>();
    window
        .with_webview(move |webview| {
            let op = webkit2gtk::PrintOperation::new(&webview.inner());
            let done = tx.clone();
            op.connect_finished(move |_| {
                let _ = done.send(Ok(()));
            });
            let failed = tx.clone();
            op.connect_failed(move |_, e| {
                let _ = failed.send(Err(e.to_string()));
            });
            match &out {
                Some(path) => {
                    let settings = gtk::PrintSettings::new();
                    settings.set_printer("Print to File");
                    settings.set(gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));
                    let uri = format!("file://{}", path.display());
                    settings.set(gtk::PRINT_SETTINGS_OUTPUT_URI, Some(&uri));
                    op.set_print_settings(&settings);
                    op.print();
                }
                None => {
                    let parent = webview.inner().toplevel().and_then(|w| w.downcast::<gtk::Window>().ok());
                    let _ = op.run_dialog(parent.as_ref());
                }
            }
        })
        .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(900)).map_err(|_| "WebKitGTK never finished printing".to_owned())?
}
