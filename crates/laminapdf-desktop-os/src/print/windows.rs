//! Windows: `PrintDlgExW`, then GDI on the printer's device context. `ResetDCW` gives each page
//! its own paper and orientation before `StartPage` (Microsoft Print to PDF and real drivers
//! honour it), and `StretchDIBits` draws each band at the page's place on the paper. The driver
//! makes the copies and collates them, from the dialog's `DEVMODE`; duplex comes from it too.
#![expect(unsafe_code, reason = "Win32's print dialog, its global memory and GDI's printer DC")]

use std::ffi::c_void;
use std::path::Path;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{GlobalFree, HGLOBAL, HWND};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDCW, DEVMODEW, DIB_RGB_COLORS, DeleteDC, GetDeviceCaps, HDC,
    LOGPIXELSX, LOGPIXELSY, PHYSICALOFFSETX, PHYSICALOFFSETY, ResetDCW, SRCCOPY, StretchDIBits,
};
use windows::Win32::Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::UI::Controls::Dialogs::{
    PD_CURRENTPAGE, PD_NOSELECTION, PD_PAGENUMS, PD_RESULT_PRINT, PD_RETURNDC, PD_USEDEVMODECOPIESANDCOLLATE,
    PRINTDLGEXW, PRINTPAGERANGE, PrintDlgExW,
};
use windows::core::{HSTRING, PCWSTR, w};

use super::{Band, PixelOrder, PrintError, PrintJob, PrintOutcome, PrintPage, PrintSource, band_edges, check_band};

const DM_ORIENTATION: u32 = 0x0000_0001;
const DM_PAPERSIZE: u32 = 0x0000_0002;
const DM_PAPERLENGTH: u32 = 0x0000_0004;
const DM_PAPERWIDTH: u32 = 0x0000_0008;
const DMORIENT_PORTRAIT: i16 = 1;
const DMORIENT_LANDSCAPE: i16 = 2;
const DMPAPER_LETTER: i16 = 1;
const DMPAPER_A4: i16 = 9;
const DMPAPER_A5: i16 = 11;
const DMPAPER_USER: i16 = 256;
/// `PRINTDLGEX`'s `nStartPage` for the General tab.
const START_PAGE_GENERAL: u32 = u32::MAX;
/// The most ranges the dialog may return.
const MAX_RANGES: usize = 32;

/// The printer and pages the user chose: the device context, its settings and the pages, in the
/// order they print. A plain value, so the spooling can run on another thread.
pub struct Chosen {
    dc: isize,
    devmode: Option<DEVMODEW>,
    pages: Vec<usize>,
}

impl std::fmt::Debug for Chosen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chosen")
            .field("pages", &self.pages)
            .finish_non_exhaustive()
    }
}

impl Drop for Chosen {
    /// Deletes the device context, whether or not the job was spooled.
    fn drop(&mut self) {
        // SAFETY: the DC `choose` created or the dialog returned, deleted once, here.
        unsafe {
            let _ = DeleteDC(HDC(self.dc as *mut c_void));
        }
    }
}

// SAFETY: the device context is a handle number GDI accepts from any thread, used by one thread
// at a time (whoever holds the `Chosen`); `DEVMODEW` is plain data.
unsafe impl Send for Chosen {}

/// The printer and pages: the print dialog over `owner` (a window handle), or, with
/// `job.output`, Microsoft Print to PDF and every page. Run it on the thread that owns `owner`.
///
/// # Errors
/// [`PrintError::Refused`] when the dialog or the PDF printer can't be opened.
pub fn choose(owner: isize, job: &PrintJob, page_count: usize) -> Result<Option<Chosen>, PrintError> {
    if job.output().is_some() {
        // SAFETY: CreateDCW with static, null-terminated names; the DC is deleted by `spool`.
        let dc = unsafe { CreateDCW(w!("WINSPOOL"), w!("Microsoft Print to PDF"), PCWSTR::null(), None) };
        if dc.is_invalid() {
            return Err(PrintError::refused("opening Microsoft Print to PDF"));
        }
        return Ok(Some(Chosen {
            dc: dc.0 as isize,
            devmode: None,
            pages: (0..page_count).collect(),
        }));
    }
    let count = u32::try_from(page_count).map_err(|_| PrintError::refused("counting the pages"))?;
    let mut ranges = [PRINTPAGERANGE {
        nFromPage: 1,
        nToPage: count.max(1),
    }; MAX_RANGES];
    let mut dialog = PRINTDLGEXW {
        lStructSize: win32_size::<PRINTDLGEXW>(),
        hwndOwner: HWND(owner as *mut c_void),
        Flags: PD_RETURNDC | PD_NOSELECTION | PD_USEDEVMODECOPIESANDCOLLATE,
        nPageRanges: 1,
        nMaxPageRanges: u32::try_from(MAX_RANGES).unwrap_or(1),
        lpPageRanges: ranges.as_mut_ptr(),
        nMinPage: 1,
        nMaxPage: count.max(1),
        nCopies: 1,
        nStartPage: START_PAGE_GENERAL,
        ..Default::default()
    };
    // SAFETY: `dialog` is fully set up, its ranges point at `ranges`, which outlives the call.
    unsafe { PrintDlgExW(&raw mut dialog) }
        .map_err(|error| PrintError::refused_with("showing the print dialog", i64::from(error.code().0)))?;
    let devmode = take_devmode(dialog.hDevMode);
    free(dialog.hDevNames);
    if dialog.dwResultAction != PD_RESULT_PRINT {
        if !dialog.hDC.is_invalid() {
            // SAFETY: the DC the dialog returned, deleted once.
            unsafe {
                let _ = DeleteDC(dialog.hDC);
            }
        }
        return Ok(None);
    }
    let pages = if dialog.Flags.0 & PD_PAGENUMS.0 != 0 {
        let chosen = usize::try_from(dialog.nPageRanges).unwrap_or(0).min(MAX_RANGES);
        pages_in(ranges.get(..chosen).unwrap_or_default(), page_count)
    } else if dialog.Flags.0 & PD_CURRENTPAGE.0 != 0 {
        job.current_page.filter(|&page| page < page_count).into_iter().collect()
    } else {
        (0..page_count).collect()
    };
    Ok(Some(Chosen {
        dc: dialog.hDC.0 as isize,
        devmode,
        pages,
    }))
}

/// A Win32 struct's size, as its size field holds it (a few hundred bytes at most).
fn win32_size<T>() -> u32 {
    u32::try_from(size_of::<T>()).unwrap_or(u32::MAX)
}

/// The pages the dialog's 1-based, inclusive ranges name, 0-based, past the end dropped.
fn pages_in(ranges: &[PRINTPAGERANGE], page_count: usize) -> Vec<usize> {
    ranges
        .iter()
        .flat_map(|range| {
            let from = usize::try_from(range.nFromPage).unwrap_or(1).max(1) - 1;
            let to = usize::try_from(range.nToPage).unwrap_or(0).min(page_count);
            from..to
        })
        .collect()
}

/// A copy of the dialog's `DEVMODE` without its driver's extra bytes, and its memory freed.
fn take_devmode(handle: HGLOBAL) -> Option<DEVMODEW> {
    if handle.is_invalid() {
        return None;
    }
    // SAFETY: the dialog's DEVMODE handle, locked while it's copied, then unlocked and freed once.
    let devmode = unsafe {
        let pointer = GlobalLock(handle).cast::<DEVMODEW>();
        let copy = (!pointer.is_null()).then(|| {
            let mut devmode = *pointer;
            devmode.dmDriverExtra = 0;
            devmode
        });
        let _ = GlobalUnlock(handle);
        copy
    };
    free(handle);
    devmode
}

fn free(handle: HGLOBAL) {
    if !handle.is_invalid() {
        // SAFETY: a handle the dialog allocated, freed once.
        unsafe {
            let _ = GlobalFree(Some(handle));
        }
    }
}

/// The paper Windows names for `size` (A4, Letter, A5), else a custom paper in tenths of a
/// millimetre: `(code, width, length)`.
fn paper(size: PrintPage) -> (i16, i16, i16) {
    let (short, long) = (size.width.min(size.height), size.width.max(size.height));
    let near = |w: f64, h: f64| (short - w).abs() < 2.0 && (long - h).abs() < 2.0;
    #[expect(clippy::cast_possible_truncation, reason = "clamped to i16's range first")]
    let tenth_mm = |pt: f64| (pt / 72.0 * 254.0).round().clamp(1.0, f64::from(i16::MAX)) as i16;
    if near(595.0, 842.0) {
        (DMPAPER_A4, 0, 0)
    } else if near(612.0, 792.0) {
        (DMPAPER_LETTER, 0, 0)
    } else if near(420.0, 595.0) {
        (DMPAPER_A5, 0, 0)
    } else {
        (DMPAPER_USER, tenth_mm(short), tenth_mm(long))
    }
}

/// The dialog's settings (or the defaults) with `size`'s paper and orientation.
fn devmode_for(base: Option<&DEVMODEW>, size: PrintPage) -> DEVMODEW {
    let mut devmode = base.copied().unwrap_or_default();
    devmode.dmSize = u16::try_from(size_of::<DEVMODEW>()).unwrap_or(u16::MAX);
    let (code, width, length) = paper(size);
    devmode.dmFields.0 |= DM_ORIENTATION | DM_PAPERSIZE;
    // SAFETY: the printer variant of DEVMODEW's union, the one a printer DC reads.
    unsafe {
        let printer = &mut devmode.Anonymous1.Anonymous1;
        printer.dmOrientation = if size.landscape() {
            DMORIENT_LANDSCAPE
        } else {
            DMORIENT_PORTRAIT
        };
        printer.dmPaperSize = code;
        if code == DMPAPER_USER {
            devmode.dmFields.0 |= DM_PAPERWIDTH | DM_PAPERLENGTH;
            printer.dmPaperWidth = width;
            printer.dmPaperLength = length;
        }
    }
    devmode
}

/// Where a page goes on the paper, in device pixels from the printable area's origin: the paper's
/// corner, and the page's size at the device's resolution. Actual size, so a page keeps its shape
/// on paper the driver chose for it; on its own paper, it fills it.
struct Paper {
    x: i32,
    y: i32,
    width: f64,
    height: f64,
}

fn paper_of(dc: HDC, size: PrintPage) -> Paper {
    // SAFETY: GetDeviceCaps on a live printer DC.
    let (x, y, dpi_x, dpi_y) = unsafe {
        (
            -GetDeviceCaps(Some(dc), PHYSICALOFFSETX),
            -GetDeviceCaps(Some(dc), PHYSICALOFFSETY),
            GetDeviceCaps(Some(dc), LOGPIXELSX),
            GetDeviceCaps(Some(dc), LOGPIXELSY),
        )
    };
    Paper {
        x,
        y,
        width: size.width * f64::from(dpi_x) / 72.0,
        height: size.height * f64::from(dpi_y) / 72.0,
    }
}

/// Draws `band` (BGRA) on `dc`'s page.
fn draw_band(dc: HDC, paper: &Paper, band: &Band<'_>) -> Result<(), PrintError> {
    check_band(band)?;
    let (top, bottom) = band_edges(band, paper.height);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a page at a printer's resolution fits an i32"
    )]
    let (top, bottom, dest_width) = (top.round() as i32, bottom.round() as i32, paper.width.round() as i32);
    let width = i32::try_from(band.page_width).map_err(|_| PrintError::Source { kind: "bad-band" })?;
    let rows = i32::try_from(band.rows).map_err(|_| PrintError::Source { kind: "bad-band" })?;
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: win32_size::<BITMAPINFOHEADER>(),
            biWidth: width,
            // Top-down rows.
            biHeight: -rows,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: `check_band` made sure the pixels hold width × rows × 4 bytes, as the header says.
    let lines = unsafe {
        StretchDIBits(
            dc,
            paper.x,
            paper.y + top,
            dest_width,
            bottom - top,
            0,
            0,
            width,
            rows,
            Some(band.pixels.as_ptr().cast()),
            &raw const info,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    };
    if lines == 0 {
        Err(PrintError::refused("drawing a band"))
    } else {
        Ok(())
    }
}

/// Spools the chosen pages of `source` to the chosen printer, then frees its device context. Any
/// thread: GDI's printer calls aren't tied to one.
///
/// # Errors
/// [`PrintError::Refused`] when the printer refuses the job, and the source's error when a page
/// couldn't be drawn; a cancel ends the job as [`PrintOutcome::Cancelled`].
pub fn spool(chosen: Chosen, job: &PrintJob, source: &mut dyn PrintSource) -> Result<PrintOutcome, PrintError> {
    let dc = HDC(chosen.dc as *mut c_void);
    let result = spool_on(dc, &chosen, job, source);
    // The device context goes before the wait: Microsoft Print to PDF finishes the file after.
    drop(chosen);
    if let (Ok(PrintOutcome::Printed { .. }), Some(output)) = (&result, job.output()) {
        wait_for_file(output)?;
    }
    result
}

fn spool_on(
    dc: HDC,
    chosen: &Chosen,
    job: &PrintJob,
    source: &mut dyn PrintSource,
) -> Result<PrintOutcome, PrintError> {
    let title = HSTRING::from(job.title.as_str());
    let output = job.output().map(|path| HSTRING::from(path.as_os_str()));
    let info = DOCINFOW {
        cbSize: i32::try_from(size_of::<DOCINFOW>()).unwrap_or(i32::MAX),
        lpszDocName: PCWSTR(title.as_ptr()),
        lpszOutput: output.as_ref().map_or(PCWSTR::null(), |path| PCWSTR(path.as_ptr())),
        ..Default::default()
    };
    // SAFETY: the DOCINFOW's strings outlive the call.
    if unsafe { StartDocW(dc, &raw const info) } <= 0 {
        return Err(PrintError::refused("starting the job"));
    }
    let sizes = source.pages().to_vec();
    source.start(chosen.pages.len());
    let mut printed = 0;
    for &page in &chosen.pages {
        let Some(&size) = sizes.get(page) else {
            continue;
        };
        let devmode = devmode_for(chosen.devmode.as_ref(), size);
        let drawn = (|| {
            // SAFETY: between StartDocW and EndDoc, with a complete DEVMODEW; the page is started
            // and ended once.
            unsafe {
                // A refused paper would print the page on the last one's: stop instead.
                if ResetDCW(dc, &raw const devmode).is_invalid() {
                    return Err(PrintError::refused("setting a page's paper"));
                }
                if StartPage(dc) <= 0 {
                    return Err(PrintError::refused("starting a page"));
                }
            }
            let paper = paper_of(dc, size);
            source.page_bands(page, PixelOrder::Bgra, &mut |band| draw_band(dc, &paper, band))?;
            // SAFETY: the page StartPage began.
            if unsafe { EndPage(dc) } <= 0 {
                return Err(PrintError::refused("ending a page"));
            }
            Ok(())
        })();
        if let Err(error) = drawn {
            // SAFETY: ends the job StartDocW began, dropping what was spooled.
            unsafe {
                let _ = AbortDoc(dc);
            }
            return match error {
                PrintError::Cancelled => Ok(PrintOutcome::Cancelled),
                other => Err(other),
            };
        }
        printed += 1;
    }
    // SAFETY: ends the job StartDocW began.
    if unsafe { EndDoc(dc) } <= 0 {
        return Err(PrintError::refused("finishing the job"));
    }
    Ok(PrintOutcome::Printed { pages: printed })
}

/// `EndDoc` returns before Microsoft Print to PDF has written the file: waits until its size
/// stops changing.
fn wait_for_file(path: &Path) -> Result<(), PrintError> {
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut last = None;
    while Instant::now() < deadline {
        let size = std::fs::metadata(path).map(|meta| meta.len()).ok();
        if size.is_some_and(|size| size > 0) && size == last {
            return Ok(());
        }
        last = size;
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(PrintError::refused("waiting for the PDF printer's file"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_in_should_turn_the_dialogs_ranges_into_page_indices() {
        let ranges = [
            PRINTPAGERANGE {
                nFromPage: 5,
                nToPage: 7,
            },
            PRINTPAGERANGE {
                nFromPage: 2,
                nToPage: 2,
            },
        ];
        assert_eq!(pages_in(&ranges, 50), vec![4, 5, 6, 1]);
    }

    #[test]
    fn pages_in_should_drop_pages_past_the_end() {
        let ranges = [PRINTPAGERANGE {
            nFromPage: 9,
            nToPage: 12,
        }];
        assert_eq!(pages_in(&ranges, 10), vec![8, 9]);
    }

    #[test]
    fn paper_should_name_a4_landscape_as_a4() {
        let landscape = PrintPage {
            width: 842.0,
            height: 595.0,
        };
        assert_eq!(paper(landscape).0, DMPAPER_A4);
    }

    #[test]
    fn paper_should_give_an_odd_size_in_tenths_of_a_millimetre() {
        let square = PrintPage {
            width: 720.0,
            height: 720.0,
        };
        assert_eq!(paper(square), (DMPAPER_USER, 2540, 2540));
    }
}
