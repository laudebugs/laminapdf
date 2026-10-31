# TypeScript / Web DX for @laminapdf/pdf (Rust WASM core, universal ESM package)

Scope note: about 17 fetches this session. The session's network blocks developers.cloudflare.com, vite.dev, typescriptlang.org and npmjs.com (403). Some sources were therefore read through their raw GitHub mirrors. Claims marked **[PK]** come from prior knowledge (training data, mid‑2026). Their URLs are the canonical ones, but those pages were **not re-fetched this session**, so treat them as lower confidence. Everything else was verified by fetch on 2026‑10‑29.

## 1. Loading .wasm across runtimes and bundlers (isomorphism)

### Takeaway
No single loading mechanism works everywhere in Oct 2026. The robust pattern has three parts:
- Ship the `.wasm` as a separate file.
- Resolve it per export condition: `new URL("x.wasm", import.meta.url)` + `fetch`/`compileStreaming` for browser/deno; `import mod from "./x.wasm"` for workerd; `fs.readFile` or `import source` for Node.
- Expose `init(input?)` that also accepts `Response | BufferSource | WebAssembly.Module | URL` as an escape hatch for odd bundlers.

wasm-bindgen's new `--target module` (source-phase imports) is the future-proof shape, but it is not universal yet.

### Cited Findings
- **Node unflagged Wasm ESM imports in v24.5.0 / v22.19.0.** The docs say: "Wasm modules no longer require the `--experimental-wasm-modules` flag. Importing both WebAssembly module instances and WebAssembly source phase imports is supported." — [Node ESM docs](https://nodejs.org/api/esm.html)
- **Node stability levels:**
  - Source-phase imports (`import source m from './x.wasm'`, `import.source()`) are "Stability: 1.2 - Release candidate" (added v24.0.0).
  - Instance-phase imports (`import * as M from './x.wasm'`) are "1.1 - Active development".
  - Importing where Wasm is unavailable throws `ERR_WEBASSEMBLY_NOT_SUPPORTED`.

  — [Node ESM docs](https://nodejs.org/api/esm.html)
- **Node source-phase example:** `import source libraryModule from './library.wasm'; const i = await WebAssembly.instantiate(libraryModule, importObject1);` — [Node ESM docs](https://nodejs.org/api/esm.html)
- **wasm-bindgen targets:**
  - `bundler` (the default) treats Wasm as an ES module. That model is "not natively implemented in any JS implementation at this time", so it needs a bundler. Webpack is fully compatible; Rolldown needs a plugin.
  - `web` emits a no-bundler ES module with manual init.
  - `nodejs` emits CommonJS `require`.
  - `deno` targets Deno.
  - `experimental-nodejs-module` is marked experimental.
  - The new `module` target uses `import source wasmModule from "./module.wasm"` + `new WebAssembly.Instance(...)`. It is "currently only supported in ESBuild and Node.js 24" and replaces `compileStreaming` and per-platform loading paths.

  — [wasm-bindgen deployment guide](https://raw.githubusercontent.com/wasm-bindgen/wasm-bindgen/main/guide/src/reference/deployment.md)
- **Cloudflare Workers:** Wrangler "will bundle any Wasm module that ends in `.wasm` or `.wasm?module`". You use `import mod from "./simple.wasm"` (you get a `WebAssembly.Module`), then `WebAssembly.instantiate(mod, importObject)`. Instantiate "at the top level of the script to avoid instantiation on every request." — [Cloudflare docs source (raw GitHub)](https://raw.githubusercontent.com/cloudflare/cloudflare-docs/production/src/content/docs/workers/runtime-apis/webassembly/javascript.mdx)
- **[PK] workerd blocks runtime compilation from bytes.** Workers disallow `WebAssembly.compile`/`instantiate` from raw bytes ("Wasm code generation disallowed by embedder"). Only Wrangler-bundled `.wasm` modules can be instantiated. This is the main reason inline base64 fails on workerd, and why a `workerd` export condition with a static `.wasm` import is needed. — [Cloudflare Workers WebAssembly docs](https://developers.cloudflare.com/workers/runtime-apis/webassembly/) (unverified this session)
- **Vite:**
  - Supports direct `.wasm` import (ESM integration: it instantiates the module and exposes its exports; the import "behaves as an async module").
  - `?init` imports give control over instantiation.
  - SSR: "Due to the lack of a universal way to load a file", these features "will only work in Node.js compatible runtimes for SSR builds."

  — [Vite features.md](https://raw.githubusercontent.com/vitejs/vite/main/docs/guide/features.md)
- **Vite dependency pre-bundling breaks wasm URL resolution.** jSquash documents `Failed to construct 'URL': Invalid URL` and the fix `optimizeDeps: { exclude: ["@jsquash/png"] }`. Nuxt needs `build.transpile` + `vite.optimizeDeps.exclude`. A known Vite bug breaks nested workers (a worker spawning a worker) in production builds. — [jSquash README](https://github.com/jamsinclair/jSquash)
- **jSquash:**
  - Ships one package per codec (`@jsquash/avif`, `jpeg`, `jxl`, `oxipng`, `png`, `qoi`, `resize`, `webp`), ESM-only.
  - Node is "limited and experimental".
  - Workers "require some additional steps so that the WASM binaries get included" (the manual init path).

  — [jSquash README](https://github.com/jamsinclair/jSquash)
- **unpdf** makes PDF.js work serverless:
  - A custom PDF.js build (v6.4.299 in the current README) bundled with Rolldown.
  - It inlines the worker ("embeds the PDF.js worker directly into the main bundle", because serverless runtimes can't load separate worker files).
  - String replacements strip browser references.
  - Polyfills cover `Promise.try` and `Map.prototype.getOrInsert`.
  - It requires the global `Iterator` constructor (Safari 18.4+).
  - Supported runtimes: Node 24+, Bun, browsers, Workers.

  — [unpdf README](https://github.com/unjs/unpdf)
- **[PK] Other libraries:**
  - `@resvg/resvg-wasm` requires an explicit `await initWasm(fetch('…/index_bg.wasm'))` (it also accepts ArrayBuffer/Module) before use, and exposes `.free()`.
  - `shiki` ships `shiki/wasm` (oniguruma as an inlined base64 module) plus a pure-JS regex engine as an alternative, to avoid wasm-loading issues.
  - `esbuild-wasm` takes `initialize({ wasmURL | wasmModule, worker })`.

  These are the three prevalent idioms: explicit init with a URL, inlined base64, or module injection. — [resvg-js](https://github.com/thx/resvg-js), [shiki engines](https://shiki.style/guide/regex-engines), [esbuild API](https://esbuild.github.io/api/#browser) (unverified this session)

### Inferences
- **Recommended export map** (sketch):
  ```jsonc
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "workerd": "./dist/workerd.js",   // import wasm from "./lamina_bg.wasm"
      "deno": "./dist/web.js",          // fetch(new URL(...import.meta.url)) works for file: and https: in Deno
      "node": "./dist/node.js",         // readFile(fileURLToPath(new URL(..., import.meta.url))) — no reliance on unflagged wasm ESM for Node 20/22.<19
      "browser": "./dist/web.js",       // compileStreaming(fetch(new URL("./lamina_bg.wasm", import.meta.url)))
      "default": "./dist/web.js"
    },
    "./lamina_bg.wasm": "./dist/lamina_bg.wasm",  // let users self-host / pass to init()
    "./inline": "./dist/inline.js"               // opt-in base64 build for single-file environments (not workerd)
  }
  ```
- **Why `new URL(..., import.meta.url)`.** Webpack 5, Vite, Rollup (with a plugin), esbuild (with the file loader), Rspack and Turbopack all recognise this static pattern, so it is the most portable asset reference. Keep it literal: no variables. Bundler support beyond Vite was not verified this session.
- **Inline base64.** It costs about +33% size before gzip, can't use `compileStreaming`, and is blocked on workerd. Offer it only as an opt-in subpath (`@laminapdf/pdf/inline`), never as the default.
- **Escape hatch.** Always accept `init({ module: WebAssembly.Module | Response | BufferSource | URL | string })`. Every surveyed library that skipped this got bug reports from bundler users (inferred from the jSquash/Vite workarounds above).

### Gaps
- I could not verify webpack 5 `asyncWebAssembly`/`experiments`, Rspack, Turbopack/Next.js, or Rollup `@rollup/plugin-wasm` behaviour (bundler docs not fetched).
- Bun's `.wasm` import semantics, and Deno's support for `import source`, are unverified.
- @napi-rs wasm (`wasm32-wasip1-threads` fallback packages) and photon-rs loading are not covered.

## 2. Runtime isomorphism: Uint8Array vs Buffer, Web Streams

### Takeaway
Public signatures should be `Uint8Array` (and accept `ArrayBuffer | ArrayBufferView`). Never return `Buffer`. Streams should be WHATWG `ReadableStream<Uint8Array>`, which is a global in Node ≥18, Deno, Bun and workerd.

### Cited Findings
- **[PK]** `Buffer` is a `Uint8Array` subclass, so accepting `Uint8Array` takes Buffers for free. Returning `Buffer` would tie output to Node. — [Node Buffer docs](https://nodejs.org/api/buffer.html) (unverified this session)
- **[PK]** `ReadableStream`, `WritableStream` and `TransformStream` are globals in Node (stable since v21), Deno, Bun and workerd. Interop helpers: `Readable.toWeb`/`fromWeb`. — [Node Web Streams](https://nodejs.org/api/webstreams.html) (unverified this session)
- **[PK]** TypeScript 5.7 made typed arrays generic over the buffer: `Uint8Array<ArrayBufferLike>`. Return `Uint8Array<ArrayBuffer>` from APIs so results can go straight into `Blob`/`Response`/`postMessage` transfer without type errors. — [TS 5.7 release notes](https://devblogs.microsoft.com/typescript/announcing-typescript-5-7/) (unverified this session)

### Inferences
- `save()` returns `Uint8Array<ArrayBuffer>` that is copied *out* of wasm memory. A view into linear memory would be invalidated by `memory.grow`.
- Add `saveStream(): ReadableStream<Uint8Array>` later, for large outputs.
- Input: `open(bytes | Blob | ReadableStream)`. In the Blob and stream cases, `await` the bytes in JS before handing them to sync wasm.

### Gaps
- Exact Bun/Deno edge cases (`BYOB` readers, `ReadableStream.from`) were not verified.

## 3. Worker offloading

### Takeaway
Keep the core API synchronous and in-thread. Ship an optional `@laminapdf/pdf/worker` that exposes the same document API as async proxies over `postMessage`:
- Transfer ArrayBuffers both ways.
- Take `AbortSignal` and an `onProgress` callback.
- The worker owns the wasm instance; documents are handles (ids).

PDF.js shows the cost of making the worker mandatory: the `workerSrc` configuration and version-mismatch pain.

### Cited Findings
- **Comlink** is a "tiny library (1.1kB)":
  - `Comlink.expose(obj)` in the worker, `Comlink.wrap(worker)` on the main thread.
  - Transferables: `await myProxy.someFunction(Comlink.transfer(data, [data.buffer]))`.
  - Callbacks go through `Comlink.proxy()`, since functions "are neither structured cloneable nor transferable".
  - `proxy[Comlink.releaseProxy]()` detaches.

  — [Comlink README](https://raw.githubusercontent.com/GoogleChromeLabs/comlink/main/README.md)
- **PDF.js requires configuring the worker URL:** `pdfjsLib.GlobalWorkerOptions.workerSrc = ".../pdf.worker.mjs"; const loadingTask = pdfjsLib.getDocument({ url }); … page.render(renderContext)`. — [pdf.js helloworld example](https://raw.githubusercontent.com/mozilla/pdf.js/master/examples/learning/helloworld.html)
- **The PDF.js worker must be the exact same version:** "it's necessary to use the _exact_ same version of the `pdf.js` and `pdf.worker.js` files". A mismatch happens when the worker comes from a CDN. — [pdf.js FAQ](https://github.com/mozilla/pdf.js/wiki/Frequently-Asked-Questions)
- **PDF.js memory advice:** the viewer "creates, renders, and holds canvases only for visible pages to reduce the amount of used memory". — [pdf.js FAQ](https://github.com/mozilla/pdf.js/wiki/Frequently-Asked-Questions)
- **PDF.js cleanup:** a loading task (`getDocument(...)`) has `.promise` and `.destroy()`, and pages have `.cleanup()`. — [pdf.js node example](https://raw.githubusercontent.com/mozilla/pdf.js/master/examples/node/getinfo.mjs)
- **Vite worker detection** works only if `new URL()` is used directly inside `new Worker()`: `new Worker(new URL('./worker.js', import.meta.url))`. `?worker` and `?url` suffixes are also available. — [Vite features.md](https://raw.githubusercontent.com/vitejs/vite/main/docs/guide/features.md)
- **unpdf:** serverless runtimes "can't load separate worker files", so unpdf inlines the worker. Browser rendering needs a DOM canvas, and a Web Worker doesn't have one in unpdf's model. — [unpdf README](https://github.com/unjs/unpdf)
- **[PK]** `SharedArrayBuffer` (needed for wasm threads) requires a cross-origin-isolated page (`Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp|credentialless`). Library users often cannot set these headers, so a threaded wasm build must be optional. — [MDN SharedArrayBuffer security requirements](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer#security_requirements) (unverified this session)
- **[PK]** `OffscreenCanvas` and `ImageBitmap` are transferable and available in workers in all major browsers. A worker can render RGBA into `ImageData`/`ImageBitmap` and transfer it, or draw into a transferred `OffscreenCanvas`. — [MDN OffscreenCanvas](https://developer.mozilla.org/en-US/docs/Web/API/OffscreenCanvas) (unverified this session)

### Inferences
Ideal API sketch:
```ts
// main thread, zero config in Vite/webpack because the URL literal is inside the package:
import { createWorkerClient } from "@laminapdf/pdf/worker";
const lamina = await createWorkerClient();            // internally: new Worker(new URL("./worker.js", import.meta.url), { type: "module" })
const ac = new AbortController();
await using doc = await lamina.open(bytes, { transfer: true, signal: ac.signal });
const bmp: ImageBitmap = await doc.page(0).render({ scale: 2, signal: ac.signal, onProgress: p => bar.value = p.ratio });
const out = await doc.save({ incremental: true });     // Uint8Array, transferred back
```
- Allow `createWorkerClient({ worker: myWorker })` for bundlers that don't detect the URL, and for CSP setups.
- Add a `lamina-version` handshake on connect so a version mismatch fails loudly (lesson from PDF.js).
- Cancellation: Rust can't be interrupted mid-call in sync wasm. Either chunk the work (per page or object) and check a flag between chunks, or `worker.terminate()` and respawn on abort. Document which one applies.
- Don't use Comlink for callbacks in the hot path. Send progress as fire-and-forget messages instead.

### Gaps
- PDF.js's internal `MessageHandler` protocol (streams over postMessage) was not fetched.
- Firefox/Safari OffscreenCanvas status in Oct 2026 was not verified.

## 4. Tree-shaking, subpath exports, multiple wasm modules

### Takeaway
A wasm-bindgen module is a single binary. JS tree shaking can drop unused *JS glue*, but not unused *wasm code*. Feature-level size control therefore needs either separate wasm binaries per subpath (`/render`, `/sign`) or a Cargo-feature-built family of binaries.

### Cited Findings
- jSquash's per-codec packages are the precedent: you install and load only the codecs you use. — [jSquash README](https://github.com/jamsinclair/jSquash)
- The wasm-bindgen deployment guide says nothing on tree shaking or size per target. — [wasm-bindgen deployment guide](https://raw.githubusercontent.com/wasm-bindgen/wasm-bindgen/main/guide/src/reference/deployment.md)
- unpdf lets users swap in the full PDF.js build: `definePDFJSModule(() => import('pdfjs-dist/legacy/build/pdf.mjs'))`. Canvas is injected: `canvasImport: () => import('@napi-rs/canvas')`. This is a pattern for optional heavy dependencies. — [unpdf README](https://github.com/unjs/unpdf)

### Inferences
- **Proposed layout:**
  - `@laminapdf/pdf`: parse, inspect, edit, save; the core wasm.
  - `@laminapdf/pdf/extract`: text and images.
  - `@laminapdf/pdf/render`: raster; a separate, heavier wasm.
  - `@laminapdf/pdf/sign`: CMS; could use WebCrypto for the private-key operations.
- Each subpath's `init()` loads its own wasm.
- **Cost:** every separate wasm binary duplicates shared Rust code (core parsing), and objects can't be passed between instances except as bytes. Options:
  - Pass serialized bytes between modules.
  - Use one "core + feature" binary per combination: core, core+render, core+render+sign.
  - Wait for wasm component-model linking. Not practical for browsers in 2026.
- Mark `"sideEffects": false` in package.json. Don't auto-init at import time (top-level await), so importing a subpath for types costs nothing.
- **Dual sync/async API:** `await init()` once, then everything is sync. That covers the core. The worker client is the async twin, with the same method names returning Promises: a `Promisified<T>` mapped type generated from the sync interface keeps the two in lockstep.

### Gaps
- No measured data on how much of a lamina wasm binary each feature accounts for. That needs `twiggy` on real builds.

## 5. Resource management (`using`, FinalizationRegistry, linear memory)

### Takeaway
- Implement `[Symbol.dispose]` on every handle (Document, Page, rendered buffers) and keep `.free()`/`.close()` as aliases.
- Rely on wasm-bindgen's FinalizationRegistry only as a backstop.
- `using` is native in Chromium 134+, Firefox 141+ and Node 24. Safari is partial or unclear, so users must transpile with TS or esbuild. Make `Symbol.dispose` usage tolerate a missing global (polyfill `Symbol.dispose ??= Symbol.for("Symbol.dispose")`).

### Cited Findings
- **Support status:** Explicit Resource Management shipped in Chromium 134 / V8 13.8, Firefox 141 and Node 24. Safari: one source says `Symbol.dispose` is in desktop Safari 26.4 but not iOS. Another says it was only in Safari Technology Preview 250 (Aug 2026), with no stable timeline. **The sources conflict.** — [V8 blog](https://v8.dev/features/explicit-resource-management); [OpenReplay](https://blog.openreplay.com/javascript-cleanup/); [ByteIota](https://byteiota.com/javascripts-using-keyword-is-now-cross-browser-act-now/); [W3Schools JS 2027 table](https://www.w3schools.com/js/js_2027.asp)
- **wasm-bindgen uses FinalizationRegistry by default:** "By default wasm-bindgen does use the TC39 weak references proposal if support is detected"; it "will ensure that all memory is cleaned up, regardless of whether it's explicitly deallocated or not". `.free()` remains supported. The guide doesn't mention `Symbol.dispose`. — [wasm-bindgen weak references](https://raw.githubusercontent.com/wasm-bindgen/wasm-bindgen/main/guide/src/reference/weak-references.md)
- **PDF.js** exposes explicit `loadingTask.destroy()` and `page.cleanup()`. — [pdf.js node example](https://raw.githubusercontent.com/mozilla/pdf.js/master/examples/node/getinfo.mjs)
- **[PK]** TypeScript 5.2 added `using`/`await using` (it downlevels with `lib: ["esnext.disposable"]`). Recent wasm-bindgen versions emit `[Symbol.dispose]` on exported structs when the symbol exists. — [TS 5.2 release notes](https://devblogs.microsoft.com/typescript/announcing-typescript-5-2/) (unverified this session; check the wasm-bindgen CHANGELOG)
- **[PK]** WebAssembly linear memory can grow (`memory.grow`) but never shrinks. A one-off 500 MB render leaves the instance at that size until the instance itself is dropped. The 32-bit (wasm32) ceiling is 4 GiB; memory64 lifts it but is not universally enabled. — [MDN WebAssembly.Memory](https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/Memory) (unverified this session)

### Inferences
- Bound memory three ways:
  - Enforce `Limits` (ADR 0003) inside Rust, including a max-allocation budget.
  - Set the wasm `maximum` memory via a linker flag (`-C link-arg=--max-memory=…`), so growth fails as an error rather than an OOM of the tab.
  - Provide `lamina.reset()` (or run in a worker that can be terminated) to reclaim grown memory.
- After dispose, any method call throws `LaminaError { code: "DISPOSED" }`, not a wasm-bindgen null-pointer panic.

### Gaps
- Exact wasm-bindgen version that added `Symbol.dispose`, and Safari's stable status (MDN compat table not fetched).

## 6. Strict TypeScript: types, errors, tooling

### Takeaway
- Use discriminated unions for options and results.
- Use branded numeric units for points and page indices.
- Use one `LaminaError` class with a `code` string-literal union generated from `fixtures/codes.json`, so Rust and TS share one source of truth.
- Use tsify for data shapes, with handwritten TS wrappers for ergonomics.
- Validate packaging with publint and attw in CI. `isolatedDeclarations` keeps .d.ts generation fast and honest.

### Cited Findings
- **@libpdf/core** shows no error types or codes in its README. **pdf-lib** shows none either. — [@libpdf/core](https://github.com/LibPDF-js/core); [pdf-lib README](https://raw.githubusercontent.com/Hopding/pdf-lib/master/README.md)
- **[PK]** TypeScript 5.5 `isolatedDeclarations` requires explicit return and exported types, so that `.d.ts` can be emitted per file without type-checking (by oxc, swc, or tsc). JSR rewards explicit types with its "fast check" (slow types are penalised in the score). — [TS 5.5 notes](https://devblogs.microsoft.com/typescript/announcing-typescript-5-5/), [JSR slow types](https://jsr.io/docs/about-slow-types) (unverified this session)
- **[PK]** `publint` lints package.json `exports`. `@arethetypeswrong/cli` (attw) checks that types resolve under the node10/node16/bundler resolutions; an ESM-only package should use `attw --profile esm-only`. — [publint](https://publint.dev/), [attw](https://github.com/arethetypeswrong/arethetypeswrong.github.io) (unverified this session)
- **[PK]** Valibot and zod v4 expose `.issues` with typed `kind`/`code` discriminants. Effect models errors as tagged unions (`_tag`). — [valibot](https://valibot.dev/), [Effect error handling](https://effect.website/docs/error-management/expected-errors/) (unverified this session)

### Inferences — ideal type sketch
```ts
// units
declare const unit: unique symbol;
export type Pt = number & { readonly [unit]: "pt" };
export type Px = number & { readonly [unit]: "px" };
export type PageIndex = number & { readonly [unit]: "pageIndex" };   // 0-based
export const pt = (n: number) => n as Pt;
export const mm = (n: number) => (n * 72 / 25.4) as Pt;

// errors: codes generated from fixtures/codes.json at build time
export type ErrorCode = "PARSE" | "LIMIT_EXCEEDED" | "STRICT" | "ENCRYPTED" | "WRONG_PASSWORD" | "DISPOSED" | "UNSUPPORTED";
export class LaminaError<C extends ErrorCode = ErrorCode> extends Error {
  readonly code: C; readonly offset?: number; readonly objRef?: `${number} ${number} R`;   // template literal
}
export const isLaminaError = <C extends ErrorCode>(e: unknown, code?: C): e is LaminaError<C> => …;

// discriminated options
export type SaveOptions =
  | { mode: "full"; compress?: boolean; objectStreams?: boolean }
  | { mode: "incremental" };                     // incremental cannot take compress → type error

export type Warning = { code: WarningCode; message: string; offset?: number };

// const generics for form filling with known field names
declare function fill<const F extends Record<string, string | boolean>>(values: F): FillResult<keyof F & string>;
```
- No `any` in public `.d.ts`. Lint the emitted declarations with `@typescript-eslint/no-explicit-any` or a grep in CI.
- Use `satisfies` for internal config tables only. It doesn't appear in emitted types.
- Publish to both npm and JSR. JSR needs explicit types (same discipline as `isolatedDeclarations`). Wasm assets on JSR are unverified (see Gaps).

### Gaps
- JSR's handling of `.wasm` assets and export conditions (JSR doesn't support `exports` conditions in the npm sense) was not verified.
- drizzle and tRPC type structure was not researched.

## 7. Ergonomics benchmark (concrete snippets)

### Takeaway
- **pdf-lib:** a clean object API, but low level. Bottom-left origin, so you write `y: height - …`. Fonts must be embedded manually, and fontkit is required for anything beyond the 14 standard fonts. There is no text layout, and it is effectively unmaintained.
- **@libpdf/core:** close to pdf-lib in shape and adds the missing pieces: `PDF.merge`, `form.fill({...})`, `sign()`, encryption credentials, incremental save, lenient recovery.
- **PDF.js:** async and worker-centric, with heavy configuration.
- **unpdf:** functional helpers over PDF.js.

lamina should copy @libpdf/core's one-liners, add a top-left/unit-aware layout layer, and keep a sync core.

### Cited Findings
- **pdf-lib, create:** `import { PDFDocument, StandardFonts, rgb } from 'pdf-lib'`, then `const timesRomanFont = await pdfDoc.embedFont(StandardFonts.TimesRoman)` and `page.drawText(..., { y: height - 4 * fontSize, ... })` (bottom-left origin). — [pdf-lib README](https://raw.githubusercontent.com/Hopding/pdf-lib/master/README.md)
- **pdf-lib, fill form:** `const pdfDoc = await PDFDocument.load(formPdfBytes); const form = pdfDoc.getForm(); nameField.setText('Mario'); characterImageField.setImage(marioImage)`. — [pdf-lib README](https://raw.githubusercontent.com/Hopding/pdf-lib/master/README.md)
- **pdf-lib, merge** goes through copyPages: `const [firstDonorPage] = await pdfDoc.copyPages(firstDonorPdfDoc, [0]); pdfDoc.addPage(firstDonorPage)`. — [pdf-lib README](https://raw.githubusercontent.com/Hopding/pdf-lib/master/README.md)
- **pdf-lib, custom fonts** need `import fontkit from '@pdf-lib/fontkit'; pdfDoc.registerFontkit(fontkit)`. — [pdf-lib README](https://raw.githubusercontent.com/Hopding/pdf-lib/master/README.md)
- **pdf-lib maintenance:** the README's latest dated item is from 2021, and the Deno examples pin `pdf-lib@^1.11.1`. The README itself doesn't state a maintenance status. — [pdf-lib README](https://raw.githubusercontent.com/Hopding/pdf-lib/master/README.md)
- **@libpdf/core** (MIT; `src/fontbox/` is Apache-2.0, derived from PDFBox; Node 20+, Bun, browsers with WebCrypto):
  ```ts
  const pdf = await PDF.load(bytes);                        // or { credentials: "password" }
  const pdf = PDF.create(); const page = pdf.addPage({ size: "letter" });
  page.drawText("Hello, World!", { x: 50, y: 700, fontSize: 24, color: rgb(0,0,0) });
  const output = await pdf.save();
  pdf.getForm().fill({ name: "Jane Doe", email: "jane@example.com", agreed: true });
  const merged = await PDF.merge([pdf1Bytes, pdf2Bytes, pdf3Bytes]);
  const signer = await P12Signer.create(p12Bytes, "password");
  const signed = await pdf.sign({ signer, reason: "I approve this document" });
  ```
  - Incremental save is described as "changes appended, signatures preserved".
  - Lenient parsing falls back to a full-file scan.
  - The README shows no code for those two.

  — [@libpdf/core README](https://github.com/LibPDF-js/core)
- **unpdf:**
  ```ts
  const pdf = await getDocumentProxy(new Uint8Array(buffer));
  const { totalPages, text } = await extractText(pdf, { mergePages: true });
  const image = await renderPageAsImage(pdf, 1, { canvasImport: () => import('@napi-rs/canvas'), scale: 2, format: 'jpeg', quality: 0.8 });
  ```
  `extractImages` with `format: 'png'` needs no canvas. — [unpdf README](https://github.com/unjs/unpdf)
- **PDF.js, extract text (Node):**
  ```js
  const loadingTask = getDocument({ url: pdfPath }); const pdfDoc = await loadingTask.promise;
  const page = await pdfDoc.getPage(i);                       // 1-based
  const viewport = page.getViewport({ scale: 1.0 });
  const { items } = await page.getTextContent(); items.map(i => i.str).join(" ");
  page.cleanup(); await loadingTask.destroy();
  ```
  — [pdf.js node example](https://raw.githubusercontent.com/mozilla/pdf.js/master/examples/node/getinfo.mjs)
- **PDF.js, render:** set `GlobalWorkerOptions.workerSrc`, then `getDocument({ url })`, then `page.render(renderContext)`. The canvas is sized as viewport × `devicePixelRatio`. — [pdf.js helloworld](https://raw.githubusercontent.com/mozilla/pdf.js/master/examples/learning/helloworld.html)
- **[PK] jsPDF:** `const doc = new jsPDF(); doc.text("Hello world!", 10, 10); doc.save("a4.pdf");`. The API is imperative, with a top-left origin and units set in the constructor (`{ unit: "mm" }`), and `save()` triggers a browser download. — [jsPDF README](https://github.com/parallax/jsPDF) (unverified this session)

### Inferences — ideal @laminapdf/pdf API (sync after init)
```ts
import { init, Document, pt, mm } from "@laminapdf/pdf";
await init();                                          // once; idempotent; accepts { wasm } override

// create + text (top-left origin by default, units explicit; layout box with wrapping)
using doc = Document.create();
const page = doc.addPage({ size: "A4" });
const font = doc.fonts.embed(interBytes);              // subsetting automatic; Standard 14 via doc.fonts.standard("Helvetica")
page.text("Hello, lamina", { at: [mm(20), mm(20)], font, size: pt(24) });
page.textBox(longText, { box: { x: mm(20), y: mm(40), w: mm(170) }, font, size: pt(11), align: "justify" });
const bytes: Uint8Array = doc.save({ mode: "full" });

// open leniently, inspect warnings
using src = Document.open(input, { limits: { maxObjects: 1e6 }, strict: false, password });
for (const w of src.warnings) console.warn(w.code, w.offset);

// fill form
src.form.fill({ name: "Jane", agreed: true });          // unknown field → LaminaError<"FORM_FIELD_NOT_FOUND">
const filled = src.save({ mode: "incremental" });

// merge
const merged = Document.merge([a, b, c]);               // bytes in, bytes out
// extract text
import { extractText } from "@laminapdf/pdf/extract";
const { pages } = extractText(src, { pages: [0, 1] as PageIndex[] });
// render (worker)
const bmp = await (await createWorkerClient()).render(bytes, { page: 0, scale: 2 });
```
- `pdf-lib` users commonly trip on bottom-left `y`. Default to top-left in the high-level API, and keep `page.raw` (content-stream ops in PDF user space) for spec-faithful work. This is an opinion.

### Gaps
- @libpdf/core's error model, its incremental-save options and its full API docs site were not fetched.
- jsPDF snippets are from prior knowledge.

## 8. Testing across runtimes

### Takeaway
Use Vitest for unit tests in Node. Run the same suite in real browsers with Vitest Browser Mode (Playwright provider) and in workerd with `@cloudflare/vitest-pool-workers`. Add tiny smoke scripts for `deno run` and `bun` that import the *packed tarball* (`pnpm pack`), so the export conditions get tested, not source paths.

### Cited Findings
- unpdf and jSquash keep per-runtime example projects (Rollup, Vite, Webpack, Deno, Cloudflare Workers) as integration checks. — [jSquash README](https://github.com/jamsinclair/jSquash); [unpdf README](https://github.com/unjs/unpdf)
- **[PK]** Vitest Browser Mode (stable in Vitest 4) runs tests in real Chromium, Firefox and WebKit via Playwright. `@cloudflare/vitest-pool-workers` runs tests inside workerd/Miniflare. It is configured via `defineWorkersConfig` in older versions, and as a `cloudflareTest` plugin in newer ones (not verified). — [Vitest browser mode](https://vitest.dev/guide/browser/), [Cloudflare Vitest integration](https://developers.cloudflare.com/workers/testing/vitest-integration/) (unverified; the README fetch returned 404)

### Inferences
- **CI matrix:**
  - node 20/22/24: `node` condition, `fs` loader.
  - Chromium/Firefox/WebKit via Vitest browser: `browser` condition, `fetch` loader.
  - workerd via vitest-pool-workers: `workerd` condition, static `.wasm` import.
  - Deno and Bun smoke tests.
  - Bundler fixture apps (Vite, webpack, esbuild, Next.js), each building a tiny page that calls `init()` + `Document.create().save()`. Assert the built output contains the `.wasm` asset.
- Run `publint` and `attw --pack` as part of `cargo xtask check ts`.

### Gaps
- Exact current config API of `@cloudflare/vitest-pool-workers` and its supported Vitest versions (docs blocked, README 404).
- Bun's `bun test` compatibility with wasm-bindgen output is unverified.
