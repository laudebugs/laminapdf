# @laminapdf/render

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
