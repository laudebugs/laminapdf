//! macOS: AppKit's `NSPrintOperation` for every path. B draws PDFium rasters from an `NSView`;
//! C hands the bytes to PDFKit (Apple's renderer, not PDFium); A asks WKWebView for its print
//! operation. AppKit printing must run on the main thread: each call hops there with
//! `AppHandle::run_on_main_thread` and waits for the result.

use std::cell::RefCell;
use std::path::Path;
use std::sync::mpsc::channel;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace, NSImage, NSPaperOrientation, NSPrintInfo,
    NSPrintJobSavingURL, NSPrintOperation, NSPrintSaveJob, NSPrintingPaginationMode, NSView, NSWindow,
};
use objc2_foundation::{NSCopying, NSData, NSInteger, NSPoint, NSRange, NSRect, NSSize, NSString, NSURL};
use objc2_pdf_kit::{PDFDocument, PDFPrintScalingMode};
use objc2_web_kit::WKWebView;
use pdfium_render::prelude::PdfDocument;
use tauri::{AppHandle, Manager};

use crate::{Fixture, PageSize, pdf};

/// The resolution of path B's rasters.
const RASTER_DPI: f32 = 300.0;

/// Runs `f` on the main thread and waits for its result.
fn on_main<T: Send + 'static>(
    handle: &AppHandle,
    f: impl FnOnce(MainThreadMarker) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = channel();
    handle
        .run_on_main_thread(move || {
            let result = MainThreadMarker::new().ok_or("not the main thread".to_owned()).and_then(f);
            let _ = tx.send(result);
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

/// A print info with no margins, sized for `first`, saving to `out` when given.
fn print_info(first: PageSize, out: Option<&Path>) -> Result<Retained<NSPrintInfo>, String> {
    let info = NSPrintInfo::sharedPrintInfo().copy();
    info.setPaperSize(NSSize::new(f64::from(first.w.min(first.h)), f64::from(first.w.max(first.h))));
    info.setOrientation(if first.landscape() { NSPaperOrientation::Landscape } else { NSPaperOrientation::Portrait });
    info.setTopMargin(0.0);
    info.setBottomMargin(0.0);
    info.setLeftMargin(0.0);
    info.setRightMargin(0.0);
    info.setHorizontallyCentered(false);
    info.setVerticallyCentered(false);
    info.setHorizontalPagination(NSPrintingPaginationMode::Clip);
    info.setVerticalPagination(NSPrintingPaginationMode::Clip);
    if let Some(out) = out {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&out.to_string_lossy()));
        // SAFETY: the dictionary's values are objects; NSPrintJobSavingURL takes an NSURL.
        unsafe {
            info.setJobDisposition(NSPrintSaveJob);
            info.dictionary().insert(NSPrintJobSavingURL, &url);
        }
    }
    Ok(info)
}

fn run(op: &NSPrintOperation, dialog: bool) -> Result<(), String> {
    op.setShowsPrintPanel(dialog);
    op.setShowsProgressPanel(dialog);
    if op.runOperation() { Ok(()) } else { Err("the print operation failed or was cancelled".into()) }
}

// ---- Path B: rasters drawn by an NSView, one page per rect ----

struct Pages {
    /// The fixture, open in PDFium for the whole job (pages render on demand in drawRect:).
    doc: PdfDocument<'static>,
    sizes: Vec<PageSize>,
    /// Each page's top in the view, in points.
    tops: Vec<f64>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = RefCell<Option<Pages>>]
    struct PageView;

    impl PageView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(knowsPageRange:))]
        fn knows_page_range(&self, range: *mut NSRange) -> bool {
            let n = self.ivars().borrow().as_ref().map_or(0, |p| p.sizes.len());
            // SAFETY: AppKit passes a valid NSRange pointer.
            unsafe { *range = NSRange::new(1, n) };
            true
        }

        #[unsafe(method(rectForPage:))]
        fn rect_for_page(&self, page: NSInteger) -> NSRect {
            let pages = self.ivars().borrow();
            let Some(p) = pages.as_ref() else { return NSRect::ZERO };
            let i = usize::try_from(page - 1).unwrap_or(0).min(p.sizes.len().saturating_sub(1));
            let s = p.sizes[i];
            // AppKit puts this rect at the printer's imageable origin, not the paper's corner.
            // Starting the rect that far into the page puts the page's corner on the paper's
            // corner (actual size; the hardware margins clip, as in any viewer).
            let (dx, dy) = MainThreadMarker::new()
                .and_then(NSPrintOperation::currentOperation)
                .map(|op| {
                    let info = op.printInfo();
                    let (paper, bounds) = (info.paperSize(), info.imageablePageBounds());
                    // Flipped view: the top margin is the paper above the imageable area.
                    (bounds.origin.x, paper.height - bounds.origin.y - bounds.size.height)
                })
                .unwrap_or((0.0, 0.0));
            NSRect::new(NSPoint::new(dx, p.tops[i] + dy), NSSize::new(f64::from(s.w) - dx, f64::from(s.h) - dy))
        }

        /// Gives each page its own paper size and orientation before AppKit starts it.
        #[unsafe(method(beginPageInRect:atPlacement:))]
        fn begin_page(&self, rect: NSRect, location: NSPoint) {
            if let Some(op) = MainThreadMarker::new().and_then(NSPrintOperation::currentOperation) {
                let info = op.printInfo();
                let (w, h) = (rect.size.width, rect.size.height);
                info.setPaperSize(NSSize::new(w.min(h), w.max(h)));
                info.setOrientation(if w > h { NSPaperOrientation::Landscape } else { NSPaperOrientation::Portrait });
            }
            // SAFETY: calling the superclass's implementation with the same arguments.
            let _: () = unsafe { msg_send![super(self), beginPageInRect: rect, atPlacement: location] };
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, dirty: NSRect) {
            let pages = self.ivars().borrow();
            let Some(p) = pages.as_ref() else { return };
            for (i, s) in p.sizes.iter().enumerate() {
                let top = p.tops[i];
                let (h, w) = (f64::from(s.h), f64::from(s.w));
                if top + h <= dirty.origin.y || top >= dirty.origin.y + dirty.size.height {
                    continue;
                }
                let Ok(raster) = pdf::render(&p.doc, i as u16, RASTER_DPI) else { continue };
                if let Some(image) = ns_image(&raster) {
                    let dest = NSRect::new(NSPoint::new(0.0, top), NSSize::new(w, h));
                    // SAFETY: drawing into the current (print) graphics context from drawRect:.
                    unsafe {
                        image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                            dest,
                            NSRect::ZERO,
                            NSCompositingOperation::Copy,
                            1.0,
                            true,
                            None,
                        );
                    }
                }
            }
        }
    }
);

/// An NSImage over a copy of `raster`'s pixels.
fn ns_image(raster: &pdf::Raster) -> Option<Retained<NSImage>> {
    let (w, h) = (raster.width as isize, raster.height as isize);
    // SAFETY: a null planes pointer makes the rep allocate its own buffer of w × h × 4 bytes,
    // which we fill with exactly that many bytes.
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            w,
            h,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            w * 4,
            32,
        )
    }?;
    let data = rep.bitmapData();
    if data.is_null() {
        return None;
    }
    // SAFETY: the rep's buffer holds exactly rgba.len() bytes (w × h × 4).
    unsafe { std::ptr::copy_nonoverlapping(raster.rgba.as_ptr(), data, raster.rgba.len()) };
    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(w as f64, h as f64));
    image.addRepresentation(&rep);
    Some(image)
}

pub fn print_raster(handle: &AppHandle, fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let (bytes, sizes, out) = (fixture.bytes.clone(), fixture.sizes.clone(), out.map(Path::to_path_buf));
    on_main(handle, move |mtm| {
        let doc = pdf::open(&bytes)?;
        let mut tops = Vec::with_capacity(sizes.len());
        let mut y = 0.0;
        for s in &sizes {
            tops.push(y);
            y += f64::from(s.h);
        }
        let width = sizes.iter().map(|s| f64::from(s.w)).fold(0.0, f64::max);
        let view = PageView::alloc(mtm).set_ivars(RefCell::new(Some(Pages { doc, sizes: sizes.clone(), tops })));
        let frame = NSRect::new(NSPoint::ZERO, NSSize::new(width, y));
        // SAFETY: NSView's designated initializer.
        let view: Retained<PageView> = unsafe { msg_send![super(view), initWithFrame: frame] };
        let info = print_info(sizes[0], out.as_deref())?;
        let op = NSPrintOperation::printOperationWithView_printInfo(&view, &info);
        run(&op, out.is_none())
    })
}

// ---- Path C: PDFKit ----

pub fn print_vector(handle: &AppHandle, fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let (bytes, first, out) = (fixture.bytes.clone(), fixture.sizes[0], out.map(Path::to_path_buf));
    on_main(handle, move |mtm| {
        let data = NSData::with_bytes(&bytes);
        // SAFETY: PDFDocument copies what it needs from the data.
        let doc = unsafe { PDFDocument::initWithData(PDFDocument::alloc(), &data) }.ok_or("PDFKit can't open the file")?;
        let info = print_info(first, out.as_deref())?;
        // SAFETY: a valid print info; the scaling mode is one of PDFKit's constants.
        let op = unsafe { doc.printOperationForPrintInfo_scalingMode_autoRotate(Some(&info), PDFPrintScalingMode::PageScaleNone, true, mtm) }
            .ok_or("PDFKit gave no print operation")?;
        run(&op, out.is_none())
    })
}

// ---- Path A: WKWebView's own print operation ----

pub fn print_webview(handle: &AppHandle, fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let window = handle.get_webview_window("main").ok_or("no main window")?;
    let (first, out) = (fixture.sizes[0], out.map(Path::to_path_buf));
    let (tx, rx) = channel();
    window
        .with_webview(move |webview| {
            let result = (|| {
                let mtm = MainThreadMarker::new().ok_or("not the main thread")?;
                // SAFETY: on macOS, wry's inner webview is its WKWebView.
                let wk: &WKWebView = unsafe { &*webview.inner().cast::<WKWebView>() };
                let info = print_info(first, out.as_deref())?;
                // SAFETY: a valid print info.
                let op = unsafe { wk.printOperationWithPrintInfo(&info) };
                let _ = mtm;
                // WebKit's operation must run modally on its window: a synchronous runOperation
                // kept paginating (a 4-page document grew past 5 GB). The job then waits for
                // the output file to stop growing.
                op.setShowsPrintPanel(out.is_none());
                op.setShowsProgressPanel(false);
                // SAFETY: on macOS, tauri's ns_window is the webview's NSWindow; no delegate.
                unsafe {
                    let window: &NSWindow = &*webview.ns_window().cast::<NSWindow>();
                    op.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
                        window,
                        None,
                        None,
                        std::ptr::null_mut(),
                    );
                }
                Ok(())
            })();
            let _ = tx.send(result);
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

#[allow(dead_code)]
fn _assert_any(_: &AnyObject) {}
