//! Prints synthetic pages of four sizes and orientations through the OS's real print path to its
//! PDF printer (Save as PDF on macOS, Microsoft Print to PDF on Windows), then reads the file back
//! with Lamina PDF: every page printed, each on paper of its own size and orientation. On macOS
//! the print system puts every page on the job's paper, in its own orientation (issue #626): there
//! the sizes are checked for the pages the size of the first. Linux is left to the desktop app's
//! end-to-end scenario, which has the display GTK needs.
//!
//! Its own `main()` (no libtest harness), which answers nextest's `--list` itself: `AppKit`'s print
//! panel and Core Printing's setup run only on the main thread.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a test: a panic is a failure; the pages are small whole numbers of points"
)]

#[cfg(any(target_os = "macos", windows))]
use laminapdf_desktop_os::print::{Band, PixelOrder, PrintError, PrintJob, PrintOutcome, PrintPage, PrintSource};

#[cfg(any(target_os = "macos", windows))]
/// A4 portrait, US Letter landscape, A4 landscape and A5 portrait, in points.
const PAGES: [PrintPage; 4] = [
    PrintPage {
        width: 595.0,
        height: 842.0,
    },
    PrintPage {
        width: 792.0,
        height: 612.0,
    },
    PrintPage {
        width: 842.0,
        height: 595.0,
    },
    PrintPage {
        width: 420.0,
        height: 595.0,
    },
];

#[cfg(any(target_os = "macos", windows))]
/// Grey pages at 72 dpi, in bands of 100 rows.
struct Grey;

#[cfg(any(target_os = "macos", windows))]
impl PrintSource for Grey {
    fn pages(&self) -> &[PrintPage] {
        &PAGES
    }

    fn page_bands(
        &mut self,
        page: usize,
        _order: PixelOrder,
        draw: &mut dyn FnMut(&Band<'_>) -> Result<(), PrintError>,
    ) -> Result<(), PrintError> {
        let size = PAGES[page];
        let (width, height) = (size.width as u32, size.height as u32);
        let mut y = 0;
        while y < height {
            let rows = 100.min(height - y);
            let pixels = vec![160; (width * rows * 4) as usize];
            draw(&Band {
                page_width: width,
                page_height: height,
                y,
                rows,
                pixels: &pixels,
            })?;
            y += rows;
        }
        Ok(())
    }
}

#[cfg(any(target_os = "macos", windows))]
fn print(job: &PrintJob) -> PrintOutcome {
    #[cfg(target_os = "macos")]
    let chosen = laminapdf_desktop_os::print::choose(job, PAGES.len()).unwrap().unwrap();
    #[cfg(windows)]
    let chosen = laminapdf_desktop_os::print::choose(0, job, PAGES.len())
        .unwrap()
        .unwrap();
    laminapdf_desktop_os::print::spool(chosen, job, &mut Grey).unwrap()
}

/// Answers nextest's `--list` (one test, none ignored) as libtest would, and runs otherwise.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--list") {
        if !args.iter().any(|arg| arg == "--ignored") {
            println!("print_to_file: test");
        }
        return;
    }
    run();
}

#[cfg(any(target_os = "macos", windows))]
fn run() {
    let out = std::env::temp_dir().join(format!("lamina-print-test-{}.pdf", std::process::id()));
    let job = PrintJob {
        title: "print_to_file".into(),
        current_page: None,
        output: Some(out.clone()),
    };
    assert_eq!(print(&job), PrintOutcome::Printed { pages: PAGES.len() });
    let bytes = std::fs::read(&out).unwrap();
    if std::env::var_os("LAMINA_KEEP_PRINT").is_none() {
        let _ = std::fs::remove_file(&out);
    } else {
        println!("print_to_file: kept {}", out.display());
    }
    let doc = laminapdf::Document::open_bytes(bytes, laminapdf::OpenOptions::default()).unwrap();
    let printed: Vec<(f64, f64)> = doc.page_infos().unwrap().iter().map(|p| (p.width, p.height)).collect();
    let off: Vec<String> = PAGES
        .iter()
        .zip(&printed)
        .enumerate()
        .filter(|(_, (want, got))| !on_its_paper(**want, **got))
        .map(|(i, (want, got))| format!("page {}: {want:?} printed as {got:?}", i + 1))
        .collect();
    assert_eq!(printed.len(), PAGES.len(), "pages printed: {printed:?}");
    assert!(off.is_empty(), "pages on the wrong paper: {off:?}");
    println!("print_to_file: {} pages printed", printed.len());
}

/// Whether a page `want` printed as `got`: its size within 2 pt (the spike's bar). On macOS, its
/// orientation, and its size when it's the first page's (#626).
#[cfg(any(target_os = "macos", windows))]
fn on_its_paper(want: PrintPage, got: (f64, f64)) -> bool {
    let near = |a: f64, b: f64| (a - b).abs() <= 2.0;
    let sized = near(want.width, got.0) && near(want.height, got.1);
    if cfg!(target_os = "macos") {
        let first = PAGES[0];
        let first_size = near(want.width.min(want.height), first.width.min(first.height))
            && near(want.width.max(want.height), first.width.max(first.height));
        (want.width > want.height) == (got.0 > got.1) && (sized || !first_size)
    } else {
        sized
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
fn run() {
    println!("print_to_file: Linux prints in the desktop app's end-to-end scenario");
}
