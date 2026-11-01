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
- **Canvases.** A shell borrows its canvas from a pool of 12, which caps the canvases' total size at 64 million device pixels (`CanvasPool`). Safari fails past an undocumented total canvas area, so the pool shrinks a canvas rather than exceed the cap. Rendered bitmaps stay in a 256 MB cache, which closes the bitmaps farthest from the current page first (`BitmapCache`).
- **Progressive rendering.** A page first paints a blurry whole-page pass, at most 256 pixels wide, at `visible` priority. Then it paints sharp tiles, 512 CSS px × the device pixel ratio on a side, over the part in view. Pages in the margin get only the blurry pass, at `prefetch` priority. A canvas is resized only when the new pixels are ready, so the page never flashes white.
- **Zoom.** The modes are fit width, fit page, actual size (96 CSS px per inch) and 10–800 %. Zoom comes from ctrl/cmd+wheel, trackpad pinch (ctrl+wheel, or Safari's gesture events), touch pinch and the `+`, `-` and `0` keys. The document point under the cursor stays where it is: always vertically, and horizontally while the pages are wider than the viewport. Narrower pages are centred, so no horizontal scroll can keep it there. During a gesture the canvases stretch, and the sharp re-render waits until the gesture pauses for 150 ms.
- **Tall documents.** A document taller than 16 million CSS px at the chosen zoom gets its zoom capped, because browsers stop scrolling near that height.

**API.**

| Member | What it does |
| --- | --- |
| `createViewer({ client, margin?, gap?, pool?, cacheBytes?, settleMs?, zoom? })` | Makes a viewer over a `ViewerClient`. |
| `mount(container)`, `open(document)`, `destroy()` | Puts it on the page, shows a document, takes it down. |
| `goToPage(index, { anchor })`, `setZoom(zoom, { anchor })`, `rotate(delta)` | Navigation. `zoom` is a mode or a percentage. |
| `getState()`, `subscribe(listener)` | The view state (`currentPage`, `pageCount`, `zoom`, `percent`, `rotation`, `scrollTop`, `visibleRange`), for `useSyncExternalStore`. |
| `on("pagechange" \| "zoomchange" \| "renderstats", …)`, `stats()` | Events and counters. `renderstats` counts renders, for measuring performance. |
| `addLayer({ name, onMount, onUnmount, onZoom })` | Mounts a layer into each page. |
| `pointAt(x, y)`, `positionOf(point)` | Converts between viewport positions and document points. |

**Layers.** Each page shell stacks a canvas and three slots above it: `div.lamina-layer-text`, `div.lamina-layer-annot` and `div.lamina-layer-highlight`. The coming text layer, find highlights and links will mount there through `addLayer`. A layer with another name gets its own slot on top.

**Tuning.** The margin, pool size, pixel cap, cache budget and settle delay are starting values, and may change once they are measured.

## The document Web Worker

The client starts the Web Worker and talks to it; you call `ViewerClient`'s methods and listen to its events.

**Events:**

| Event | Source | Data | Meaning |
| --- | --- | --- | --- |
| `warning` | Web Worker | a reader warning | for example, a repaired xref |
| `render-start` | Web Worker | `{ page, scale }` | a render began, for measuring performance |
| `render-slow` | page | `{ id, page }` | the soft timeout passed, counted from `render-start`: waiting in the queue isn't slow |
| `render-stuck` | page | `{ id, page, needsPassword }` | an encrypted document's render passed the hard cap |
| `crashed` | page | `{ reason }` | the Web Worker crashed |
| `restarted` | page | `{ reason, reopened }` | a new Web Worker serves again |
| `password-required` | page | — | the new Web Worker can't reopen without the password |
| `restart-failed` | page | `{ reason, error }` | a restart failed (for example, the backend wouldn't load again) |

### How it behaves

- **Priorities and coalescing:** the Web Worker runs one call at a time, the highest priority first, FIFO within one.
  - Before each call it reads every message already sent, so a burst of renders and cancels is ordered and pruned before anything runs.
  - Renders with an equal normalised `RenderRequest` (`renderKey`: page, scale, clip, rotation, flags, background) run once, and each caller gets its own bitmap.
- **Cancellation:** `signal` rejects the call at once with a `DOMException` named `AbortError`.
  - A queued render is dropped.
  - A running one finishes, because synchronous WebAssembly can't be interrupted. Its late answer's bitmap is closed unpainted (`onDiscard`).
  - (`@laminapdf/sdk/worker` itself rejects with a `cancelled` LaminaError; the viewer maps it to `AbortError`.)
- **Timeouts:** the page keeps the timers, because a Web Worker inside a render can't post anything.
  - Defaults are `soft` 10 s (`render-slow`) and `hard` 120 s.
  - At the hard cap the client calls `stop("hard-timeout")`. For a document that needed a password, it emits `render-stuck` instead and waits for the user's `stop()`.
- **Crashes:** a crash is the Web Worker's `error` or `messageerror`, or a WebAssembly trap it reports, never silence.
  - Pending calls reject with `worker-crashed`.
  - A new Web Worker reopens the document from the same source, but nothing that was pending is re-sent. The UI offers Retry, so a page that crashes the engine can't loop.
  - `stop()` takes the same path with `reason: "cancelled"`.
- **Secrets:** the password crosses to the Web Worker once, when the document opens, and is kept only there. The page keeps the source and the other options.
  - After a crash of an encrypted document, `password-required` fires and the caller opens again.
  - lamina's tests count every message that carries the password, so it can't start crossing more often unnoticed.
- **Sources:**
  - A `Blob`/`File` is sent by reference, and the Web Worker reads it in 4 MiB `slice()`s.
  - Bytes are copied, and kept on the page for a reopen.
  - `ByteSource` (`size`, `read(offset, length, signal)`) is the interface for other sources; an HTTP range source is coming.

## For contributors

The viewer is the web client of [ADR 0009](../../docs/decisions/0009-web-client-architecture.md); it was built in [C3a](../../docs/todos/done/C3a-viewer-worker-rpc.md) (the document worker and its typed RPC) and [C3b](../../docs/todos/done/C3b-viewer-virtualiser-and-zoom.md) (the viewer). [C3c](../../docs/todos/ready/C3c-viewer-perf-harness.md) measures the tuning values with `renderstats`. The layers are for [C5a](../../docs/todos/ready/C5a-text-layer-and-selection.md) (text), [C5b](../../docs/todos/ready/C5b-find-in-document.md) (find) and [C6b](../../docs/todos/ready/C6b-links-and-history.md) (links); C4c builds the HTTP range source on `ByteSource`. Cancellation follows ADR 0009 §3.5, and warnings are [ADR 0003](../../docs/decisions/0003-reader-tolerance-and-limits.md)'s.

### Architecture

```text
 main thread                                   document worker (one per open document)
┌────────────────────────────┐   postMessage   ┌──────────────────────────────────────────┐
│ ViewerClient                │ ─────────────▶ │ @laminapdf/sdk/worker/server  serve({ … })   │
│  call(method, params, opts) │  call / cancel │  handshake · priority queue · coalescing  │
│  render timers (slow/stuck) │                │  cancellation · ADR 0026 errors · traps   │
│  restart + reopen on crash  │ ◀───────────── │   ├─ viewer methods (src/worker/)         │
│  AbortError on abort        │ result / event │   │   openDocument → lamina Document +    │
│ @laminapdf/sdk/worker client   │  (transferred  │   │                  RenderDocument       │
│  request() · onDiscard      │   ImageBitmap) │   │   render → backend → ImageBitmap      │
└────────────────────────────┘                 │   └─ lamina's own methods (open, …)       │
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
| `render` | ADR 0004 `RenderRequest` | `{ bitmap: ImageBitmap, width, height }` (transferred) | `visible` > `prefetch` > `background` |
| `__busy`, `__renderLog`, `__crash` | test-only | | |

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
- Unit tests sit next to their modules (`src/**/*.test.ts`) and run in Node. They cover the layout's cost, counted in steps, and the fit modes for portrait, landscape and rotated pages. They also cover zoom anchoring, tiles, cache eviction and the pool's caps. `test/dependencies.test.ts` checks that `src/` imports no UI framework, and `test/types.test.ts` checks the RPC types.
- Two demo pages are manual checks: run `pnpm --filter @laminapdf/viewer exec vite examples`. `examples/viewer-demo.html` is the viewer, with zoom buttons, and C3c's harness uses it; `examples/rpc-demo.html` draws page 1 through the RPC alone.
