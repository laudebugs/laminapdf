//! Printing through the OS (ADR 0008 §9): the OS's print dialog, then each page drawn from
//! Lamina PDF's own rasters on paper of the page's size and orientation, with no browser headers
//! or margins. This crate never parses a PDF: the bands come from a [`PrintSource`], which the
//! app fills from the sandboxed engine.
//!
//! - **macOS:** `AppKit`'s print panel, then a Core Printing session that gives each page its own
//!   page format and draws its bands with Core Graphics. The print system puts every page on the
//!   job's paper, in the page's own orientation (issue #626).
//! - **Windows:** `PrintDlgExW`, then GDI on the printer's device context: `ResetDCW` sets each
//!   page's paper and orientation, `StretchDIBits` draws each band.
//! - **Linux:** `GtkPrintOperation`: each page's paper set in `request-page-setup`, its bands
//!   drawn with cairo in `draw-page`.
//!
//! On macOS and Windows the dialog runs on the main thread (`choose`) and the spooling on any
//! thread (`spool`). GTK prints only from the main thread, and draws from it too, so on Linux the
//! source is asked for bands on the main thread (`print`).

use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{Owner, print};
#[cfg(target_os = "macos")]
pub use macos::{Chosen, choose, spool};
#[cfg(windows)]
pub use windows::{Chosen, choose, spool};

/// A page's displayed size in points: its visible box with `/Rotate` applied, as it prints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrintPage {
    /// Width in points.
    pub width: f64,
    /// Height in points.
    pub height: f64,
}

impl PrintPage {
    /// Whether the page is wider than it is tall: it prints on landscape paper.
    #[must_use]
    pub fn landscape(self) -> bool {
        self.width > self.height
    }
}

/// The byte order of a band's pixels: what the OS's drawing call takes as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelOrder {
    /// R, G, B, A (`AppKit`'s bitmap image rep).
    Rgba,
    /// B, G, R, A (GDI's DIBs; cairo's `ARGB32` on a little-endian machine).
    Bgra,
}

/// One band of a page, as the OS code draws it.
#[derive(Debug, Clone, Copy)]
pub struct Band<'a> {
    /// The whole page's width in pixels, which is the band's width too.
    pub page_width: u32,
    /// The whole page's height in pixels.
    pub page_height: u32,
    /// The band's first row.
    pub y: u32,
    /// The band's height in rows.
    pub rows: u32,
    /// `page_width × rows × 4` bytes, opaque, rows top to bottom, in the order asked for.
    pub pixels: &'a [u8],
}

/// Where a document's pages come from: the app's link to the engine.
pub trait PrintSource {
    /// Every page's displayed size, in page order.
    fn pages(&self) -> &[PrintPage];

    /// Calls `draw` for each band of `page` (0-based), top to bottom, with pixels in `order`.
    /// Only one band is held at a time.
    ///
    /// # Errors
    /// [`PrintError::Cancelled`] once the user cancels, [`PrintError::Source`] when the engine
    /// fails, and whatever `draw` returns.
    fn page_bands(
        &mut self,
        page: usize,
        order: PixelOrder,
        draw: &mut dyn FnMut(&Band<'_>) -> Result<(), PrintError>,
    ) -> Result<(), PrintError>;
}

/// One print job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrintJob {
    /// The job's name in the OS's print queue.
    pub title: String,
    /// The page shown when Print was chosen, 0-based, for the dialog's "Current page".
    pub current_page: Option<usize>,
    /// Print to this PDF file with no dialog (the app's end-to-end tests): Save as PDF on
    /// macOS, Microsoft Print to PDF on Windows, GTK's export on Linux.
    pub output: Option<PathBuf>,
}

/// How a print job ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintOutcome {
    /// The pages went to the printer (or the file).
    Printed {
        /// How many pages were drawn.
        pages: usize,
    },
    /// The user closed the dialog or stopped the job.
    Cancelled,
}

/// Why a print job failed. Never carries a path or document content.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PrintError {
    /// The call ran off the thread the OS prints from.
    #[error("printing must start on the main thread")]
    NotMainThread,
    /// The user cancelled while the pages were drawn.
    #[error("the print job was cancelled")]
    Cancelled,
    /// The engine couldn't draw a page.
    #[error("a page couldn't be drawn: {0}")]
    Source(String),
    /// The OS refused a call.
    #[error("the OS refused to print: {0}")]
    Refused(String),
}

/// Where a band's rows go on the page, in the page's own units (points, or device pixels): the
/// band's top and bottom edges. Each edge is rounded from the page's top on its own, so one
/// band's bottom is the next one's top and no gap or overlap opens between them.
#[must_use]
pub fn band_edges(band: &Band<'_>, page_extent: f64) -> (f64, f64) {
    let height = f64::from(band.page_height.max(1));
    let top = f64::from(band.y) * page_extent / height;
    let bottom = f64::from(band.y + band.rows) * page_extent / height;
    (top, bottom)
}

/// Checks a band's numbers against its pixels before any OS call reads them: a compromised
/// engine's band must not make the OS read past the buffer.
///
/// # Errors
/// [`PrintError::Source`] when the band is empty, runs past its page, or its pixels aren't
/// `page_width × rows × 4` bytes.
pub fn check_band(band: &Band<'_>) -> Result<(), PrintError> {
    let expected = u64::from(band.page_width) * u64::from(band.rows) * 4;
    let fits = band.page_width > 0
        && band.rows > 0
        && u64::from(band.y) + u64::from(band.rows) <= u64::from(band.page_height)
        && band.pixels.len() as u64 == expected;
    if fits {
        Ok(())
    } else {
        Err(PrintError::Source(
            "the engine sent a band that doesn't fit its page".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band(y: u32, rows: u32, pixels: &[u8]) -> Band<'_> {
        Band {
            page_width: 2,
            page_height: 10,
            y,
            rows,
            pixels,
        }
    }

    #[test]
    fn band_edges_should_meet_where_one_band_ends_and_the_next_begins() {
        let pixels = [0; 2 * 3 * 4];
        let (_, bottom) = band_edges(&band(0, 3, &pixels), 792.0);
        let (top, _) = band_edges(&band(3, 3, &pixels), 792.0);
        assert!((bottom - top).abs() < f64::EPSILON);
    }

    #[test]
    fn band_edges_should_put_the_last_band_at_the_bottom_of_the_page() {
        let pixels = [0; 2 * 4 * 4];
        let (_, bottom) = band_edges(&band(6, 4, &pixels), 792.0);
        assert!((bottom - 792.0).abs() < f64::EPSILON);
    }

    #[test]
    fn check_band_should_accept_a_band_whose_pixels_fill_it() {
        assert_eq!(check_band(&band(6, 4, &[0; 2 * 4 * 4])), Ok(()));
    }

    #[test]
    fn check_band_should_refuse_a_band_running_past_its_page() {
        assert!(check_band(&band(8, 4, &[0; 2 * 4 * 4])).is_err());
    }

    #[test]
    fn check_band_should_refuse_pixels_shorter_than_the_band() {
        assert!(check_band(&band(0, 4, &[0; 2 * 3 * 4])).is_err());
    }

    #[test]
    fn a_page_wider_than_tall_should_print_landscape() {
        assert!(
            PrintPage {
                width: 842.0,
                height: 595.0
            }
            .landscape()
        );
    }
}
