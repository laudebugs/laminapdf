# @laminapdf/render-pdfium

## 0.2.0-alpha.5

### Minor Changes

- 771fafd: Pages can be drawn in Sepia, Dark or High contrast instead of their own colours. Images and annotations keep theirs, and so does text under a highlight.
  
  - **Render request:** `RenderRequest.appearance` is `"original"` (the default), `"sepia"`, `"dark"` or `{ kind: "high-contrast", ink, paper }`, for both backends.
  - **Colour maps:** `recolourRgb` and `recolourTile` in `@laminapdf/render` give the same bytes as the Rust crate's.
  - **Viewer:** `setAppearance()` redraws the pages in view in the new colours, keeping the scroll position. The thumbnails follow.

### Patch Changes

- Updated dependencies [771fafd]
  - @laminapdf/render@0.2.0-alpha.5

## 0.2.0-alpha.4

### Patch Changes

- Updated dependencies [baec4bc]
  - @laminapdf/render@0.2.0-alpha.4

## 0.2.0-alpha.3

### Patch Changes

- @laminapdf/render@0.2.0-alpha.3

## 0.2.0-alpha.2

### Patch Changes

- @laminapdf/render@0.2.0-alpha.2

## 0.2.0-alpha.1

### Minor Changes

- 5c2624e: The library is called Lamina PDF, and its public names follow. `@laminapdf/sdk`'s error class is `LaminaPdfError`, checked with `isLaminaPdfError`, and its warnings are `LaminaPdfWarning`. The error object's JSON Schema is `laminapdf.error/1`. `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro` (`.js`, `.d.ts` and `.wasm`).
- 6f7ae81: `@laminapdf/render` has one `WasmSource` type for where an engine's `.wasm` comes from (a URL, a `Response` or a promise of one, the bytes or a compiled module). `CloneableWasmSource` is the subset a Web Worker can be sent. `initPdfium` and `initHayro` both take it: `@laminapdf/render-pdfium`'s own `PdfiumWasm` type is gone, and `initHayro` no longer takes a `Request`, a promise of anything but a `Response`, or a view of a `SharedArrayBuffer`. `RenderBackend` declares `memory()`, which both backends already had; a backend written outside this repository has to add it. The adapters share the page-index check, the read handling and the pending-range merging, so hayro now reports adjacent missing reads as one range, as PDFium did.

### Patch Changes

- Updated dependencies [5c2624e]
- Updated dependencies [1c6e773]
- Updated dependencies [6f7ae81]
- Updated dependencies [2a9e364]
  - @laminapdf/render@0.2.0-alpha.1

## 0.2.0-alpha.0

### Patch Changes

- @laminapdf/render@0.2.0-alpha.0
