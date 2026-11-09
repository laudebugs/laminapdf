# @laminapdf/viewer

`@laminapdf/viewer` shows a PDF in a web page: it scrolls a 1,000-page document smoothly, draws each page sharp, and zooms around the cursor. It has no UI framework inside, so a React toolbar or the desktop app drives it from outside. It has two parts:

- **The view** handles layout, virtualisation, canvases and zoom.
- **The document Web Worker**, one per open document, opens and renders the file away from the page's main thread.

The package is private and ESM-only.

```ts
import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { createViewer, ViewerClient } from "@laminapdf/viewer";

const client = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
const viewer = createViewer({ client });
viewer.mount(container); // the container gives the viewer its size
viewer.open(await client.open(file)); // a File, a Blob or bytes
viewer.subscribe(() => toolbar.update(viewer.getState())); // page, zoom, rotation
```

## The view

The view keeps only the pages near the viewport in the DOM and paints each in two passes through the document Web Worker. Six parts do that work:

- **Layout.** Every page's size comes from its displayed size in points, the zoom and the view rotation. The pages stack in one column with a gap. The offsets are prefix sums, so finding the page at a scroll position is a binary search, and a zoom change costs one pass over the pages (`PageLayout`).
- **Virtualisation.** The viewer mounts a shell for each page in view, plus a margin of 2 pages or one viewport each side, whichever is larger. A shell that leaves the margin aborts its renders. If a result still arrives for that page, the viewer drops it and never draws it. Scroll handling reads the DOM first and writes it once per animation frame.
- **Canvases.** A shell borrows its canvas from a pool of 12, which caps the canvases' total size at 40 million device pixels (`CanvasPool`). Safari fails past an undocumented total canvas area, so the pool shrinks a canvas rather than exceed the cap.
- **Detail canvases.** A page's canvas holds at most 8 million pixels (`BACKDROP_MAX_PIXELS`, below Safari's limit for one canvas). A page larger than that at the full pixel ratio, or one whose canvas the pool shrank, keeps its canvas as a backdrop. A detail canvas then covers the part in view at the full ratio. The detail canvas's size follows the viewport, not the zoom, so the page in view is sharp at every zoom up to 800 %.
- **Detail pool.** Detail canvases come from a second pool of 3 canvases and 24 million pixels. Together with the page pool, that is the 64 million pixels one pool held before. A page out of view gives its detail canvas back. When none is free, a page shows its backdrop until one is.
- **Bitmaps.** Rendered bitmaps stay in a 256 MB cache, which closes the bitmaps farthest from the current page first (`BitmapCache`).
- **Progressive rendering.** A page first paints a blurry whole-page pass, at most 256 pixels wide, at `visible` priority. Then it paints sharp tiles, 512 CSS px × the device pixel ratio on a side, over the part in view. Pages in the margin get only the blurry pass, at `prefetch` priority. A canvas is resized only when the new pixels are ready, so the page never flashes white.
- **Zoom.** The modes are fit width, fit page, actual size (96 CSS px per inch) and 10–800 %. Zoom comes from ctrl/cmd+wheel, trackpad pinch (ctrl+wheel, or Safari's gesture events), touch (below) and the `+`, `-` and `0` keys. The document point under the cursor stays where it is: always vertically, and horizontally while the pages are wider than the viewport. Narrower pages are centred, so no horizontal scroll can keep it there. During a gesture the canvases stretch, and the sharp re-render waits until the gesture pauses for 150 ms.
- **Touch.** The viewer handles touch itself, and its root has `touch-action: none`, so the browser's page zoom never fights it. The browser still zooms the rest of the page, so accessibility zoom keeps working. A pen pans like a finger until a mode draws with it; the mouse stays the browser's.
- **Gestures.** One finger pans once it moves 10 px, and a flick carries the page on as it slows. Two fingers pinch: the document point that started under them stays under them, even when they also move together. At fit width a double tap zooms in; at any other zoom it returns to fit width. A finger held still for half a second is left to the browser, so a long press still selects text.
- **Canvas size on touch devices.** A phone or tablet can't hold the desktop's canvases: iOS Safari reloads a tab whose canvases pass 224–384 MB in all. With a coarse pointer, every canvas holds at most 5 million pixels; on a tablet whose window covers more than 5 million device pixels, the page in view shows its backdrop rather than a detail canvas.
- **Memory on touch devices.** The two pools and the bitmap cache together stay under 184 MB by default. On a device that reports 2 GB of memory or less (`navigator.deviceMemory`, in Chromium) the limit is 116 MB, and on one that reports 4 GB or more, 304 MB. `createViewer`'s options override each budget, and `device` gives the hints the budgets are chosen from instead of detecting them.
- **Tall documents.** A document taller than 16 million CSS px at the chosen zoom gets its zoom capped, because browsers stop scrolling near that height.

**API.**

| Member | What it does |
| --- | --- |
| `createViewer({ client, margin?, gap?, pool?, detailPool?, cacheBytes?, backdropMaxPixels?, canvasMaxPixels?, device?, settleMs?, zoom?, text? })` | Makes a viewer over a `ViewerClient`. `text` is on by default; `false` leaves the text layer out, and an object sets its `outline`, `untaggedLabel` and `onSelection`. |
| `mount(container)`, `open(document)`, `destroy()` | Puts it on the page, shows a document, takes it down. |
| `goToPage(index, { anchor })`, `setZoom(zoom, { anchor })`, `rotate(delta)` | Navigation. `zoom` is a mode or a percentage. |
| `goToDestination(target)` | Goes to an outline item's or a named destination's target (below). |
| `getState()`, `subscribe(listener)` | The view state (`currentPage`, `pageCount`, `zoom`, `percent`, `rotation`, `scrollTop`, `visibleRange`), for `useSyncExternalStore`. |
| `on("pagechange" \| "zoomchange" \| "renderstats", …)`, `stats()` | Events and counters. `renderstats` counts renders, for measuring performance. |
| `addLayer({ name, onMount, onUnmount, onZoom })` | Mounts a layer into each page. |
| `keepMounted(pages)` | Keeps these pages mounted wherever the view scrolls, replacing the previous set; `[]` releases them. The text layer pins the pages a selection touches. |
| `pointAt(x, y)`, `positionOf(point)` | Converts between viewport positions and document points. |
| `MIN_ZOOM`, `MAX_ZOOM` | The bounds of a percentage zoom, 10 and 800, for a zoom control's range. |

**Layers.** Each page shell stacks a canvas and three slots above it: `div.lamina-layer-text`, `div.lamina-layer-annot` and `div.lamina-layer-highlight`. The text layer fills the first; find's highlights mount in `highlight`, and links will mount in `annot`, through `addLayer`. A layer with another name gets its own slot on top. Every slot above the text lets the pointer through (`pointer-events: none`), so text stays selectable under it; an element a layer wants clicked, such as a link, sets `pointer-events: auto` on itself.

**Text.** The text layer makes every page's text selectable, copyable and readable by screen readers, whichever backend draws the page.
- **Spans.** Each text run of Lamina PDF's extraction gets one transparent span, placed from its glyphs' quads, never from the renderer's. A rotated or vertical run gets a rotated span, and a span reads right to left when its first letter does. The spans sit in a box sized in points, which one CSS transform scales and turns to the page, so a zoom or a rotation moves no span.
- **Selection.** Only the spans are selectable. A drag runs from one page into the next and never takes in a canvas or a gap. A drag that leaves the viewer also highlights your app's text; set `user-select: none` on text you want to keep out. Copy, the pinned pages and `onSelection` keep to the part inside the viewer either way. Keep `user-select: none` off every ancestor of the viewer: Firefox then stops selecting. The pages a selection touches stay mounted while it lasts. Ctrl/⌘+A in the viewer selects the text of the mounted pages only.
- **Copy** writes the extracted text in reading order: a space at a gap, a new line at a new baseline and between pages, soft hyphens dropped. `onSelection` gives the app the selected text, its rects, its pages and the pointer that made it.
- **Structure.** For a tagged document, a hidden tree of ARIA elements follows the structure tree. Headings carry their levels, and figures their alternate text; paragraphs, lists and tables keep their roles. Each element owns its text spans through `aria-owns`, matched by page and marked-content ID (the number a tagged PDF gives each piece of a page's content), so a screen reader reads the document in structure order. For a document without a structure tree, a screen reader hears "Untagged document" once (`untaggedLabel`).

The document Web Worker reads each page's text at `background` priority, after the tiles in view, and caches the text of the last 24 pages. Text needs the extract module, which the document Web Worker loads once the render backend is ready, so the first page never waits for it. Selection colours are the app's: style `.lamina-text-layer span::selection`, since the viewer adds no style sheet.

**Find.** `viewer.find` searches the whole document and highlights what it finds.
- **How it searches.** `find(query, { matchCase, wholeWords, matchDiacritics })` starts from the current page, reaches the last page and wraps to the first. The document Web Worker searches four pages per call, at `background` priority, so the pages in view render first and scrolling never waits. It keeps each page's searchable text in a cache of its own, up to 32 MB, so the next query, such as one you typed more letters into, searches the pages it has already read without reading their text again.
- **What it compares.** Find normalises text the way pdf.js does, so the counts match: ligatures unfolded, soft hyphens dropped, a word split by a hyphen at a line's end joined, white space collapsed. Case and diacritics are ignored unless asked, and diacritics are every combining mark: Latin accents, Hebrew points, Arabic harakat, Indic vowel signs. Whole words means no letter or digit runs on at either end.
- **Results.** Matches stream in by page: `getState()` gives `total`, `current`, `searched`, `pagesWithoutText` and `done` as matches arrive. The first match on or after the current page shows as soon as it's found. `next()` and `previous()` move in document order and wrap, scrolling a match to the viewport's centre when it's out of view.
- **The list of results.** `results()` lists every match found so far in document order, each with its `ordinal`, its `page` and a `snippet` of `{ before, match, after }`. The snippet keeps up to about 40 characters of the page's text on each side, cut at a word, with the match as the page spells it. `goTo(ordinal)` makes one current and scrolls it into view, and `next()` and `previous()` go on from there. The list is the same array until find's state next changes, so a React component can read it on each render.
- **Cancelling.** A new query, or `close()`, stops the search under way; its late results never show.
- **Highlights.** Every match on a mounted page gets a box with the class `lamina-find-match` in the `highlight` slot; the current match also has `lamina-find-current`. Colour them with `--lamina-find-match` and `--lamina-find-current`; the current one also has an outline, `--lamina-find-current-outline`, so it never differs by colour alone. The boxes take no pointer events, so they never get in a selection's way.

**Destinations.** `ViewerClient.outline()` reads the document's outline in the document Web Worker, and `namedDestination(name)` finds one named destination. Each item's target is a page and a view in page space: points from the displayed page's top-left, y down, whichever coordinates the file used. `goToDestination(target)` follows the target's view (ISO 32000-2 §12.3.2.2). It applies the target's zoom, if it has one, and then puts the target's point at the viewport's top-left:
- **A point** (`at`, from `/XYZ`) at its zoom, or at the current zoom when the file gives none.
- **Fit the page** (`fit`, from `/Fit` and `/FitB`).
- **Fit the width** from a top (`fit-width`, from `/FitH` and `/FitBH`), or the height from a left (`fit-height`, from `/FitV` and `/FitBV`).
- **Fit a rectangle** (`fit-rect`, from `/FitR`).

A left the file leaves out keeps the current horizontal scroll, and a top it leaves out goes to the page's top. A view rotated by `rotate()` turns the point with the page, and a fit always fits the viewport's own width or height, as in pdf.js.

**Thumbnails.** `new ThumbnailStrip(client, viewer, { width?, margin?, cacheBytes?, label?, name? })` makes a list of the pages as small pictures, for a sidebar. `mount(container)` puts it on the page, and `open(document)` shows the document the viewer opened. Size its `element` with CSS; it scrolls inside.
- **Like the viewer, it mounts only what is near its view.** It keeps the thumbnails in view and 4 either side. Each borrows a canvas from the strip's own pool, and a thumbnail scrolled away aborts its render.
- **Renders wait for the main view.** Thumbnails render at `background` priority, so the pages in view always come first. Bitmaps stay in a cache of their own, 32 MB by default.
- **It follows the viewer.** The current page is marked `aria-selected` and scrolled into the strip; a rotation turns every thumbnail.
- **Keyboard and pointer.** A click, or Enter or Space, goes to that page. The arrow keys, Home and End move the focus, and only one thumbnail is in the Tab order.
- **Names.** Each thumbnail is named by its page label and number ("Page iv (4 of 120)") and captioned with its label. For another language, pass `label` to replace both and `name` to replace the list's own name.

**Tuning.** The margin, pool size, pixel cap, cache budget and settle delay are measured by the perf harness ([Performance](#performance)); the first full run kept them.

## The document Web Worker

The client starts the Web Worker and talks to it; you call `ViewerClient`'s methods and listen to its events.

**Events:**

| Event | Source | Data | Meaning |
| --- | --- | --- | --- |
| `warning` | Web Worker | a reader warning | for example, a repaired xref |
| `progress` | Web Worker | `{ loaded, total }` | bytes of a remote document read so far, and its size |
| `render-start` | Web Worker | `{ page, scale }` | a render began, for measuring performance |
| `render-fetching` | Web Worker | `{ key }` | a remote document's render stopped to fetch bytes; `render-start` follows |
| `network` | Web Worker | `{ worker, index, record }` | the Web Worker made a network request (`NetworkRecord`: URL, method, what made it, bytes, failed) |
| `csp-violation` | Web Worker | `{ worker, index, violation }` | the Web Worker's Content Security Policy blocked a request |
| `render-slow` | page | `{ id, page }` | the soft timeout passed, counted from `render-start`: waiting in the queue isn't slow |
| `render-stuck` | page | `{ id, page, needsPassword }` | an encrypted document's render passed the hard cap |
| `crashed` | page | `{ reason }` | the Web Worker crashed |
| `restarted` | page | `{ reason, reopened }` | a new Web Worker serves again |
| `password-required` | page | — | the new Web Worker can't reopen without the password |
| `restart-failed` | page | `{ reason, error }` | a restart failed (for example, the backend wouldn't load again) |

**Network requests:** the Web Worker counts every request it makes from the moment it starts, with `watchNetwork`, which this package also exports for the page. The function wraps `fetch`, `XMLHttpRequest`, `WebSocket` and `sendBeacon`, and reads everything else from Resource Timing, so a reader can show what left the device.

### How it behaves

- **Priorities and coalescing:** the Web Worker runs one call at a time, the highest priority first, FIFO within one.
  - Before each call it reads every message already sent, so a burst of renders and cancels is ordered and pruned before anything runs.
  - Renders with an equal normalised `RenderRequest` (`renderKey`: page, scale, clip, rotation, flags, background) run once, and each caller gets its own bitmap.
- **Cancellation:** `signal` rejects the call at once with a `DOMException` named `AbortError`.
  - A queued render is dropped.
  - A running one finishes, because synchronous WebAssembly can't be interrupted. Its late answer's bitmap is closed unpainted (`onDiscard`).
  - (`@laminapdf/sdk/worker` itself rejects with a `cancelled` LaminaPdfError; the viewer maps it to `AbortError`.)
- **Timeouts:** the page keeps the timers, because a Web Worker inside a render can't post anything.
  - Defaults are `soft` 10 s (`render-slow`) and `hard` 120 s.
  - At the hard cap the client calls `stop("hard-timeout")`. For a document that needed a password, it emits `render-stuck` instead and waits for the user's `stop()`.
- **Crashes:** a crash is the Web Worker's `error` or `messageerror`, or a WebAssembly trap it reports, never silence.
  - Pending calls reject with `worker-crashed`.
  - A new Web Worker reopens the document from the same source, but nothing that was pending is re-sent. The UI offers Retry, so a page that crashes the engine can't loop.
  - `stop()` takes the same path with `reason: "cancelled"`.
- **Secrets:** the password crosses to the Web Worker once, when the document opens, and is kept only there. The page keeps the source and the other options.
  - After a crash of an encrypted document, `password-required` fires and the caller opens again.
  - Lamina PDF's tests count every message that carries the password, so a change that sends it more often fails them.
- **Sources:**
  - A `Blob`/`File` is sent by reference, and the Web Worker reads it in 4 MiB `slice()`s.
  - Bytes are copied, and kept on the page for a reopen.
  - `{ url }` is a remote file the Web Worker fetches itself, so its bytes never reach the page:
    - It reads the file with HTTP Range requests through `@laminapdf/sdk`'s `httpRangeSource`, in 64 KiB blocks with one block read ahead, and keeps up to 64 MiB of them.
    - Lamina PDF opens the file range by range, and a render that needs bytes not yet read fetches them and draws again, so page 1 of a large file needs only a small part of it.
    - While a render fetches, it emits `render-fetching` and its timeouts pause until it draws again (`render-start`): a slow network is never a stuck render. The Web Worker still runs one call at a time, so other calls wait for that fetch.
    - Requests send no cookies or referrer and skip the HTTP cache.
    - A server that ignores `Range` sends the whole file, which the Web Worker accepts up to 256 MiB (`source-budget-exceeded` above that).

## For contributors

The viewer is the web client of [ADR 0009](../../docs/decisions/0009-web-client-architecture.md); it was built in [C3a](../../docs/todos/done/C3a-viewer-worker-rpc.md) (the document worker and its typed RPC) and [C3b](../../docs/todos/done/C3b-viewer-virtualiser-and-zoom.md) (the viewer). [C3c](../../docs/todos/done/C3c-viewer-perf-harness.md) measures the tuning values with `renderstats`. The layers are for [C5a](../../docs/todos/done/C5a-text-layer-and-selection.md) (text), [C5b](../../docs/todos/done/C5b-find-in-document.md) (find) and [C6b](../../docs/todos/ready/C6b-links-and-history.md) (links); C4c will read documents by range through `@laminapdf/sdk`'s `RangeSource`. Cancellation follows ADR 0009 §3.5, and warnings are [ADR 0003](../../docs/decisions/0003-reader-tolerance-and-limits.md)'s.

### Architecture

```text
 main thread                                   document worker (one per open document)
┌────────────────────────────┐   postMessage   ┌──────────────────────────────────────────┐
│ ViewerClient                │ ─────────────▶ │ @laminapdf/sdk/worker/server  serve({ … })   │
│  call(method, params, opts) │  call / cancel │  handshake · priority queue · coalescing  │
│  render timers (slow/stuck) │                │  cancellation · ADR 0026 errors · traps   │
│  restart + reopen on crash  │ ◀───────────── │   ├─ viewer methods (src/worker/)         │
│  AbortError on abort        │ result / event │   │   openDocument → SDK Document +       │
│ @laminapdf/sdk/worker client   │  (transferred  │   │                  RenderDocument       │
│  request() · onDiscard      │   ImageBitmap) │   │   render → backend → ImageBitmap      │
└────────────────────────────┘                 │   └─ the SDK's own methods (open, …)      │
                                               └──────────────────────────────────────────┘
```

There is one protocol, not two: the viewer's worker is `@laminapdf/sdk/worker`'s server with the viewer's methods added (`serve({ methods })`), and the client calls them through `WorkerClient.request` (ADR 0009 §3, [L2a](../../docs/todos/done/L2a-worker-client.md)).

### Methods

| Method | Params | Result | Priority |
| --- | --- | --- | --- |
| `initBackend` | `{ backend: "pdfium" \| "hayro", wasm? }` | `{ name, version }` | document. Sent by `create()`, before a file is chosen, so the WebAssembly downloads in parallel with file selection. |
| `openDocument` | `source` (`Blob`, `ArrayBuffer`, `Uint8Array`), `{ password?, limits?, renderLimits? }` | `{ pageCount, pages, needsPassword, backend }` | document |
| `closeDocument` | — | — | document |
| `pageInfo` | `index` | `PageInfo` (displayed size in points, rotation, ID, label) | document |
| `find` | `query`, `FindOptions`, `pages` | `FindPageResult[]`: each page's matches as quads, and whether it has no text | `background` |
| `render` | ADR 0004 `RenderRequest` | `{ bitmap: ImageBitmap, width, height }` (transferred) | `visible` > `prefetch` > `background` |
| `memory` | — | `WorkerMemory`: the bytes of the backend's and Lamina PDF's WebAssembly memories | document. `client.memory()`; the perf harness samples it. |
| `pageText` | `index` | `PageTextData`: the page's runs and glyph quads | `background` |
| `structure` | — | `StructureData`: whether the document is tagged, and its structure tree | document |
| `outline` | — | `OutlineNode[]`: the outline, each item's target in page space | document |
| `namedDestination` | `name` | `Target \| undefined` | document |
| `networkLog` | — | `WorkerNetworkLog`: every request and blocked request since the Web Worker started | document. `client.networkLog()`: what the `network` and `csp-violation` events reported, those sent before anyone listened included; `index` merges the two. |

`client.call(method, params, { priority, signal })` is typed by the `Methods` map: an unknown method or wrong params fails `tsc` (`test/types.test.ts`).

### Tests

The browser suites run in Chromium, Firefox and WebKit over the built package: run `cargo xtask pdf-runtimes chromium|firefox|webkit`, or `pnpm exec vitest run -c vitest.browser.config.ts` after `cargo xtask wasm`.

- `test/browser/viewer.browser.test.ts` scrolls a 1,000-page document that the test builds with `@laminapdf/sdk`'s builder. It covers:
  - scrolling one viewport per frame from page 1 to 1,000 with at most visible + 2 × margin shells, and no more canvases than the pool holds;
  - a fast scroll past 200 pages that paints no cancelled render;
  - every visible page reaching sharp tiles once scrolling stops, including the part of a tall zoomed page scrolled into later;
  - ctrl+wheel zoom 100 % → 200 % → 50 % holding the point under the cursor within 1 CSS px;
  - keyboard zoom, rotation and layers.
- `test/browser/rpc.browser.test.ts` covers the document worker, and counts every `postMessage` that carries the password:
  - the corpus page counts and sizes against `fixtures/corpus/expected.json`;
  - hayro;
  - limits;
  - 100 renders with 90 aborted, and a render aborted mid-run;
  - priorities and coalescing;
  - slow, stuck and stopped renders;
  - crashes, including an encrypted file;
  - the password count.
- `test/find-parity.test.ts` (Node) checks find's counts against pdf.js's find controller on every case of `fixtures/text/search.json`.
- Unit tests sit next to their modules (`src/**/*.test.ts`) and run in Node. They cover the layout's cost, counted in steps, and the fit modes for portrait, landscape and rotated pages. They also cover zoom anchoring, tiles, cache eviction and the pool's caps. `test/dependencies.test.ts` checks that `src/` imports no UI framework, and `test/types.test.ts` checks the RPC types.
- `test/browser/navigation.browser.test.ts` covers the thumbnails strip over a 1,000-page document:
  - while the whole strip scrolls, no more than the thumbnails in view and 4 either side mounted;
  - a visible render, queued behind thumbnails, running first;
  - a click and the keyboard going to a page;
  - page labels in the names;
  - the outline and a named destination of the links-and-outlines reference PDF landing on their pages.
- Unit tests sit next to their modules (`src/**/*.test.ts`) and run in Node. They cover the layout's cost, counted in steps, and the fit modes for portrait, landscape and rotated pages. They also cover zoom anchoring, tiles, cache eviction and the pool's caps, and where each kind of destination puts the view at each rotation. `test/dependencies.test.ts` checks that `src/` imports no UI framework, and `test/types.test.ts` checks the RPC types.
- Two demo pages are manual checks: run `pnpm --filter @laminapdf/viewer exec vite examples`. `examples/viewer-demo.html` is the viewer, with zoom buttons; `examples/rpc-demo.html` draws page 1 through the RPC alone. The perf harness has its own page, `perf/page/` ([Performance](#performance)).

### Performance

A harness in `perf/` measures the viewer against the [web plan's targets](../../docs/plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) in Chromium, Firefox and WebKit:

```sh
cargo xtask wasm                                   # once: the package the harness runs
pnpm --filter @laminapdf/viewer perf               # every browser; -- --browsers chromium --quick for a short run
cargo xtask viewer-perf [chromium|firefox|webkit]  # the same, plus the job summary and the nightly history
```

It writes `target/viewer-perf/<browser>.json` and `summary.md`, and never fails on a missed target: no pull request lane asserts a duration ([ADR 0018](../../docs/decisions/0018-test-strategy.md#2-ci-lanes)).

**What it does.** It opens three seeded files, built with `@laminapdf/sdk` and cached in `perf/.fixtures/`: 10 pages of text with an image each (about 1 MB), 1,000 pages of text, and 300 pages of full-page images. A Vite build of `perf/page/` shows them, and `perf/serve.ts` serves that page gzipped, as Cloudflare serves the app. Playwright drives real input, so every run scrolls the same way:

| Scenario | Measures | Target |
| --- | --- | --- |
| Open the 10-page file, warm (after a reload) | time to the first page's first paint | ≤ 700 ms |
| The same, cold, on Chrome DevTools' Fast 4G (Chromium only) | time from navigation to the first page | ≤ 3 s |
| Scroll the 1,000-page file end to end, twice, with wheel steps every 20 ms; stop at a quarter, a half and three quarters | the longest main-thread task, time to sharp after each stop, peak memory, memory after each pass, cancelled renders that painted | no task > 50 ms; sharp ≤ 300 ms; ≤ 600 MB; second pass ≤ first + 5 %; 0 |
| Fling across the 300 image pages, then stop | time to sharp | ≤ 300 ms |
| Three ctrl+wheel bursts in and out on the 10-page file | the longest frame, time to sharp after each burst | reported |

"Time to sharp" runs from the last scroll or zoom event to the moment every visible page turned sharp. Memory is the page's JS heap (Chromium only), the document Web Worker's WebAssembly memories (`client.memory()`), the bitmap cache and the canvases' pixels. Firefox and WebKit have no Long Tasks API, so their longest task is the longest gap between frames, which is an upper bound.

**History.** The nightly lane runs `cargo xtask viewer-perf --history … --record` on `main` and keeps each browser's figures in the benchmark history (`viewer/<browser>/<figure>`: nanoseconds, or bytes for memory). A figure more than 10 % worse than the median of its last five runs on the same runner opens an issue, as a benchmark does; `--dry-run` prints that issue. `--quick` covers 100 pages and is never recorded.

**The reference machine.** The official numbers are the nightly lane's, from the Mac that hosts the self-hosted runners, with nothing else running on it. Each report ends with the machine it ran on (CPU, cores, memory, OS). Numbers from other machines, CI containers included, are relative: compare them with each other, not with the targets.

**First measurements and tuning.** The first full run (Chromium 153 on a 4-core Linux container, 2026-11-05) came in under every scrolling and memory target, a first reading until the reference machine's numbers arrive. It had no long task while scrolling, pages were sharp 0–17 ms after a scroll stopped and 33 ms after a fling, memory peaked at 302 MB, the second pass ended 0.5 % above the first, and no cancelled render painted. So the defaults above stay: a margin of 2 pages or one viewport, a pool of 12 canvases and 64 million pixels, a 256 MB bitmap cache, 512 CSS px tiles and a 150 ms settle. It missed one target and found one slow spot, both for [C7c](../../docs/todos/ready/C7c-performance-gates-and-launch.md):

- **The cold first page took 4.2 s on Fast 4G (target 3 s).** The download alone takes about 3.7 s at Fast 4G's 1 MB/s: PDFium's WebAssembly (2.2 MB gzipped), Lamina PDF's base module (0.4 MB) and the 1 MB file.
- **Zooming in took about 1.2 s to sharp, with a 317 ms frame.** Zooming out took about 300 ms. The plan sets no zoom target.
