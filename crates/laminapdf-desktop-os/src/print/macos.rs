//! macOS: `NSPrintOperation` over a view that draws each page's bands.
//!
//! `AppKit` places a page's rectangle at the printer's imageable origin, not at the paper's
//! corner, so each page's rectangle starts that far into the page: the page's corner then lands
//! on the paper's corner (actual size; a printer's hardware margins clip, as in any viewer). The
//! offset depends on the paper and its orientation, so it's read for each page's own paper.
#![expect(
    unsafe_code,
    reason = "AppKit's print classes: a custom NSView, bitmap buffers and print-info keys"
)]

use std::cell::RefCell;

use objc2::rc::{Retained, autoreleasepool};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace, NSImage, NSPaperOrientation, NSPrintInfo,
    NSPrintJobSavingURL, NSPrintOperation, NSPrintSaveJob, NSPrintingPaginationMode, NSView,
};
use objc2_foundation::{NSCopying, NSInteger, NSPoint, NSRange, NSRect, NSSize, NSString, NSURL};

use super::{Band, PixelOrder, PrintError, PrintJob, PrintOutcome, PrintPage, PrintSource, band_edges, check_band};

/// What the view prints: the source, every page's place, and how the job went.
struct Job {
    source: Box<dyn PrintSource>,
    pages: Vec<PrintPage>,
    /// Each page's top in the view, in points: the pages stand one above the next.
    tops: Vec<f64>,
    /// Each page's printer imageable origin from its paper's top-left corner, read before the
    /// job for the page's own paper (the live print info holds the previous page's paper).
    offsets: Vec<(f64, f64)>,
    /// The first failure; later pages print blank, and the job reports it.
    failed: Option<PrintError>,
    drawn: usize,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = RefCell<Option<Job>>]
    struct PageView;

    impl PageView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(knowsPageRange:))]
        fn knows_page_range(&self, range: *mut NSRange) -> bool {
            let count = self.ivars().borrow().as_ref().map_or(0, |p| p.pages.len());
            // The method's value goes back to AppKit as a `BOOL`, which only the last expression
            // is converted to: no early `return`.
            if range.is_null() {
                false
            } else {
                // SAFETY: AppKit passes a valid, writable NSRange pointer, checked non-null above.
                unsafe { *range = NSRange::new(1, count) };
                true
            }
        }

        #[unsafe(method(rectForPage:))]
        fn rect_for_page(&self, page: NSInteger) -> NSRect {
            let state = self.ivars().borrow();
            let place = state.as_ref().and_then(|p| {
                let i = usize::try_from(page.checked_sub(1)?).ok()?;
                Some((*p.pages.get(i)?, *p.tops.get(i)?, *p.offsets.get(i)?))
            });
            let Some((size, top, (dx, dy))) = place else {
                return NSRect::ZERO;
            };
            NSRect::new(NSPoint::new(dx, top + dy), NSSize::new(size.width - dx, size.height - dy))
        }

        /// Gives each page its own paper size and orientation before `AppKit` starts it.
        #[unsafe(method(beginPageInRect:atPlacement:))]
        fn begin_page(&self, rect: NSRect, location: NSPoint) {
            let op = MainThreadMarker::new().and_then(NSPrintOperation::currentOperation);
            let size = op.as_ref().and_then(|op| {
                let i = usize::try_from(op.currentPage().checked_sub(1)?).ok()?;
                self.ivars().borrow().as_ref()?.pages.get(i).copied()
            });
            if let (Some(op), Some(size)) = (op, size) {
                set_paper(&op.printInfo(), size);
            }
            // SAFETY: the superclass's implementation, with the arguments AppKit passed.
            let () = unsafe { msg_send![super(self), beginPageInRect: rect, atPlacement: location] };
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, dirty: NSRect) {
            let mut state = self.ivars().borrow_mut();
            let Some(p) = state.as_mut() else {
                return;
            };
            for (i, size) in p.pages.clone().into_iter().enumerate() {
                let top = p.tops.get(i).copied().unwrap_or_default();
                let below = top + size.height <= dirty.origin.y;
                let above = top >= dirty.origin.y + dirty.size.height;
                if below || above || p.failed.is_some() {
                    continue;
                }
                let drawn = p.source.page_bands(i, PixelOrder::Rgba, &mut |band| draw_band(band, size, top));
                match drawn {
                    Ok(()) => p.drawn += 1,
                    Err(error) => p.failed = Some(error),
                }
            }
        }
    }
);

/// Draws one band into the current (print) graphics context, `top` points down the view.
fn draw_band(band: &Band<'_>, size: PrintPage, top: f64) -> Result<(), PrintError> {
    check_band(band)?;
    let (band_top, band_bottom) = band_edges(band, size.height);
    // Release the band's image as soon as it's drawn, not when the job ends.
    autoreleasepool(|_| {
        let image = image_of(band).ok_or_else(|| PrintError::Refused("AppKit couldn't hold a band".into()))?;
        let dest = NSRect::new(
            NSPoint::new(0.0, top + band_top),
            NSSize::new(size.width, band_bottom - band_top),
        );
        // SAFETY: drawing into the current (print) graphics context from `drawRect:`; no hints.
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
        Ok(())
    })
}

/// An image over a copy of `band`'s RGBA pixels.
fn image_of(band: &Band<'_>) -> Option<Retained<NSImage>> {
    let wide = isize::try_from(band.page_width).ok()?;
    let high = isize::try_from(band.rows).ok()?;
    // SAFETY: a null planes pointer makes the rep allocate its own buffer of wide × high × 4
    // bytes (4 samples of 8 bits, rows of wide × 4 bytes, not planar).
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            wide,
            high,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            wide.checked_mul(4)?,
            32,
        )
    }?;
    let data = rep.bitmapData();
    if data.is_null() {
        return None;
    }
    // SAFETY: the rep's buffer holds wide × high × 4 bytes, and `check_band` made sure the band's
    // pixels are exactly that many.
    unsafe { std::ptr::copy_nonoverlapping(band.pixels.as_ptr(), data, band.pixels.len()) };
    let image = NSImage::initWithSize(
        NSImage::alloc(),
        NSSize::new(f64::from(band.page_width), f64::from(band.rows)),
    );
    image.addRepresentation(&rep);
    Some(image)
}

/// Sets `info`'s paper to `size`, portrait paper turned for a landscape page.
fn set_paper(info: &NSPrintInfo, size: PrintPage) {
    info.setPaperSize(NSSize::new(size.width.min(size.height), size.width.max(size.height)));
    info.setOrientation(if size.landscape() {
        NSPaperOrientation::Landscape
    } else {
        NSPaperOrientation::Portrait
    });
}

/// The printer's imageable origin from the top-left corner of paper of `size`.
fn imageable_offset(base: &NSPrintInfo, size: PrintPage) -> (f64, f64) {
    let info = base.copy();
    set_paper(&info, size);
    let (paper, bounds) = (info.paperSize(), info.imageablePageBounds());
    // The view is flipped: the top offset is the paper above the imageable area.
    (bounds.origin.x, paper.height - bounds.origin.y - bounds.size.height)
}

/// A print info with no margins, for the first page's paper, saving to `job.output` when set.
fn print_info(first: PrintPage, job: &PrintJob) -> Retained<NSPrintInfo> {
    let info = NSPrintInfo::sharedPrintInfo().copy();
    set_paper(&info, first);
    info.setTopMargin(0.0);
    info.setBottomMargin(0.0);
    info.setLeftMargin(0.0);
    info.setRightMargin(0.0);
    info.setHorizontallyCentered(false);
    info.setVerticallyCentered(false);
    info.setHorizontalPagination(NSPrintingPaginationMode::Clip);
    info.setVerticalPagination(NSPrintingPaginationMode::Clip);
    if let Some(path) = job.output.as_deref().and_then(std::path::Path::to_str) {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        // SAFETY: NSPrintSaveJob is a disposition AppKit defines, and NSPrintJobSavingURL's value
        // is the NSURL it documents.
        unsafe {
            info.setJobDisposition(NSPrintSaveJob);
            info.dictionary().insert(NSPrintJobSavingURL, &url);
        }
    }
    info
}

/// Prints `source`'s pages: `AppKit`'s print panel, then its progress panel (whose Stop cancels).
/// With `job.output`, saves a PDF there with no panel.
///
/// # Errors
/// [`PrintError::NotMainThread`] off the main thread; the source's error when a page couldn't be
/// drawn.
pub fn print(job: &PrintJob, source: Box<dyn PrintSource>) -> Result<PrintOutcome, PrintError> {
    let main = MainThreadMarker::new().ok_or(PrintError::NotMainThread)?;
    let pages = source.pages().to_vec();
    let Some(&first) = pages.first() else {
        return Ok(PrintOutcome::Printed { pages: 0 });
    };
    let info = print_info(first, job);
    let mut tops = Vec::with_capacity(pages.len());
    let mut height = 0.0;
    for page in &pages {
        tops.push(height);
        height += page.height;
    }
    let width = pages.iter().map(|p| p.width).fold(0.0, f64::max);
    let offsets = pages.iter().map(|&page| imageable_offset(&info, page)).collect();
    let state = Job {
        source,
        pages,
        tops,
        offsets,
        failed: None,
        drawn: 0,
    };
    let view = PageView::alloc(main).set_ivars(RefCell::new(Some(state)));
    let frame = NSRect::new(NSPoint::ZERO, NSSize::new(width, height));
    // SAFETY: NSView's designated initializer, on a freshly allocated view.
    let view: Retained<PageView> = unsafe { msg_send![super(view), initWithFrame: frame] };
    let op = NSPrintOperation::printOperationWithView_printInfo(&view, &info);
    op.setJobTitle(Some(&NSString::from_str(&job.title)));
    let dialog = job.output.is_none();
    op.setShowsPrintPanel(dialog);
    op.setShowsProgressPanel(dialog);
    let finished = op.runOperation();
    let outcome = view.ivars().borrow_mut().take();
    match outcome {
        Some(Job {
            failed: Some(PrintError::Cancelled),
            ..
        }) => Ok(PrintOutcome::Cancelled),
        Some(Job {
            failed: Some(error), ..
        }) => Err(error),
        Some(Job { drawn, .. }) if finished => Ok(PrintOutcome::Printed { pages: drawn }),
        _ => Ok(PrintOutcome::Cancelled),
    }
}
