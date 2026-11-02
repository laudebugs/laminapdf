# @laminapdf/sdk

## 0.2.0-alpha.1

### Minor Changes

- 5c2624e: The library is called Lamina PDF, and its public names follow. `@laminapdf/sdk`'s error class is `LaminaPdfError`, checked with `isLaminaPdfError`, and its warnings are `LaminaPdfWarning`. The error object's JSON Schema is `laminapdf.error/1`. `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro` (`.js`, `.d.ts` and `.wasm`).
- 08e123d: `@laminapdf/viewer` exports only its documented API: `ViewerClient`, `createViewer` and `Viewer`, the types of their options, events and state, the protocol types `client.call` takes, and `MIN_ZOOM`/`MAX_ZOOM`. Its internal helpers (layout, canvas pool, bitmap cache, tiles, zoom maths), its byte-source functions and its protocol constants are no longer exported. Its worker no longer keeps a log of every render, which grew without bound, and no longer serves test-only methods. `@laminapdf/sdk/worker/server`'s `serve()` takes a `saveBudget` (work units per save step), and `workerStats()` reports the handles and calls a worker holds. `@laminapdf/sdk/worker` exports the `Emitter` its client uses.

### Patch Changes

- 1c6e773: A render `data-pending` thrown inside `@laminapdf/sdk`'s worker now reaches the page with its byte ranges: `LaminaPdfError.ranges` lists them. Before, they arrived empty. `RenderError` has a new `toDiagnostic()`, which returns the plain error object every Lamina PDF surface uses, and the worker posts that object for any error that has the method. `RenderError`'s `toJSON()`, `reviveRenderError` and `SerializedRenderError` are gone, and `ByteRange` and `RenderErrorKind` name its types. The viewer's worker now throws `RenderError`s for `cancelled` and `disposed` instead of plain objects, so they reach the page with a `docsUrl`.
- 2a9e364: The `rotatePage` op now reads a `/Rotate` stored as an integral real, such as `90.0`, as the rotation the page displays; before, it read 0. `PageBuilder.rotate` now refuses angles other than 0, 90, 180 and 270, as its type says. In the Rust crates, `Rotation` from `laminapdf-syntax` replaces `laminapdf-render`'s `ViewRotation` and the degrees in `RenderPage::rotate`, and `laminapdf_cos::Page` drops `media_box`, `crop_box` and `rotate`: read them, and their strict-mode errors, from `laminapdf-model`'s page view.
- Updated dependencies [5c2624e]
  - @laminapdf/wasm@0.2.0-alpha.1

## 0.2.0-alpha.0

### Minor Changes

- 2b4384a: Accessible and archival PDFs. A `profiles` option on the builder (`PDF/UA-1`, `PDF/A-2b`, `PDF/A-2u`, `PDF/A-3b`; `BuildOptions::with_profile` in Rust) checks each machine-checkable rule the document breaks when it finishes, with a coded error per rule: `missing-title`, `missing-lang`, `untagged-content`, `missing-alt-text`, `heading-level-skipped`, `unnumbered-heading-misused`, `invalid-structure-nesting`, `link-without-structure`, `link-without-contents`, `font-not-embedded`, `glyph-without-unicode`, `device-color-without-output-intent`, `embedded-file-forbidden`, `embedded-file-without-mime-type` and `profile-version-conflict`. The builder writes the XMP identification, `/DisplayDocTitle` for PDF/UA-1 and, for PDF/A, an sRGB output intent with a 480-byte embedded ICC profile. `attach(name, bytes, mime, { relationship })` adds associated files for PDF/A-3b, such as an e-invoice's XML. Every example that claims a profile is validated by veraPDF each night.
- 592d285: `@laminapdf/sdk/worker` can be extended: `@laminapdf/sdk/worker/server`'s `serve({ methods })` adds methods to the same worker, called with `client.request(method, params, { priority, key })`. The worker runs higher priorities first and coalesces calls with the same key. `emit()` sends events to `client.on("event")`, and `connect({ onDiscard })` sees late answers (by default it closes their ImageBitmaps). The worker protocol is now version 2.

### Patch Changes

- Updated dependencies [2b4384a]
  - @laminapdf/wasm@0.2.0-alpha.0

## 0.1.0-alpha.0

### Minor Changes

- First alpha, `0.1.0-alpha.0`, versioned and tagged but not published (ADR 0017). The `lamina` façade and its core crates at the same version: open, read, edit and save PDFs (incremental and full rewrites, the save planner, undo/redo), generate them (pages, vectors, colour, images, standard 14 and embedded fonts with shaping, bidi and line breaking, links, outlines, metadata, tagged PDF), and extract page text; `@laminapdf/sdk` over the base, text and extract WebAssembly modules in browsers, Node, Deno, Bun and Cloudflare Workers.

### Patch Changes

- Updated dependencies
  - @laminapdf/wasm@0.1.0-alpha.0
