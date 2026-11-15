//! Linux: `GtkPrintOperation`. `request-page-setup` gives each page a custom paper of its own size
//! with no margins (GTK honoured every size in the spike), and `draw-page` paints the page's bands
//! with cairo. The operation's own progress window shows while it spools, with its Cancel.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{PageOrientation, PageSetup, PaperSize, PrintOperation, PrintOperationAction, PrintOperationResult, Unit};

use super::{Band, PixelOrder, PrintError, PrintJob, PrintOutcome, PrintPage, PrintSource, check_band};

/// The window the print dialog belongs to: Tauri's own GTK window (`WebviewWindow::gtk_window`).
pub type Owner = gtk::ApplicationWindow;

/// What the operation's handlers share: the source and how the job went.
struct Job {
    source: Box<dyn PrintSource>,
    pages: Vec<PrintPage>,
    /// The first failure; the operation is cancelled and the job reports it.
    failed: Option<PrintError>,
    drawn: usize,
}

/// Paints `band` (BGRA, cairo's `ARGB32` on a little-endian machine) at its rows of the page; `cr`
/// is scaled to the page's pixels.
fn paint_band(cr: &gtk::cairo::Context, band: &Band<'_>) -> Result<(), PrintError> {
    check_band(band)?;
    let refused = |error: gtk::cairo::Error| PrintError::Refused(error.to_string());
    let width = i32::try_from(band.page_width).map_err(|_| PrintError::Source("band too wide".into()))?;
    let rows = i32::try_from(band.rows).map_err(|_| PrintError::Source("band too tall".into()))?;
    let stride = width
        .checked_mul(4)
        .ok_or_else(|| PrintError::Source("band too wide".into()))?;
    // cairo keeps the surface's data while the page records it: a copy it owns.
    let surface = gtk::cairo::ImageSurface::create_for_data(
        band.pixels.to_vec(),
        gtk::cairo::Format::ARgb32,
        width,
        rows,
        stride,
    )
    .map_err(refused)?;
    let y = f64::from(band.y);
    cr.set_source_surface(&surface, 0.0, y).map_err(refused)?;
    cr.rectangle(0.0, y, f64::from(band.page_width), f64::from(band.rows));
    cr.fill().map_err(refused)
}

/// Draws page `page` of `job` on `cr`, whose units are the page's points.
fn draw_page(job: &mut Job, cr: &gtk::cairo::Context, page: usize) -> Result<(), PrintError> {
    let size = *job
        .pages
        .get(page)
        .ok_or_else(|| PrintError::Source("no such page".into()))?;
    job.source.page_bands(page, PixelOrder::Bgra, &mut |band| {
        let refused = |error: gtk::cairo::Error| PrintError::Refused(error.to_string());
        cr.save().map_err(refused)?;
        // From the page's points to its pixels, one scale per axis, so the bands fill the page.
        cr.scale(
            size.width / f64::from(band.page_width.max(1)),
            size.height / f64::from(band.page_height.max(1)),
        );
        let painted = paint_band(cr, band);
        cr.restore().map_err(refused)?;
        painted
    })
}

/// Prints `source`'s pages: GTK's print dialog over `owner`, then its progress window. With
/// `job.output`, exports a PDF there with no dialog (every page).
///
/// # Errors
/// [`PrintError::NotMainThread`] off GTK's main thread; [`PrintError::Refused`] when GTK fails;
/// the source's error when a page couldn't be drawn.
pub fn print(owner: Option<&Owner>, job: &PrintJob, source: Box<dyn PrintSource>) -> Result<PrintOutcome, PrintError> {
    if !gtk::is_initialized_main_thread() {
        return Err(PrintError::NotMainThread);
    }
    let pages = source.pages().to_vec();
    let count = i32::try_from(pages.len()).map_err(|_| PrintError::Refused("too many pages".into()))?;
    let state = Rc::new(RefCell::new(Job {
        source,
        pages: pages.clone(),
        failed: None,
        drawn: 0,
    }));
    let op = PrintOperation::new();
    op.set_job_name(&job.title);
    op.set_n_pages(count);
    op.set_use_full_page(true);
    op.set_unit(Unit::Points);
    op.set_show_progress(true);
    if let Some(current) = job.current_page.and_then(|page| i32::try_from(page).ok()) {
        op.set_current_page(current);
    }
    op.connect_request_page_setup(move |_, _, page, setup: &PageSetup| {
        let Some(size) = usize::try_from(page).ok().and_then(|page| pages.get(page)) else {
            return;
        };
        setup.set_paper_size(&PaperSize::new_custom(
            "page",
            "page",
            size.width,
            size.height,
            Unit::Points,
        ));
        setup.set_orientation(PageOrientation::Portrait);
        setup.set_top_margin(0.0, Unit::Points);
        setup.set_bottom_margin(0.0, Unit::Points);
        setup.set_left_margin(0.0, Unit::Points);
        setup.set_right_margin(0.0, Unit::Points);
    });
    let drawing = Rc::clone(&state);
    op.connect_draw_page(move |op, ctx, page| {
        let mut job = drawing.borrow_mut();
        if job.failed.is_some() {
            return;
        }
        let drawn = match (ctx.cairo_context(), usize::try_from(page)) {
            (Some(cr), Ok(page)) => draw_page(&mut job, &cr, page),
            _ => Err(PrintError::Refused("GTK gave no page to draw on".into())),
        };
        match drawn {
            Ok(()) => job.drawn += 1,
            Err(error) => {
                job.failed = Some(error);
                op.cancel();
            }
        }
    });
    let action = match job.output.as_deref() {
        Some(path) => {
            op.set_export_filename(path);
            PrintOperationAction::Export
        }
        None => PrintOperationAction::PrintDialog,
    };
    let result = op.run(action, owner);
    let mut job = state.borrow_mut();
    match (job.failed.take(), result) {
        (Some(PrintError::Cancelled), _) | (None, Ok(PrintOperationResult::Cancel)) => Ok(PrintOutcome::Cancelled),
        (Some(error), _) => Err(error),
        (None, Ok(PrintOperationResult::Apply | PrintOperationResult::InProgress)) => {
            Ok(PrintOutcome::Printed { pages: job.drawn })
        }
        (None, Ok(_)) => Err(PrintError::Refused("GTK couldn't print".into())),
        (None, Err(error)) => Err(PrintError::Refused(error.to_string())),
    }
}
