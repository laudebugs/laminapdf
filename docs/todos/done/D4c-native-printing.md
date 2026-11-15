# D4c — Native printing per the `desktop-print` spike

| Field | Value |
| --- | --- |
| ID | `D4c` |
| Status | `done` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C6c](./C6c-print-download-properties.md), [D1b](./D1b-desktop-print-spike.md), [D4b](./D4b-native-menus.md) |
| Dependency notes | [D4b](../done/D4b-native-menus.md) (Print menu item), [D1b](../done/D1b-desktop-print-spike.md) (recommended path). Cross-plan: web **C6** ([web-app §C6](../../plans/web-app.md#c6-navigation-print-and-properties)) only if D1b recommends the webview raster path, which reuses C6's print-at-print-DPI code. |
| ADRs | Follows ADR 0008 (engine renders, main owns printer handles), ADR 0010 (`hasNativePrint`) |
| Branch | `feat/d4c-native-printing` |
| PR title | `feat(desktop): print documents through the native print path on each OS` |
| Size | L (600–1,200 lines, per-OS code) |
| PR | [laudebugs/laminapdf#620](https://github.com/laudebugs/laminapdf/pull/620) |

## Goal

Ship faithful printing on macOS, Windows and Linux using the path D1b recommended per OS: the native print dialog with page range, copies, scaling and duplex where the OS provides it; correct page sizes and orientation per page; no browser headers or footers. Requirement 7 of the [desktop plan](../../plans/desktop-app.md#functional); D4 is accepted when "a 50-page document prints with page ranges on each OS (manual check recorded)".

## Context

- **Design (Paper):** [S05 Print](../../design/paper/desktop-shell.md#s05-print). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- D1b README (`spikes/desktop-print/`): per-OS recommendation, fidelity numbers, platform-code notes (main-thread rules, banding), and the comparison script.
- [Desktop plan §7](../../plans/desktop-app.md#7-risks): ship the `window.print()` raster path first if native lags.
- ADR 0008: the engine renders; the main process owns OS handles (printer DC, `NSPrintOperation`, `GtkPrintOperation`) because the engine is sandboxed (D2c) and has no access to printers.
- Web C6's print path (if the webview raster path is used anywhere).
- APIs per D1b, restricted to the raster paths by the decision below: `NSPrintOperation` + custom `NSView` (macOS), `PrintDlgEx` + GDI `StretchDIBits` (Windows), `GtkPrintOperation` + cairo (Linux).

## Scope

**In scope**

- `src-tauri/src/print/{mod.rs,macos.rs,windows.rs,linux.rs}` implementing `fn print_document(app: &AppHandle, window: &WebviewWindow, doc: DocId, opts: PrintRequest) -> Result<PrintOutcome, PrintError>` with the D1b-chosen dialog/spooling API per OS.
- **Decision: raster printing from the engine on every OS.** ADR 0008 says only the sandboxed engine parses untrusted PDF bytes. So the main process never hands the PDF to PDFKit (`PDFDocument`/`PDFView` printing on macOS), to PDFium on a printer DC (Windows), or to a parser of its own: the engine renders each page to a bitmap at the printer's DPI (`RenderPrintPage`) and the main process only draws that bitmap into the OS print context (`NSPrintOperation` with a custom `NSView` on macOS, the printer DC via `StretchDIBits` on Windows, cairo in `GtkPrintOperation` on Linux), checking the dimensions and byte length first. If D1b recommended a vector path (macOS PDFKit, Windows PDFium-on-DC or EMF playback, Linux raw PDF to CUPS), it is **not** taken in this PR: it would need either a separate sandboxed print helper (follow-up todo) or an exception to ADR 0008 that the owner explicitly accepts in the PR. Cost: bigger spool files and no text selectable in print-to-PDF output; fidelity matches the on-screen renderer.
- Engine protocol additions (in `crates/laminapdf-desktop-protocol`): `RenderPrintPage { doc, page, dpi, format: Rgba | Bgra }` streaming pages on demand as the OS print loop asks for them (never all pages in memory; large pages rendered in bands), cancellation when the user cancels the print job. No EMF/XPS format: playing engine-produced metafiles in the main process would hand it bytes a compromised engine controls.
- Print options surfaced by the OS dialog: page range (all, current, range), copies, collation, scaling (fit to paper / actual size), orientation auto per page, duplex when the printer supports it.
- Progress UI: a non-modal progress indicator in the window while spooling, with Cancel.
- Fallback: the webview raster path (`WebviewWindow::print()` with the C6 print DOM) selectable in settings for troubleshooting, and automatically offered if the native path fails.
- `hasNativePrint: true` semantics as defined by D1b, and the Print menu item / `CmdOrCtrl+P` enabled when a document is open.
- Manual print checks recorded in the PR per OS (template below) using the D1b comparison script for print-to-PDF fidelity.

**Out of scope** (and which todo covers it)

- Print preview inside lamina (the OS dialogs' previews suffice for now).
- Printing annotations/forms state beyond what the renderer draws (editor plan).
- Booklet/n-up printing (later).

## Tasks

- [x] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
- [x] Port the D1b spike's chosen code paths into `src-tauri/src/print/`, replacing spike shortcuts (`unwrap`, blocking on the main thread) with proper error handling and `run_on_main_thread` where APIs require it; every FFI `unsafe` with `// SAFETY:`.
- [x] Engine `RenderPrintPage` message + streaming with back-pressure; protocol version bump; tests. (Built as `RenderPrintBand`: the main process asks for one band at a time.)
- [x] `print` command (manifest + capability) invoked from the menu event and the toolbar's print button via `Platform.print()`. (Built as `print_document`, run by the `file.print` command's desktop handler; `Platform.print` stays the web reader's path.)
- [x] Progress + cancel UI in `@laminapdf/ui` (shared component; web uses it for C6's print preparation too if useful). (Built as a status-bar item from `@laminapdf/ui`'s `ProgressBar` and `StatusBarItem`, as S05-print-progress draws it; see Found while building it.)
- [ ] ~~Settings: "Print using: system (recommended) / compatibility (webview)".~~ Not built: D1b measured the webview path as the worst on every OS and recommended no fallback (see Found while building it).
- [x] Fixtures: reuse D1b's three fixtures (50-page, mixed sizes, vector-heavy).
- [x] Manual print checklist `apps/desktop/e2e/MANUAL-PRINT.md` with a results table: OS, version, printer/driver, fixture, range, duplex, result, SSIM vs reference (print-to-PDF).

## Found while building it

- **No webview fallback.** The Scope above asked for one, selectable in settings. D1b recommended against it, and this todo's own Risks section repeats that: WebView2's `PrintToPdf` put 0 of 50 pages at the right size, WebKitGTK printed 100 sheets for 50 pages, and `window.print()` does nothing in WKWebView inside a Tauri app. No settings choice, and no `WebviewWindow::print()` path.
- **The engine picks the band height.** `RenderPrintBand { doc, page, dpi, band, format }` asks for a band by number. The reply carries the page's size in pixels, the band's rows and the number of bands, so the main process never repeats the renderer's rounding. A band is at most 4 Mi pixels (16 MiB), within the renderer's per-tile limit. A page too wide to render at the asked resolution prints at a lower one, which the reply states. Bands of a letter page at 600 dpi equal one whole render to within one level: PDFium rounds a few hundred anti-aliased bytes differently at another origin.
- **macOS prints through Core Printing, not `NSPrintOperation`.** `NSPrintOperation` keeps one page format for the whole job, so the new print test reproduced D1b's finding: US Letter and A5 snapped to A4. Setting each page's paper size, or its paper name, didn't help. A Core Printing session gives each page its own validated format, and the formats read back right: 792 × 612 for Letter landscape, 420 × 595 for A5. But the print system still puts every page on the job's paper in its own orientation ([#626](https://github.com/laudebugs/laminapdf/issues/626), with options). The gain: macOS spools off the main thread like Windows, so the webview shows the progress and Cancel there. The imageable offset comes from each page's own format (`PMGetAdjustedPaperRect`), which settles D1b's 17.3 pt landscape offset.
- **Progress in the status bar.** Progress shows as S05-print-progress draws it: the status bar's "Printing · 3 of 8 sent", a 140 px bar and Cancel printing, so reading goes on. It appears once the user has chosen the pages, at 0 of their count, and counts only those pages. A toast says how the job ended.
- **No "Current page" on macOS.** AppKit's print panel offers All, a range and Selection, but no current page, so only Windows and Linux offer the page on screen.
- **Actual size, no fit to paper.** Each page prints at its own size from the paper's corner. Windows maps it at the device's resolution, so a driver that puts it on other paper keeps its shape rather than stretching it. Scaling to fit is left to the OS dialog where it offers one.
- **The OS code lives in `laminapdf-desktop-os`.** The Scope put it in `src-tauri/src/print/`. It went in the crate that already holds the desktop's other OS code, where it builds and tests without Tauri. The app's `src/print.rs` holds the commands and the engine source.
- **Cancel drops what was spooled.** A cancel stops before the next band. On macOS it marks the session cancelled before ending it, and on Windows `AbortDoc` ends the job, so the pages already spooled never reach the printer.
- **Where each part runs.** The dialog runs on the main thread. On macOS and Windows the pages spool on a worker thread. GTK prints only from the main thread, so on Linux the webview waits while a page is drawn, and GTK's progress window offers Cancel.
- **Tests that drive the real OS path.** `laminapdf-desktop-os/tests/print_to_file.rs` has its own `main()`, so AppKit gets the main thread, and it answers nextest's `--list`. It prints four page sizes to the OS's PDF printer on the macOS and Windows Rust legs and reads the file back with Lamina PDF.

## Acceptance criteria

- [ ] On macOS, Windows and Linux, printing the 50-page fixture with a page range (e.g. 5–12) via the native dialog produces exactly those pages at the correct size and orientation (manual check recorded per OS in the PR, at least one on a physical printer).
- [ ] Mixed page sizes print each page at its own size/orientation (print-to-PDF check, SSIM ≥ the threshold set in D1b). Windows: the print test; Linux: the e2e scenario; macOS: orientation only ([#626](https://github.com/laudebugs/laminapdf/issues/626)).
- [x] No browser headers/footers/margins appear in native-path output.
- [ ] Cancelling mid-spool stops engine rendering within 1 s and leaves the app responsive.
- [ ] Peak main-process memory while printing the 50-page fixture stays within 2× the single-page raster size at print DPI (streaming, not buffering).
- [ ] ~~The compatibility path works on each OS as a fallback (manual check).~~ No compatibility path (see Found while building it).

## Post-merge checks

- [ ] The owner records the manual checks of `apps/desktop/e2e/MANUAL-PRINT.md` on each OS, at least one on a physical printer: a 5–12 page range of the 50-page fixture, mixed sizes, Cancel mid-spool (within 1 s, the app responsive), copies and duplex, and the main process's peak memory. No runner has a printer or someone to press Print.
- [ ] The owner decides [#626](https://github.com/laudebugs/laminapdf/issues/626): macOS's mixed page sizes.

## Tests

- Rust unit tests: `PrintRequest` → page list (ranges, current page, reverse order if requested), DPI and scaling maths, per-page orientation selection.
- Engine protocol tests for `RenderPrintPage` and cancellation.
- e2e: print to the OS PDF printer is not automatable across OSes reliably; add a test-only `e2e_print_to_file(path)` (feature `e2e`) that runs the same pipeline into a PDF/raster sink and compares page count and sizes.
- Manual checklist (above).

## Docs to update

- `docs/plans/desktop-app.md` §9 D4: tick "Implement printing per the D1 recommendation" and "Record a manual print check per OS in the PR".
- ADR 0008: any sandbox exception for printing; final per-OS print path.
- `apps/desktop/README.md`: printing architecture and troubleshooting.

## Risks and open questions

- **From D1b** ([desktop-print spike](../../../spikes/desktop-print/README.md)): print path B (engine rasters through AppKit, GDI, GTK); no webview fallback. On macOS, AppKit snapped US Letter and A5 to A4 when each page's paper was set in `beginPageInRect:atPlacement:`, pages need the printer's imageable origin as an offset, read for each paper and orientation (read once for portrait, it left a landscape page 17.3 pt off), and 50 pages at 300 dpi peaked at 1.7–1.8 GB: per-page paper and memory need work here. The fidelity bar D1b measured is SSIM ≥ 0.95 at 150 dpi after alignment, sizes and placement within 2 pt. `hasNativePrint` should mean the OS dialog plus Lamina PDF's own rasters on paper of each page's size, not handing the PDF to the OS (ADR 0010's table): restate it in ADR 0008 or here.

- **Decision recorded (owner to confirm in the PR): raster only, no vector printing in the main process.** The macOS PDFKit path, Windows PDFium-on-DC/EMF playback and Linux raw-PDF-to-CUPS all make something other than the sandboxed engine parse untrusted bytes, breaking ADR 0008. Vector printing comes back only through (a) a sandboxed print helper process (follow-up todo) or (b) an ADR 0008 exception the owner signs off explicitly, naming the OS, the parser and the accepted risk. ADR 0008 gets a "printing" line saying this.
- **Raster cost**: 600 dpi A4 RGBA is ~140 MB per page uncompressed; render in bands and measure spool size and time for the 50-page check; drop to 300 dpi for non-photo printers if needed (record).
- **Linux** uses `GtkPrintOperation` + cairo (no direct CUPS PDF submission); `libcups` comes in through GTK — coordinate with D6b's packaging.
- **Driver quirks** (duplex unsupported, custom paper sizes) — the OS dialog handles most; record failures.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
