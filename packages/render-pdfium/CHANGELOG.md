# @laminapdf/render-pdfium

## 0.6.0

### Patch Changes

- 3b02d1b: A render can now leave out chosen annotations, so an editor can draw the annotation it moves without the page showing it twice. `RenderRequest.hiddenAnnotations` lists the annotations to leave out by their place in the page's `/Annots`, the `index` that `doc.pageAnnotations` reports, and only that render leaves them out. The place names the annotation, not its name (`/NM`), which two annotations may share. PDFium sets the annotation's Hidden flag for the render and clears it afterwards. hayro draws every annotation or none, so it draws the tile again without annotations and copies that over each hidden annotation's rectangle; an annotation that overlaps a hidden one is left out inside that rectangle too.
  
  In Rust, `RenderRequest` gains `hidden_annotations` and `with_hidden_annotations`, and is no longer `Copy`: clone a request you reuse. The hayro module's `HayroDocument.render` in `@laminapdf/wasm` takes the hidden places as a `Uint32Array` before the output buffer.
- Updated dependencies [3b02d1b]
  - @laminapdf/render@0.6.0

## 0.5.0

### Patch Changes

- @laminapdf/render@0.5.0

## 0.4.0

### Patch Changes

- @laminapdf/render@0.4.0

## 0.3.0

### Patch Changes

- @laminapdf/render@0.3.0

## 0.2.0

### Patch Changes

- 2e74d7d: Every error's and warning's `docsUrl` now opens its page on the docs site, `https://docs.laminapdf.com/reference/codes/<code>/`, instead of the Markdown file on GitHub. The code still ends the path.
- 8ae1bae: `FlowTemplate` is now an interface rather than a type alias, with the same keys and the same checks, so the API reference names it where `builder.flow()` takes it instead of spelling out every key of every block. Its reference page explains the placeholders, with an example. Every type declaration in the TypeScript reference is now a code block, one key or union case to a line.
- bd0148e: The packages' descriptions, keywords and README openings now say what each package does for you, and each package links to the issue tracker.
- Updated dependencies [414a955]
- Updated dependencies [2e74d7d]
- Updated dependencies [8ae1bae]
- Updated dependencies [bd0148e]
  - @laminapdf/render@0.2.0

## 0.2.0-alpha.6

### Patch Changes

- @laminapdf/render@0.2.0-alpha.6

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
