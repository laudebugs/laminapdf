# 0009 — Web client: React chrome around a framework-free viewer, one document worker, typed RPC

- Status: Accepted (2026-10-31, when C2a's PR merges)
- Date: 2026-10-31

## Context

The web reader ([web-app plan](../plans/web-app.md)) and later the editor and the desktop app share one UI. Every serious viewer splits the work the same way ([rendering notes §2](../research/architecture/notes/rendering.md#2-viewer-architectures-pdfjs-chromepdfium-nutrientapryse-desktop-viewers); [web client research](../research/architecture/app-and-package-architecture.md#the-web-client-is-react-chrome-around-an-imperative-viewer-core)):

- parsing and rasterising happen off the main thread;
- the raster canvas sits under DOM text, annotation and form layers;
- a virtualised viewer mounts only a few dozen page shells, so the framework's raw DOM speed hardly matters ([web-frontend notes §1](../research/architecture/notes/web-frontend.md#1-framework-choice-react-19--compiler-solid-svelte-5-vue-preact-lit-and-what-production-pdfdocument-editors-use)).

What has to be decided:
- the framework for the chrome;
- what stays framework-free;
- the worker topology and the RPC across it;
- how bitmaps reach the screen;
- whether to use WebAssembly threads (and so COOP/COEP);
- where view state lives;
- the build and test tools.

The [C1b spike](../../spikes/pdfium-wasm/README.md) measured the worker on one machine (Apple M1 Pro) in Chromium, Firefox and WebKit:
- **Instantiating:** PDFium-WASM and lamina's base module instantiate side by side in one worker in **25–60 ms**, each with its own `WebAssembly.Memory`.
- **First page:** page 1 of a 1 MB file is visible **15–39 ms** after the `File` reaches the worker, and 6–8 ms warm.
- **Render cost:** copying a rendered bitmap out of the heap and `createImageBitmap` add **under 1 ms at 1×** and 1–5 ms at 2×.
- **Input by ranges:** PDFium's `FPDF_LoadCustomDocument` and lamina's `Document.openRange`, both over `File.slice()`, cut the 100 MB scan from 95 + 237 MiB of heap (plus a JS copy) to 1 + 113 MiB. Ranges are also the only way WebKit opened a 500 MB file.
- **Saves:** Chromium runs one long save about 10× slower in a fresh worker until V8 optimises the code.

[ADR 0004](0004-render-backend.md) fixes the render contract, and L2a's `@laminapdf/sdk/worker` ([shareable-libraries §5 L2](../plans/shareable-libraries.md)) is the async twin of the sync API.

## Decision

### 1. Packages and the framework line

- **`@laminapdf/viewer`** is framework-free TypeScript with no React import. That is enforced by a dependency check in `cargo xtask check ts` (C3a). It holds:
  - the worker RPC client;
  - the prefix-sum page virtualiser with zoom anchoring;
  - the canvas pool;
  - the tile scheduler (low-resolution pass, then sharp 256–512 px tiles, cancelled on scroll and zoom);
  - the DOM text layer, the find controller and the link layer;
  - a small view store.

  It is the hot path, and it is what other shells and embedders reuse.
- **`@laminapdf/ui`** holds the chrome only, in **React 19 with React Compiler 1.0**: toolbar, sidebars, dialogs and panels. Accessible primitives come from **React Aria Components** (Apache-2.0, chosen for keyboard, i18n and RTL support).
- **View state** (current page, zoom, scroll anchor, selection, search results) lives in the viewer's external store. React reads it with `useSyncExternalStore`, and **page shells are never React state**.
- **Interaction modes** (empty → opening → needs-password → open → error, and later the editor's tools) are a state machine in `@laminapdf/ui`'s `ReaderController`. ADR 0024 builds on it.
- **Embedding:** `@laminapdf/viewer`'s API and its RPC types stay **internal until the API settles**. They are published with the workspace but documented as unstable, and nothing promises compatibility before the owner decides to publish them ([web-app §8](../plans/web-app.md#8-open-questions) Q4).

### 2. One document worker per open document

- **Each open document gets its own module worker**, which runs:
  - `@laminapdf/sdk` (lamina's WebAssembly: the base module, plus `lamina_pdf_extract` when C5's text layer needs it, loaded lazily);
  - the ADR 0004 render adapter: `@laminapdf/render-pdfium` by default, `@laminapdf/render-hayro` from the debug menu.
- **Isolation:** a hostile file can crash its own worker, never the page or another document.
- **Single-threaded, one call at a time:** within a worker, every WebAssembly call is synchronous, so render and lamina calls are serialised by the scheduler (§3).
- **Thumbnails** use the same worker at `background` priority. A separate thumbnail worker is allowed only if C3c's perf harness shows the visible queue starving, and it would open its own copy of the document.
- **Input:** the worker gets the `File` or an OPFS `FileSystemFileHandle` (both structured-cloned in the message body; neither is transferable), or the URL of a `#url=` document (ADR 0011), and **never copies it whole**. For OPFS it creates the `FileSystemSyncAccessHandle` itself (`createSyncAccessHandle()` is worker-only, and the handle can't be posted). The worker builds one `RenderSource` (ADR 0004 §1) over that input: lamina opens it with `Document.openRange(callbackSource(size, read))`, and PDFium with custom file access over the same reads (C1b's custom strategy). Whole-buffer opening is allowed only for files under 32 MiB, where it is simpler and the copies are small.
- **Saves** run as `saveJob()` stepped in small budgets (ADR 0028), never one long `save()`, which keeps the worker answering and avoids Chromium's slow first save (C1b, finding 6).
- **Loading:**
  - The app shell starts `WebAssembly.compileStreaming` for both modules as soon as it loads, in parallel with the user picking a file, so the download (about 2.5 MB gzip together) overlaps the user's action.
  - Each document worker instantiates the compiled modules it receives (a `WebAssembly.Module` can be posted to a same-origin worker), so compiling happens once per page load. Where a browser refuses that, the worker streams its own compile from the service-worker cache. C3a tests both paths.
  - The service worker caches the modules immutably, by content hash (C7b).

### 3. The worker RPC

The viewer's protocol **extends `@laminapdf/sdk/worker`** (L2a) rather than defining a second one. It is hand-written and typed, not Comlink. C3a implements these requirements:

1. **Request IDs.** Every request has a monotonically increasing `id` per connection. Responses are `{ id, ok: true, result }` or `{ id, ok: false, error }`, where `error` is the ADR 0026 object; an `Error` is never posted, because structured clone loses its fields. **Worker events** use L2a's envelope `{ kind, id? }`. `progress` carries the `id` of the call it belongs to, so per-call `onProgress` routes correctly. `warning` is document-wide and carries none. **Client events** are raised on the main thread by the client's own timers and crash detection, because a worker stuck in a synchronous call can't post anything:
   - `render-slow { id, page }`;
   - `render-stuck { id, page, needsPassword }`;
   - `crashed`.
2. **A typed method union.** Methods and their params and results are one TypeScript union derived from the sync API's interfaces (`Promisified<T>`, L2a), plus the render methods. An unknown method or wrong params fails `tsc`.
3. **Priorities.** The levels are `visible` > `prefetch` > `background` (thumbnails, search). The worker's scheduler runs one call at a time, takes the highest priority first, and coalesces renders whose whole ADR 0004 `RenderRequest` is equal (page, scale, clip, rotation, flags and background), so a rotated or differently backed tile is never answered with another's bitmap.
4. **Transfer lists.** `ImageBitmap`s and `ArrayBuffer`s cross by transfer, never by copy: bitmaps from the worker, file chunks and text buffers (`PageText.buffers`) in either direction.
5. **Cancellation.** `call(method, params, { signal })`: aborting sends `cancel { id }` and rejects at once with an `AbortError`.
   - The worker drops a queued request.
   - A request already running can't see the cancel: the worker is single-threaded, so the `cancel` message waits until the call returns (ADR 0028 §5). It finishes, and its result is discarded. That's why renders are tiles and long work is stepped jobs: cancellation lands between them. Only terminating the worker stops a running call sooner (§3.7).
   - A late response for a cancelled ID is discarded, and any bitmap in it is `close()`d. **A cancelled render never paints.**
   - **An open is the exception** (amended 2026-11-09): the worker may have opened the new file and closed the old one before the cancel arrived. After a cancelled open, the client asks the worker which document it holds (`currentDocument`, tagged with the open's ticket). If that open made it, the open resolves with it after all; the page and the worker never disagree about the document. So an aborted open settles only after the worker finishes it; `stop()` (terminating the worker) ends a stuck one.
6. **Restart after a crash.** A crash is the worker's `error` or `messageerror` event, or a WebAssembly trap reported by the server. It is never silence: a synchronous render can't answer, so there is no heartbeat.
   - On a crash, pending calls reject with `worker-crashed`. A new worker reopens the document from the same source, and the viewer re-requests its visible tiles **except the request that was running when the worker died**.
   - That page shows "Couldn't render this page" with Retry instead of being re-sent automatically, so a page that crashes the engine can't loop through crash and restart. Retry is the user's choice.
   - An encrypted document asks for the password again. The password is never remembered outside the worker.
7. **Long work.** A render has a soft timeout (default 10 s), after which the client emits `render-slow` (a client event, §3.1) so the UI can offer Stop, and a hard cap (default 120 s).
   - Only the user's Stop, or the hard cap, terminates the worker.
   - **Encrypted documents are the exception:** the password lives only in that worker, so reaching the hard cap makes the client emit `render-stuck { needsPassword: true }`. The worker is terminated only after the user confirms, knowing they will be asked for the password again.
8. **Secrets.** A password crosses the boundary exactly once, in the `open` call, and no main-thread object keeps it afterwards (C3a asserts this).

### 4. Pixels to the screen

The worker renders tiles into its WebAssembly heap (ADR 0004's straight RGBA), wraps them as `ImageData`, calls `createImageBitmap`, and **transfers the `ImageBitmap`**. The main thread `drawImage`s it onto a **pooled** canvas and closes it.

- **Bounded:** no canvas exceeds ADR 0004's `max_render_pixels`, and the pool caps the total canvas area per document. iOS Safari's limit is about 224–384 MB across all canvases (UI/UX plan §5.6).
- **OffscreenCanvas in the worker** is supported everywhere, Safari since 16.4, but is a later option, not the baseline. Transferred bitmaps keep the virtualiser and the pool on the main thread, where layout lives, and C1b measured the copy at under 1 ms.

### 5. Threads and cross-origin isolation

- **v0.1 is single-threaded and does not ship COOP/COEP.** Nothing in v0.1 needs `SharedArrayBuffer`, and `require-corp` would break cross-origin `#url=` documents without CORP headers, OAuth popups later, and embeds.
- **A threaded variant** (the same single-threaded core instantiated in a pool of workers sharing memory, so independent pages parse and render in parallel) may be added when C3c's perf harness shows rendering is the bottleneck. It:
  - runs only when `crossOriginIsolated` is true;
  - serves the isolation headers on its own route or subdomain;
  - always keeps the single-threaded baseline.

  The core still spawns nothing (ADR 0001).

### 6. Build and test

- **Build:** Vite 8 (Rolldown), ESM-only (ADR 0016). `@laminapdf/*` WASM packages are excluded from Vite's pre-bundling.
- **Unit and component tests:** Vitest. `@laminapdf/viewer` runs in Vitest browser mode on Chromium, Firefox and WebKit through Playwright.
- **End-to-end:** Playwright on Chromium, Firefox and WebKit (C3c's perf harness, C4's privacy assertions, C7's accessibility pass with axe).

## Consequences

- **The framework choice is reversible.** Replacing React touches `@laminapdf/ui` only, and the desktop app (ADR 0008) reuses both packages.
- **One worker protocol for lamina** (L2a's), so the library's users and the viewer can't drift apart.
- **The memory budget is the hard part, and it is bounded by design:** range input instead of copies, tiles under `max_render_pixels`, and a capped canvas pool. The §4 web-app targets (600 MB for 1,000 pages) are C3c's to measure.
- **More main-thread code than OffscreenCanvas would need** (pool, scheduler, bitmap lifetime), traded for simpler layout and recycling.
- **No `SharedArrayBuffer`** means no parallel rendering in v0.1. A document renders one tile at a time, which C1b's 2–4 ms text pages make acceptable.
- **The cold download of about 2.5 MB gzip** is mitigated by early streaming compile and the service worker; it is not eliminated.
- **Debuggability:** worker crashes become a typed `worker-crashed` path with an automatic reopen, which C3a tests.

## Alternatives considered

- **Solid or Svelte 5 for the chrome.** Smaller and faster on benchmarks. Rejected: Solid 2.0 is still at release candidates, both have smaller contributor pools and fewer mature accessible primitives, and the hot path is framework-free anyway (web-frontend notes §1).
- **Lit / web components for the chrome.** Embeddable everywhere. Rejected for the chrome, whose accessible primitives are weaker. The viewer core is framework-free plain TS instead, which keeps embedding possible.
- **Comlink.** Ergonomic proxies. Rejected: render jobs need priorities, coalescing, transfer control and cancellation per request, which a proxy model hides (web-frontend notes §5).
- **OffscreenCanvas owned by the worker** (`transferControlToOffscreen`). Rejected as the baseline: it couples layout to the worker and complicates pooling. Kept as a later option in §4.
- **Threaded WASM by default.** Rejected: it requires COOP/COEP for the whole app, which breaks cross-origin documents and embeds. It is a measured, opt-in variant (§5).
- **One shared worker for all documents.** Fewer instances. Rejected: one hostile file would take down every open document, and the queues would interfere.
- **A rendering worker separate from the lamina worker.** Rejected: both engines would need the input, so it means two copies or cross-worker range plumbing. C1b showed one worker holds both without conflict.
- **Copying whole files into both modules.** Simplest. Rejected above 32 MiB: it doubles memory (95 + 237 MiB for the 100 MB scan) and fails in WebKit at 500 MB (C1b).
- **Server-side rendering of tiles.** Rejected: it conflicts with privacy first (ADR 0011) and costs compute.
