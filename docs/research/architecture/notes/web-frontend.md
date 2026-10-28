# Web frontend stack for Lamina (PDF reader/editor, web on Cloudflare + Tauri desktop, Rust/WASM engine in a Worker) — as of 2026-10-28

> Research scope note: about 18 search/fetch calls. Primary sources fetched in full: Tauri v2 HTTP-headers doc and MDN `showOpenFilePicker`. Everything else comes from search-result snippets: official docs where possible, and third-party pages otherwise, flagged as such. Items marked **[unverified]** come from background knowledge or from one secondary source. Check them before they decide anything.

## 1. Framework choice (React 19 + Compiler, Solid, Svelte 5, Vue, Preact, Lit) and what production PDF/document editors use

### Takeaway
Raw DOM-update speed is a weak tiebreaker here. A well-built viewer renders pages to canvas and mounts only a few dozen virtualized page shells, so "thousands of elements" is never in the DOM. The decision turns on ecosystem: a11y primitives, headless PDF toolkits, and the contributor pool. On that measure React 19 + React Compiler 1.0 is the safe default. Solid and Svelte 5 lead on raw speed and bundle size, but Solid 2.0 is not yet stable (rc builds only). EmbedPDF, the closest open-source analogue (PDFium→WASM, headless), ships React, Vue and Svelte adapters, which shows a framework-agnostic core with thin adapters works.

### Cited Findings
- React Compiler v1.0 was released stable on 2025-10-07 ([react.dev blog](https://de.react.dev/blog/2025/10/07/react-compiler-1)), around React Conf 2025 and React 19.2 ([InfoQ](https://www.infoq.com/news/2025/12/react-compiler-meta)). Next.js 16 lists compiler support as stable ([stacknotice](https://stacknotice.com/blog/react-compiler-usememo-dead), secondary).
- One secondary source mentions React 19.3.0 (2026-09-09) and Svelte 5.57.0 (2026-08-28) **[unverified, single secondary source]** ([pkgpulse](https://www.pkgpulse.com/guides/solidjs-vs-svelte-5-vs-react-reactivity-2026)).
- Solid 2.0: v2.0.0-beta.0 shipped 2026-03-03, and InfoQ covered the beta in May 2026. Its headline change is async-aware reactivity: computations may return Promises ([InfoQ](https://www.infoq.com/news/2026/05/solidjs-2-async/)). npm shows 2.0.0-rc.1…rc.5 ([socket.dev rc.5](https://socket.dev/npm/package/solid-js/overview/2.0.0-rc.5)). A September 2026 comparison still says stable is 1.9.x ([tech-insider](https://tech-insider.org/solidjs-vs-react-vs-svelte-2026/)). I found no evidence that 2.0 final has shipped.
- js-framework-benchmark: secondary sources agree on the ranking (Solid ≈ Svelte 5 ≈ near-vanilla; React with Compiler improved but behind) and disagree on the numbers. One gives Solid 1.02×, Svelte 5 1.07×, Vue 3.5 1.38×, React 19 1.41× of vanilla ([hacecuentas](https://hacecuentas.com/comparar/react-vs-vue-vs-svelte-vs-solid-frameworks-2026)). Another says React with Compiler still carries 12–18% overhead and Svelte 5 5–8% ([tech-insider](https://tech-insider.org/svelte-vs-react-2026/)). Vue Vapor mode is said to be competitive with Solid ([pkgpulse](https://www.pkgpulse.com/guides/what-happened-javascript-framework-wars-2026)). **All of these are unverified; the official krausest results table was not fetched.**
- EmbedPDF: an open-source JS PDF viewer running PDFium via WebAssembly, with a drop-in viewer and headless plugins (zoom, scroll, search, redaction). It has adapters for React, Vue 3 and Svelte, and the Svelte adapter is marked "in progress" ([EmbedPDF Svelte docs](https://www.embedpdf.com/docs/svelte/introduction), [Vue docs](https://www.embedpdf.com/docs/vue/introduction), [next.embedpdf.com](https://next.embedpdf.com/docs)). Sources disagree on the license, MIT vs Apache-2.0 ([alternativeto](https://alternativeto.net/software/embedpdf/about) vs [bestofjs](https://bestofjs.org/projects/embed-pdf)).
- pdf.js viewer is vanilla JS: a canvas per page plus HTML text, annotation and XFA layers. Annotations are parsed in a worker and rendered as HTML elements in a div over the canvas ([Mozilla Attack & Defense blog](https://blog.mozilla.org/attack-and-defense/?p=253)).
- tldraw is React with its own signals library (`@tldraw/state`, formerly "Signia"). The `track` HOC re-renders components when the signals they read change ([tldraw store docs](https://tldraw.dev/sdk-features/store), [Introducing Signia](https://tldraw.substack.com/p/introducing-signia)).
- Figma: browser clients talk to per-document server processes over WebSockets ([Figma blog](https://www.figma.com/blog/how-figmas-multiplayer-technology-works/)). It is widely reported that the editor canvas is C++ compiled to WASM and the UI chrome is React **[unverified this session]**.

### Inferences
- The hot path (page raster, text layer, overlays) should be framework-independent: imperative TS modules or custom elements driven by the worker. The framework then renders only chrome (toolbar, sidebars, dialogs, panels). This is how pdf.js, Figma, tldraw and EmbedPDF all separate engine from UI, and it makes the framework choice reversible.
- React 19 + Compiler: biggest hiring and contributor pool, React Aria/Radix/Base UI, and an EmbedPDF React adapter. Costs are a larger runtime (~45–60 KB gz react+react-dom **[unverified figure]**) and reconciliation overhead, which is mitigated by keeping page shells out of React state or using external stores with `useSyncExternalStore`.
- Solid/Svelte 5 give smaller bundles and fine-grained updates, but smaller pools. Solid 2.0 churn is a risk until it is final. Svelte 5's runes are stable.
- Lit/web components are attractive for the *viewer core* (framework-neutral, embeddable by third parties), but weak for app chrome a11y primitives.
- Tauri is framework-agnostic: any Vite-based framework works, and the Tauri docs list Vite config for React, Solid, Svelte, Vue and Qwik ([Tauri HTTP headers](https://v2.tauri.app/security/http-headers/)).

### Gaps
- Official js-framework-benchmark numbers (Chrome version, date) not retrieved.
- Nutrient Web SDK internals (framework, rendering) not researched. My background knowledge says it is framework-agnostic and WASM-based, which is unverified.
- Google Docs' 2021 move to canvas-based rendering: the search returned nothing. Background knowledge only (Workspace Updates blog, May 2021) **[unverified]**.
- Excalidraw is React + Canvas2D (background knowledge, not cited).

## 2. Virtualization, tiling, canvas pooling, OffscreenCanvas, ImageBitmap

### Takeaway
Use a custom page virtualizer with a precomputed prefix-sum of page heights at the current zoom (variable sizes are known from the PDF up front). Add zoom-aware scroll anchoring, a small pool of reusable canvases, and tile-based rendering at high zoom. Rendering can happen in the worker via OffscreenCanvas, now supported in all engines (Safari since 16.4), or as ImageBitmaps transferred zero-copy to the main thread.

### Cited Findings
- OffscreenCanvas is supported in Safari from 16.4 per the web.dev compat table ([web.dev OffscreenCanvas](https://web.dev/articles/offscreen-canvas)). `transferControlToOffscreen()` hands canvas control to a worker ([MDN](https://developer.mozilla.org/docs/Web/API/HTMLCanvasElement/transferControlToOffscreen)). After transfer, the original canvas cannot get a context, and transfer to a SharedWorker is unsupported (Firefox test suite) ([searchfox](https://searchfox.org/firefox-main/source/dom/canvas/test/test_offscreencanvas_neuter.html)).
- WebGL contexts can be created off the main thread through OffscreenCanvas ([Mozilla Hacks](https://hacks.mozilla.org/?p=29649)).
- pdf.js uses one canvas per page under a transparent-span text layer for selection and search ([Mozilla blog](https://blog.mozilla.org/attack-and-defense/?p=253)).

### Inferences
- Two viable render paths:
  - (a) The worker owns per-tile OffscreenCanvases. This couples layout to the worker and complicates pooling.
  - (b) The worker rasterizes to RGBA in WASM memory, wraps it as `ImageData` → `createImageBitmap`, and posts it with transfer. The main thread `drawImage`s onto pooled canvases.

  (b) is simpler for virtualization and recycling and is the more common pattern **[inference]**.
- Low-res thumbnails and "preview" rasters are shown first, then replaced by full-res tiles. Cap the device-pixel canvas area per page, because Safari/iOS enforce total canvas memory limits **[unverified specifics]**.
- Generic virtualizers like TanStack Virtual handle variable sizes, but zoom anchoring (keeping the point under the cursor fixed) needs custom logic either way.

### Gaps
- Safari WebGL-in-worker limits, and canvas memory caps per browser, not found.
- Production details of pdf.js `PDFViewer` buffer and pooling (default page cache size) not fetched.

## 3. Overlays: canvas vs SVG vs DOM for text layer, selection, annotations, forms; hit-testing; HiDPI

### Takeaway
Follow the industry layering: raster canvas → transparent DOM text layer (native selection, find, copy, screen readers) → annotation layer (SVG or DOM for editable or interactive annotations) → DOM form widgets. Hit-testing for shapes uses geometry in document coordinates from the model, not DOM events on thousands of nodes.

### Cited Findings
- pdf.js: canvas plus a transparent-span text layer; annotation (AcroForm) and XFA layers are HTML on top ([Mozilla blog](https://blog.mozilla.org/attack-and-defense/?p=253)).
- The struct tree can reference nodes in both the text and annotation layers, which makes it a "third source of truth." Proposed fix: a separate struct-tree layer linked with `aria-owns` ([pdf.js #6269](https://github.com/mozilla/pdf.js/issues/6269), [GSoC 2022 write-up](https://blog.learningequality.org/google-summer-of-code-2022-improve-accessibility-of-pdf-renderer-alex-velez-b87abf46a355)).

### Inferences
- The DOM text layer is non-negotiable for native selection, accessibility and browser find. Canvas-drawn selection highlights are fine on top.
- HiDPI: size canvas backing stores at `cssSize × devicePixelRatio × zoom`, clamped, and re-render on DPR change (`matchMedia('(resolution: …)')`) **[standard practice, not cited]**.

### Gaps
- No primary source fetched on EmbedPDF's or Nutrient's overlay implementations.

## 4. State management, document model, undo/redo

### Takeaway
The best-documented modern pattern (tldraw) is a normalized reactive record store on signals. All store changes are recorded automatically as diffs, grouped by "marks" for undo. Excalidraw likewise moved to delta-based history. For Lamina the authoritative document lives in the Rust/WASM worker, so the UI store should mirror view state plus an annotation/edit projection. Undo/redo is better owned by the engine (command log or patches), with the UI dispatching commands.

### Cited Findings
- tldraw store: a "reactive database" of shapes, pages, bindings and assets. Reads create dependencies, `unsafeGetWithoutCapture` reads without one, indexes are reactive computeds, and records are JSON-serializable for snapshots ([tldraw store](https://tldraw.dev/sdk-features/store), [@tldraw/state README](https://cdn.jsdelivr.net/npm/@tldraw/state@4.5.9/README.md)).
- tldraw replaced command-based undo with automatic recording of all store changes, separated into batches by marks. Creating a mark flushes pending changes and clears redo ([tldraw PR #3364](https://github.com/tldraw/tldraw/pull/3364), [tldraw history docs](https://tldraw.dev/sdk-features/history)).
- Excalidraw: `History` in `packages/excalidraw/history.ts` keeps undo and redo stacks of delta objects ([pyshine source tour](https://pyshine.com/Excalidraw-Virtual-Whiteboard-Source-Tour/), secondary). A later commit refactors this into a `TransactionLedger` ([mirror commit](https://gitea.maison43.duckdns.org/gilles/excalidraw/src/commit/bfd4af23673df9904a37b9f194c1823cd6e831b7), unofficial mirror; current state unconfirmed).
- Figma: document as tree of `ObjectID → {property → value}`; last-writer-wins per property; server validates tree structure ([Figma blog](https://www.figma.com/blog/how-figmas-multiplayer-technology-works/), [Evan Wallace](https://madebyevan.com/figma/multiplayer-editing-in-figma/)).
- Rust `undoredo` crate offers delta or snapshot recording ([docs.rs](https://docs.rs/crate/undoredo/0.13.5/source/README.md)).

### Inferences
- Recommended split:
  - (1) The engine (Rust) owns the document and an operation log. It returns inverse ops and patches.
  - (2) The UI keeps a signal or external store (Zustand/Jotai/tldraw-style signals/Solid stores) for view state: zoom, current page, selection, tool, panels. It also keeps a cache of per-page annotation records keyed by id.
  - (3) Patches from the worker are applied as fine-grained updates.

  Immer patches work well if the JS side owns the annotation model. A per-property LWW record model à la Figma or tldraw keeps future collaboration (CRDT) open.
- XState fits tool modes and interaction state machines (select / draw / ink / form-fill), not the document store.

### Gaps
- No primary-source detail on Nutrient's or EmbedPDF's state model.

## 5. Worker architecture: Comlink vs custom RPC, SharedArrayBuffer, COOP/COEP on Cloudflare and Tauri, WASM threads

### Takeaway
A plain `postMessage` RPC with transferables (Comlink or a thin typed custom layer) needs no special headers and works everywhere. SharedArrayBuffer/WASM threads need cross-origin isolation: COOP `same-origin` + COEP `require-corp`/`credentialless`. Both are configurable on Cloudflare static assets (`_headers`) and in Tauri ≥2.1 (`app.security.headers`). Whether all Tauri webviews (WKWebView, WebKitGTK, WebView2) actually report `crossOriginIsolated === true` is **unverified**. Design so threads are an optional enhancement.

### Cited Findings
- SharedArrayBuffer requires cross-origin isolation, which you check with `self.crossOriginIsolated` ([web.dev guide](https://web.dev/cross-origin-isolation-guide/)). Under `require-corp`, cross-origin resources without CORP/CORS are blocked ([MDN COEP](https://developer.mozilla.org/docs/Web/HTTP/Reference/Headers/Cross-Origin-Embedder-Policy)). A Permissions-Policy that blocks `cross-origin-isolated` forces it to false ([MDN](https://developer.mozilla.org/docs/Web/HTTP/Headers/Permissions-Policy/cross-origin-isolated)).
- Cloudflare Workers static assets / Pages: a `_headers` file in the assets dir sets headers. It is **not** applied to responses generated by Worker code, which must set headers themselves ([CF Workers static-assets headers](https://developers.cloudflare.com/workers/static-assets/headers/), [CF Pages headers](https://developers.cloudflare.com/pages/configuration/headers/index.md)). BentoPDF (an open-source PDF tool) documents exactly this COOP/COEP `_headers` setup on Cloudflare for SharedArrayBuffer ([BentoPDF docs](https://www.bentopdf.com/docs/self-hosting/cloudflare)).
- Tauri v2: since **2.1.0**, `app.security.headers` in `tauri.conf.json` can set COEP, COOP, CORP, Permissions-Policy, Service-Worker-Allowed and others. The docs say COOP `same-origin` + COEP `require-corp` "allow for the use of SharedArrayBuffer." The headers apply only to the production build, so the Vite dev server needs `server.headers` too. They don't apply to IPC or error responses. The page lists no platform caveats ([Tauri HTTP Headers](https://v2.tauri.app/security/http-headers/), fetched).
- A Google comment, quoted on a third-party app's known-issues page, says Android WebView doesn't support cross-origin isolation at all ([Hermit known issues](https://hermit.chimbori.com/help/known-issues)). This matters only for a future Tauri Android target.
- Vendor note: COEP `credentialless` isn't reliable when Safari/iOS support is required ([Shen.ai troubleshooting](https://developer.shen.ai/getting-started/web-troubleshooting)).
- Document-Isolation-Policy: a newer per-document isolation header ([http.dev](https://http.dev/document-isolation-policy), [TAG review](https://github.com/w3ctag/design-reviews/issues/995)). Support status is unconfirmed (believed Chromium-only **[unverified]**).

### Inferences
- COEP `require-corp` will break cross-origin embeds: third-party fonts, images, OAuth popups (COOP `same-origin` severs `window.opener`), and analytics. Self-host all assets. Consider serving the isolated viewer on its own route or subdomain.
- Build the WASM engine in two flavors (single-threaded baseline; threaded `+atomics` when `crossOriginIsolated`) and pick at runtime.
- Comlink's proxy model is ergonomic, but a hand-written typed RPC (request id, method enum, transfer lists, cancellation via AbortSignal, priority queues for render jobs) gives better control over render-job cancellation when the user scrolls fast **[inference]**.

### Gaps
- No confirmation that WKWebView (macOS) or WebKitGTK (Linux) honour COOP/COEP on Tauri's custom protocol (`tauri://` / `http://tauri.localhost`). This needs an empirical spike that logs `crossOriginIsolated` on each OS.
- Comlink's current version and maintenance status not checked.

## 6. Offline/PWA, File System Access API, OPFS, large local files, drag and drop

### Takeaway
OPFS (including `createSyncAccessHandle` in workers) is available in all three engines and is the right cache and scratch store for the WASM engine. `showOpenFilePicker`/`showSaveFilePicker` (the File System Access API) is still **not Baseline**: Chromium only. Safari and Firefox need `<input type=file>`, drag-and-drop `File` objects, and download-based save. In Tauri, use native dialogs and fs plugins instead.

### Cited Findings
- MDN: `showOpenFilePicker` "is not Baseline because it does not work in some of the most widely-used browsers" and is marked experimental; it requires a secure context ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/Window/showOpenFilePicker), fetched). The compat table did not render in the fetch. Chromium-only support is background knowledge **[unverified for Oct 2026]**.
- OPFS baseline: Chrome/Edge 86, Firefox 111, Safari 15.2. `createSyncAccessHandle` is worker-only, supported since Firefox 111 and Safari 15.2 ([web.dev OPFS](https://web.dev/opfs), [MDN via w3cub](https://docs.w3cub.com/dom/file_system_api/origin_private_file_system)).
- Safari 26 added `FileSystemFileHandle.createWritable` (File System WritableStream). A WebKit bug thread asking for the picker methods remains a request ([WebKit bug 231706](https://bugs.webkit.org/show_bug.cgi?id=231706)).
- Tauri can set the `Service-Worker-Allowed` header ([Tauri HTTP Headers](https://v2.tauri.app/security/http-headers/)).

### Inferences
- For large files: `File` is a `Blob`, so stream with `file.slice()`/`stream()` into the worker. Or copy once into OPFS and let the engine do random access via a SyncAccessHandle. Avoid reading the whole file into a JS ArrayBuffer and then copying into WASM memory twice.
- PWA: the service worker precaches the app shell and WASM (Vite PWA plugin/Workbox). Under COEP, the SW must not strip headers.
- Persist recent files as `FileSystemFileHandle` in IndexedDB on Chromium only.

### Gaps
- Firefox's position on `showOpenFilePicker` (historically "harmful") and any 2026 change not verified.
- PWA install support on Safari macOS/iOS in 2026 not researched.

## 7. Accessibility, i18n, theming

### Takeaway
Accessibility for a canvas viewer comes almost entirely from DOM layers: the text layer for reading order, a struct-tree layer mapping tagged-PDF semantics (headings, tables, lists, alt text, lang) via ARIA/`aria-owns`, DOM form widgets, and keyboard-navigable annotations. The Rust engine must expose the StructTreeRoot and MarkInfo to the UI.

### Cited Findings
- Tagged-PDF semantics live in StructTreeRoot: element types, alt text, language ([pdf.js #6269](https://github.com/mozilla/pdf.js/issues/6269)). pdf.js renders a struct-tree layer linked to the text layer via `aria-owns` ([GSoC 2022](https://blog.learningequality.org/google-summer-of-code-2022-improve-accessibility-of-pdf-renderer-alex-velez-b87abf46a355)). Nutrient says pdf.js `PDFViewer` renders the struct tree automatically and `getMarkInfo()` reveals whether a document is tagged ([Nutrient blog](https://www.nutrient.io/blog/pdfjs-accessibility-structtree-printing/), vendor source).
- Chromium's built-in PDF viewer maintains a dedicated PDF accessibility tree ([Chromium source](https://chromium.googlesource.com/chromium/src/+/282cd8a/components/pdf/renderer/pdf_accessibility_tree.h)).

### Inferences
- Engine API needs: `getStructTree(page)`, `getMarkInfo()`, text runs with char boxes, and annotation/form metadata with roles.
- Only the virtualized (mounted) pages have DOM, so screen-reader "read whole document" is limited. Mitigations: an outline/landmark navigation, `aria-setsize`/`aria-posinset` on page regions, and a keyboard page jump.
- Chrome a11y: React Aria (Adobe; i18n, RTL, date/number formatting built in), Radix/Base UI (React), Ark UI (React/Solid/Vue/Svelte via Zag state machines), Kobalte (Solid), Bits UI (Svelte). These names are background knowledge; current versions **[unverified]**.
- Theming: CSS custom properties. Ensure PDF page rendering is not inverted by dark mode except via an explicit "night reading" filter.

### Gaps
- Current maturity and versions of Ark UI, Kobalte and Bits UI not fetched.
- WCAG/PDF-UA expectations for viewers not researched.

## 8. Build tooling and testing

### Takeaway
Vite 8 (stable 2026-03-12) uses Rolldown as its single bundler, replacing esbuild + Rollup. Rolldown hit 1.0 in May 2026. Use Vite + `vite-plugin-wasm` or native `?init`/URL imports for WASM, Vitest (browser mode for worker/canvas code), and Playwright with screenshot comparisons for visual regression of rendered pages.

### Cited Findings
- Vite 8.0 stable on 2026-03-12: Rolldown replaces esbuild + Rollup. Integrated devtools and Wasm SSR also landed ([Vite blog](https://vite.dev/blog/announcing-vite8), [AlternativeTo news](https://alternativeto.net/news/2026/3/vite-8-debuts-with-unified-and-faster-rolldown-bundler-integrated-devtools-and-wasm-ssr/)).
- Rolldown 1.0 in May 2026, with a semver-locked API. The specific date given, 2026-05-07, comes from secondary sources ([VoidZero](https://voidzero.dev/posts/announcing-rolldown-1-0)).
- A Vite 8.1 beta on 2026-06-15 is reported by one non-English source only **[unverified]** ([tech-insider.org](https://tech-insider.org/nl/?p=948)).
- Secondary claim: Rolldown speeds builds (~"13×") but not the dev server ([var.gg](https://var.gg/en/blog/vite-8-rolldown)).
- Tauri dev mode needs COOP/COEP on Vite's `server.headers` ([Tauri HTTP Headers](https://v2.tauri.app/security/http-headers/)).

### Inferences
- Visual regression: render fixture PDFs at fixed DPR in Playwright (Chromium, WebKit, Firefox) and diff against goldens. Also run engine-level pixel tests in Rust (cheaper, deterministic), and keep UI tests about layout and interaction.
- WebKit in Playwright approximates Safari/WKWebView, but Tauri-specific behaviour (custom protocol, COOP/COEP) needs tests in the real app (WebDriver via `tauri-driver`) **[unverified current status]**.

### Gaps
- Current Vitest and Playwright versions not checked.
- Vite 8's handling of WASM + module workers under Rolldown (e.g. `new Worker(new URL(...), {type:'module'})`) not verified.
