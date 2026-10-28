# D4c — Native printing per the `desktop-print` spike

| Field | Value |
| --- | --- |
| ID | `D4c` |
| Status | `ready` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C6c](./C6c-print-download-properties.md), [D1b](./D1b-desktop-print-spike.md), [D4b](./D4b-native-menus.md) |
| Blocks | [D7c](./D7c-desktop-beta-release.md), [E1d](./E1d-edit-session-undo-ui.md), [E10i](./E10i-platform-signers.md) |
| Dependency notes | [D4b](D4b-native-menus.md) (Print menu item), [D1b](D1b-desktop-print-spike.md) (recommended path). Cross-plan: web **C6** ([web-app §C6](../../plans/web-app.md#c6-navigation-print-and-properties)) only if D1b recommends the webview raster path, which reuses C6's print-at-print-DPI code. |
| ADRs | Follows ADR 0008 (engine renders, main owns printer handles), ADR 0010 (`hasNativePrint`) |
| Branch | `feat/d4c-native-printing` |
| PR title | `feat(desktop): print documents through the native print path on each OS` |
| Size | L (600–1,200 lines, per-OS code) |
| PR | — |

## Goal

Ship faithful printing on macOS, Windows and Linux using the path D1b recommended per OS: the native print dialog with page range, copies, scaling and duplex where the OS provides it; correct page sizes and orientation per page; no browser headers or footers. Requirement 7 of the [desktop plan](../../plans/desktop-app.md#functional); D4 is accepted when "a 50-page document prints with page ranges on each OS (manual check recorded)".

## Context

- D1b README (`spikes/desktop-print/`): per-OS recommendation, fidelity numbers, platform-code notes (main-thread rules, banding), and the comparison script.
- [Desktop plan §7](../../plans/desktop-app.md#7-risks): ship the `window.print()` raster path first if native lags.
- ADR 0008: the engine renders; the main process owns OS handles (printer DC, `NSPrintOperation`, `GtkPrintOperation`) because the engine is sandboxed (D2c) and has no access to printers.
- Web C6's print path (if the webview raster path is used anywhere).
- APIs per D1b (NSPrintOperation/PDFKit, PrintDlgEx/GDI or PDFium `FPDF_RenderPage` on an HDC, GtkPrintOperation/cairo or CUPS).

## Scope

**In scope**

- `src-tauri/src/print/{mod.rs,macos.rs,windows.rs,linux.rs}` implementing `fn print_document(app: &AppHandle, window: &WebviewWindow, doc: DocId, opts: PrintRequest) -> Result<PrintOutcome, PrintError>` with the D1b-chosen path per OS.
- Engine protocol additions: `RenderPrintPage { doc, page, dpi, format: Rgba | Bgra | Emf? }` streaming pages on demand as the OS print loop asks for them (never all pages in memory); cancellation when the user cancels the print job. If D1b chose a vector path that needs PDFium on the printer DC (Windows), document how this reconciles with the sandbox (e.g. the engine produces EMF/XPS bytes which main plays back on the DC) or record an accepted exception in ADR 0008.
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

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2` and `rust-best-practices`.
- [ ] Port the D1b spike's chosen code paths into `src-tauri/src/print/`, replacing spike shortcuts (`unwrap`, blocking on the main thread) with proper error handling and `run_on_main_thread` where APIs require it; every FFI `unsafe` with `// SAFETY:`.
- [ ] Engine `RenderPrintPage` message + streaming with back-pressure; protocol version bump; tests.
- [ ] `print` command (manifest + capability) invoked from the menu event and the toolbar's print button via `Platform.print()`.
- [ ] Progress + cancel UI in `@lamina/ui` (shared component; web uses it for C6's print preparation too if useful).
- [ ] Settings: "Print using: system (recommended) / compatibility (webview)".
- [ ] Fixtures: reuse D1b's three fixtures (50-page, mixed sizes, vector-heavy).
- [ ] Manual print checklist `apps/desktop/e2e/MANUAL-PRINT.md` with a results table: OS, version, printer/driver, fixture, range, duplex, result, SSIM vs reference (print-to-PDF).

## Acceptance criteria

- [ ] On macOS, Windows and Linux, printing the 50-page fixture with a page range (e.g. 5–12) via the native dialog produces exactly those pages at the correct size and orientation (manual check recorded per OS in the PR, at least one on a physical printer).
- [ ] Mixed page sizes print each page at its own size/orientation (print-to-PDF check, SSIM ≥ the threshold set in D1b).
- [ ] No browser headers/footers/margins appear in native-path output.
- [ ] Cancelling mid-spool stops engine rendering within 1 s and leaves the app responsive.
- [ ] Peak main-process memory while printing the 50-page fixture stays within 2× the single-page raster size at print DPI (streaming, not buffering).
- [ ] The compatibility path works on each OS as a fallback (manual check).

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

- **Vector printing vs sandbox** (Windows `FPDF_RenderPage` on a printer DC needs PDFium in the process that owns the DC): EMF playback or an accepted exception — decide and record.
- **macOS PDFKit path** renders with Apple's engine (fidelity differs from PDFium) — only acceptable if D1b said so.
- **Linux CUPS path** depends on `libcups` at runtime (dynamic dependency in the deb's `Depends:`/AppImage) — coordinate with D6b's packaging.
- **Driver quirks** (duplex unsupported, custom paper sizes) — the OS dialog handles most; record failures.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
