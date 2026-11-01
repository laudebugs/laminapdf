# @laminapdf/render-pdfium

Lamina PDF's default web renderer: PDFium as WebAssembly behind [`@laminapdf/render`](../render/README.md)'s `RenderBackend`. It runs only inside the viewer's document Web Worker. It is private, and `@laminapdf/sdk` never depends on it.

```ts
import wasmUrl from "@embedpdf/pdfium/pdfium.wasm?url";
import { initPdfium } from "@laminapdf/render-pdfium";

const pdfium = await initPdfium(wasmUrl); // a URL, Response, bytes or compiled WebAssembly.Module
```

- **The build:** [`@embedpdf/pdfium`](https://www.npmjs.com/package/@embedpdf/pdfium) **2.15.1**, an npm dependency pinned exactly. Its `pdfium.wasm` must have SHA-256 `5e4cd023…4eaa`.
  - **Checking:** the hash is checked before every build and test, so any other file fails them.
  - **Fetched, not vendored:** pnpm's lockfile pins the tarball's integrity, and the 4.6 MB binary stays out of git.
  - **Revision:** the package doesn't record its PDFium revision. It is built from EmbedPDF's fork, [`embedpdf/runtime`](https://github.com/embedpdf/runtime), without XFA, V8 or Skia.
- **Loading:** `initPdfium(wasm)` loads the module through Emscripten's `instantiateWasm` hook. URLs are compiled while they download when served as `application/wasm`. The caller decides where the `.wasm` comes from, which controls caching and lazy loading. There are no top-level side effects.
- **Input:** PDFium's custom file access (`FPDF_LoadCustomDocument`) reads the `RenderSource` block by block, with no whole copy. A block the source can't give yet makes the call throw `data-pending` with its ranges, even if PDFium carried on with defaults. Input is limited to 4 GiB (a wasm32 `unsigned long`).
- **Rendering:** each document has a form-fill environment (`PDFiumExt_InitFormFillEnvironment`), as the native adapter does.
  - **With widgets** (`FPDF_RenderPageBitmap`, then `FPDF_FFLDraw`), the page is laid out at its whole pixel size and shifted, so only the tile lands in a tile-sized bitmap.
  - **Without widgets,** `FPDF_RenderPageBitmapWithMatrix` gets `pageToTile`'s matrix.
  - **Output:** `FPDF_REVERSE_BYTE_ORDER` gives RGBA, which is copied out of the heap once. Bitmap and buffer are freed on every path.
- **Cancellation:** the `AbortSignal` is checked before and after the synchronous engine call. Progressive rendering is exported by the build, but not used yet; that's a follow-up if long single tiles need it.
- **Notices:** `@embedpdf/pdfium` ships none for PDFium's bundled libraries. [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) lists them, and apps ship it.
- **Size:** `pdfium.wasm` is 2,136,390 B gzip.

## For contributors

It is the default renderer of [ADR 0004](../../docs/decisions/0004-render-backend.md), pinned per [§3](../../docs/decisions/0004-render-backend.md#3-the-pinned-pdfium-builds), in the document worker of [ADR 0009](../../docs/decisions/0009-web-client-architecture.md) §2; its size was measured by [C1b](../../docs/todos/done/C1b-pdfium-wasm-spike.md). A re-pin changes `package.json`, the hash in the script, ADR 0004 and `THIRD_PARTY_NOTICES.md` together.

- **Hash check:** `scripts/verify-pdfium.mjs` checks the hash before every `build`, `typecheck` and `test`; `test/verify.test.ts` tests the verifier.
- **Size budget:** `thirdParty.pdfium` in `packages/wasm/size-budgets.json` is the measured size plus 5 %, checked by `cargo xtask size`.

### Tests

- **Node:** `pnpm --filter @laminapdf/render-pdfium test` (part of `cargo xtask check ts`) runs the verifier and the RenderBackend contract over the installed `pdfium.wasm`.
- **Browsers:** `pnpm --filter @laminapdf/render-pdfium exec vitest run -c vitest.browser.config.ts --project chromium` runs the contract in Chromium, and likewise `firefox` and `webkit`. `cargo xtask pdf-runtimes <browser>` runs it too. The browser runs add [C2b](../../docs/todos/done/C2b-lamina-render-crate.md)'s goldens: EmbedPDF's build against native `chromium/7881` goldens, at SSIM ≥ 0.98.
