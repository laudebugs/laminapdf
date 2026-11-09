# @laminapdf/viewer

## 0.2.0-alpha.4

### Minor Changes

- 703c2d8: Read a document's outline and named destinations, and move through it with thumbnails. `Document.outline()` gives the outline's items with their titles, whether each starts expanded, and where each goes: a page and a view in page space (points from the displayed page's top-left, y down), whichever coordinates the file used. `Document.namedDestination(name)` finds one named destination. Both need the extract or text module, like `page.text()`.
  
  In `@laminapdf/viewer`, `ViewerClient.outline()` and `namedDestination(name)` read them in the document Web Worker, and `Viewer.goToDestination(target)` shows an item's or a destination's target, with each kind of fit. `ThumbnailStrip` is a list of the pages as small pictures. It mounts only the thumbnails near its view, renders after the main view's pages and goes to a page on a click or a key.
- 80193fe: The viewer keeps the page in view sharp at every zoom up to 800 %. Each page's canvas now holds at most 8 million pixels; a larger page keeps that canvas as a backdrop, and a detail canvas over the part in view takes the sharp tiles at the full pixel ratio. The detail canvas's size follows the viewport rather than the zoom, so the canvases fit the same memory as before. `createViewer` takes a `detailPool` option, and the render stats count `detailCanvases`.
- 9fffc56: Find now lists its results. `viewer.find.results()` gives every match found so far in document order, each with its page and a snippet of the text around it, and `goTo(ordinal)` goes to one. The web reader shows them in a Results tab in the sidebar while find is open.
- c399862: Find text across a whole document. `viewer.find` searches from the current page forward and wraps round, so scrolling never waits. Results stream in by page, and the first match shows as soon as it's found. `next()` and `previous()` move between matches and scroll each into view, and the matches on the page are highlighted. Options cover case, whole words and diacritics. Match counts agree with pdf.js's find. `ViewerClient.find(query, options, pages)` is the call underneath.
- b82b385: The document worker now counts every network request it makes from the moment it starts: `client.networkLog()` returns them all, with the requests its Content Security Policy blocked, and the `network` and `csp-violation` events report each as it happens. `watchNetwork` does the counting and is exported for the page too: it wraps `fetch`, `XMLHttpRequest`, `WebSocket` and `sendBeacon` and reads everything else the browser loads from Resource Timing.
- 8302db7: The viewer opens remote PDFs: `client.open({ url })` makes the document worker fetch the file itself with HTTP Range requests, so its bytes never pass through the page. Lamina PDF reads the file range by range, and a render fetches the bytes it needs, so page 1 of a large file appears after a small part of it has arrived. Requests send no cookies or referrer and skip the HTTP cache. A server that ignores `Range` sends the whole file, which the viewer accepts up to 256 MiB. The new `progress` event reports how much has been read.
- 60495bf: Every page's text is selectable and copyable, and a tagged document reads well in a screen reader. The viewer lays one transparent span over each text run of Lamina PDF's extraction, aligned to the rendered glyphs at every zoom and rotation, in any script. A selection runs from one page into the next, and the pages it touches stay mounted while you scroll away. Copy writes the extracted text in reading order. A tagged document's structure tree becomes a hidden tree of headings, paragraphs, lists, tables and figures that owns the spans.
  
  `createViewer`'s `text` option turns the layer off or sets `outline`, `untaggedLabel` and `onSelection`. `Viewer.keepMounted(pages)` pins pages. `ViewerClient.pageText(index)` and `structure()` give a page's runs and glyph quads and the document's structure tree. The document Web Worker loads the extract module once the render backend is ready, so the first page never waits for it.

### Patch Changes

- d9bb894: `ViewerClient.open` now returns the document's title from its metadata as `title`, when the file has one, so an app can name its window after the document.
- 26a9987: A mouse drag selects a page's text again. The layer slots above the text covered the whole page and took every press; every slot but `text` now lets the pointer through. A layer added with `addLayer` gets such a slot too, so an element in it that should be clicked, such as a link, sets `pointer-events: auto` on itself.
  
  A selection's colour no longer hides the text it selects: the text layer (`.lamina-text-layer`) now has `mix-blend-mode: multiply`, so an opaque `::selection` tint shows under the ink.
- baec4bc: The viewer is sharp at every zoom level. It sized each page's canvas and sharp tiles from the page's CSS width times the pixel ratio, while the renderer sizes the page in 32-bit floats, so at most zoom levels the right-hand tile came out one pixel wider than the page. The renderer refused it, and the rest of the page stayed blurry. `@laminapdf/render` now exports `pagePixels`, the page's size in pixels at a scale as its planner computes it, and the viewer sizes canvases and tiles with it.
- 70fec67: The page no longer flickers while zooming. Each pause in a pinch or ctrl+wheel zoom repainted the page for its new zoom with the blurry low-resolution pass, throwing away the sharp pixels it showed; the repaint now keeps those pixels, scaled, until the sharp tiles for the new zoom replace them.
- 19e70ca: When you zoom out far enough that more than three pages show, every page now turns sharp. Before, the first three did and the rest stayed blurry for good, because the canvas pool's rounding to whole pixels counted as a cap and every page waited for one of three detail canvases.
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
  - @laminapdf/sdk@0.2.0-alpha.4
  - @laminapdf/render@0.2.0-alpha.4
  - @laminapdf/render-hayro@0.2.0-alpha.4
  - @laminapdf/render-pdfium@0.2.0-alpha.4

## 0.2.0-alpha.3

### Patch Changes

- Updated dependencies [471ac52]
- Updated dependencies [0a68798]
- Updated dependencies [860bf03]
- Updated dependencies [110d8b9]
- Updated dependencies [bdd0af4]
- Updated dependencies [48dcd44]
- Updated dependencies [ee80542]
- Updated dependencies [9b579a4]
- Updated dependencies [f4a2d8e]
- Updated dependencies [7b2c22f]
- Updated dependencies [3536f81]
- Updated dependencies [658d8b8]
- Updated dependencies [2440a9a]
  - @laminapdf/sdk@0.2.0-alpha.3
  - @laminapdf/render-hayro@0.2.0-alpha.3
  - @laminapdf/render@0.2.0-alpha.3
  - @laminapdf/render-pdfium@0.2.0-alpha.3

## 0.2.0-alpha.2

### Patch Changes

- Updated dependencies [32a4963]
- Updated dependencies [2107764]
- Updated dependencies [a59a32a]
  - @laminapdf/sdk@0.2.0-alpha.2
  - @laminapdf/render-hayro@0.2.0-alpha.2
  - @laminapdf/render@0.2.0-alpha.2
  - @laminapdf/render-pdfium@0.2.0-alpha.2

## 0.2.0-alpha.1

### Minor Changes

- 5c2624e: The library is called Lamina PDF, and its public names follow. `@laminapdf/sdk`'s error class is `LaminaPdfError`, checked with `isLaminaPdfError`, and its warnings are `LaminaPdfWarning`. The error object's JSON Schema is `laminapdf.error/1`. `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro` (`.js`, `.d.ts` and `.wasm`).
- 08e123d: `@laminapdf/viewer` exports only its documented API: `ViewerClient`, `createViewer` and `Viewer`, the types of their options, events and state, the protocol types `client.call` takes, and `MIN_ZOOM`/`MAX_ZOOM`. Its internal helpers (layout, canvas pool, bitmap cache, tiles, zoom maths), its byte-source functions and its protocol constants are no longer exported. Its worker no longer keeps a log of every render, which grew without bound, and no longer serves test-only methods. `@laminapdf/sdk/worker/server`'s `serve()` takes a `saveBudget` (work units per save step), and `workerStats()` reports the handles and calls a worker holds. `@laminapdf/sdk/worker` exports the `Emitter` its client uses.

### Patch Changes

- 6f7ae81: `@laminapdf/render` has one `WasmSource` type for where an engine's `.wasm` comes from (a URL, a `Response` or a promise of one, the bytes or a compiled module). `CloneableWasmSource` is the subset a Web Worker can be sent. `initPdfium` and `initHayro` both take it: `@laminapdf/render-pdfium`'s own `PdfiumWasm` type is gone, and `initHayro` no longer takes a `Request`, a promise of anything but a `Response`, or a view of a `SharedArrayBuffer`. `RenderBackend` declares `memory()`, which both backends already had; a backend written outside this repository has to add it. The adapters share the page-index check, the read handling and the pending-range merging, so hayro now reports adjacent missing reads as one range, as PDFium did.
- 1c6e773: A render `data-pending` thrown inside `@laminapdf/sdk`'s worker now reaches the page with its byte ranges: `LaminaPdfError.ranges` lists them. Before, they arrived empty. `RenderError` has a new `toDiagnostic()`, which returns the plain error object every Lamina PDF surface uses, and the worker posts that object for any error that has the method. `RenderError`'s `toJSON()`, `reviveRenderError` and `SerializedRenderError` are gone, and `ByteRange` and `RenderErrorKind` name its types. The viewer's worker now throws `RenderError`s for `cancelled` and `disposed` instead of plain objects, so they reach the page with a `docsUrl`.
- Updated dependencies [5c2624e]
- Updated dependencies [1c6e773]
- Updated dependencies [6f7ae81]
- Updated dependencies [08e123d]
- Updated dependencies [2a9e364]
  - @laminapdf/sdk@0.2.0-alpha.1
  - @laminapdf/render@0.2.0-alpha.1
  - @laminapdf/render-hayro@0.2.0-alpha.1
  - @laminapdf/render-pdfium@0.2.0-alpha.1

## 0.2.0-alpha.0

### Patch Changes

- Updated dependencies [2b4384a]
- Updated dependencies [592d285]
  - @laminapdf/sdk@0.2.0-alpha.0
  - @laminapdf/render-hayro@0.2.0-alpha.0
  - @laminapdf/render@0.2.0-alpha.0
  - @laminapdf/render-pdfium@0.2.0-alpha.0
