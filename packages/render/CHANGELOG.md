# @laminapdf/render

## 0.4.0

No changes in this release.

## 0.3.0

No changes in this release.

## 0.2.0

### Patch Changes

- 414a955: The Rust crate `laminapdf-content` can now add content to a page without changing what the page already draws. `PageContentWriter` wraps a page's content in `q … Q`, adds streams before or after it, and registers Form XObjects, graphics states and fonts under names the page doesn't use yet. A page that shares its resources or content streams with other pages keeps them unchanged.
- 2e74d7d: Every error's and warning's `docsUrl` now opens its page on the docs site, `https://docs.laminapdf.com/reference/codes/<code>/`, instead of the Markdown file on GitHub. The code still ends the path.
- 8ae1bae: `FlowTemplate` is now an interface rather than a type alias, with the same keys and the same checks, so the API reference names it where `builder.flow()` takes it instead of spelling out every key of every block. Its reference page explains the placeholders, with an example. Every type declaration in the TypeScript reference is now a code block, one key or union case to a line.
- bd0148e: The packages' descriptions, keywords and README openings now say what each package does for you, and each package links to the issue tracker.

## 0.2.0-alpha.6

No changes in this release.

## 0.2.0-alpha.5

### Minor Changes

- 771fafd: Pages can be drawn in Sepia, Dark or High contrast instead of their own colours. Images and annotations keep theirs, and so does text under a highlight.
  
  - **Render request:** `RenderRequest.appearance` is `"original"` (the default), `"sepia"`, `"dark"` or `{ kind: "high-contrast", ink, paper }`, for both backends.
  - **Colour maps:** `recolourRgb` and `recolourTile` in `@laminapdf/render` give the same bytes as the Rust crate's.
  - **Viewer:** `setAppearance()` redraws the pages in view in the new colours, keeping the scroll position. The thumbnails follow.

## 0.2.0-alpha.4

### Minor Changes

- baec4bc: The viewer is sharp at every zoom level. It sized each page's canvas and sharp tiles from the page's CSS width times the pixel ratio, while the renderer sizes the page in 32-bit floats, so at most zoom levels the right-hand tile came out one pixel wider than the page. The renderer refused it, and the rest of the page stayed blurry. `@laminapdf/render` now exports `pagePixels`, the page's size in pixels at a scale as its planner computes it, and the viewer sizes canvases and tiles with it.

## 0.2.0-alpha.3

No changes in this release.

## 0.2.0-alpha.2

No changes in this release.

## 0.2.0-alpha.1

### Minor Changes

- 5c2624e: The library is called Lamina PDF, and its public names follow. `@laminapdf/sdk`'s error class is `LaminaPdfError`, checked with `isLaminaPdfError`, and its warnings are `LaminaPdfWarning`. The error object's JSON Schema is `laminapdf.error/1`. `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro` (`.js`, `.d.ts` and `.wasm`).
- 1c6e773: A render `data-pending` thrown inside `@laminapdf/sdk`'s worker now reaches the page with its byte ranges: `LaminaPdfError.ranges` lists them. Before, they arrived empty. `RenderError` has a new `toDiagnostic()`, which returns the plain error object every Lamina PDF surface uses, and the worker posts that object for any error that has the method. `RenderError`'s `toJSON()`, `reviveRenderError` and `SerializedRenderError` are gone, and `ByteRange` and `RenderErrorKind` name its types. The viewer's worker now throws `RenderError`s for `cancelled` and `disposed` instead of plain objects, so they reach the page with a `docsUrl`.
- 6f7ae81: `@laminapdf/render` has one `WasmSource` type for where an engine's `.wasm` comes from (a URL, a `Response` or a promise of one, the bytes or a compiled module). `CloneableWasmSource` is the subset a Web Worker can be sent. `initPdfium` and `initHayro` both take it: `@laminapdf/render-pdfium`'s own `PdfiumWasm` type is gone, and `initHayro` no longer takes a `Request`, a promise of anything but a `Response`, or a view of a `SharedArrayBuffer`. `RenderBackend` declares `memory()`, which both backends already had; a backend written outside this repository has to add it. The adapters share the page-index check, the read handling and the pending-range merging, so hayro now reports adjacent missing reads as one range, as PDFium did.
- 2a9e364: The `rotatePage` op now reads a `/Rotate` stored as an integral real, such as `90.0`, as the rotation the page displays; before, it read 0. `PageBuilder.rotate` now refuses angles other than 0, 90, 180 and 270, as its type says. In the Rust crates, `Rotation` from `laminapdf-syntax` replaces `laminapdf-render`'s `ViewRotation` and the degrees in `RenderPage::rotate`, and `laminapdf_cos::Page` drops `media_box`, `crop_box` and `rotate`: read them, and their strict-mode errors, from `laminapdf-model`'s page view.

## 0.2.0-alpha.0

No changes in this release.
