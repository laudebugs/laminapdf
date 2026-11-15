//! macOS: `AppKit`'s print panel for the printer, the pages and the copies, then a Core Printing
//! session (`PMPrintSession`) that spools the pages with Core Graphics.
//!
//! Not `NSPrintOperation` over a view: it keeps one page format for a whole job, so the spike's
//! per-page paper changes snapped US Letter and A5 pages to A4 (and this crate's print test did
//! too). `PMSessionBeginPageNoDialog` takes a page format for each page, which is how a document
//! of mixed sizes prints on paper of each page's size. Each page gets the printer's own paper of
//! that size when it has one, else a custom paper with no margins, in the page's orientation; the
//! page's corner goes on the paper's corner (`PMGetAdjustedPaperRect`), and a printer's hardware
//! margins clip, as in any viewer.
//!
//! The panel runs on the main thread ([`choose`]); the session is the job's own, holding the
//! chosen printer and a copy of the chosen settings, so the spooling runs on any thread
//! ([`spool`]), and the webview shows the progress, as on Windows.
#![expect(
    unsafe_code,
    reason = "Core Printing's C API, AppKit's print panel and Core Graphics images"
)]

use std::ffi::c_void;
use std::ptr::{self, NonNull};

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSModalResponseOK, NSPrintInfo, NSPrintPanel, NSPrintPanelOptions};
use objc2_core_foundation::{CFData, CFRetained, CFString, CFURL, CFURLPathStyle};
use objc2_core_graphics::{
    CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGContext, CGDataProvider, CGImage, CGImageAlphaInfo,
};
use objc2_foundation::{NSCopying, NSPoint, NSRect, NSSize};

use super::{Band, PixelOrder, PrintError, PrintJob, PrintOutcome, PrintPage, PrintSource, band_edges, check_band};

/// Core Printing (`PrintCore`, part of `ApplicationServices`): the calls the job makes, with their
/// C signatures from `PMCore.h` and `PMPrintAPI.h`. Its objects are reference-counted pointers,
/// released with `PMRelease`.
mod pm {
    use std::ffi::c_void;

    /// `PMRect` and `PMPaperMargins`: top, left, bottom, right, in points.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    pub(super) struct Rect {
        pub top: f64,
        pub left: f64,
        pub bottom: f64,
        pub right: f64,
    }

    pub(super) type Object = *mut c_void;
    pub(super) type Status = i32;

    /// `kPMPortrait`, `kPMLandscape`.
    pub(super) const PORTRAIT: u16 = 1;
    pub(super) const LANDSCAPE: u16 = 2;
    /// `kPMDestinationFile`.
    pub(super) const DESTINATION_FILE: u16 = 2;
    /// `kPMPrintAllPages`: the last page when every page prints.
    pub(super) const ALL_PAGES: u32 = u32::MAX;
    /// `kPMCancel`: the user stopped the job.
    pub(super) const CANCEL: Status = 128;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        pub(super) fn PMCreateSession(session: *mut Object) -> Status;
        pub(super) fn PMCreatePrintSettings(settings: *mut Object) -> Status;
        pub(super) fn PMCopyPrintSettings(source: Object, dest: Object) -> Status;
        pub(super) fn PMRetain(object: Object) -> Status;
        pub(super) fn PMRelease(object: Object) -> Status;
        pub(super) fn PMSessionGetCurrentPrinter(session: Object, printer: *mut Object) -> Status;
        pub(super) fn PMSessionSetCurrentPMPrinter(session: Object, printer: Object) -> Status;
        pub(super) fn PMPrinterGetPaperList(printer: Object, list: *mut *const c_void) -> Status;
        pub(super) fn PMPaperGetWidth(paper: Object, width: *mut f64) -> Status;
        pub(super) fn PMPaperGetHeight(paper: Object, height: *mut f64) -> Status;
        pub(super) fn PMPaperCreateCustom(
            printer: Object,
            id: *const c_void,
            name: *const c_void,
            width: f64,
            height: f64,
            margins: *const Rect,
            paper: *mut Object,
        ) -> Status;
        pub(super) fn PMCreatePageFormatWithPMPaper(format: *mut Object, paper: Object) -> Status;
        pub(super) fn PMSetOrientation(format: Object, orientation: u16, lock: u8) -> Status;
        pub(super) fn PMSessionValidatePageFormat(session: Object, format: Object, changed: *mut u8) -> Status;
        pub(super) fn PMGetAdjustedPaperRect(format: Object, rect: *mut Rect) -> Status;
        pub(super) fn PMGetAdjustedPageRect(format: Object, rect: *mut Rect) -> Status;
        pub(super) fn PMGetFirstPage(settings: Object, first: *mut u32) -> Status;
        pub(super) fn PMGetLastPage(settings: Object, last: *mut u32) -> Status;
        pub(super) fn PMPrintSettingsSetJobName(settings: Object, name: *const c_void) -> Status;
        pub(super) fn PMSessionSetDestination(
            session: Object,
            settings: Object,
            destination: u16,
            format: *const c_void,
            location: *const c_void,
        ) -> Status;
        pub(super) fn PMSessionBeginCGDocumentNoDialog(session: Object, settings: Object, format: Object) -> Status;
        pub(super) fn PMSessionBeginPageNoDialog(session: Object, format: Object, frame: *const Rect) -> Status;
        pub(super) fn PMSessionGetCGGraphicsContext(session: Object, context: *mut Object) -> Status;
        pub(super) fn PMSessionEndPageNoDialog(session: Object) -> Status;
        pub(super) fn PMSessionEndDocumentNoDialog(session: Object) -> Status;
        pub(super) fn PMSessionError(session: Object) -> Status;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        pub(super) fn CFArrayGetCount(array: *const c_void) -> isize;
        pub(super) fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
    }
}

/// A Core Printing object this code created or retained, released once when dropped.
struct Owned(pm::Object);

impl Owned {
    /// Takes `object` from a `Create`/`Copy` call that answered `status`.
    fn created(status: pm::Status, object: pm::Object, what: &str) -> Result<Self, PrintError> {
        if status == 0 && !object.is_null() {
            Ok(Self(object))
        } else {
            Err(refused(what, status))
        }
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: an object this code created or retained, released once.
        unsafe {
            let _ = pm::PMRelease(self.0);
        }
    }
}

fn refused(what: &str, status: pm::Status) -> PrintError {
    PrintError::Refused(format!("{what} failed ({status})"))
}

fn check(status: pm::Status, what: &str) -> Result<(), PrintError> {
    if status == 0 {
        Ok(())
    } else {
        Err(refused(what, status))
    }
}

/// The printer, settings and pages the user chose. Owns its Core Printing objects, used by one
/// thread at a time (whoever holds it), so it may move to the spooling thread.
pub struct Chosen {
    printer: Owned,
    settings: Owned,
    pages: Vec<usize>,
}

impl std::fmt::Debug for Chosen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chosen")
            .field("pages", &self.pages)
            .finish_non_exhaustive()
    }
}

// SAFETY: Core Printing's objects are reference-counted and thread-safe; the `Chosen` is their
// only holder in this code, and is used from one thread at a time.
unsafe impl Send for Chosen {}

/// The printer and pages: `AppKit`'s print panel (printer, pages, copies), or, with `job.output`,
/// the default printer's settings and every page. Run it on the main thread.
///
/// # Errors
/// [`PrintError::NotMainThread`] off the main thread; [`PrintError::Refused`] when Core Printing
/// refuses the printer or its settings.
pub fn choose(job: &PrintJob, page_count: usize) -> Result<Option<Chosen>, PrintError> {
    let main = MainThreadMarker::new().ok_or(PrintError::NotMainThread)?;
    let info: Retained<NSPrintInfo> = NSPrintInfo::sharedPrintInfo().copy();
    if job.output.is_none() {
        let panel = NSPrintPanel::printPanel(main);
        panel.setOptions(NSPrintPanelOptions::ShowsCopies | NSPrintPanelOptions::ShowsPageRange);
        if panel.runModalWithPrintInfo(&info) != NSModalResponseOK {
            return Ok(None);
        }
    }
    // SAFETY: the info's own session and settings, live while `info` is; the printer is retained
    // and the settings copied, so the job owns what it keeps.
    unsafe {
        let mut printer = ptr::null_mut();
        check(
            pm::PMSessionGetCurrentPrinter(info.PMPrintSession().as_ptr(), &raw mut printer),
            "reading the printer",
        )?;
        check(pm::PMRetain(printer), "keeping the printer")?;
        let printer = Owned(printer);
        let mut settings = ptr::null_mut();
        let settings = Owned::created(
            pm::PMCreatePrintSettings(&raw mut settings),
            settings,
            "the print settings",
        )?;
        check(
            pm::PMCopyPrintSettings(info.PMPrintSettings().as_ptr(), settings.0),
            "copying the print settings",
        )?;
        let pages = chosen_pages(&settings, page_count, job.output.is_some());
        Ok(Some(Chosen {
            printer,
            settings,
            pages,
        }))
    }
}

/// The pages the settings' range names, 0-based; every page when printing to a file.
fn chosen_pages(settings: &Owned, page_count: usize, all: bool) -> Vec<usize> {
    let (mut first, mut last) = (1, pm::ALL_PAGES);
    if !all {
        // SAFETY: reads two numbers of live settings.
        unsafe {
            let _ = pm::PMGetFirstPage(settings.0, &raw mut first);
            let _ = pm::PMGetLastPage(settings.0, &raw mut last);
        }
    }
    let first = usize::try_from(first).unwrap_or(1).max(1) - 1;
    let last = usize::try_from(last).unwrap_or(usize::MAX).min(page_count);
    (first..last).collect()
}

/// The printer's own paper of `size` (portrait), when it has one, else a custom paper with no
/// margins.
fn paper_for(printer: &Owned, size: PrintPage) -> Result<Owned, PrintError> {
    let (short, long) = (size.width.min(size.height), size.width.max(size.height));
    // SAFETY: the paper list belongs to the printer (not retained: the Get rule); a paper is
    // retained before it's kept; the strings outlive the call that copies them.
    unsafe {
        let mut list = ptr::null();
        if pm::PMPrinterGetPaperList(printer.0, &raw mut list) == 0 && !list.is_null() {
            for i in 0..pm::CFArrayGetCount(list) {
                let paper = pm::CFArrayGetValueAtIndex(list, i).cast_mut();
                let (mut width, mut height) = (0.0, 0.0);
                if pm::PMPaperGetWidth(paper, &raw mut width) == 0
                    && pm::PMPaperGetHeight(paper, &raw mut height) == 0
                    && (width - short).abs() < 2.0
                    && (height - long).abs() < 2.0
                    && pm::PMRetain(paper) == 0
                {
                    return Ok(Owned(paper));
                }
            }
        }
        let id = CFString::from_str(&format!("lamina-{short:.0}x{long:.0}"));
        let name = CFString::from_str(&format!("{short:.0} × {long:.0} pt"));
        let margins = pm::Rect::default();
        let mut paper = ptr::null_mut();
        let status = pm::PMPaperCreateCustom(
            printer.0,
            cf(&id),
            cf(&name),
            short,
            long,
            &raw const margins,
            &raw mut paper,
        );
        Owned::created(status, paper, "making the page's paper")
    }
}

/// A Core Foundation object as the untyped pointer Core Printing's C calls take.
fn cf<T: ?Sized + objc2_core_foundation::Type>(object: &CFRetained<T>) -> *const c_void {
    CFRetained::as_ptr(object).as_ptr().cast_const().cast()
}

/// A page format for `size`: its paper in its orientation, validated for the session's printer.
fn format_for(session: &Owned, printer: &Owned, size: PrintPage) -> Result<Owned, PrintError> {
    let paper = paper_for(printer, size)?;
    // SAFETY: a format made from a live paper, oriented and validated against the live session.
    unsafe {
        let mut format = ptr::null_mut();
        let format = Owned::created(
            pm::PMCreatePageFormatWithPMPaper(&raw mut format, paper.0),
            format,
            "the page's format",
        )?;
        let orientation = if size.landscape() { pm::LANDSCAPE } else { pm::PORTRAIT };
        check(pm::PMSetOrientation(format.0, orientation, 0), "the page's orientation")?;
        let mut changed = 0;
        check(
            pm::PMSessionValidatePageFormat(session.0, format.0, &raw mut changed),
            "the page's format",
        )?;
        Ok(format)
    }
}

/// Draws one band of a page on `context`, whose origin is the printable area's bottom-left
/// corner: `paper` and `page` are the format's paper and printable rectangles (top-down, from the
/// printable area's top-left).
fn draw_band(context: &CGContext, paper: pm::Rect, page: pm::Rect, band: &Band<'_>) -> Result<(), PrintError> {
    check_band(band)?;
    let (width, height) = (paper.right - paper.left, paper.bottom - paper.top);
    let (top, bottom) = band_edges(band, height);
    let image = image_of(band).ok_or_else(|| PrintError::Refused("Core Graphics couldn't hold a band".into()))?;
    // From top-down paper coordinates to the context's bottom-up ones.
    let page_height = page.bottom - page.top;
    let rect = NSRect::new(
        NSPoint::new(paper.left, page_height - (paper.top + bottom)),
        NSSize::new(width, bottom - top),
    );
    CGContext::draw_image(Some(context), rect, Some(&image));
    Ok(())
}

/// An image over a copy of `band`'s RGBA pixels (opaque: the alpha byte is skipped).
fn image_of(band: &Band<'_>) -> Option<CFRetained<CGImage>> {
    let width = usize::try_from(band.page_width).ok()?;
    let rows = usize::try_from(band.rows).ok()?;
    let data = CFData::from_bytes(band.pixels);
    let provider = CGDataProvider::with_cf_data(Some(&data))?;
    let space = CGColorSpace::new_device_rgb()?;
    let info = CGBitmapInfo(CGImageAlphaInfo::NoneSkipLast.0);
    // SAFETY: the provider holds width × rows × 4 bytes (`check_band`), 8 bits a sample and 32 a
    // pixel; no decode array.
    unsafe {
        CGImage::new(
            width,
            rows,
            8,
            32,
            width.checked_mul(4)?,
            Some(&space),
            info,
            Some(&provider),
            ptr::null(),
            true,
            CGColorRenderingIntent::RenderingIntentDefault,
        )
    }
}

/// Spools the chosen pages of `source` to the chosen printer (or to `job.output` as a PDF). Any
/// thread: the session is the job's own.
///
/// # Errors
/// [`PrintError::Refused`] when Core Printing refuses the job, and the source's error when a page
/// couldn't be drawn; a cancel ends the job as [`PrintOutcome::Cancelled`].
#[expect(
    clippy::needless_pass_by_value,
    reason = "the chosen printer and settings are used up"
)]
pub fn spool(chosen: Chosen, job: &PrintJob, source: &mut dyn PrintSource) -> Result<PrintOutcome, PrintError> {
    let sizes = source.pages().to_vec();
    let Some(first) = chosen.pages.first().and_then(|&page| sizes.get(page).copied()) else {
        return Ok(PrintOutcome::Printed { pages: 0 });
    };
    // SAFETY: a new session, released when dropped.
    let session = unsafe {
        let mut session = ptr::null_mut();
        Owned::created(pm::PMCreateSession(&raw mut session), session, "the print session")?
    };
    begin(&session, &chosen, job)?;
    let first_format = format_for(&session, &chosen.printer, first)?;
    // SAFETY: begins the document once, with live settings and format; every exit below ends it.
    let begun = unsafe { pm::PMSessionBeginCGDocumentNoDialog(session.0, chosen.settings.0, first_format.0) };
    check(begun, "starting the job")?;
    let mut printed = 0;
    let mut failed = None;
    for &page in &chosen.pages {
        let Some(&size) = sizes.get(page) else {
            continue;
        };
        if let Err(error) = spool_page(&session, &chosen.printer, source, page, size) {
            failed = Some(error);
            break;
        }
        printed += 1;
    }
    // SAFETY: ends the document begun above, whatever happened to its pages; then reads the live
    // session's error.
    let (ended, error) = unsafe {
        (
            pm::PMSessionEndDocumentNoDialog(session.0),
            pm::PMSessionError(session.0),
        )
    };
    match failed {
        Some(PrintError::Cancelled) => Ok(PrintOutcome::Cancelled),
        Some(error) => Err(error),
        None if error == pm::CANCEL => Ok(PrintOutcome::Cancelled),
        None => {
            check(ended, "finishing the job")?;
            Ok(PrintOutcome::Printed { pages: printed })
        }
    }
}

/// Sets the session's printer, the job's name and, with `job.output`, the PDF file it goes to.
fn begin(session: &Owned, chosen: &Chosen, job: &PrintJob) -> Result<(), PrintError> {
    // SAFETY: the session, printer and settings are live; the strings and URL outlive the calls.
    unsafe {
        check(
            pm::PMSessionSetCurrentPMPrinter(session.0, chosen.printer.0),
            "choosing the printer",
        )?;
        let title = CFString::from_str(&job.title);
        let _ = pm::PMPrintSettingsSetJobName(chosen.settings.0, cf(&title));
        if let Some(path) = job.output.as_deref().and_then(std::path::Path::to_str) {
            let path = CFString::from_str(path);
            let url = CFURL::with_file_system_path(None, Some(&path), CFURLPathStyle::CFURLPOSIXPathStyle, false)
                .ok_or_else(|| PrintError::Refused("the output path".into()))?;
            let pdf = CFString::from_str("application/pdf");
            check(
                pm::PMSessionSetDestination(session.0, chosen.settings.0, pm::DESTINATION_FILE, cf(&pdf), cf(&url)),
                "printing to the file",
            )?;
        }
    }
    Ok(())
}

/// One page: its own format, then its bands on the session's context.
fn spool_page(
    session: &Owned,
    printer: &Owned,
    source: &mut dyn PrintSource,
    page: usize,
    size: PrintPage,
) -> Result<(), PrintError> {
    let format = format_for(session, printer, size)?;
    let (mut paper, mut printable) = (pm::Rect::default(), pm::Rect::default());
    // SAFETY: the page is begun and ended once, on the live session; the context is the session's
    // own, valid until the page ends, and only borrowed meanwhile.
    unsafe {
        check(pm::PMGetAdjustedPaperRect(format.0, &raw mut paper), "the paper's size")?;
        check(
            pm::PMGetAdjustedPageRect(format.0, &raw mut printable),
            "the printable area",
        )?;
        check(
            pm::PMSessionBeginPageNoDialog(session.0, format.0, ptr::null()),
            "starting a page",
        )?;
        let mut context: pm::Object = ptr::null_mut();
        let status = pm::PMSessionGetCGGraphicsContext(session.0, &raw mut context);
        let drawn = match NonNull::new(context) {
            Some(context) if status == 0 => {
                let context: &CGContext = context.cast::<CGContext>().as_ref();
                source.page_bands(page, PixelOrder::Rgba, &mut |band| {
                    draw_band(context, paper, printable, band)
                })
            }
            _ => Err(PrintError::Refused("the page has no graphics context".into())),
        };
        let ended = pm::PMSessionEndPageNoDialog(session.0);
        drawn?;
        check(ended, "finishing a page")
    }
}
