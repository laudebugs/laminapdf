# Spike: `desktop-print` (Round D1, todo D1b)

- **Date:** 2026-11-09
- **Status:** done. The recommendation below is an input to ADR 0008 (todo [D2a](../../docs/todos/ready/D2a-adr-0008-and-desktop-scaffold.md)) and to native printing (todo [D4c](../../docs/todos/ready/D4c-native-printing.md)).
- **Plan:** [desktop app §6 D1](../../docs/plans/desktop-app.md#d1-ipc-and-print-spikes), [architecture §4](../../docs/plans/architecture.md#4-spikes-to-close-open-questions) (`desktop-print`)
- **Code:** throwaway quality, outside both workspaces, never imported by product code ([code organisation §12](../../docs/guidelines/code-organization.md#12-spikes-vs-product-code)). Tauri 2.12 (crate `tauri` 2.12.5, `@tauri-apps/cli` 2.12.4), `pdfium-render` 0.9.4 over PDFium `chromium/7881`, Rust 1.99.1.

## Question

Tauri has no print API beyond printing the webview. How should Lamina PDF's desktop app print a PDF **faithfully** on macOS, Windows and Linux: every page at its own size and orientation, no browser headers or margins, with the OS dialog's page range, copies and duplex? Three paths were candidates:

- **A, webview:** print a print-only DOM of 300 dpi page images with named `@page` sizes, as the web reader does (C6c).
- **B, native raster:** draw PDFium rasters through the OS print API (AppKit, GDI, GTK).
- **C, native vector:** hand the page to a vector path: PDFKit on macOS, PDFium's `FPDF_RenderPage` onto the printer's device context on Windows, CUPS with the original bytes on Linux.

## Answer

**Print with Lamina PDF's own rasters through each OS's print API (path B).** It is the only path that works on all three OSes, and on Windows and Linux it is also faithful. There, every page of every fixture printed at its own size and orientation (all four pages of the mixed-size fixture, including US Letter landscape and A5), with no content offset, SSIM 0.96–0.999 against the original, and no added text.

**macOS falls short on placement and mixed page sizes.** On macOS, path B printed every page of the 50-page A4 fixture at the right size, but every even page landed 17.8 pt low, even after correcting for the printer's imageable area. AppKit also snapped the US Letter and A5 pages to A4 (two of four sizes right).

**PDFium's vector path on Windows was the most faithful result of all, but it can't ship as it stands.** `FPDF_RenderPage` onto the printer's device context scored SSIM 0.98–0.997 with every size right, in a 17 MB file against path B's 42 MB. It parses the untrusted PDF in the main process, though, which ADR 0008's sandboxed engine rules out. The other vector paths lost page sizes: PDFKit put every page on A4 portrait and moved the content 18 pt, and CUPS printed on the printer's default paper.

**The webview path is the worst everywhere and shouldn't ship, even as a fallback.** WebView2's `PrintToPdf` put every page on US Letter (no page of the 50 sized right; SSIM 0.79). WebKitGTK split every page across two sheets (100 sheets for 50 pages). WKWebView printed uniform A4 pages well (SSIM 0.96) but forced every page of the mixed-size fixture onto A4 portrait. In a Tauri app on macOS, `window.print()` does nothing at all. The webview also peaked at 1.4–2.3 GB while holding 50 page images.

## Recommendation (inputs to D4c and ADR 0008)

| Decision | Recommendation | The numbers behind it |
| --- | --- | --- |
| Shipping path | **B on every OS:** Lamina PDF's PDFium rasters at the printer's resolution (capped at 300 dpi) through `NSPrintOperation`, GDI and `GtkPrintOperation`, with the OS print dialog | Windows and Linux: 4/4 mixed sizes, no page moved more than 2 pt, SSIM ≥ 0.96. macOS, 50 A4 pages: SSIM ≥ 0.943 |
| Who renders | The sandboxed engine renders each print page and streams it to the main process, which owns the printer handle (D4c, on D2b's protocol) | Path B needs no PDF parsing outside the engine; path C does |
| Webview fallback | **None.** Don't ship path A first, and don't keep it as a fallback | WebView2: 0/50 pages at the right size; WebKitGTK: 100 sheets for 50 pages; WKWebView: `window.print()` does nothing |
| Windows vector upgrade | Path C (`FPDF_RenderPage` onto the printer DC) later, only behind a sandboxed print helper or an owner-accepted ADR 0008 exception | SSIM 0.98–0.997, 4/4 sizes, 17 MB against 42 MB for 50 pages |
| macOS work for D4c | Per-page paper sizes (AppKit snapped US Letter and A5 to A4), the even-page offset (17.8 pt), and memory: AppKit kept every drawn page until the job ended | 2/4 mixed sizes right; 25 of 50 pages moved; 3.3 GB peak for 50 pages, against 127 MB with GDI |
| D4c's fidelity bar | Print-to-PDF output at ≥ 0.94 SSIM (150 dpi, after aligning), each page's size within 2 pt and its content within 2 pt of place | The lowest path B score on a correctly placed page: 0.943 (macOS); 0.96 on Windows and Linux |
| `hasNativePrint` | `true` on all three OSes, meaning: the OS print dialog (printer, page range, copies, and the driver's duplex and scaling); every page rendered by Lamina PDF's own renderer at print resolution, on paper of its own size and orientation; no browser headers, footers or margins. It doesn't promise vector output | ADR 0010's table says the flag means "print can hand the PDF to the OS print system". Path B doesn't hand over the PDF, so ADR 0008 or D4c should restate the flag this way |

ADR 0008 should record that printing renders in the engine and prints rasters through each OS's API, that no webview print path ships, and that a vector path needs a sandboxed helper first.

## Method

**Fixtures** (`fixtures/`, written by `src-tauri/src/bin/make_fixtures.rs` with Lamina PDF's own builder; byte-identical on every run):

- `mixed-50.pdf`: 50 A4 pages of text, a colour band and a chart, with a photo on every fifth page. Each page's number is printed large, so a page range is easy to check.
- `mixed-sizes.pdf`: A4 portrait, US Letter landscape, A4 with `/Rotate 90` (displayed landscape) and A5 portrait, each with a border 18 pt inside the edge and red corner squares, so clipping and scaling show.
- `vectors.pdf`: hairlines (width 0) and 0.1–1 pt lines, a 2 pt grid of hairlines, concentric circles 3 pt apart, and text at 4–8 pt.

**The app** (`src-tauri/`, `src/`) prints each fixture through each path. Its `auto` mode writes every job to a PDF with no dialog:

| Path | macOS | Windows | Linux |
| --- | --- | --- | --- |
| A webview | WKWebView's `printOperationWithPrintInfo:`, run modally on its window, saving to a file | WebView2's `ICoreWebView2_7::PrintToPdf` (default settings) | WebKitGTK's `PrintOperation::print()` to GTK's "Print to File" printer |
| B native raster | `NSPrintOperation` over an `NSView` that draws a 300 dpi PDFium raster per page; each page's paper size set in `beginPageInRect:atPlacement:` | `StartDocW` to "Microsoft Print to PDF", per page `ResetDCW` (paper and orientation), `StretchDIBits` of a raster at the device's resolution, capped at 300 dpi | `gtk::PrintOperation` with a custom paper per page and a `draw-page` handler painting the raster through cairo, to "Print to File" |
| C native vector | PDFKit's `printOperationForPrintInfo:scalingMode:autoRotate:` (Apple's renderer) | `FPDF_RenderPage` onto the printer DC (GDI calls), taken from the loaded `pdfium.dll` with `GetProcAddress` | `lp` to a CUPS `cups-pdf` printer with the original bytes |

A page-range job prints pages 3–7 of `mixed-50` through path B, setting the range where the dialog would: `NSPrintFirstPage`/`NSPrintLastPage` on macOS, the page list `PrintDlgExW` returns on Windows, GTK print settings on Linux.

**The comparison** (`src-tauri/src/bin/compare.rs`) opens each printed PDF and its fixture in PDFium and, page by page, records:

- **Page count and size:** the printed page's size against the original's (within 2 pt).
- **Offset:** how far the content moved, found by lining up rows and columns of ink (up to ±29 pt).
- **SSIM:** both pages rendered at 150 dpi at the original's pixel size, scored after undoing the offset (`laminapdf-test-fixtures`' SSIM: luma, 8 × 8 windows).
- **Added text:** words of two or more letters or digits on the printed page that the original document never contains (a header or footer).

`spool s` is from the print call to a finished file; peak memory is the app's and its webview processes' RSS, sampled every 100 ms. Each run then opens every path's print dialog and takes a screenshot (`results/dialogs/`).

## Environment

| Label | Machine | Webview | Printer |
| --- | --- | --- | --- |
| `macos-m1pro` | Apple M1 Pro, 16 GiB; macOS 27.0.1 | WKWebView (macOS 27.0.1) | Save as PDF |
| `macos-15-hosted` | GitHub `macos-15`: Apple M1 (virtual), 3 cores; macOS 15.8.2 | WKWebView (macOS 15.8.2) | Save as PDF |
| `windows-2025` | GitHub `windows-2025`: AMD EPYC 7763, 4 cores; Windows Server 2025 (26100) | WebView2 156.0.4314.58 | Microsoft Print to PDF |
| `linux-x11` | GitHub `ubuntu-24.04`: AMD EPYC, 4 cores; Ubuntu 24.04.5 on Xvfb | WebKitGTK 2.52.6, DMABUF renderer off | GTK "Print to File"; CUPS 2.4 with `cups-pdf` |

No physical printer was available (see [Limits](#limits-of-this-spike)).

## Results

Each row is one path and fixture on one machine. **right size**: pages printed at the original's size and orientation (within 2 pt); a note gives the count when the number of pages printed was wrong. **moved > 2 pt**: pages whose content sat more than 2 pt from its place. **SSIM**: minimum and median over the pages, after undoing the offset. **added text**: words a page gained (a header or footer); none appeared on any path. Spool time, peak memory and file size are for the 50-page fixture and the page-range job (pages 3–7, `mixed-50@3-7`). `results/summary.md` holds the same tables; the per-page rows are in `results/<label>-fidelity.csv`.

#### linux-x11

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 50/50 (100 pages printed for 50) | 50 | 0.693 / 0.746 | no | 13.6 | 2019 / 2053 | 30.9 |
| A webview | mixed-sizes | 1/4 | 4 | 0.912 / 0.923 | no |  |  |  |
| A webview | vectors | 1/1 (2 pages printed for 1) | 1 | 0.818 / 0.818 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 0 | 0.961 / 0.964 | no | 13.5 | 255 / 198 | 30.2 |
| B native raster | mixed-sizes | 4/4 | 0 | 0.997 / 0.998 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.960 / 0.960 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 0 | 0.952 / 0.952 | no | 8.3 | 391 / 519 | 4.0 |
| C native vector | mixed-50 | 50/50 | 0 | 0.949 / 0.951 | no | 1.3 | 186 / 198 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 2 | 0.900 / 0.936 | no |  |  |  |
| C native vector | vectors | 1/1 | 0 | 0.911 / 0.911 | no |  |  |  |

#### macos-15-hosted

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 50/50 | 0 | 0.958 / 0.959 | no | 20.1 | 1978 / 2307 | 37.8 |
| A webview | mixed-sizes | 1/4 | 4 | 0.909 / 0.925 | no |  |  |  |
| A webview | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 25 | 0.943 / 0.952 | no | 14.0 | 3491 / 41 | 37.8 |
| B native raster | mixed-sizes | 2/4 | 3 | 0.894 / 0.957 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 2 | 0.943 / 0.958 | no | 2.3 | 1114 / 741 | 3.8 |
| C native vector | mixed-50 | 50/50 | 50 | 0.936 / 0.938 | no | 0.7 | 218 / 65 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 4 | 0.895 / 0.911 | no |  |  |  |
| C native vector | vectors | 1/1 | 1 | 0.872 / 0.872 | no |  |  |  |

#### macos-m1pro

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 50/50 | 0 | 0.961 / 0.964 | no | 11.6 | 1737 / 1582 | 37.8 |
| A webview | mixed-sizes | 1/4 | 4 | 0.909 / 0.927 | no |  |  |  |
| A webview | vectors | 1/1 | 0 | 0.957 / 0.957 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 25 | 0.943 / 0.952 | no | 8.8 | 3287 / 60 | 37.8 |
| B native raster | mixed-sizes | 2/4 | 3 | 0.894 / 0.957 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.954 / 0.954 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 2 | 0.943 / 0.958 | no | 1.3 | 714 / 216 | 3.8 |
| C native vector | mixed-50 | 50/50 | 50 | 0.936 / 0.938 | no | 0.6 | 224 / 60 | 0.1 |
| C native vector | mixed-sizes | 1/4 | 4 | 0.895 / 0.911 | no |  |  |  |
| C native vector | vectors | 1/1 | 1 | 0.872 / 0.872 | no |  |  |  |

#### windows-2025

| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |
| --- | --- | --: | --: | --: | --- | --: | --: | --: |
| A webview | mixed-50 | 0/50 | 50 | 0.782 / 0.797 | no | 15.6 | 1405 / 861 | 31.1 |
| A webview | mixed-sizes | 1/4 | 3 | 0.887 / 0.946 | no |  |  |  |
| A webview | vectors | 0/1 | 1 | 0.863 / 0.863 | no |  |  |  |
| B native raster | mixed-50 | 50/50 | 0 | 0.979 / 0.982 | no | 31.4 | 127 / 286 | 41.6 |
| B native raster | mixed-sizes | 4/4 | 0 | 0.994 / 0.999 | no |  |  |  |
| B native raster | vectors | 1/1 | 0 | 0.976 / 0.976 | no |  |  |  |
| B native raster | mixed-50@3-7 | 5/5 | 0 | 0.979 / 0.981 | no | 3.8 | 130 / 517 | 4.6 |
| C native vector | mixed-50 | 50/50 | 0 | 0.980 / 0.980 | no | 50.5 | 63 / 292 | 16.6 |
| C native vector | mixed-sizes | 4/4 | 0 | 0.998 / 0.999 | no |  |  |  |
| C native vector | vectors | 1/1 | 0 | 0.997 / 0.997 | no |  |  |  |

### Print dialogs

Every run opened each path's dialog and took a screenshot after 12 s (`results/dialogs/`).

| OS | Paths B and C | Path A from Rust | Path A, `window.print()` |
| --- | --- | --- | --- |
| macOS 15 | AppKit's print panel: printer, presets, copies, a page range (all, from–to, selection), a page preview, Layout (pages per sheet; two-sided where the printer has it) and the PDF menu | No panel within 12 s | No panel: WKWebView ignores it in a Tauri app |
| Windows Server 2025 | `PrintDlgExW`: printer, page range (all, pages), copies, collate; duplex and scaling sit in the driver's Preferences | WebView2's system dialog: the same | A print dialog opened behind the app window, and the page went blank while the print DOM was built |
| Ubuntu 24.04 | GTK's print dialog. The Xvfb screenshots came out blank, so this column is from GTK's documentation: a General tab (printer, page range, copies, collate, reverse) and a Page Setup tab (two-sided, pages per side, scale, paper size, orientation) ([GtkPrintUnixDialog](https://docs.gtk.org/gtk3/class.PrintUnixDialog.html)) | Not captured (blank screenshot) | Not captured (blank screenshot) |

The macOS 27 screenshots were black too: the terminal had no Screen Recording permission. macOS 15 shows the same AppKit panel.

## Observations per OS

- **macOS:** the three paths share AppKit's print operation, and with it two problems. First, AppKit places a page at the printer's imageable area, not at the paper's corner: content moved 18 pt down and right until path B started each page's rectangle that far into the page, which fixed odd pages but left even pages 17.8 pt low. Second, the paper size set per page in `beginPageInRect:atPlacement:` changed the orientation but snapped US Letter and A5 to A4. PDFKit (C) spooled 50 pages in 0.6 s as a 0.1 MB file, but with Apple's renderer, every page on A4 portrait, and all 50 pages offset. Path B held every drawn page until the job ended: 3.3 GB at peak for 50 pages at 300 dpi.
- **Windows:** GDI is the most faithful platform for both native paths. Microsoft Print to PDF took each page's paper and orientation from `ResetDCW` (all four mixed sizes right, including A5 and US Letter landscape), and the main process stayed near 130 MB for path B because each page is drawn and released. Path C's GDI output took 50 s to spool, against 31 s for path B, in a smaller file. WebView2's `PrintToPdf` with default settings ignored the named `@page` sizes and put every page on US Letter.
- **Linux:** GTK honoured a custom paper per page (4/4 sizes right, SSIM 0.997–0.999), and with Export it spooled 50 pages in 13.5 s at 255 MB. Export ignores the print settings, so it printed all 50 pages for a 3–7 range; through GTK's print pipeline to the CUPS PDF printer, the same range printed exactly pages 3–7. Printing to GTK's own "Print to File" printer without a dialog wrote nothing. WebKitGTK split each page image across two sheets (100 sheets for 50 pages), and CUPS printed the original bytes on its default A4 whatever the page size.

Five things the docs don't say, or that needed care:

- **WebKit's print operation must run modally.** WKWebView's `printOperationWithPrintInfo:` run with the synchronous `runOperation` never finished: a 4-page document grew past 5 GB in 15 minutes. `runOperationModalForWindow:delegate:didRunSelector:contextInfo:` finished in seconds.
- **`window.print()`** does nothing in WKWebView inside a Tauri app; WebView2 and WebKitGTK open a dialog.
- **PDFium's raw API from Rust:** `pdfium-render` 0.9.4 keeps its bindings and document handles private and refuses a second binding (`PdfiumLibraryBindingsAlreadyInitialized`). Path C on Windows takes the five functions it needs from the loaded `pdfium.dll` with `GetProcAddress`; D4c would put that in its own edge crate.
- **Main-thread rules:** AppKit and GTK print only from the main thread. Each call hops there with `AppHandle::run_on_main_thread` and waits on a channel; GTK and Win32 handles cross threads only as numbers or as values looked up on the main thread.
- **Spooling isn't instant on Windows.** `EndDoc` returns before Microsoft Print to PDF finishes the file, so the job waits for the file's size to settle.

## Limits of this spike

- **No physical printer.** Every result is print-to-PDF: Save as PDF on macOS, Microsoft Print to PDF on Windows, GTK's file output and `cups-pdf` on Linux. Two things stay unverified: duplex, and how a real printer's unprintable margins clip a page printed at actual size. D4c's acceptance already asks for a recorded print on a physical printer.
- **Dialogs were driven only by script.** No one pressed Print in a dialog. The page ranges were set where the dialog would set them, and the dialogs themselves were checked by screenshot, except on Linux, where the Xvfb screenshots came out blank.
- **Cloud machines.** Windows and Linux ran on 4-core VMs with virtual displays and PDF printers; a real driver may handle `ResetDCW` paper changes or GTK's custom papers differently.
- **Generated fixtures.** Three files from Lamina PDF's builder: no scanned pages, forms, annotations or transparency groups, which are where Apple's renderer (PDFKit) and PDFium differ most.
- **Raster resolution.** Path B printed at 300 dpi; a 600 dpi printer gets an upscaled raster, and D4c should measure 600 dpi's memory before raising the cap.
- **The even-page offset on macOS** is unexplained: odd pages land exactly, even pages 17.8 pt low, on both Macs.

## Reproduce

Needs Rust (the root `rust-toolchain.toml`), Node 22+ and pnpm; on Linux also `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`, Xvfb, `x11-apps`, ImageMagick, and CUPS with `printer-driver-cups-pdf`.

```sh
cargo xtask fetch-pdfium                       # prints LAMINA_PDF_PDFIUM_DIR=…
export LAMINA_PDF_PDFIUM_DIR=…
cd spikes/desktop-print
scripts/run-all.sh <label>                     # macOS, Linux (under xvfb-run); Windows: scripts\run-all.ps1 <label>
python3 scripts/summarise.py results/ > results/summary.md
```

`run-all` builds the app and the comparison tool, prints every fixture through every path into `results/out/` (not committed: path A's files run to tens of megabytes), scores them into `results/<label>-fidelity.csv`, then screenshots each path's dialog. `src-tauri/target/release/desktop-print` with no `LAMINA_SPIKE_MODE` opens the window with a button per path, for a print through the real dialog. `cargo run --release --bin make_fixtures -- <repo-root> fixtures` rewrites the fixtures.

The hosted runs came from a temporary workflow on the spike's branch, deleted before merge. Spikes are not in CI.
