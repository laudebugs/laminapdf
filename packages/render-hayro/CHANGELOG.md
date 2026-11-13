# @laminapdf/render-hayro

## 0.3.0

### Patch Changes

- Updated dependencies [bc1bff5]
- Updated dependencies [2948e3f]
- Updated dependencies [6e9dd39]
- Updated dependencies [d9b6acf]
- Updated dependencies [e416ac6]
- Updated dependencies [2081146]
- Updated dependencies [8f21b98]
- Updated dependencies [bab4c9f]
  - @laminapdf/wasm@0.3.0
  - @laminapdf/render@0.3.0

## 0.2.0

### Patch Changes

- 2e74d7d: Every error's and warning's `docsUrl` now opens its page on the docs site, `https://docs.laminapdf.com/reference/codes/<code>/`, instead of the Markdown file on GitHub. The code still ends the path.
- 8ae1bae: `FlowTemplate` is now an interface rather than a type alias, with the same keys and the same checks, so the API reference names it where `builder.flow()` takes it instead of spelling out every key of every block. Its reference page explains the placeholders, with an example. Every type declaration in the TypeScript reference is now a code block, one key or union case to a line.
- bd0148e: The packages' descriptions, keywords and README openings now say what each package does for you, and each package links to the issue tracker.
- Updated dependencies [e5fa57d]
- Updated dependencies [414a955]
- Updated dependencies [2e74d7d]
- Updated dependencies [abd2520]
- Updated dependencies [8ae1bae]
- Updated dependencies [97cbf89]
- Updated dependencies [cafed9f]
- Updated dependencies [755d6ec]
- Updated dependencies [bd0148e]
- Updated dependencies [82d3c07]
- Updated dependencies [9f555a2]
  - @laminapdf/wasm@0.2.0
  - @laminapdf/render@0.2.0

## 0.2.0-alpha.6

### Patch Changes

- Updated dependencies [810d0d4]
- Updated dependencies [64f7860]
- Updated dependencies [70eb584]
  - @laminapdf/wasm@0.2.0-alpha.6
  - @laminapdf/render@0.2.0-alpha.6

## 0.2.0-alpha.5

### Minor Changes

- 771fafd: Pages can be drawn in Sepia, Dark or High contrast instead of their own colours. Images and annotations keep theirs, and so does text under a highlight.
  
  - **Render request:** `RenderRequest.appearance` is `"original"` (the default), `"sepia"`, `"dark"` or `{ kind: "high-contrast", ink, paper }`, for both backends.
  - **Colour maps:** `recolourRgb` and `recolourTile` in `@laminapdf/render` give the same bytes as the Rust crate's.
  - **Viewer:** `setAppearance()` redraws the pages in view in the new colours, keeping the scroll position. The thumbnails follow.

### Patch Changes

- Updated dependencies [526aae3]
- Updated dependencies [771fafd]
- Updated dependencies [5fc1f96]
- Updated dependencies [95d2a70]
- Updated dependencies [513aeb3]
  - @laminapdf/wasm@0.2.0-alpha.5
  - @laminapdf/render@0.2.0-alpha.5

## 0.2.0-alpha.4

### Patch Changes

- Updated dependencies [00bf63b]
- Updated dependencies [915c253]
- Updated dependencies [4fdbfd4]
- Updated dependencies [f69228f]
- Updated dependencies [2254184]
- Updated dependencies [c74d78f]
- Updated dependencies [cc7f9ce]
- Updated dependencies [703c2d8]
- Updated dependencies [60495bf]
- Updated dependencies [2886263]
- Updated dependencies [baec4bc]
  - @laminapdf/wasm@0.2.0-alpha.4
  - @laminapdf/render@0.2.0-alpha.4

## 0.2.0-alpha.3

### Patch Changes

- Updated dependencies [110d8b9]
  - @laminapdf/wasm@0.2.0-alpha.3
  - @laminapdf/render@0.2.0-alpha.3

## 0.2.0-alpha.2

### Patch Changes

- Updated dependencies [32a4963]
  - @laminapdf/wasm@0.2.0-alpha.2
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
  - @laminapdf/wasm@0.2.0-alpha.1
  - @laminapdf/render@0.2.0-alpha.1

## 0.2.0-alpha.0

### Patch Changes

- Updated dependencies [2b4384a]
  - @laminapdf/wasm@0.2.0-alpha.0
  - @laminapdf/render@0.2.0-alpha.0
