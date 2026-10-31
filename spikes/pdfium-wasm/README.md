# Spike: `pdfium-wasm` (Round C1b)

- **Date:** 2026-10-30
- **Status:** done. The recommendation at the end, with [C1a](../hayro-parity/README.md)'s, is input to ADRs 0004 and 0009 ([C2a](../../docs/todos/ready/C2a-render-adrs.md)); it decides nothing on its own.
- **Plan:** [web-app plan §6 C1](../../docs/plans/web-app.md#c1-rendering-spikes), [architecture §4](../../docs/plans/architecture.md#4-spikes-to-close-open-questions) (`pdfium-wasm`)
- **Todo:** [C1b](../../docs/todos/done/C1b-pdfium-wasm-spike.md)
- **Code:** throwaway-quality, outside both workspaces and the pnpm workspace, never imported by product code ([code organisation §12](../../docs/guidelines/code-organization.md#12-spikes-vs-product-code)); not in CI.

## Question

Can PDFium-WASM and lamina's WebAssembly share one web worker in Chromium, Firefox and WebKit, and what does that cost? That covers download size, instantiate time, first page visible, render time per page, peak memory per input strategy, and above all the [two-parsers](../../docs/research/architecture/app-and-package-architecture.md#pdfium-ships-now-hayro-is-the-destination-and-a-trait-decides) cost: how long PDFium takes to show an edit after lamina appends an incremental update.

## Answer in one paragraph

**Yes, and it is cheap, except for four things a product must design around.** Both engines instantiate side by side in 24–57 ms (Chromium fastest, Firefox slowest). Each module has its own `WebAssembly.Memory`, so they don't conflict. Page 1 of the 10-page 1 MB file is visible in **15–39 ms cold and 6–8 ms warm**, far inside the 700 ms target. A text page renders in 2–4 ms and a full-page image in 13–15 ms (Chromium, WebKit). Encrypted files open in both engines, with the password kept in the worker.

The four things:
1. **Memory doubles when both engines hold a copy.** The 100 MB scan peaks at 95 MiB (lamina) + 237 MiB (PDFium), plus the whole file as a JS `ArrayBuffer`. Custom file access (`FPDF_LoadCustomDocument` and `Document.openRange`, both over `File.slice()`) brings that down to 1 + 113 MiB and no JS copy.
2. **WebKit can't read a 500 MB `File` into an `ArrayBuffer`** ("The I/O read operation failed"). With custom access it opens, saves and reloads the same file in every browser.
3. **lamina's save is the reload cost, not PDFium.** PDFium closes and reopens in milliseconds (custom access: under 20 ms even at 500 MB). But an incremental save reads the whole base to hash it for `/ID`, even when it outputs only the 0.4 KiB revision: about 4 s and the whole file in lamina's heap for 500 MB opened by ranges. Saving the whole output (copy strategy) peaks at about 3× the file in lamina's heap.
4. **Chromium runs a large first whole-output save about 10× slower than Firefox and WebKit:** 3 s for 50 MB, 30 s for 500 MB. A small save in the same worker first brings the 50 MB save down to about 230 ms, consistent with V8's tiering.

## Method

- **One page and one module worker** (`index.html`, `src/main.ts`, `src/worker.ts`; Vite 8.3.5, cross-origin isolated with COOP/COEP).
- **The worker** instantiates `@embedpdf/pdfium` 2.15.1 (its `pdfium.wasm`, compiled while it downloads through Emscripten's `instantiateWasm` hook) and `@lamina/pdf` (the local build of `main` at this PR, packed by `scripts/pack-lamina.sh`) in parallel with `Promise.all`.
- **Rendering:** PDFium renders into an RGBA bitmap (`FPDF_REVERSE_BYTE_ORDER`, annotations on), which is copied out of its heap into `ImageData`, then `createImageBitmap`, and transferred to the page, which draws it on a canvas.
- **Input strategies:**
  - **copy:** `File.arrayBuffer()`, then a copy into each module (`Document.open`, `FPDF_LoadMemDocument`).
  - **custom:** no whole copy. PDFium uses `FPDF_LoadCustomDocument` with a `getBlock` callback that reads `File.slice()` through `FileReaderSync`; lamina uses `Document.openRange(callbackSource(size, read))` over `File.slice()`.
- **Reload:** lamina applies `SetInfo` and saves incrementally; PDFium closes the old document (freeing its copy), opens the new bytes and re-renders page 1.
  - **copy:** lamina returns the whole saved file, which is copied into PDFium's heap.
  - **custom:** lamina returns only the appended revision (`output: "appendedOnly"`); PDFium reopens `new File([file, appended])` by ranges, so nothing is copied whole.
- **Loading:** both modules compile while they download (`WebAssembly.instantiateStreaming`; PDFium through Emscripten's `instantiateWasm` hook).
- **Runs:** `tests/bench.spec.mjs` (Playwright 1.64.1) gives every measured run (and every corpus file) a fresh browser context, so the worker, the WebAssembly memories and the cache are all fresh (WebAssembly memory never shrinks, so peaks would otherwise add up). The 10-page file runs twice in one context, cold and warm.
- **Inputs:** `scripts/make-fixtures.mjs` writes them with `@lamina/pdf`:
  - `ten-page.pdf`: 10 pages of Noto Sans text, each with a 200 × 160 picture, 1.0 MB.
  - `thousand-page.pdf`: 1,000 text pages, 0.9 MB.
  - `scan-100mb.pdf`: 17 full-page 1,600 × 1,200 noise images, 98 MB. A worst case: noise doesn't compress, and a real scan's JPEGs are smaller.
  - `reload-50mb.pdf` and `reload-500mb.pdf`: the same kind of pages, 52 MB and 501 MB.
  - The render-time sample: the 14 files of `fixtures/corpus` and the 6 golden examples. C1a's external corpora weren't checked out in this worktree.
  - Encrypted: `fixtures/encrypted/r6-aes-256-user.pdf` (password `Pässwört-6`, non-ASCII) and `r4-aes-128-user.pdf`.
- **Toolchain:** Vite 8.3.5 (rolldown 1.2.16, lightningcss 1.33.0), Playwright 1.64.1, TypeScript 5.9.3 for the type check; no lockfile is committed (see `pnpm-workspace.yaml`).
- **Machine:** C1a's: Apple M1 Pro, 16 GB, macOS 27.0.1. Browsers are Playwright 1.64.1's Chromium 156, Firefox 157 and WebKit (Safari 27.2), headless. The machine was also running other builds (a second agent's CI work), so single numbers are noisy, Firefox's most. The tables are the last of five runs; earlier runs are quoted where they differ.

### Reproduce

```sh
cargo xtask wasm                                  # at the repository root: the local @lamina/wasm and @lamina/pdf
cd spikes/pdfium-wasm
sh scripts/pack-lamina.sh                         # vendor/*.tgz
pnpm install                                      # outside the root workspace; no lockfile is committed, since the packed tarballs change with every lamina build
node scripts/make-fixtures.mjs                    # out/public/fixtures/ (about 660 MB; --skip-500 leaves out the 500 MB file)
pnpm exec playwright test                         # Chromium, Firefox, WebKit; out/results-<browser>.json
node scripts/summarise.mjs                        # the tables below
pnpm exec vite                                    # or by hand: http://localhost:5199, pick a file
```

## Results

### Download size

| Module | Raw | gzip -9 | brotli -11 |
| --- | ---: | ---: | ---: |
| `pdfium.wasm` (`@embedpdf/pdfium` 2.15.1) | 4,646,932 | 2,136,390 | 1,649,881 |
| PDFium's JS wrapper (`index.browser.js`) | 293,211 | 62,663 | 50,767 |
| `lamina_base.wasm` (this build) | 1,058,308 | 379,652 | 304,197 |

PDFium is 5.6× lamina's base module gzipped; together they are about 2.5 MB gzip (2.0 MB brotli). That's the download a web reader pays before page 1 on a cold cache, about 5 s on Fast 4G at 4 Mbit/s. That's over the 3 s cold target in [web-app §4](../../docs/plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) unless the fetch starts with the app shell and runs in parallel with the user picking a file.

**User agents:**
- chromium: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/156.0.8078.4 Safari/537.36
- firefox: Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:157.0) Gecko/20100101 Firefox/157.0
- webkit: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.2 Safari/605.1.15

### Instantiate (ms; median over the runs' fresh workers)

|  | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| laminaMs | 17 | 40 | 38 |
| pdfiumMs | 24 | 57 | 45 |
| combinedMs | 24 | 57 | 46 |

### First page visible (ms, from the File handed to the worker to page 1 drawn on the page)

| File | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| ten-page, cold (first file after boot) | 38 | 15 | 39 |
| ten-page, warm (second time) | 6.3 | 7.3 | 8.4 |
| ten-page, custom file access | 38 | 17 | 52 |
| thousand-page, copy | 35 | 24 | 49 |
| thousand-page, custom | 49 | 33 | 69 |
| scan-100mb, copy | 105 | 203 | 127 |
| scan-100mb, custom | 51 | 47 | 73 |

### Render steps, ten-page page 1 (ms: PDFium render / copy out of the heap / createImageBitmap)

|  | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| first@1x | 4.8 / 0.3 / 0.9 (595×841) | 6.0 / 0.5 / 0.7 (595×841) | 5.0 / 0.5 / 0.2 (595×841) |
| first@2x | 8.2 / 1.0 / 3.9 (1190×1683) | 13 / 1.3 / 3.2 (1190×1683) | 7.6 / 1.1 / 0.9 (1190×1683) |
| middle@1x | 2.7 / 0.3 / 0.8 (595×841) | 3.7 / 0.2 / 0.6 (595×841) | 2.9 / 0.4 / 0.5 (595×841) |
| middle@2x | 7.2 / 1.1 / 4.6 (1190×1683) | 7.9 / 1.0 / 2.7 (1190×1683) | 6.5 / 1.1 / 1.0 (1190×1683) |

### Per-page render, 1× (ms, PDFium render + copy + ImageBitmap; median / p95)

| File | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| thousand-page, every page | 2.8 / 3.2 | 2.6 / 2.8 | 2.1 / 2.4 |
| scan-100mb, every page | 14 / 16 | 15 / 16 | 13 / 15 |
| ten-page | 3.8 / 6.3 | 3.8 / 5.4 | 3.3 / 5.4 |
| corpus sample (median of the files' medians) | 1.3 | 0.9 | 1.6 |

### Peak WebAssembly memory (MiB: lamina base module / PDFium), per input strategy

| File, strategy | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| thousand-page, copy | 4 / 18 | 4 / 18 | 4 / 18 |
| thousand-page, custom | 4 / 18 | 4 / 18 | 4 / 18 |
| scan-100mb, copy | 95 / 237 | 95 / 237 | 95 / 237 |
| scan-100mb, custom | 1 / 113 | 1 / 113 | 1 / 113 |
| reload-50mb, copy | 51 / 128 | 51 / 128 | 51 / 128 |
| reload-500mb, copy | 479 / 776 | 479 / 776 | fails |
| reload-500mb, custom | 2 / 334 | 2 / 334 | 2 / 334 |

### Reload after an incremental save (ms: lamina edit + save / PDFium close / copy + reopen / re-render page 1 / total)

| Input | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| ten-page (1 MB) | 62 / 0.6 / 0.4 / 5.2 / **68** | 6.3 / 0.1 / 0.3 / 11 / **18** | 8.7 / 0.4 / 0.6 / 4.6 / **14** |
| reload-50mb | 2920 / 0.3 / 7.0 / 17 / **2944** | 384 / 0.1 / 6.2 / 19 / **409** | 274 / 0.6 / 11 / 14 / **299** |
| reload-50mb, heap already grown by the 100 MB file | 2931 / 0.1 / 6.4 / 16 / **2953** | 348 / 0.1 / 2.6 / 19 / **370** | 259 / 0.2 / 7.1 / 13 / **280** |
| reload-50mb, after a small save in the same worker | 208 / 0.1 / 2.1 / 15 / **225** | 339 / 0.1 / 5.6 / 29 / **374** | 254 / 0.2 / 2.8 / 13 / **270** |
| reload-500mb | 29702 / 10 / 1090 / 24 / **30826** | 3396 / 1.1 / 404 / 24 / **3825** | fails: Error: The I/O read operation failed. |
| reload-50mb, custom (appended revision only, reopened by ranges) | 412 / 0.3 / 2.9 / 16 / **431** | 433 / 0.4 / 1.6 / 20 / **455** | 483 / 0.6 / 8.5 / 17 / **509** |
| reload-500mb, custom | 3900 / 0.8 / 2.5 / 16 / **3919** | 3904 / 0.8 / 2.1 / 20 / **3927** | 4714 / 0.8 / 1011 / 17 / **5743** |

### Peak WebAssembly memory during the reload (MiB: lamina / PDFium, from the save to the re-render)

| Input | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| reload-50mb, copy | 179 / 128 (appended 0.4 KiB) | 179 / 128 (appended 0.4 KiB) | 179 / 128 (appended 0.4 KiB) |
| reload-50mb, custom | 62 / 65 (appended 0.4 KiB) | 62 / 65 (appended 0.4 KiB) | 62 / 65 (appended 0.4 KiB) |
| reload-500mb, copy | 1504 / 776 (appended 0.4 KiB) | 1504 / 776 (appended 0.4 KiB) | fails |
| reload-500mb, custom | 491 / 334 (appended 0.4 KiB) | 486 / 334 (appended 0.4 KiB) | 491 / 334 (appended 0.4 KiB) |

### Encrypted files (both engines, password inside the worker only)

| File | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| r6 AES-256 (`Pässwört-6`), copy | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) |
| r6 AES-256, custom | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) |
| r4 AES-128 (`user-pw`), copy | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) |

Earlier runs (before the review fixes: no streaming compile for PDFium, and the copy reload kept the original copy in PDFium's heap) differ mostly for Firefox:
- **ten-page cold:** 14–17 ms.
- **scan-100mb copy, first page:** 72–794 ms in Firefox (noisy), 112–196 ms in WebKit.
- **Chromium:** the slow first large save showed in every fresh-worker run: 2.9–3.4 s for 50 MB, 29–30 s for 500 MB.

## Findings

1. **One worker holds both engines without conflict.** Each module has its own `WebAssembly.Memory` (two heaps, sizes in the tables), and nothing clashes: no symbol, import or memory conflicts. `@embedpdf/pdfium` is an Emscripten build with a **growable heap** (`emscripten_resize_heap` is linked), so the fixed-heap out-of-memory problem pdfium-render warns about doesn't apply. PDFium isn't thread-safe; one instance per worker, as here, is the model.
2. **First page is fast; the download is the cost.** In every browser, cold first page of the 1 MB file is under 40 ms once both modules are instantiated (≈ 25–60 ms). The 2.1 MB gzip PDFium download dominates a cold start.
3. **Render cost is PDFium's, and small.** Copying the bitmap out of the heap and `createImageBitmap` add under 1 ms at 1× and 1–5 ms at 2× DPR. RGBA output (`FPDF_REVERSE_BYTE_ORDER`) avoids a BGRA swizzle in JS.
4. **Memory: custom access halves PDFium's heap and removes both copies.** On the 100 MB scan:
   - **copy:** the whole file once in JS (`File.arrayBuffer()`, kept until the reload), then 95 MiB in lamina's heap and 237 MiB in PDFium's (the file plus its decode buffers).
   - **custom:** no JS copy; lamina holds 1 MiB (the ranges it read) and PDFium 113 MiB.

   Text files are small either way, since their decode buffers are small.
5. **PDFium's side of the reload is cheap; lamina's save is the cost.**
   - **PDFium:** closing takes under 2 ms. Reopening takes 2–11 ms for 50 MB; at 500 MB it is 0.4–1.1 s with the copy strategy (copying the bytes into its heap) and under 3 ms with custom access in Chromium and Firefox (WebKit: about 1 s, reading the joined `File`). Re-rendering is a normal render.
   - **lamina, copy strategy (whole output):** the save peaks at about 3× the file in lamina's heap (179 MiB for 50 MB, 1,504 MiB for 500 MB: the base, the output, and the rebase onto it).
   - **lamina, custom strategy (appended revision only):** the output is 0.4 KiB, but the save still reads the whole base once to hash it for `/ID` (`IncrementalWriter`'s documented two passes). Opened by ranges, that pulls the whole file into lamina's sparse source: about 4 s and 491 MiB for 500 MB, in every browser. *Follow-up for the save API:* an `/ID` that doesn't need the whole base (keep the base's first `/ID` element, which ISO 32000-2 §14.4 says stays permanent, and generate the second from what the save already knows, such as the revision's bytes, the base's length and its trailer), so an appended-only save on a range-opened document reads only what it changes.
6. **Chromium (V8) runs one long whole-output save in a fresh instance about 10× slower.** The same save after a small one in the same worker takes 210–340 ms for 50 MB, like Firefox and WebKit. That fits V8's tiering: functions start in the Liftoff baseline compiler and are optimised only after enough calls, so a save called once runs its hashing and writing loops in baseline code throughout. Firefox optimises in the background. The custom-strategy save (no whole output) runs at the same speed in all three browsers. *Recommendation:* run large saves as lamina save jobs, `doc.saveJob()` stepped in small budgets, which also keeps the worker responsive; or warm the save path once after `init()`. Measure both in C2b.
7. **WebKit can't read a 500 MB `File` into one `ArrayBuffer`**: `File.arrayBuffer()` in the worker fails with "The I/O read operation failed" (the page's `fetch()` of the same file into a `Blob` succeeded). Chromium and Firefox do. With custom access, WebKit opens, renders, saves and reloads the same file (first page and reload numbers in the tables).
8. **The wasm32 ceiling is close when both engines hold a copy.** At 500 MB the copy strategy's heaps peak at about 2.3 GB together during the reload (1,504 + 776 MiB), plus the JS `ArrayBuffer`. Each module has its own 4 GiB limit, but a browser tab's total budget is the real limit. Custom access stays at about 0.8 GB, and only lamina's `/ID` hash keeps it from being far less (finding 5). Custom access reads positions as unsigned 32-bit, so it works up to 4 GiB; beyond that PDFium's 64-bit file access (`FPDF_FILEACCESS` has a 32-bit length on wasm32) would be needed.
9. **Encryption works in both engines**, including R6 AES-256 with a non-ASCII password (`Pässwört-6`: PDFium takes it as UTF-8 through `cwrap`'s `string`) and both input strategies. The password is passed to both inside the worker only. lamina refuses to save incrementally onto an encrypted base (`encrypted-base`, by design until editor round E11), so editing an encrypted file in the reader needs E11 or a decrypted full rewrite. A failed open (a wrong password) frees PDFium's file-access struct and callback, so a reader can retry passwords without leaking.
10. **Browser memory APIs:** `performance.measureUserAgentSpecificMemory()` returned nothing in headless Chromium even with cross-origin isolation, and it doesn't exist in Firefox or WebKit. The tables use `memory.buffer.byteLength` of each module, the same method in every browser, which counts WebAssembly heaps only, not JS objects or bitmaps.

## Licences

- **`@embedpdf/pdfium` 2.15.1:** the wrapper is **MIT** (`LICENSE`, © 2024 CloudPDF, Ji Chang). `LICENSE.pdfium` holds PDFium's own **BSD-3-Clause** and **Apache-2.0** texts.
- **Third-party code compiled into `pdfium.wasm`:**
  - **Found in the binary's strings:** FreeType (`FREETYPE_PROPERTIES`; FTL or GPLv2, used under the **FTL**), zlib (**zlib**), OpenJPEG (**BSD-2-Clause**), libpng (**libpng/PNG Reference Library**).
  - **Inferred from PDFium's build, no identifying strings:** Little-CMS 2 (**MIT**, PDFium's ICC path), libjpeg-turbo (**IJG + BSD-3-Clause**, PDFium's DCT path), PDFium's AGG copy (**BSD-3** in PDFium's tree). EmbedPDF's build script is the place to confirm them before ADR 0004 pins the package.
- **No GPL or AGPL code is linked.** A "GPL" string in the binary is glyph data, not a licence. FreeType's dual licence lets it be used under the FTL.
- **The package ships none of the third-party notices.** A product must bundle them itself: FreeType's FTL credit, OpenJPEG's, zlib's, libpng's, lcms2's and libjpeg-turbo's.
- **This spike's own dependencies:** Vite (MIT) and Playwright (Apache-2.0), dev tools only.

## Limitations

- **One machine, headless browsers, shared load:** single numbers are noisy, Firefox's most. Compare within a column.
- **Synthetic inputs:** the scan is noise (incompressible), not real scanned JPEGs. The render sample is lamina's own corpus (20 small files), not C1a's 1,188-document corpora.
- **Not done:**
  - the OPFS input strategy (`createSyncAccessHandle` read by both engines), only `File.slice()`;
  - a self-built pinned PDFium WASM to compare size;
  - the threaded build;
  - memory while scrolling, beyond the WebAssembly heaps.
- **Threaded build (not built):** it would need cross-origin isolation (COOP `same-origin`, COEP `require-corp` or `credentialless`, as this page sets) for `SharedArrayBuffer`, an Emscripten `-pthread` build of PDFium and its worker pool. PDFium itself still isn't thread-safe per document, so the gain is parallel rendering across documents or tiles, at the cost of the isolation headers for the whole app.

## Recommendation for ADRs 0004 and 0009 (joint with C1a)

These are recommendations, not decisions; C2a records the ADRs.

1. **ADR 0004, the renderer:** PDFium in the web reader, as C1a recommends. Pin `@embedpdf/pdfium` at an exact version.
   - Verify `pdfium.wasm`'s SHA-256 in the build, and record which PDFium revision it carries.
   - Track upstream PDFium security fixes through the package's releases, re-pinning deliberately (ADR 0019).
   - Bundle the third-party notices the package leaves out.
   - Build PDFium ourselves only if the package lags a security fix.
   - hayro stays C1a's CI oracle; its smaller download is the reason to revisit when its parity gates are met.
2. **ADR 0009, the worker topology:** one module worker holding **both engines**, with lamina as the model and PDFium as the rasteriser, single-threaded. Typed RPC to the page; rendered pages go back as transferred `ImageBitmap`s. A threaded variant only if C3c's perf harness shows rendering is the bottleneck.
3. **Loading:**
   - Start fetching and compiling both modules when the app shell loads (`WebAssembly.compileStreaming`, in parallel with the user picking a file).
   - Instantiate in the worker.
   - Cache with the service worker, keyed by the pinned hash.
4. **Input:** use the **custom strategy by default**. `FPDF_LoadCustomDocument` over the `File` (or an OPFS handle) for PDFium, and `Document.openRange(callbackSource(…))` for lamina. That halves PDFium's heap, removes lamina's copy and the JS copy, and is the only way WebKit opened the 500 MB file. Copying whole bytes stays an option for small files only.
5. **Reload after edits:**
   - lamina saves incrementally with `output: "appendedOnly"`, and PDFium closes and reopens `File + appended revision` by ranges. Measured: under 25 ms for PDFium's part in Chromium and Firefox at any size, so no partial-reload API is needed.
   - lamina's save is the cost: it reads the whole base for `/ID` (finding 5). Fix that in the save API before large files are editable in the reader; until then, run big saves as stepped save jobs so they don't block the worker, and so Chromium's tiering doesn't make a first whole-output save 10× slower.
6. **The §4 budgets look achievable for rendering and memory.** First page under 50 ms warm; 1,000 pages render at 2–4 ms each, and the WebAssembly heaps stay under 25 MiB. The cold-start download (≈ 2.5 MB gzip) is the one at risk on Fast 4G, and the parallel fetch in point 3 is what keeps it inside 3 s.
