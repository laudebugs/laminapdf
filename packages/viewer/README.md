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
| `createViewer({ client, margin?, gap?, pool?, detailPool?, cacheBytes?, backdropMaxPixels?, canvasMaxPixels?, device?, settleMs?, zoom? })` | Makes a viewer over a `ViewerClient`. |
| `mount(container)`, `open(document)`, `destroy()` | Puts it on the page, shows a document, takes it down. |
| `goToPage(index, { anchor })`, `setZoom(zoom, { anchor })`, `rotate(delta)` | Navigation. `zoom` is a mode or a percentage. |
| `getState()`, `subscribe(listener)` | The view state (`currentPage`, `pageCount`, `zoom`, `percent`, `rotation`, `scrollTop`, `visibleRange`), for `useSyncExternalStore`. |
| `on("pagechange" \| "zoomchange" \| "renderstats", …)`, `stats()` | Events and counters. `renderstats` counts renders, for measuring performance. |
| `addLayer({ name, onMount, onUnmount, onZoom })` | Mounts a layer into each page. |
| `pointAt(x, y)`, `positionOf(point)` | Converts between viewport positions and document points. |
| `MIN_ZOOM`, `MAX_ZOOM` | The bounds of a percentage zoom, 10 and 800, for a zoom control's range. |

**Layers.** Each page shell stacks a canvas and three slots above it: `div.lamina-layer-text`, `div.lamina-layer-annot` and `div.lamina-layer-highlight`. The coming text layer, find highlights and links will mount there through `addLayer`. A layer with another name gets its own slot on top.

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

The viewer is the web client of [ADR 0009](../../docs/decisions/0009-web-client-architecture.md); it was built in [C3a](../../docs/todos/done/C3a-viewer-worker-rpc.md) (the document worker and its typed RPC) and [C3b](../../docs/todos/done/C3b-viewer-virtualiser-and-zoom.md) (the viewer). [C3c](../../docs/todos/done/C3c-viewer-perf-harness.md) measures the tuning values with `renderstats`. The layers are for [C5a](../../docs/todos/ongoing/C5a-text-layer-and-selection.md) (text), [C5b](../../docs/todos/ready/C5b-find-in-document.md) (find) and [C6b](../../docs/todos/ready/C6b-links-and-history.md) (links); C4c will read documents by range through `@laminapdf/sdk`'s `RangeSource`. Cancellation follows ADR 0009 §3.5, and warnings are [ADR 0003](../../docs/decisions/0003-reader-tolerance-and-limits.md)'s.

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
| `render` | ADR 0004 `RenderRequest` | `{ bitmap: ImageBitmap, width, height }` (transferred) | `visible` > `prefetch` > `background` |
| `memory` | — | `WorkerMemory`: the bytes of the backend's and Lamina PDF's WebAssembly memories | document. `client.memory()`; the perf harness samples it. |

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
