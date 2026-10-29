# D4c — Native printing per the `desktop-print` spike

| Field | Value |
| --- | --- |
| ID | `D4c` |
| Status | `ready` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C6c](./C6c-print-download-properties.md), [D1b](./D1b-desktop-print-spike.md), [D4b](./D4b-native-menus.md) |
| Blocks | [D7c](./D7c-desktop-beta-release.md) |
| Dependency notes | [D4b](D4b-native-menus.md) (Print menu item), [D1b](D1b-desktop-print-spike.md) (recommended path). Cross-plan: web **C6** ([web-app §C6](../../plans/web-app.md#c6-navigation-print-and-properties)) only if D1b recommends the webview raster path, which reuses C6's print-at-print-DPI code. |
| ADRs | Follows ADR 0008 (engine renders, main owns printer handles), ADR 0010 (`hasNativePrint`) |
| Branch | `feat/d4c-native-printing` |
| PR title | `feat(desktop): print documents through the native print path on each OS` |
| Size | L (600–1,200 lines, per-OS code) |
| PR | — |

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
- Engine protocol additions (in `crates/lamina-desktop-protocol`): `RenderPrintPage { doc, page, dpi, format: Rgba | Bgra }` streaming pages on demand as the OS print loop asks for them (never all pages in memory; large pages rendered in bands), cancellation when the user cancels the print job. No EMF/XPS format: playing engine-produced metafiles in the main process would hand it bytes a compromised engine controls.
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

- [ ] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
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

## Post-merge checks

None.

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

- **Decision recorded (owner to confirm in the PR): raster only, no vector printing in the main process.** The macOS PDFKit path, Windows PDFium-on-DC/EMF playback and Linux raw-PDF-to-CUPS all make something other than the sandboxed engine parse untrusted bytes, breaking ADR 0008. Vector printing comes back only through (a) a sandboxed print helper process (follow-up todo) or (b) an ADR 0008 exception the owner signs off explicitly, naming the OS, the parser and the accepted risk. ADR 0008 gets a "printing" line saying this.
- **Raster cost**: 600 dpi A4 RGBA is ~140 MB per page uncompressed; render in bands and measure spool size and time for the 50-page check; drop to 300 dpi for non-photo printers if needed (record).
- **Linux** uses `GtkPrintOperation` + cairo (no direct CUPS PDF submission); `libcups` comes in through GTK — coordinate with D6b's packaging.
- **Driver quirks** (duplex unsupported, custom paper sizes) — the OS dialog handles most; record failures.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
