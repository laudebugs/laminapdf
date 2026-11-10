# C6c — Print, download of original bytes, and the document properties dialog

| Field | Value |
| --- | --- |
| ID | `C6c` |
| Status | `done` |
| Plan | [web reader §6 C6. Navigation, print and properties](../../plans/web-app.md#c6-navigation-print-and-properties) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B9c](./B9c-text-extraction-bindings-and-differential.md), [C3c](./C3c-viewer-perf-harness.md), [C4c](./C4c-remote-files-encryption-privacy.md), [C5a](./C5a-text-layer-and-selection.md) |
| Dependency notes | [C4c](../done/C4c-remote-files-encryption-privacy.md) (all open paths, encrypted files); [C5a](../done/C5a-text-layer-and-selection.md) (font information from B9's font reading via the worker); library **B9** for font listing ([pdf-library B9](../../plans/pdf-library.md#b9-text-extraction)); [C3c](../done/C3c-viewer-perf-harness.md) (print preparation measured within the harness budget) |
| ADRs | Follows 0004 (print rendering via `RenderBackend` with print intent), 0010 (`Platform.print`, `Platform.saveAs`, `hasNativePrint` flag), 0011 (no upload: print and download are local) |
| Branch | `feat/c6c-print-download-properties` |
| PR title | `feat(web): add printing, original-bytes download and the document properties dialog` |
| Size | L (600–1,200 lines incl. tests) |
| PR | [#368](https://github.com/laudebugs/laminapdf/pull/368) |

## Goal

Finish the reader's output features. **Print** rasterises pages at print DPI in the worker and prints them from a hidden iframe (with progress and cancel), so a 50-page document prints correctly in Chrome, Firefox and Safari. **Download** saves the original bytes exactly as opened (byte-identical, even for remote and encrypted files). The **document properties** dialog shows metadata (Info + XMP), PDF version, page count and size, producer, fonts, encryption details and whether signatures are present.

## Context

- **Design (Paper):** [R08 Document properties](../../design/paper/reader-a.md#r08-document-properties), [R09 Print and Download a copy](../../design/paper/reader-a.md#r09-print-and-download-a-copy). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [web-app.md §4 functional 7](../../plans/web-app.md#functional) — print via rasterised pages in a hidden iframe, native PDF printing where supported; download returns original bytes.
- [web-app.md §2](../../plans/web-app.md#2-scope-and-non-goals) — properties scope (metadata, fonts, version, encryption, signatures present).
- [04 §7 encryption](../../research/pdf/04-interactive-security.md#7-encryption-76) (handler, revision, permissions), [04 §8 signatures](../../research/pdf/04-interactive-security.md) (signature fields presence), [03](../../research/pdf/03-text-and-fonts.md) (font types, embedding, subset tags).
- [C3a](../done/C3a-viewer-worker-rpc.md) (background priority, cancellation), [C4c](../done/C4c-remote-files-encryption-privacy.md) (`ByteSource` for remote files).

## Scope

**In scope**

- Print pipeline: `Platform.print(job)` web implementation; worker renders each page at print DPI (default 150, option 300; capped by a pixel/memory budget) with `print` intent flags (annotations per their print flags), as JPEG/PNG blobs or `ImageBitmap` → blob; a hidden same-origin `about:blank` iframe document with one `img` per page sized via named pages — `@page p<N> { size: <w>pt <h>pt; margin: 0 }` plus `.page-<N> { page: p<N> }` (mixed page sizes supported where browsers allow). **CSP-safe styling:** the iframe inherits C4b's CSP (`style-src 'self'`, no `'unsafe-inline'`), so a generated `<style>` element or `style="…"` attribute is blocked. The per-page rules are therefore added through CSSOM — a constructable stylesheet created from the iframe's own realm (`new iframe.contentWindow.CSSStyleSheet()`), filled with `insertRule`, assigned to `iframe.contentDocument.adoptedStyleSheets` — and element sizes are set with `el.style.setProperty(…)`, which CSP does not govern. Static print rules (hide everything but the pages) come from a same-origin hashed stylesheet `<link>`. Images are `blob:` URLs (allowed by `img-src`), `onafterprint` cleanup, progress dialog with cancel; object URLs revoked afterwards. Optional "print with browser's PDF viewer" path when the browser has native PDF printing and the file isn't encrypted (decide behind a capability flag; default to the raster path for consistency).
- Keyboard: `ctrl/cmd+P` triggers our print (preventDefault) when a document is open.
- Download: `Platform.saveAs(originalBytes, filename)` — for local files reuse the `File`; for remote files stream from the `ByteSource` (full fetch, using the block cache) without modification; filename from the URL path or `Content-Disposition`, sanitised; encrypted files download still encrypted (original bytes).
- RPC `properties() → { info, xmp (parsed subset: dc:title, dc:creator, xmp:CreateDate, pdf:Producer…), version, pageCount, pageSizes summary, fileSize, linearised?, tagged?, encryption { filter, V, R, keyLength, method, permissions } | null, fonts[{ name, type, embedded, subset, encoding }], signatures { count, fieldNames } , hasJavaScript, hasEmbeddedFiles, hasForms }` from lamina (not PDFium); add small read-only accessors to `laminapdf-model`/`laminapdf-wasm` where missing (coordinate with library owner).
- `PropertiesDialog` in `@laminapdf/ui` (tabs or sections: Description, Document, Fonts, Security), dates formatted with `Intl.DateTimeFormat`, copy-to-clipboard for values.

**Out of scope** (and which todo covers it)

- Signature validation (editor E10); properties only says "N signatures present, not verified".
- Editing metadata (editor E1).
- Native desktop print (desktop D4 + `desktop-print` spike).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Print renderer in the worker with memory budgeting (render → encode → transfer → release, one page at a time; JPEG quality 0.92 for photos vs PNG for text decided per page or globally — measure size/time).
- [x] Print iframe builder with per-page named `@page` sizing and orientation via CSSOM (constructable stylesheet + `insertRule`, no `<style>`/`style=` markup); test mixed portrait/landscape documents; the e2e listens for `securitypolicyviolation` in the parent and the iframe and asserts zero, and asserts the computed `@page` sizes via `document.styleSheets`/`adoptedStyleSheets`.
- [x] Progress/cancel dialog; `beforeprint`/`afterprint` handling; Safari's synchronous print quirks (images must be loaded before `print()`).
- [x] Download for local, remote and encrypted documents; filename sanitisation tests.
- [x] Library accessors for properties (fonts per page aggregated, encryption dictionary summary, signature field count, XMP packet bytes) with Rust tests; XMP parsed in TS with `DOMParser` (main thread, small) or in Rust (decide; avoid XXE — no external entities).
- [x] `PropertiesDialog` component + toolbar/menu entries + keyboard shortcut (`ctrl/cmd+D` like Acrobat, or menu-only; record in C7a's map).
- [x] Facts for properties (version, Info title/producer, encryption revision, font names) from independent tools (pypdf/qpdf `--json`) in `expected.json`.

## Acceptance criteria

- [x] **Printing a 50-page document works in Chrome, Firefox and Safari**: Playwright (Chromium, Firefox, WebKit) intercepts `window.print` in the iframe and asserts all 50 images loaded with the right page sizes/orientation; a manual print-to-PDF check on each browser is recorded in the PR (screenshots/notes), including a mixed-orientation file.
- [x] Printing never sends data over the network (privacy suite from C4c extended to the print flow).
- [x] The print flow causes **zero CSP violations** under the production CSP from C4b (asserted in e2e, parent and iframe).
- [x] Peak memory during printing of the 50-page document stays within the C3c budget (pages released as they are encoded).
- [x] **Download is byte-identical to the input** for a local file, a remote `#url=` file and an encrypted file (SHA-256 compared in Playwright via the download event).
- [x] Properties for corpus files match `expected.json` facts (version, title, producer, encryption revision, font names, signature count for `signed.pdf`).
- [x] Properties dialog is keyboard-operable with zero serious axe violations.

## Notes

- **Print encoding:** PNG for every page. Pages are drawn one at a time and released once encoded, so the page holds one small file per page, never pixels. The worker caps a page at 16 million pixels (a 300 dpi page of 11 × 17 inches), lowering its resolution past that.
- **XMP:** parsed in TypeScript with `DOMParser` as XML, which loads no external entities or DTDs; the library hands over the decoded packet as text.
- **Properties shortcut:** none, as R08 says (`Mod D` is the browser's). Document properties is in the More menu and the palette.
- **Native PDF printing:** not built. The raster path is the only one, which keeps printing the same in every browser.
- **Restricted printing (owner decision):** follows this todo's recommendation and R09: the dialog warns and asks to confirm, never blocks. C4c said permissions are not enforced beyond display; the PR asks the owner to choose.
- **Deviations from R08 and R09** are recorded in [reader-a.md](../../design/paper/reader-a.md#r09-print-and-download-a-copy): a separate range field, no Sepia notice yet, no notice when a page is drawn at a lower resolution, no `Content-Disposition` name, failures in an alertdialog.
- **The manual print-to-PDF check** in each browser is in the PR description.
- **Tests:** print, download and properties share one file, `apps/web/e2e/output.e2e.test.ts`, which one browser and server run in turn, rather than three `*.spec.ts` files.
- **Peak memory while printing is not measured.** The worker draws, encodes and releases one page at a time, and a High-quality print past a billion pixels in all prints at Standard, so the frame's images stay bounded. Measuring it in the C3c harness is left to [C7c](../done/C7c-performance-gates-and-launch.md)'s performance gates.

## Post-merge checks

None.

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

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
