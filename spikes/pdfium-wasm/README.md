# Spike: `pdfium-wasm` (Round C1b)

- **Date:** 2026-10-30
- **Status:** done. The recommendation at the end, with [C1a](../hayro-parity/README.md)'s, is input to ADRs 0004 and 0009 ([C2a](../../docs/todos/ready/C2a-render-adrs.md)); it decides nothing on its own.
- **Plan:** [web-app plan §6 C1](../../docs/plans/web-app.md#c1-rendering-spikes), [architecture §4](../../docs/plans/architecture.md#4-spikes-to-close-open-questions) (`pdfium-wasm`)
- **Todo:** [C1b](../../docs/todos/done/C1b-pdfium-wasm-spike.md)
- **Code:** throwaway-quality, outside both workspaces and the pnpm workspace, never imported by product code ([code organisation §12](../../docs/guidelines/code-organization.md#12-spikes-vs-product-code)); not in CI.

## Question

Can PDFium-WASM and lamina's WebAssembly share one web worker in Chromium, Firefox and WebKit, and what does that cost? That covers download size, instantiate time, first page visible, render time per page, peak memory per input strategy, and above all the [two-parsers](../../docs/research/architecture/app-and-package-architecture.md#pdfium-ships-now-hayro-is-the-destination-and-a-trait-decides) cost: how long PDFium takes to show an edit after lamina appends an incremental update.

## Answer in one paragraph

**Yes, and it is cheap, except for three things a product must design around.** Both engines instantiate side by side in about 30 ms (WebKit, Chromium) to 120 ms (Firefox, noisy). Each module has its own `WebAssembly.Memory`, so they don't conflict. Page 1 of the 10-page 1 MB file is visible in **17–44 ms cold and 7–10 ms warm**, far inside the 700 ms target. A text page renders in 2–4 ms and a full-page image in 13–15 ms (Chromium, WebKit). Encrypted files open in both engines, with the password kept in the worker.

The three things:
1. **Memory doubles when both engines hold a copy.** The 100 MB scan peaks at 95 MiB (lamina) + 237 MiB (PDFium). Giving PDFium the file through `FPDF_LoadCustomDocument` over `File.slice()`, and lamina through `openRange`, brings that down to 1 + 113 MiB.
2. **Chromium runs a large first save about 10× slower than Firefox and WebKit:** 3 s for 50 MB, 30 s for 500 MB. A small save in the same worker first brings the 50 MB save down to about 230 ms. V8 starts every function in its baseline compiler and optimises only after repeated calls, so one long save never gets the optimised code.
3. **WebKit can't read a 500 MB `File` into an `ArrayBuffer`** ("The I/O read operation failed"). Large files must go in by ranges.

The reload itself, PDFium closing and reopening the saved bytes, is milliseconds for 50 MB and 0.3–0.6 s for 500 MB. The cost is lamina's save, not the second parser.

## Method

- **One page and one module worker** (`index.html`, `src/main.ts`, `src/worker.ts`; Vite 8.3.5, cross-origin isolated with COOP/COEP).
- **The worker** instantiates `@embedpdf/pdfium` 2.15.1 (its `pdfium.wasm`, fetched and passed as `wasmBinary`) and `@lamina/pdf` (the local build of `main` at this PR, packed by `scripts/pack-lamina.sh`) in parallel with `Promise.all`.
- **Rendering:** PDFium renders into an RGBA bitmap (`FPDF_REVERSE_BYTE_ORDER`, annotations on), which is copied out of its heap into `ImageData`, then `createImageBitmap`, and transferred to the page, which draws it on a canvas.
- **Input strategies:**
  - **copy:** `File.arrayBuffer()`, then a copy into each module (`Document.open`, `FPDF_LoadMemDocument`).
  - **custom:** no whole copy. PDFium uses `FPDF_LoadCustomDocument` with a `getBlock` callback that reads `File.slice()` through `FileReaderSync`; lamina uses `Document.openRange(callbackSource(size, read))` over `File.slice()`.
- **Reload:** lamina applies `SetInfo` and saves incrementally. PDFium closes the old document, gets a copy of the new bytes and re-renders page 1.
- **Runs:** `tests/bench.spec.mjs` (Playwright 1.64.1) gives every measured run a fresh browser context, so the worker, the WebAssembly memories and the cache are all fresh (WebAssembly memory never shrinks, so peaks would otherwise add up). The 10-page file runs twice in one context, cold and warm.
- **Inputs:** `scripts/make-fixtures.mjs` writes them with `@lamina/pdf`:
  - `ten-page.pdf`: 10 pages of Noto Sans text, each with a 200 × 160 picture, 1.0 MB.
  - `thousand-page.pdf`: 1,000 text pages, 0.9 MB.
  - `scan-100mb.pdf`: 17 full-page 1,600 × 1,200 noise images, 98 MB. A worst case: noise doesn't compress, and a real scan's JPEGs are smaller.
  - `reload-50mb.pdf` and `reload-500mb.pdf`: the same kind of pages, 52 MB and 501 MB.
  - The render-time sample: the 14 files of `fixtures/corpus` and the 6 golden examples. C1a's external corpora weren't checked out in this worktree.
  - Encrypted: `fixtures/encrypted/r6-aes-256-user.pdf` (password `Pässwört-6`, non-ASCII) and `r4-aes-128-user.pdf`.
- **Machine:** C1a's: Apple M1 Pro, 16 GB, macOS 27.0.1. Browsers are Playwright 1.64.1's Chromium 156, Firefox 157 and WebKit (Safari 27.2), headless. The machine was also running other builds (a second agent's CI work), so single numbers are noisy, Firefox's most. The tables are the last of five runs; earlier runs are quoted where they differ.

### Reproduce

```sh
cargo xtask wasm                                  # at the repository root: the local @lamina/wasm and @lamina/pdf
cd spikes/pdfium-wasm
sh scripts/pack-lamina.sh                         # vendor/*.tgz
pnpm install                                      # its own lockfile, outside the root workspace
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
- chromium: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.8078.4 Safari/537.36
- firefox: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:157.0) Gecko/20100101 Firefox/157.0
- webkit: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.2 Safari/605.1.15

### Instantiate (ms; median over the runs' fresh workers)

|  | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| laminaMs | 7.4 | 31 | 19 |
| pdfiumMs | 29 | 118 | 32 |
| combinedMs | 29 | 118 | 32 |

### First page visible (ms, from the File handed to the worker to page 1 drawn on the page)

| File | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| ten-page, cold (first file after boot) | 29 | 17 | 44 |
| ten-page, warm (second time) | 6.9 | 7.1 | 9.8 |
| ten-page, custom file access | 42 | 18 | 56 |
| thousand-page, copy | 36 | 24 | 48 |
| thousand-page, custom | 54 | 34 | 67 |
| scan-100mb, copy | 72 | 794 | 196 |
| scan-100mb, custom | 53 | 94 | 72 |

### Render steps, ten-page page 1 (ms: PDFium render / copy out of the heap / createImageBitmap)

|  | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| first@1x | 5.9 / 0.3 / 0.9 (595×841) | 6.3 / 0.4 / 0.9 (595×841) | 4.9 / 0.3 / 0.1 (595×841) |
| first@2x | 8.1 / 1.0 / 3.7 (1190×1683) | 22 / 1.1 / 4.6 (1190×1683) | 9.0 / 1.8 / 1.8 (1190×1683) |
| middle@1x | 2.9 / 0.3 / 0.9 (595×841) | 3.7 / 0.2 / 0.5 (595×841) | 3.7 / 0.4 / 0.3 (595×841) |
| middle@2x | 7.1 / 0.9 / 3.9 (1190×1683) | 8.0 / 0.9 / 13 (1190×1683) | 7.9 / 3.1 / 2.2 (1190×1683) |

### Per-page render, 1× (ms, PDFium render + copy + ImageBitmap; median / p95)

| File | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| thousand-page, every page | 2.9 / 3.3 | 3.8 / 16 | 2.1 / 2.7 |
| scan-100mb, every page | 15 / 16 | 88 / 257 | 13 / 17 |
| ten-page | 3.7 / 5.3 | 4.1 / 4.6 | 3.6 / 7.6 |
| corpus sample (median of the files' medians) | 1.3 | 1.0 | 0.6 |

### Peak WebAssembly memory (MiB: lamina base module / PDFium), per input strategy

| File, strategy | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| thousand-page, copy | 4 / 18 | 4 / 18 | 4 / 18 |
| thousand-page, custom | 4 / 18 | 4 / 18 | 4 / 18 |
| scan-100mb, copy | 95 / 237 | 95 / 237 | 95 / 237 |
| scan-100mb, custom | 1 / 113 | 1 / 113 | 1 / 113 |
| reload-50mb, copy | 51 / 128 | 51 / 128 | 51 / 128 |
| reload-500mb, copy | 479 / 776 | 479 / 776 | fails |

### Reload after an incremental save (ms: lamina edit + save / PDFium close / copy + reopen / re-render page 1 / total)

| Input | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| ten-page (1 MB) | 63 / 0.6 / 0.4 / 5.4 / **70** | 6.3 / 0.1 / 0.3 / 6.0 / **13** | 11 / 0.5 / 0.6 / 6.4 / **18** |
| reload-50mb | 2938 / 0.3 / 2.9 / 16 / **2957** | 1451 / 0.1 / 36 / 192 / **1679** | 266 / 0.5 / 7.4 / 14 / **288** |
| reload-50mb, heap already grown by the 100 MB file | 2921 / 0.1 / 2.4 / 15 / **2939** | 431 / 0.2 / 7.6 / 26 / **465** | 255 / 0.1 / 6.7 / 13 / **275** |
| reload-50mb, after a small save in the same worker | 210 / 0.1 / 2.3 / 16 / **228** | 746 / 0.2 / 7.2 / 24 / **778** | 242 / 0.1 / 2.7 / 14 / **258** |
| reload-500mb | 29545 / 1.7 / 318 / 99 / **29963** | 4012 / 1.1 / 426 / 25 / **4465** | fails: Error: The I/O read operation failed. |

### Encrypted files (both engines, password inside the worker only)

| File | chromium | firefox | webkit |
| --- | ---: | ---: | ---: |
| r6 AES-256 (`Pässwört-6`), copy | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) |
| r6 AES-256, custom | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) |
| r4 AES-128 (`user-pw`), copy | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) | opens, 2 page(s) (PDFium 2) |


Earlier runs, on a quieter machine, differ mostly for Firefox:
- **ten-page cold:** 14–17 ms.
- **scan-100mb copy, first page:** 128–171 ms.
- **scan per-page median:** 15 ms.
- **reload-50mb total:** 298–426 ms.
- **Chromium:** the slow first large save showed in every fresh-worker run: 2.9–3.4 s for 50 MB, 29–30 s for 500 MB.

## Findings

1. **One worker holds both engines without conflict.** Each module has its own `WebAssembly.Memory` (two heaps, sizes in the tables), and nothing clashes: no symbol, import or memory conflicts. `@embedpdf/pdfium` is an Emscripten build with a **growable heap** (`emscripten_resize_heap` is linked), so the fixed-heap out-of-memory problem pdfium-render warns about doesn't apply. PDFium isn't thread-safe; one instance per worker, as here, is the model.
2. **First page is fast; the download is the cost.** In every browser, cold first page of the 1 MB file is under 50 ms once both modules are instantiated (≈ 30 ms). The 2.1 MB gzip PDFium download dominates a cold start.
3. **Render cost is PDFium's, and small.** Copying the bitmap out of the heap and `createImageBitmap` add under 1 ms at 1× and 1–5 ms at 2× DPR. RGBA output (`FPDF_REVERSE_BYTE_ORDER`) avoids a BGRA swizzle in JS.
4. **Memory: the custom strategy halves PDFium's heap and removes lamina's copy.** On the 100 MB scan:
   - **copy:** lamina holds 95 MiB (the file) and PDFium 237 MiB (the file, plus its decode buffers).
   - **custom:** lamina holds 1 MiB (the ranges it read) and PDFium 113 MiB.

   Text files are small either way, since their decode buffers are small. Neither strategy adds a JS copy beyond the `File`.
5. **The reload is cheap; lamina's save is the cost.** Closing and reopening the new bytes in PDFium takes 0.3 ms + 3–8 ms for 50 MB, and 1–4 ms + 300–600 ms for 500 MB (that's copying the bytes into its heap). Re-rendering is a normal render. lamina's incremental save of a 500 MB file holds the base plus a whole new copy, about 480 MiB in its heap, before PDFium copies it again (776 MiB).
6. **Chromium (V8) runs one long save in a fresh instance about 10× slower.** The same save after a small one in the same worker takes 210–250 ms for 50 MB, like Firefox and WebKit. That fits V8's tiering: functions start in the Liftoff baseline compiler and are optimised only after enough calls. A save called once runs its hashing and writing loops in baseline code throughout. Firefox optimises in the background. *Recommendation:* run large saves as lamina save jobs, `doc.saveJob()` stepped in small budgets, which also keeps the worker responsive; or warm the save path once after `init()`. Measure both in C2b.
7. **WebKit can't read a 500 MB `File` into one `ArrayBuffer`**: `File.arrayBuffer()` in the worker fails with "The I/O read operation failed" (the page's `fetch()` of the same file into a `Blob` succeeded). Chromium and Firefox do. Large files must go in by ranges: the custom strategy here, or OPFS.
8. **The wasm32 ceiling is close for both-engines-hold-a-copy.** At 500 MB the two heaps peak at about 1.25 GB together, plus the JS `File`/`ArrayBuffer`. Each module has its own 4 GiB limit, but a browser tab's total budget is the real limit. A 1 GB+ file would only work with the custom strategy.
9. **Encryption works in both engines**, including R6 AES-256 with a non-ASCII password (`Pässwört-6`: PDFium takes it as UTF-8 through `cwrap`'s `string`) and both input strategies. The password is passed to both inside the worker only. lamina refuses to save incrementally onto an encrypted base (`encrypted-base`, by design until editor round E11), so editing an encrypted file in the reader needs E11 or a decrypted full rewrite.
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
4. **Input:** use the **custom strategy by default**. `FPDF_LoadCustomDocument` over the `File` (or an OPFS handle) for PDFium, and `Document.openRange(callbackSource(…))` for lamina. That halves PDFium's heap, removes lamina's copy, and is the only way WebKit opens files of hundreds of MB. Copying whole bytes stays an option for small files only.
5. **Reload after edits:**
   - lamina saves incrementally, and PDFium closes and reopens the new bytes (through custom access, so no extra copy). That costs a few ms up to about 50 MB, so no partial-reload API is needed.
   - Run big saves as stepped save jobs, so Chromium's tiering doesn't make the first one 10× slower.
   - For files of hundreds of MB, the product's real cost is lamina holding base plus output: L2c-style sparse sources and an append-only save output, a follow-up on the save API, are what bound it.
6. **The §4 budgets look achievable for rendering and memory.** First page under 50 ms warm; 1,000 pages render at 2–4 ms each, and the WebAssembly heaps stay under 25 MiB. The cold-start download (≈ 2.5 MB gzip) is the one at risk on Fast 4G, and the parallel fetch in point 3 is what keeps it inside 3 s.
