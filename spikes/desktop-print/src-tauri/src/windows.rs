//! Windows: GDI for paths B and C, WebView2 for path A.
//!
//! - B: `StartDocW`, then per page `ResetDCW` (paper size and orientation), `StartPage`,
//!   `StretchDIBits` of a PDFium raster, `EndPage`.
//! - C: the same document, but PDFium's `FPDF_RenderPage` draws each page onto the printer DC
//!   (GDI drawing calls, so text and paths reach the driver as vectors where PDFium emits them).
//! - A: `ICoreWebView2_7::PrintToPdf` without a dialog; with one, `ShowPrintUI` (the system
//!   dialog). The page's `window.print()` button shows WebView2's own preview.
//!
//! Without a dialog the job goes to "Microsoft Print to PDF" with `DOCINFOW::lpszOutput` set,
//! so nothing asks for a file name. With one, `PrintDlgExW` picks the printer and pages.

use std::ffi::c_void;
use std::path::Path;
use std::sync::mpsc::channel;

use tauri::{AppHandle, Manager};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM, ICoreWebView2_7, ICoreWebView2_16,
};
use webview2_com::PrintToPdfCompletedHandler;
use windows::Win32::Foundation::{GlobalFree, HWND};
use windows::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateDCW, DEVMODEW, DIB_RGB_COLORS, DeleteDC, GetDeviceCaps, HDC,
    PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, LOGPIXELSX, ResetDCW, SRCCOPY,
    StretchDIBits,
};
use windows::Win32::Storage::Xps::{DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::UI::Controls::Dialogs::{
    PD_NOCURRENTPAGE, PD_NOSELECTION, PD_RESULT_PRINT, PD_RETURNDC, PRINTDLGEXW, PRINTPAGERANGE, PrintDlgExW,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::{Fixture, PageSize, pdf};

/// The largest raster path B sends: device resolution, capped (A4 at 600 dpi is 139 MB of BGRA).
const MAX_RASTER_DPI: f32 = 300.0;
const PDF_PRINTER: PCWSTR = w!("Microsoft Print to PDF");

// DEVMODE fields and values (wingdi.h).
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
const FPDF_ANNOT: i32 = 0x01;
const FPDF_PRINTING: i32 = 0x800;

/// The nearest standard paper for a page (portrait dimensions), or a custom size in 0.1 mm.
fn paper(size: PageSize) -> (i16, i16, i16) {
    let (short, long) = (size.w.min(size.h), size.w.max(size.h));
    let near = |w: f32, h: f32| (short - w).abs() < 3.0 && (long - h).abs() < 3.0;
    let tenth_mm = |pt: f32| (pt / 72.0 * 254.0).round() as i16;
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

/// `base` (or a blank DEVMODE) set to `size`'s paper and orientation.
fn devmode_for(base: Option<&DEVMODEW>, size: PageSize) -> DEVMODEW {
    let mut dm = base.copied().unwrap_or_default();
    dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    let (code, width, length) = paper(size);
    dm.dmFields.0 |= DM_ORIENTATION | DM_PAPERSIZE;
    // SAFETY: DEVMODEW's first union holds the printer fields for a printer DC.
    unsafe {
        let p = &mut dm.Anonymous1.Anonymous1;
        p.dmOrientation = if size.landscape() { DMORIENT_LANDSCAPE } else { DMORIENT_PORTRAIT };
        p.dmPaperSize = code;
        if code == DMPAPER_USER {
            dm.dmFields.0 |= DM_PAPERWIDTH | DM_PAPERLENGTH;
            p.dmPaperWidth = width;
            p.dmPaperLength = length;
        }
    }
    dm
}

/// The printable page's rectangle in device pixels, relative to the printable area's origin.
struct DeviceRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    dpi: f32,
}

fn device_rect(dc: HDC) -> DeviceRect {
    // SAFETY: plain queries on a valid printer DC.
    unsafe {
        DeviceRect {
            x: -GetDeviceCaps(Some(dc), PHYSICALOFFSETX),
            y: -GetDeviceCaps(Some(dc), PHYSICALOFFSETY),
            w: GetDeviceCaps(Some(dc), PHYSICALWIDTH),
            h: GetDeviceCaps(Some(dc), PHYSICALHEIGHT),
            dpi: GetDeviceCaps(Some(dc), LOGPIXELSX) as f32,
        }
    }
}

/// What draws one page onto the DC.
type DrawPage<'a> = dyn FnMut(HDC, usize, &DeviceRect) -> Result<(), String> + 'a;

/// A printer DC: from the dialog (`None` when cancelled) or Microsoft Print to PDF.
struct Target {
    dc: HDC,
    devmode: Option<DEVMODEW>,
    pages: Vec<usize>,
}

fn open_target(handle: &AppHandle, n: usize, out: Option<&Path>) -> Result<Option<Target>, String> {
    if out.is_some() {
        // SAFETY: a printer name and no initial DEVMODE.
        let dc = unsafe { CreateDCW(w!("WINSPOOL"), PDF_PRINTER, PCWSTR::null(), None) };
        if dc.is_invalid() {
            return Err("Microsoft Print to PDF is not installed".into());
        }
        let pages = match crate::page_range() {
            Some((first, last)) => (first - 1..last.min(n)).collect(),
            None => (0..n).collect(),
        };
        return Ok(Some(Target { dc, devmode: None, pages }));
    }
    let hwnd = handle
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as isize)
        .ok_or("no main window")?;
    let (tx, rx) = channel();
    let n32 = n as u32;
    handle
        .run_on_main_thread(move || {
            let mut ranges = [PRINTPAGERANGE { nFromPage: 1, nToPage: n32 }; 16];
            let mut dlg = PRINTDLGEXW {
                lStructSize: std::mem::size_of::<PRINTDLGEXW>() as u32,
                // The handle crosses threads as a number: HWND isn't Send.
                hwndOwner: HWND(hwnd as *mut c_void),
                Flags: PD_RETURNDC | PD_NOSELECTION | PD_NOCURRENTPAGE,
                nPageRanges: 1,
                nMaxPageRanges: ranges.len() as u32,
                lpPageRanges: ranges.as_mut_ptr(),
                nMinPage: 1,
                nMaxPage: n32,
                nCopies: 1,
                nStartPage: u32::MAX, // START_PAGE_GENERAL
                ..Default::default()
            };
            // SAFETY: a fully initialised PRINTDLGEXW whose range buffer outlives the call.
            let result = unsafe { PrintDlgExW(&mut dlg) }.map_err(|e| e.to_string()).and_then(|()| {
                if dlg.dwResultAction != PD_RESULT_PRINT {
                    return Ok(None);
                }
                let devmode = if dlg.hDevMode.is_invalid() {
                    None
                } else {
                    // SAFETY: the dialog returns a movable block holding a DEVMODEW.
                    unsafe {
                        let p = GlobalLock(dlg.hDevMode).cast::<DEVMODEW>();
                        let dm = (!p.is_null()).then(|| *p);
                        let _ = GlobalUnlock(dlg.hDevMode);
                        let _ = GlobalFree(Some(dlg.hDevMode));
                        dm
                    }
                };
                let pages = ranges[..dlg.nPageRanges as usize]
                    .iter()
                    .flat_map(|r| (r.nFromPage as usize - 1)..(r.nToPage as usize))
                    .filter(|&i| i < n32 as usize)
                    .collect();
                Ok(Some((dlg.hDC.0 as usize, devmode, pages)))
            });
            let _ = tx.send(result);
        })
        .map_err(|e| e.to_string())?;
    Ok(rx.recv().map_err(|e| e.to_string())??.map(|(dc, devmode, pages)| Target {
        dc: HDC(dc as *mut c_void),
        devmode,
        pages,
    }))
}

/// One document: a page per `pages` entry, each on its own paper, drawn by `draw`.
fn print_document(
    handle: &AppHandle,
    sizes: &[PageSize],
    out: Option<&Path>,
    name: &str,
    draw: &mut DrawPage<'_>,
) -> Result<(), String> {
    let Some(target) = open_target(handle, sizes.len(), out)? else {
        return Err("the print dialog was cancelled".into());
    };
    let dc = target.dc;
    let doc_name = HSTRING::from(name);
    let output = out.map(|p| HSTRING::from(p.as_os_str()));
    let info = DOCINFOW {
        cbSize: std::mem::size_of::<DOCINFOW>() as i32,
        lpszDocName: PCWSTR(doc_name.as_ptr()),
        lpszOutput: output.as_ref().map_or(PCWSTR::null(), |o| PCWSTR(o.as_ptr())),
        ..Default::default()
    };
    let result = (|| {
        // SAFETY: a valid printer DC and DOCINFOW whose strings outlive the job.
        if unsafe { StartDocW(dc, &info) } <= 0 {
            return Err("StartDocW failed".to_owned());
        }
        for &i in &target.pages {
            let dm = devmode_for(target.devmode.as_ref(), sizes[i]);
            // SAFETY: resetting a printer DC between pages, with a DEVMODEW we own.
            unsafe { ResetDCW(dc, &dm) };
            // SAFETY: page brackets on a DC inside StartDoc/EndDoc.
            if unsafe { StartPage(dc) } <= 0 {
                return Err(format!("StartPage failed on page {}", i + 1));
            }
            draw(dc, i, &device_rect(dc))?;
            // SAFETY: as above.
            if unsafe { EndPage(dc) } <= 0 {
                return Err(format!("EndPage failed on page {}", i + 1));
            }
        }
        // SAFETY: closes the document StartDocW opened.
        unsafe { EndDoc(dc) };
        Ok(())
    })();
    // SAFETY: the DC is ours to delete.
    unsafe {
        let _ = DeleteDC(dc);
    }
    result
}

pub fn print_raster(handle: &AppHandle, fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let doc = pdf::open(&fixture.bytes)?;
    print_document(handle, &fixture.sizes, out, fixture.name, &mut |dc, i, r| {
        let raster = pdf::render(&doc, i as u16, r.dpi.min(MAX_RASTER_DPI))?;
        let bgra = raster.bgra();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: raster.width as i32,
                biHeight: -(raster.height as i32), // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        // SAFETY: bgra holds width × height × 4 bytes as the header describes.
        let lines = unsafe {
            StretchDIBits(
                dc,
                r.x,
                r.y,
                r.w,
                r.h,
                0,
                0,
                raster.width as i32,
                raster.height as i32,
                Some(bgra.as_ptr().cast()),
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            )
        };
        if lines == 0 { Err(format!("StretchDIBits failed on page {}", i + 1)) } else { Ok(()) }
    })
}

/// The five PDFium entry points path C calls directly. pdfium-render 0.9.4 keeps its raw
/// bindings and handles private and refuses a second binding, so they come from the DLL it
/// already loaded and initialised (`GetProcAddress`); D4c would do the same in its own crate.
struct Raw {
    load_mem_document64: unsafe extern "C" fn(*const c_void, usize, *const i8) -> *mut c_void,
    load_page: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void,
    render_page: unsafe extern "C" fn(HDC, *mut c_void, i32, i32, i32, i32, i32, i32),
    close_page: unsafe extern "C" fn(*mut c_void),
    close_document: unsafe extern "C" fn(*mut c_void),
}

fn raw() -> Result<Raw, String> {
    pdf::pdfium()?; // loads and initialises pdfium.dll
    // SAFETY: GetModuleHandleW on a loaded module; each symbol is cast to its C signature
    // from fpdfview.h.
    unsafe {
        let module = GetModuleHandleW(w!("pdfium.dll")).map_err(|e| format!("pdfium.dll: {e}"))?;
        let sym = |name: &[u8]| {
            GetProcAddress(module, windows::core::PCSTR(name.as_ptr())).ok_or_else(|| {
                format!("pdfium.dll has no {}", String::from_utf8_lossy(&name[..name.len() - 1]))
            })
        };
        Ok(Raw {
            load_mem_document64: std::mem::transmute(sym(b"FPDF_LoadMemDocument64\0")?),
            load_page: std::mem::transmute(sym(b"FPDF_LoadPage\0")?),
            render_page: std::mem::transmute(sym(b"FPDF_RenderPage\0")?),
            close_page: std::mem::transmute(sym(b"FPDF_ClosePage\0")?),
            close_document: std::mem::transmute(sym(b"FPDF_CloseDocument\0")?),
        })
    }
}

pub fn print_vector(handle: &AppHandle, fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let raw = raw()?;
    // SAFETY: the bytes outlive the document, which is closed before returning.
    let doc = unsafe { (raw.load_mem_document64)(fixture.bytes.as_ptr().cast(), fixture.bytes.len(), std::ptr::null()) };
    if doc.is_null() {
        return Err("PDFium can't open the file".into());
    }
    let result = print_document(handle, &fixture.sizes, out, fixture.name, &mut |dc, i, r| {
        // SAFETY: a page of the open document, rendered onto the DC inside StartPage/EndPage
        // and closed at once.
        unsafe {
            let page = (raw.load_page)(doc, i as i32);
            if page.is_null() {
                return Err(format!("PDFium can't load page {}", i + 1));
            }
            (raw.render_page)(dc, page, r.x, r.y, r.w, r.h, 0, FPDF_ANNOT | FPDF_PRINTING);
            (raw.close_page)(page);
        }
        Ok(())
    });
    // SAFETY: closes the document opened above.
    unsafe { (raw.close_document)(doc) };
    result
}

pub fn print_webview(handle: &AppHandle, _fixture: &Fixture, out: Option<&Path>) -> Result<(), String> {
    let window = handle.get_webview_window("main").ok_or("no main window")?;
    let out = out.map(|p| HSTRING::from(p.as_os_str()));
    let (tx, rx) = channel::<Result<(), String>>();
    window
        .with_webview(move |webview| {
            let result = (|| -> Result<(), String> {
                // SAFETY: COM calls on the UI thread on wry's live controller.
                unsafe {
                    let core = webview.controller().CoreWebView2().map_err(|e| e.to_string())?;
                    match &out {
                        Some(path) => {
                            let core7: ICoreWebView2_7 = windows::core::Interface::cast(&core).map_err(|e| e.to_string())?;
                            let done = tx.clone();
                            let handler = PrintToPdfCompletedHandler::create(Box::new(move |hr, ok| {
                                let _ = done.send(match (hr, ok) {
                                    (Ok(()), true) => Ok(()),
                                    (Ok(()), false) => Err("PrintToPdf reported failure".into()),
                                    (Err(e), _) => Err(e.to_string()),
                                });
                                Ok(())
                            }));
                            core7.PrintToPdf(PCWSTR(path.as_ptr()), None, &handler).map_err(|e| e.to_string())?;
                            return Ok(()); // the handler reports the result
                        }
                        None => {
                            let core16: ICoreWebView2_16 = windows::core::Interface::cast(&core).map_err(|e| e.to_string())?;
                            core16.ShowPrintUI(COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM).map_err(|e| e.to_string())?;
                        }
                    }
                }
                let _ = tx.send(Ok(()));
                Ok(())
            })();
            if let Err(e) = result {
                let _ = tx.send(Err(e));
            }
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}
