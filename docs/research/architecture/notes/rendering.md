# PDF rendering engine choices for Lamina (web + desktop), as of October 2026

Research date: 2026-10-28. Version numbers for npm/crates.io were checked live with `npm view` and the crates.io API on that date. Those checks are cited as registry URLs.

## 1. Candidate engines: version, maintenance, licence, fidelity, size, features

### Takeaway
pdf.js (Apache-2.0) is the most actively released permissive web engine, at 6.4.299 on 3 Oct 2026. PDFium-in-WASM (Apache-2.0 engine with MIT wrappers, about 4.6 MB raw WASM) is the leading high-fidelity option that covers forms and annotations, and native PDFium can serve the desktop app through pdfium-render. hayro (MIT/Apache-2.0, 0.8.0 on 4 Oct 2026) is a credible pure-Rust renderer, but it is still renderer-only and experimental. MuPDF.js (1.28.1) is AGPL-3.0-or-later and must be excluded from the core unless a commercial licence is bought.

### Cited Findings

**pdf.js (Mozilla)**
- Release cadence is roughly monthly. v6.4.299 shipped on 03 Oct (annotation rendering, font conversion, forms, patterns, text selection). v6.3.289 on 29 Aug (accessibility, alt text, annotation editing; `getFieldObjects`/`getJSActions` now return Maps). v6.2.108 on 28 Jul (digital signature/certificate verification, sound annotations). v6.1.200 on 27 Jun (merging files, `/AuthEvent` on-demand decryption). v6.0.227 on 30 May, a major release that raised minimum browsers, removed polyfills, made ImageDecoder the default in Chrome, and added a `WasmImage` class for the JBIG2/JPEG 2000 decoders. v5.7.284 on 27 Apr rewrote the PostScript function parser with "a small Wasm compiler" and uses OffscreenCanvas unconditionally for thumbnails. — [pdf.js releases](https://github.com/mozilla/pdf.js/releases)
- `pdfjs-dist` on npm is at 6.4.299, licence Apache-2.0, unpacked size about 34.9 MB (all builds, legacy builds, maps and viewer included; the shipped worker plus display bundle is far smaller), last modified 2026-10-03. — [npm pdfjs-dist](https://www.npmjs.com/package/pdfjs-dist)
- Wikipedia's listing of 6.3.289 (29 Aug 2026) as the stable release is stale compared with GitHub. — [Wikipedia](https://en.wikipedia.org/wiki/PDF.js)
- Fidelity: a vendor-adjacent comparison calls PDFium more precise on complex files and pdf.js lighter but weaker on complex fonts. These are not measured results. — [Syncfusion blog](https://www.syncfusion.com/blogs/post/pdf-rendering-engines-comparison)

**PDFium → WASM (EmbedPDF, @hyzyla/pdfium, paulocoutinhox/pdfium-lib)**
- `@embedpdf/pdfium` is at 2.15.1, MIT, last modified 2026-09-16, unpacked about 7.55 MB. — [npm @embedpdf/pdfium](https://www.npmjs.com/package/@embedpdf/pdfium)
- The `pdfium.wasm` in `@embedpdf/pdfium@2.14.3` is 4,632,999 bytes (about 4.6 MB raw, measured by Content-Length on jsDelivr). — [jsDelivr @embedpdf/pdfium](https://www.jsdelivr.com/package/npm/@embedpdf/pdfium)
- The EmbedPDF README says the package is MIT and bundles PDFium under Apache-2.0. Listed features: rendering, text extraction and search, form filling, annotations, digital signature verification, and PDF modification/creation. — [@embedpdf/pdfium README](https://cdn.jsdelivr.net/npm/@embedpdf/pdfium@2.14.3/README.md)
- EmbedPDF engine architecture: `PdfiumNative` (a thin executor over the WASM module) plus a `PdfEngine` orchestrator. The docs recommend running it in a Web Worker (`WebWorkerEngine` from `@embedpdf/engines/worker`). It works in browsers, Node, and serverless. Docs last updated 3 Aug 2026. — [EmbedPDF engines docs](https://www.embedpdf.com/docs/engines/introduction)
- `@hyzyla/pdfium` is at 3.0.0, MIT, last modified 2026-10-10, unpacked about 14.1 MB. It ships `pdfium.wasm` and can load it either as base64-embedded (a bigger bundle) or as a separate asset. — [npm @hyzyla/pdfium](https://www.npmjs.com/package/@hyzyla/pdfium); [pdfium.js.org docs](https://pdfium.js.org/docs/init-library)
- An HN commenter reports pdfium.wasm at about 2.7 MB gzipped in a viewer. It is unconfirmed which build that was. — [HN thread](https://hn.nuxt.dev/item/44126177)

**pdfium-render (Rust bindings, native + WASM)**
- The latest docs.rs version is 0.9.4, which fixes double-frees in font and page-object handling and WASM memory-growth issues. 0.9.3 bumped `pdfium_latest` to `pdfium_7881`. 0.9.1 added a `console_log` feature for WASM. — [docs.rs pdfium-render](https://docs.rs/crate/pdfium-render); [GitHub ajrcarey/pdfium-render](https://github.com/ajrcarey/pdfium-render)
- On WASM, the external PDFium build must be packaged as a separate WASM module and bound at runtime with `initialize_pdfium_render()`. The prebuilt PDFium WASM comes from paulocoutinhox/pdfium-lib. — [pdfium-render README](https://github.com/ajrcarey/pdfium-render); [pdfium-lib](https://github.com/paulocoutinhox/pdfium-lib)

**hayro (pure Rust, LaurenzV / Typst ecosystem)**
- crates.io reports `hayro` max_version 0.8.0, updated 2026-10-04. — [crates.io hayro](https://crates.io/crates/hayro)
- Licence is MIT and Apache-2.0 (dual), with MSRV 1.92. It is CPU-only rendering to bitmaps (PNG) or SVG (`hayro-svg`). The crates are split into `hayro-syntax` (parsing), `hayro-interpret`, `hayro-font` (Type1/CFF), and `hayro-svg`. — [hayro GitHub](https://github.com/LaurenzV/hayro); [docs.rs hayro](https://docs.rs/hayro)
- The README claims you can "basically expect almost any PDF that renders in Chrome (PDFium) or Firefox (pdf.js) to render correctly". The visual regression suite has 1,600+ PDFs, largely drawn from the pdf.js and PDFBox corpora. No pass/fail table is published. Knockout groups, CMYK blending, and 3D content are noted as unsupported, as they are in most non-Acrobat viewers. Document manipulation is out of scope. The demo has no text selection, search, annotations, or forms. — [hayro GitHub](https://github.com/LaurenzV/hayro); [docs.rs hayro 0.7.1](https://docs.rs/crate/hayro/0.7.1)
- Performance: it "can already render many PDFs faster than other PDF viewers", but glyph caching is missing. A browser demo exists at laurenzv.github.io/hayro and is described as "vibe-coded, not an actual viewer". — [hayro GitHub](https://github.com/LaurenzV/hayro)
- Older notes said encrypted PDFs and blend modes/knockout were unsupported. Current README wording suggests that blend modes have since landed, but this is unverified. — [rust-digger hayro](https://crates.io/crates/hayro)
  - *Update (2026-10-28, [`hayro-parity` spike](../../../../spikes/hayro-parity/README.md)):* hayro 0.8.0 opens encrypted files that have an empty user password; its remaining encryption failures (password-protected files, PDFBox's AES samples) are shared with PDFium. Knockout groups and non-isolated groups with soft masks still differ from PDFium (5 failing pages on the pdf.js corpus).
- Typst uses it for PDF-as-image, and a third-party crate, `zenpdf`, builds on it. — [docs.rs zenpdf](https://docs.rs/zenpdf); [users.rust-lang.org](https://users.rust-lang.org/t/converting-pdf-to-tif/137663)

**MuPDF.js (Artifex)**
- npm `mupdf` is at 1.28.1, licence AGPL-3.0-or-later, modified 2026-09-06, unpacked about 14.3 MB. — [npm mupdf](https://npmjs.com/package/mupdf)
- It is dual-licensed AGPL or commercial. Artifex states that any code linking MuPDF must also be AGPL. The AGPL network clause reaches SaaS use. The commercial licence removes those obligations, and pricing is by inquiry only. — [MuPDF licence docs](https://mupdf.readthedocs.io/en/1.27.0/license.html); [MuPDF Explored A.1](https://ghostscript.com/~robin/mupdf_explored/mupdf_exploredse159.html)
- Fidelity: the 2017 Alfresco study (MuPDF 1.10a against PDFium of that era) ranked MuPDF first and PDFium second, yet Alfresco chose PDFium anyway. — [Alfresco/Hyland blog](https://hub.alfresco.com/t5/alfresco-content-services-blog/pdf-rendering-engine-performance-and-fidelity-comparison/ba-p/287618)

**Commercial WASM cores (context only)**
- Nutrient (PSPDFKit) Web standalone ships a WASM engine larger than 8 MB that renders client-side. The alternative is server-backed rendering through Document Engine, which fetches only the needed pages and speeds up large PDFs, but can overload a single server. — [Nutrient performance guide](https://nutrient.io/guides/web/best-practices/performance); [Nutrient operational modes](https://nutrient.io/guides/web/about/operational-modes)
- Apryse WebViewer is a C++ core compiled to WASM that runs in a worker, with modes for a WASM worker, a threaded WASM worker, and an asm.js fallback. The "full API" WASM is about 2–3× larger than the default render-only module. — [Apryse full API overview](https://docs.apryse.com/documentation/web/guides/full-api-overview/); [Apryse Core API 8.3](https://docs.apryse.com/api/web/8.3/Core.html)

### Inferences
- Licence posture: pdf.js (Apache-2.0), PDFium (Apache-2.0 / BSD-style), EmbedPDF and hyzyla wrappers (MIT), pdfium-render and hayro (MIT/Apache) are all compatible with an MIT/Apache Lamina core. MuPDF is not. Apple PDFKit is a proprietary system framework that is only callable on Apple platforms, so it cannot be redistributed or used cross-platform.
- Bundle size order (raw, approximate): hayro WASM is likely smallest (unmeasured), then pdf.js worker + display (~1–2 MB unminified JS, unmeasured here), then PDFium WASM (~4.6 MB raw / ~2.7 MB gzip), then the commercial cores (>8 MB). Edge caching on Cloudflare mitigates this, but first load on mobile is still affected.
- For an editor, PDFium's built-in form-fill (FPDF_FORMHANDLE), annotation APIs, and save support are a major advantage over hayro, which is renderer-only by design.

### Gaps
- No published, current neutral fidelity benchmark comparing pdf.js, PDFium, hayro, and MuPDF on the same corpus. A 2026 dev.to post notes the same gap ([dev.to feed](https://dev.to/feed/9haroon)). The PDF Association's 2021 survey ([pdfa.org](https://pdfa.org/wp-content/uploads/2021/06/Survey-of-OpenSource-Solutions.pdf)) could not be parsed into rankings.
- Not found or verified: hayro WASM bundle size, memory profile, ICC/colour-management support, encryption support status in 0.8.0.
- pdfium-render's latest version date and exact licence string were not confirmed in this session. It is believed to be MIT OR Apache-2.0, so check crates.io.
  - *Update (2026-10-28):* confirmed on crates.io: `pdfium-render` 0.9.4 (2026-09-06) is `MIT OR Apache-2.0`. See the [`hayro-parity` spike](../../../../spikes/hayro-parity/README.md#licences), which also measures hayro against PDFium on the pdf.js and PDFBox corpora (the "no published pass/fail table" gap above).
- Apple PDFKit and Core Graphics details (fidelity, API changes in macOS 26/27) were not researched with sources.
- No measured memory-use numbers were found for any engine.

## 2. Viewer architectures (pdf.js, Chrome/PDFium, Nutrient/Apryse, desktop viewers)

### Takeaway
Every serious viewer separates parsing and rasterization from the UI thread or process: a pdf.js worker, Chrome's sandboxed OOPIF plugin process, or the Nutrient/Apryse WASM worker. They then composite text, annotation, and form layers as DOM or native overlays above a raster canvas. Lamina should copy this split: a Rust engine in a worker or process, with TS overlays.

### Cited Findings
- pdf.js: parsing runs in `pdf.worker.js`, which talks to the display layer over a message transport. The main thread receives operator lists and paints them to canvas. Text and annotation layers (`text_layer.js`, `annotation_layer.js`) are DOM overlays aligned to the canvas. — [Nutrient blog on pdf.js workers](https://www.nutrient.io/blog/pdfjs-advanced-loading-streaming-workers/)
- Mozilla Bug 1763330 shipped OffscreenCanvas for pdf.js so the worker can produce ImageBitmaps. In 5.7.284, OffscreenCanvas became unconditional for thumbnails. — [Tor Browser mirror of commit](https://gitlab.torproject.org/clairehurst/tor-browser/-/commit/81f305e03319cdcfe740dbc2306d93aad4772250); [pdf.js releases](https://github.com/mozilla/pdf.js/releases)
- Text in workers is hard: drawing text off the main thread needs font parsing, shaping, and bidi to work there. This is one reason pdf.js keeps the text layer as DOM. — [Bugzilla 801176](https://bugzilla.mozilla.org/show_bug.cgi?id=801176)
- Known pitfall: pdf.js's font loader references `document`, which breaks naive worker-side canvas rendering. — [lightrun summary of pdf.js issue](https://lightrun.com/answers/mozilla-pdf-js-run-pdfjs-canvas-rendering-in-a-web-worker)
- Chrome's PDF viewer is moving from GuestView to an out-of-process-iframe (OOPIF) design, gated by the `PdfViewerOutOfProcessIframeEnabled` policy. `//pdf` holds the PDFium-based plugin. The viewer has an extra sandbox layer against malicious PDFs. — [Chromium OOPIF policy commit](https://chromium.googlesource.com/chromium/src/+/b46ec9b532ab689cbf907474097cdcc156ef186e); [Chromium pdf/README](https://chromium.googlesource.com/chromium/src/+show/f303bc03a281b55dbd066185f79daa61dd8ba8d1/pdf/README.md); [Samsung policy page](https://developer.samsung.com/browser/policy/pdf-viewer-out-of-process-iframe-enabled.html)
- Nutrient's first Web SDK was server-rendered and the later one is WASM standalone. Both modes ship web workers, fonts, and assets that must be self-hosted. — [Nutrient deployment options](https://www.nutrient.io/sdk/web/getting-started/deployment-options/); [Nutrient self-host assets](https://www.nutrient.io/guides/web/self-host-assets/)
- Apryse "Core" handles document processing, canvas rendering, and annotation management. The same C++ core builds natively for Windows, macOS, Linux, iOS, and Android, as well as WASM. — [Apryse Core guide](https://docs.apryse.com/web/guides/core); [Apryse overview](https://docs.apryse.com/web/guides/overview)
- Sioyek: Qt (Qt 6.7/6.8 on the dev branch), GPL-3.0, with MuPDF vendored as a git submodule. — [Sioyek GitHub](https://github.com/ahrm/sioyek)

### Inferences
- One engine core in two shells, as Apryse does (native C++ alongside WASM), maps directly onto Lamina's Rust native + WASM plan. Either PDFium (C++, via pdfium-render) or hayro (Rust) can follow it.
- Chrome's sandboxed-process approach is the security model to copy on desktop: run the renderer in a separate process or worker so a malformed PDF crashes the worker, not the app.
- Server-backed rendering (Nutrient Document Engine) is an option on Cloudflare: Workers/Containers could serve page tiles for huge documents. It conflicts with local-first and privacy goals, though, and adds compute cost.

### Gaps
- No primary Chromium design doc was found on PDF tiling and paint scheduling. Firefox large-document behaviour (page virtualization, canvas memory caps) was not sourced in this session.
- Zathura (plugin backends: zathura-pdf-poppler / zathura-pdf-mupdf), Okular (Poppler), and Skim (PDFKit) are known to use these backends from background knowledge. That was not verified with sources this session.
- PDF.js Express (Apryse's pdf.js-based product) was not researched.

## 3. Text layer, selection, search, accessibility, hit-testing

### Takeaway
The proven pattern is an invisible, positioned DOM text layer over the canvas, which provides selection, find, screen readers, and copy, plus DOM or SVG annotation and form layers for hit-testing and editing. This holds regardless of engine. PDFium and hayro can supply glyph boxes, but Lamina must build these layers itself unless it uses pdf.js's viewer components.

### Cited Findings
- pdf.js text and annotation layers are DOM overlays. — [Nutrient blog](https://www.nutrient.io/blog/pdfjs-advanced-loading-streaming-workers/)
- pdf.js keeps investing in accessibility in 2026: alt text exposed without prohibited ARIA names (6.3.289), and Firefox font-size handling in the text layer (6.4.299). — [pdf.js releases](https://github.com/mozilla/pdf.js/releases)
- The EmbedPDF/PDFium engine exposes text extraction and search, annotation CRUD (get all, get page, create, update, remove), and rendering of a page rectangle or page annotations separately. — [EmbedPDF engines docs](https://www.embedpdf.com/docs/engines/introduction); [@embedpdf/pdfium README](https://cdn.jsdelivr.net/npm/@embedpdf/pdfium@2.14.3/README.md)
- The hayro demo explicitly lacks text selection, search, annotations, and forms. — [hayro GitHub](https://github.com/LaurenzV/hayro)

### Inferences
- Render annotations in a separate pass from page content (PDFium supports rendering with or without annotations via the `FPDF_ANNOT` flag), so editable annotations can be live DOM or SVG while flattened content stays raster.
- Hit-testing should run on the engine's object and annotation geometry, kept in the Rust core and exposed through TS bindings, not on pixels.

### Gaps
- No sourced data on screen-reader quality of canvas viewers versus PDF/UA tagged-structure exposure. pdf.js struct-tree support was not checked this session.

## 4. Rendering at scale and desktop bitmap transfer

### Takeaway
Use virtualized pages, tiles at zoom, a low-res pass followed by a sharp pass, worker or thread pools, and an LRU bitmap cache. On desktop, the main cost is moving bitmaps from Rust into the webview. Prefer raw-bytes IPC or a custom protocol, or render inside the webview with the WASM build.

### Cited Findings
- Server-backed rendering fetches only needed pages, which helps large PDFs, but too many render requests to one server cause slowdowns. — [Nutrient performance guide](https://nutrient.io/guides/web/best-practices/performance)
- The Apryse threaded WASM worker mode shows that WASM threads (SharedArrayBuffer, which needs COOP/COEP headers) are used in production for render parallelism. — [Apryse Core API 8.3](https://docs.apryse.com/api/web/8.3/Core.html); [Apryse blog](https://apryse.com/blog/embed-secure-document-viewer)
- Tauri IPC is either raw bytes or JSON. JSON encodes bytes as one number per byte, and per-crossing overhead dominates, so batch large messages. `invoke` accepts and returns `Uint8Array`. Custom URI schemes can serve binary, and wry's streaming example uses ranged responses of up to 1000 KiB. — [connectrpc-tauri notes](https://libraries.io/cargo/connectrpc-tauri); [Tauri discussion #5690](https://github.com/tauri-apps/tauri/discussions/5690); [wry streaming example](https://docs.rs/crate/wry/0.56.0/source/examples/streaming.rs)
- Another webview host reports that postMessage with ArrayBuffer is "terribly slow". — [Adobe CC dev forum](https://forums.creativeclouddeveloper.com/t/webview-postmessage-with-arraybuffer-terribly-slow/7055.md)
- hayro is missing glyph caching, which is a known performance gap. — [hayro GitHub](https://github.com/LaurenzV/hayro)

### Inferences
- A 2× DPI letter page is about 1700×2200×4 B ≈ 15 MB of RGBA per frame. Pushing full-page bitmaps per zoom step through IPC is costly. Send 256–512 px tiles, compressed (PNG/WebP or QOI) or raw over a custom protocol, or keep rendering in the webview with the same Rust→WASM engine. The second option removes IPC entirely and makes desktop and web identical. A native window or surface layer under the webview is the high-performance alternative, at the cost of complexity.
- For thousands of pages: keep only visible ± N pages as canvases, use placeholder sizes from page boxes, and render thumbnails at low DPI in a separate worker queue with lower priority.

### Gaps
- No published Tauri, Electron, or WebKit benchmark for MB-scale image transfer was found. Measure it.
- No measured pdf.js or PDFium-WASM memory figures for 1,000+ page documents were found.

## 5. Recommendation options and risks

### Takeaway
The lowest-risk path is a hybrid. Ship web v1 on PDFium-WASM (via EmbedPDF's MIT wrapper or Lamina's own pdfium-render build), or on pdf.js if bundle size or editing needs are lower. Use native PDFium on desktop through pdfium-render behind a Lamina `Renderer` trait, and keep hayro as a pluggable pure-Rust backend to adopt as it matures. Avoid MuPDF.

### Cited Findings
- PDFium offers forms, annotations, signatures, and modification. — [@embedpdf/pdfium README](https://cdn.jsdelivr.net/npm/@embedpdf/pdfium@2.14.3/README.md)
- pdf.js now includes annotation editing, signature verification, and merging. — [pdf.js releases](https://github.com/mozilla/pdf.js/releases)
- hayro is renderer-only, with editing out of scope. — [hayro GitHub](https://github.com/LaurenzV/hayro)
- MuPDF requires AGPL compliance or a commercial licence. — [MuPDF licence](https://mupdf.readthedocs.io/en/1.27.0/license.html)

### Inferences
Options and trade-offs:
- **A. pdf.js now, own Rust or hayro later.** Smallest risk to ship, with best-in-class text, accessibility layers, and release cadence. However, it is a JS engine, so it cannot be shared with the Rust core. Two document models (pdf.js for display, Rust for editing) could diverge, and fidelity on complex fonts lags PDFium.
- **B. PDFium everywhere (WASM web + native desktop via pdfium-render).** High fidelity, the same engine on both platforms, and forms, annotations, and saving built in, under Apache-2.0. Downsides: a ~4.6 MB WASM, a C++ memory-safety surface (sandbox it in a worker or process), PDFium build complexity, and pdfium-render on WASM needing a separately loaded PDFium module.
- **C. hayro everywhere.** Pure Rust, memory-safe, MIT/Apache, natural WASM fit, and the same crate in core and UI. Downsides: version 0.x, no forms or annotations or text-selection infrastructure, and an unverified fidelity tail (encryption, colour management). It is a real bet on one maintainer ecosystem (LaurenzV/Typst).
- **D. Hybrid (recommended).** Define a Lamina `RenderBackend` trait. Default to PDFium (native + WASM). Run hayro in CI as a second backend and diff it against PDFium on the corpus. Expose a TS API that matches pdf.js-style layers (canvas + text layer + annotation layer). Implement text, annotations, and editing in Lamina's own Rust model, using PDFium only where needed.
- Security: every option parses untrusted input. Run it in a dedicated Web Worker on the web, and in a separate process (or at least a thread with crash isolation) on desktop, as Chrome's sandboxed OOPIF does.

### Gaps
- No current independent fidelity numbers support picking B over C on quality. Lamina should build its own corpus-diff harness from the pdf.js and PDFBox test PDFs, the same corpora hayro uses.
- Apple PDFKit as a macOS-only native path was not evaluated with sources. It is proprietary and platform-locked, so it is only a possible optional backend.
