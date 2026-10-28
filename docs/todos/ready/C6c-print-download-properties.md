# C6c — Print, download of original bytes, and the document properties dialog

| Field | Value |
| --- | --- |
| ID | `C6c` |
| Status | `ready` |
| Plan | [web reader §6 C6. Navigation, print and properties](../../plans/web-app.md#c6-navigation-print-and-properties) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B9c](./B9c-text-extraction-bindings-and-differential.md), [C4c](./C4c-remote-files-encryption-privacy.md), [C5a](./C5a-text-layer-and-selection.md) |
| Blocks | [C7a](./C7a-accessibility-and-theming.md), [C7c](./C7c-performance-gates-and-launch.md), [D4c](./D4c-native-printing.md) |
| Dependency notes | [C4c](./C4c-remote-files-encryption-privacy.md) (all open paths, encrypted files); [C5a](./C5a-text-layer-and-selection.md) (font information from B9's font reading via the worker); library **B9** for font listing ([pdf-library B9](../../plans/pdf-library.md#b9-text-extraction)) |
| ADRs | Follows 0004 (print rendering via `RenderBackend` with print intent), 0010 (`Platform.print`, `Platform.saveAs`, `hasNativePrint` flag), 0011 (no upload: print and download are local) |
| Branch | `feat/c6c-print-download-properties` |
| PR title | `feat(web): add printing, original-bytes download and the document properties dialog` |
| Size | L (600–1,200 lines incl. tests) |
| PR | — |

## Goal

Finish the reader's output features. **Print** rasterises pages at print DPI in the worker and prints them from a hidden iframe (with progress and cancel), so a 50-page document prints correctly in Chrome, Firefox and Safari. **Download** saves the original bytes exactly as opened (byte-identical, even for remote and encrypted files). The **document properties** dialog shows metadata (Info + XMP), PDF version, page count and size, producer, fonts, encryption details and whether signatures are present.

## Context

- [web-app.md §4 functional 7](../../plans/web-app.md#functional) — print via rasterised pages in a hidden iframe, native PDF printing where supported; download returns original bytes.
- [web-app.md §2](../../plans/web-app.md#2-scope-and-non-goals) — properties scope (metadata, fonts, version, encryption, signatures present).
- [04 §7 encryption](../../research/pdf/04-interactive-security.md#7-encryption-76) (handler, revision, permissions), [04 §8 signatures](../../research/pdf/04-interactive-security.md) (signature fields presence), [03](../../research/pdf/03-text-and-fonts.md) (font types, embedding, subset tags).
- [C3a](./C3a-viewer-worker-rpc.md) (background priority, cancellation), [C4c](./C4c-remote-files-encryption-privacy.md) (`ByteSource` for remote files).

## Scope

**In scope**

- Print pipeline: `Platform.print(job)` web implementation; worker renders each page at print DPI (default 150, option 300; capped by a pixel/memory budget) with `print` intent flags (annotations per their print flags), as JPEG/PNG blobs or `ImageBitmap` → blob; a hidden iframe document with one `img` per page sized via `@page { size: <w>pt <h>pt; margin: 0 }` per page (mixed page sizes supported where browsers allow), `onafterprint` cleanup, progress dialog with cancel; object URLs revoked afterwards. Optional "print with browser's PDF viewer" path when the browser has native PDF printing and the file isn't encrypted (decide behind a capability flag; default to the raster path for consistency).
- Keyboard: `ctrl/cmd+P` triggers our print (preventDefault) when a document is open.
- Download: `Platform.saveAs(originalBytes, filename)` — for local files reuse the `File`; for remote files stream from the `ByteSource` (full fetch, using the block cache) without modification; filename from the URL path or `Content-Disposition`, sanitised; encrypted files download still encrypted (original bytes).
- RPC `properties() → { info, xmp (parsed subset: dc:title, dc:creator, xmp:CreateDate, pdf:Producer…), version, pageCount, pageSizes summary, fileSize, linearised?, tagged?, encryption { filter, V, R, keyLength, method, permissions } | null, fonts[{ name, type, embedded, subset, encoding }], signatures { count, fieldNames } , hasJavaScript, hasEmbeddedFiles, hasForms }` from lamina (not PDFium); add small read-only accessors to `lamina-model`/`lamina-wasm` where missing (coordinate with library owner).
- `PropertiesDialog` in `@lamina/ui` (tabs or sections: Description, Document, Fonts, Security), dates formatted with `Intl.DateTimeFormat`, copy-to-clipboard for values.

**Out of scope** (and which todo covers it)

- Signature validation (editor E10); properties only says "N signatures present, not verified".
- Editing metadata (editor E1).
- Native desktop print (desktop D4 + `desktop-print` spike).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`.
- [ ] Print renderer in the worker with memory budgeting (render → encode → transfer → release, one page at a time; JPEG quality 0.92 for photos vs PNG for text decided per page or globally — measure size/time).
- [ ] Print iframe builder with per-page `@page` sizing and orientation; test mixed portrait/landscape documents.
- [ ] Progress/cancel dialog; `beforeprint`/`afterprint` handling; Safari's synchronous print quirks (images must be loaded before `print()`).
- [ ] Download for local, remote and encrypted documents; filename sanitisation tests.
- [ ] Library accessors for properties (fonts per page aggregated, encryption dictionary summary, signature field count, XMP packet bytes) with Rust tests; XMP parsed in TS with `DOMParser` (main thread, small) or in Rust (decide; avoid XXE — no external entities).
- [ ] `PropertiesDialog` component + toolbar/menu entries + keyboard shortcut (`ctrl/cmd+D` like Acrobat, or menu-only; record in C7a's map).
- [ ] Facts for properties (version, Info title/producer, encryption revision, font names) from independent tools (pypdf/qpdf `--json`) in `expected.json`.

## Acceptance criteria

- [ ] **Printing a 50-page document works in Chrome, Firefox and Safari**: Playwright (Chromium, Firefox, WebKit) intercepts `window.print` in the iframe and asserts all 50 images loaded with the right page sizes/orientation; a manual print-to-PDF check on each browser is recorded in the PR (screenshots/notes), including a mixed-orientation file.
- [ ] Printing never sends data over the network (privacy suite from C4c extended to the print flow).
- [ ] Peak memory during printing of the 50-page document stays within the C3c budget (pages released as they are encoded).
- [ ] **Download is byte-identical to the input** for a local file, a remote `#url=` file and an encrypted file (SHA-256 compared in Playwright via the download event).
- [ ] Properties for corpus files match `expected.json` facts (version, title, producer, encryption revision, font names, signature count for `signed.pdf`).
- [ ] Properties dialog is keyboard-operable with zero serious axe violations.

## Tests

Rust unit tests for new accessors; Vitest unit tests (filename sanitisation, XMP subset parsing, print layout CSS generation); Playwright `apps/web/e2e/print.spec.ts`, `download.spec.ts`, `properties.spec.ts`; manual print checklist in the PR description.

## Docs to update

`apps/web/README.md` (print strategy and limits), `packages/ui/README.md`; tick "Print path at print DPI; download of original bytes" and "Document properties dialog…" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Browser print engines** differ in honouring per-page `@page` sizes (Safari and Firefox historically weaker); fall back to fit-to-paper with a notice.
- **Memory for 300 DPI** prints of large pages; cap DPI by page area and warn.
- **Owner-password permissions** (no-print flag): v0.1 policy needs a decision — recommendation: respect `/P` print bits by showing a warning and requiring confirmation (record in ADR 0011 or this PR's description for owner review).
- **Native PDF printing** path would bypass our rendering and differs across browsers; keep it off by default.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
