# D1b — `desktop-print` spike: faithful printing from Tauri on each OS

| Field | Value |
| --- | --- |
| ID | `D1b` |
| Status | `ready` |
| Plan | [Desktop app §6 D1](../../plans/desktop-app.md#d1-ipc-and-print-spikes) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C2c](./C2c-render-packages.md), [C3b](./C3b-viewer-virtualiser-and-zoom.md), [D1a](./D1a-desktop-ipc-spike.md) |
| Blocks | [D2a](./D2a-adr-0008-and-desktop-scaffold.md), [D4c](./D4c-native-printing.md) |
| Dependency notes | Web plan **C3** (viewer core, [web-app §C3](../../plans/web-app.md#c3-viewer-core)) — same start gate as D1a per the plan; the spike renders pages with C2's `lamina-render` (PDFium adapter), which C3 already requires. Starts after [D1a](D1a-desktop-ipc-spike.md) (see Depends on), not in parallel, so it reuses D1a's spike scaffold (a Tauri 2 app outside both workspaces) and its lessons instead of building a second one at the same time. |
| ADRs | Informs ADR 0008 (D2a) and the `hasNativePrint` capability flag of ADR 0010 (web plan C2). Follows [0001](../../decisions/0001-core-language-and-licence.md) (no AGPL/GPL: no Ghostscript/Poppler linkage), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `spike/d1b-desktop-print` |
| PR title | `spike(desktop): compare webview and native print paths per OS` |
| Size | M (200–600 lines of spike code plus the README) |
| PR | — |

## Goal

Find out how lamina's desktop app can print a PDF **faithfully** (correct page size and orientation, no browser headers/footers, vector text where possible, page ranges, scaling, duplex) on macOS, Windows and Linux, and recommend the path D4c implements. Tauri has no first-class print API beyond printing the webview ([desktop notes Q4](../../research/architecture/notes/desktop-shell.md#q4-file-system-file-associations-drag--drop-printing-sandboxing-multi-window), [report](../../research/architecture/app-and-package-architecture.md#tauri-2-is-the-desktop-shell-with-electron-held-in-reserve-for-linux): "Printing is unresolved … deserves its own spike"). The plan's risk table allows shipping a `window.print()` raster path first if native lags; this spike decides whether that is necessary and what each OS's native path costs.

## Context

- [Desktop plan §4 req. 7](../../plans/desktop-app.md#functional): native print dialog with page range, scaling and duplex where the OS provides it. [§7 risks](../../plans/desktop-app.md#7-risks): "Faithful printing needs per-OS code — dedicated spike; ship `window.print()` raster path first if native lags".
- [Architecture §4](../../plans/architecture.md#4-spikes-to-close-open-questions): `desktop-print` row.
- Web plan [C6](../../plans/web-app.md#c6-navigation-print-and-properties): the web reader's print path ("print path at print DPI") — the desktop webview path should reuse it, so measure it here as candidate A.
- Tauri: `WebviewWindow::print()` (Rust) prints the webview contents through the OS dialog; JS `window.print()` does the same.
- OS APIs to evaluate (primary docs, cite in the README):
  - macOS: AppKit `NSPrintOperation` with a custom `NSView` drawing page rasters or PDF pages; PDFKit `PDFDocument.printOperation(for:scalingMode:autoRotate:)` (prints with Apple's PDF renderer — different from PDFium, note fidelity implications).
  - Windows: `PrintDlgEx` + GDI `StartDoc`/`StartPage` with `StretchDIBits` of rasters; the XPS Print API / `IPrintDocumentPackageTarget`; `Windows.Graphics.Printing` (UWP-style `PrintManager`, needs an HWND interop). PDFium itself has `FPDF_RenderPage(HDC, …)` for GDI printing on Windows (vector through EMF) — a strong candidate.
  - Linux: `GtkPrintOperation` (GTK 3, already loaded by WebKitGTK) drawing with cairo (`cairo_set_source_surface` of rasters, or PDFium → cairo); or submitting the original PDF bytes to CUPS (`cupsPrintFile`/`lp`) since CUPS accepts PDF natively — fast and vector, but bypasses our renderer and the GTK dialog.
- Crates to check licences of: `objc2-app-kit`/`objc2-pdf-kit` (MIT/Apache/Zlib), `windows` (MIT/Apache), `gtk`/`gtk-sys` (MIT; gtk-rs 0.18 is what Tauri's Linux stack uses), `pdfium-render` (MIT/Apache). No `cups` crate under GPL (libcups itself is Apache-2.0 since CUPS 2.3 — verify).

## Scope

**In scope**

- `spikes/desktop-print/`: a minimal Tauri 2 app (outside both workspaces) with a "Print" button per candidate path, opening one of three fixtures: a 50-page mixed document, a document with mixed page sizes/orientations (A4 portrait + US Letter landscape), and a vector-heavy drawing with hairlines and small text.
- Candidate paths per OS:
  - **A. Webview print of rasters** (`window.print()` / `WebviewWindow::print()`): render pages at 300 dpi into `<img>`/`<canvas>` in a print-only DOM with `@page { size: <w> <h>; margin: 0 }`, as web C6 does.
  - **B. Native print of rasters**: our PDFium renderer → per-page RGBA at the printer DPI → OS print API (NSPrintOperation / GDI / GtkPrintOperation+cairo), with the OS dialog for range/copies/duplex.
  - **C. Native vector print where available**: Windows `FPDF_RenderPage` onto the printer HDC; macOS PDFKit `printOperation`; Linux CUPS submission of the original PDF (or GTK print with PDFium-rendered cairo surfaces). Note for the recommendation: in the app, every path C variant parses the untrusted PDF outside the sandboxed engine (main process or CUPS), against ADR 0008 (D2a). D4c therefore ships raster (path B) by default; a path C win here is recorded as "needs a sandboxed print helper or an owner-accepted ADR 0008 exception".
- For each path and OS record: dialog shown (native? page range/scaling/duplex controls present?), page size/orientation correctness on mixed-size fixture, presence of browser headers/footers/margins, output fidelity (print to PDF / "Microsoft Print to PDF" / CUPS-PDF, then compare against PDFium renders with a perceptual diff), time to spool 50 pages, peak memory, spool file size, cancel behaviour, and lines of platform code needed.
- A README with results and a **recommendation**: which path to ship in D4c per OS, whether to ship path A first as a fallback, and what `hasNativePrint` means.

**Out of scope** (and which todo covers it)

- Production print implementation, Print menu item and shortcut: [D4c](D4c-native-printing.md) (with menus from [D4b](D4b-native-menus.md)).
- Tile transport: [D1a](D1a-desktop-ipc-spike.md).
- Print from the sandboxed engine process (who renders print pages): decided in D4c on top of D2b's protocol.
- Physical-printer testing on every OS: only one physical printer check is required (any OS); others use print-to-PDF drivers.

## Tasks

- [ ] Fresh worktree `spike-d1b-desktop-print`; this todo is already claimed via the claim PR (see DoD), so do not move it here. Load `tauri-v2` and `rust-best-practices`.
- [ ] Scaffold `spikes/desktop-print/` (Tauri 2, vanilla TS + Vite), thin `main.rs`, logic in `lib.rs`; exclude from both workspaces; capability `default.json` with `core:default` and app-manifest permissions for `print_webview`, `print_native_raster`, `print_native_vector`.
- [ ] Fixtures: pick three from `fixtures/` (add any missing ones with provenance in `fixtures/README.md` — or keep spike-local fixtures under `spikes/desktop-print/fixtures/` with a provenance note, per the code-organisation rules).
- [ ] Path A: print-only DOM (`@media print`), images rendered by PDFium at 300 dpi; invoke `WebviewWindow::print()` from Rust and `window.print()` from JS; note differences.
- [ ] Path B macOS: `print_native_raster` → `NSPrintOperation` with an `NSView` subclass (objc2 `define_class!`) whose `drawRect:` draws the page `CGImage`; `NSPrintInfo` paper size per page; `knowsPageRange:` / `rectForPage:`. Must run on the main thread (`app.run_on_main_thread`).
- [ ] Path B Windows: `PrintDlgExW` (page ranges, copies, collate) → `CreateDCW` → `StartDocW`/`StartPage`/`StretchDIBits`/`EndPage` per page, with `DEVMODE` orientation per page (`ResetDCW`).
- [ ] Path B Linux: `gtk::PrintOperation` with `connect_draw_page` painting a cairo `ImageSurface`; `set_n_pages`, `set_use_full_page`, per-page `PageSetup` via `connect_request_page_setup`.
- [ ] Path C Windows: PDFium `FPDF_RenderPage(hdc, page, …, FPDF_PRINTING | FPDF_ANNOT)` onto the printer DC (via `pdfium-render`'s raw bindings).
- [ ] Path C macOS: PDFKit `PDFDocument(data:)` → `printOperationForPrintInfo:scalingMode:autoRotate:` → `runOperationModalForWindow:`.
- [ ] Path C Linux: CUPS — `lp`-equivalent via libcups (`cupsPrintFile`) after a GTK print dialog (`gtk::PrintUnixDialog`) collects printer and options; and GTK + PDFium-on-cairo if feasible.
- [ ] Fidelity measurement: print each fixture to the OS PDF printer, rasterise both the printed PDF and the original with PDFium at 150 dpi, compute a perceptual diff (e.g. `dssim` or SSIM via `image-compare`, MIT/Apache) per page; record mismatched page sizes.
- [ ] Physical printer check on at least one OS (duplex + page range) with a photo-free description in the README.
- [ ] `spikes/desktop-print/README.md`: method, environment, a matrix (OS × path) of the criteria above, platform-code line counts, recommendation and limits.
- [ ] Update plan/architecture spike row and `CLAUDE.md` spikes tree.

## Acceptance criteria

- [ ] README reports, for each OS, every candidate path's results on all three fixtures: correct page sizes (yes/no per page), headers/footers present (yes/no), SSIM vs reference, spool time and peak memory for the 50-page fixture, dialog features (range, scaling, duplex).
- [ ] README recommends one shipping path per OS for D4c, states whether path A ships first as a fallback, and defines what `hasNativePrint: true` guarantees to the UI.
- [ ] A 50-page document prints with a page range via the recommended path on each OS (recorded: OS, printer/driver, result).
- [ ] No GPL/AGPL code linked (`cargo deny check licenses` run inside the spike, result pasted in the README).

## Post-merge checks

None.

## Tests

Spike: no test suite. The fidelity numbers are the evidence. Keep the diff script (`scripts/compare.rs` or a small `cargo run --bin compare`) so D4c can reuse it as a manual-check helper.

## Docs to update

- `docs/plans/desktop-app.md` §9 D1: tick "Build `spikes/desktop-print`…" and (together with D1a) "Write both results with recommendations for ADR 0008"; link the README from §6 D1.
- `docs/plans/architecture.md` §4: `desktop-print` row → **Done** with link.
- `CLAUDE.md`: `spikes/` tree entry.

## Risks and open questions

- **Path C on macOS uses Apple's PDF renderer**, not PDFium: output can differ from what the reader shows (fonts, blend modes). The recommendation must say whether that is acceptable.
- **Linux CUPS submission** bypasses both our renderer and some print-preview UX; and Flatpak/sandboxed environments need the print portal — out of scope (Flathub is a non-goal), but note it.
- **Windows GDI printing at 600 dpi** of large pages can exhaust memory; measure banding if needed.
- **Main-thread requirements** (AppKit, GTK) interact with Tauri's event loop; document the `run_on_main_thread` pattern that works.
- Who renders print pages once the engine is sandboxed (engine renders rasters and streams them to the main process, which owns the printer handle) — D4c decides with D2b's protocol; note the implication here.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
