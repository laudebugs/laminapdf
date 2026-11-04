# `@laminapdf/sdk`

Create, read, edit and save PDFs from TypeScript. Lamina PDF's Rust core runs as WebAssembly, so after one `await init()` the API is synchronous and the same in browsers, Node, Deno, Bun and Cloudflare Workers.

```ts
import { Document, init } from "@laminapdf/sdk";

await init();
const doc = Document.open(bytes);
console.log(doc.pageCount);
```

The package is ESM only. It's on npm as an alpha release: `npm install @laminapdf/sdk@alpha`.

## Quick start

```ts
import { Builder, Document, Font, init } from "@laminapdf/sdk";

await init({ text: true }); // the base module, and the text module for fonts and text

// Generate
const b = new Builder({ producer: "my app" });
const page = b.page("A4");
page.surface().text("Hello, world!", { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
page.finish();
const bytes: Uint8Array = b.finish().save();

// Read and edit
const doc = Document.open(bytes);
doc.transact((tx) => tx.setInfo({ title: "Hello" })); // synchronous: collected ops are applied as one transaction
const edited = doc.save({ mode: "incremental" }); // the original bytes, then one appended revision
doc.free();
```

Coordinates are in page space: points from the top-left of the displayed page, with y going down. A text run's `at[1]` is its baseline. Page indexes start at 0.

## Two modules

| Module | gzip | What it can do |
| --- | ---: | --- |
| base (`init()`) | ≈ 373 KB | Open (passwords, limits, strict mode), read pages and Info, warnings, edit with ops (transactions, undo/redo), save (auto, incremental, full rewrite; save jobs); build vector and image pages with links, outline, page labels, metadata |
| extract (`init({ extract: true })` or `loadExtract()`) | ≈ 526 KB | Everything in base, plus text extraction (`page.text()`) |
| text (`init({ text: true })` or `loadText()`) | ≈ 1,009 KB | Everything in base, plus fonts (`Font.fromBytes`, the standard 14), shaped text runs and text boxes, and text extraction |

A page that never draws text never downloads the text module. A `Builder` is built in the text module when it is loaded at the time the builder is created (or with `new Builder({ text: true })`); drawing text on a base-module builder throws `text-module-not-loaded`.

## Loading per runtime

`package.json` `exports` resolves one entry per runtime, each with its own loader (no environment sniffing):

| Condition | Entry | How the `.wasm` is loaded |
| --- | --- | --- |
| `workerd` | `dist/entry/workerd.js` | `import wasm from "@laminapdf/wasm/lamina_pdf_base.wasm"`: a `WebAssembly.Module` bundled with the Worker (both modules) |
| `deno`, `browser`, `default` | `dist/index.js` | `new URL("lamina_pdf_base.wasm", import.meta.url)` in the package's loader, fetched and compiled streaming; bundlers emit it as an asset |
| `node` (Node, Bun) | `dist/entry/node.js` | the file of `@laminapdf/wasm`, read as bytes |

The escape hatch is the input of `init`: a URL or string, a `Response` (or a promise of one), bytes (`ArrayBuffer`, `Uint8Array`) or a compiled `WebAssembly.Module`, for the base module (`init(source)` or `init({ module })`) and the text module (`init({ text: source })` or `loadText(source)`). `init` is idempotent and a failed load can be retried; naming a different source after a module is loaded (or loading) throws `already-initialized`.

**Vite:** exclude the package from dependency pre-bundling, so the `.wasm` files stay next to the JavaScript that loads them:

```ts
// vite.config.ts
export default { optimizeDeps: { exclude: ["@laminapdf/sdk"] } };
```

## Passwords, fonts and images

- A password is tried as given, then (after `wrong-password`) in its NFKC form, so older passwords reach the core unchanged and AES-256 ones match SASLprep's normalisation.
- Equal fonts are one PDF font per builder: standard 14 fonts by name, others by their bytes, however many `Font` objects name them.
- `Image` copies its input and decodes it in the module of first use; bad data surfaces there (or when `width`/`height` is read).

## Tagged PDF

`new Builder({ tagged: true })` tags a document before anything is drawn; a page that uses `tag` or `artifact` tags it either way. Structure is drawn with callbacks, as paths and transactions are:

```ts
s.artifact("pagination", (s) => s.text("Page 1", { font, size: 9, at: [520, 810] }));
s.text("Invoice 42", { font, size: 24, at: [56, 80], tag: "H1" });
s.tag("Table", (s) => {
  s.tag("TR", (s) => {
    s.text("Item", { font, size: 11, at: [56, 160], tag: { type: "TH", scope: "column" } });
  });
});
s.image(logo, [440, 56, 100, 40], "The Acme logo"); // a Figure with /Alt
s.text("Pay online", { font, size: 11, at: [56, 320], link: { target, contents: "Pay invoice 42 online" }, tag: "P" });
```

`tag` takes a standard type (`TagName`) or a `TagSpec` (`{ type, role, alt, actualText, lang, expansion, scope }`; a custom `type` needs the standard `role` it maps to); text and text boxes take a `tag` option; a link inside an element becomes a `Link` element holding its annotation. Elements and artifacts the callback leaves open end with it, also when it throws. Elements are per page.

## Errors and warnings

Every error is a `LaminaPdfError`. Its fields are `code` (one of the `ErrorCode` union), `kind`, `message`, `hint`, `didYouMean`, `location`, `data` (the only place document text appears) and `docsUrl`, a link to the code's page in the [error reference](https://docs.laminapdf.com/reference/codes/). `toJSON()` gives the plain object back.

```ts
try {
  Document.open(bytes, { password });
} catch (e) {
  if (isLaminaPdfError(e, "wrong-password")) askAgain();
  else throw e;
}
```

`isLaminaPdfError(e, code)` narrows: inside the `if`, `e.code` has the type `"wrong-password"`. `LaminaPdfError<C>` is generic over the `ErrorCode` union, so a `switch (e.code)` with a `never` default is checked for exhaustiveness, and stops compiling when a new code arrives until you handle it. `toJSON()` validates against the `laminapdf.error/1` JSON Schema, shipped as `@laminapdf/sdk/schemas/error.schema.json`.

`doc.warnings` lists what the reader repaired, as `LaminaPdfWarning` objects of the same shape (without `didYouMean`), typed by `WarningCode`.

`@laminapdf/sdk/explain` is a separate entry, kept out of the main bundle: `explain(code)` returns the code's page as Markdown (what happened, why, how to fix, an example, the `data` fields), with `KINDS`, `HINTS` and `docsUrl(code)` for every code.

```ts
import { explain } from "@laminapdf/sdk/explain";
if (isLaminaPdfError(e)) console.error(e.message, e.hint, explain(e.code));
```

## Extracting text

`page.text()` returns a page's text in four forms:

- `lines`: the lines in content order, trimmed.
- `text()`: the lines joined with `\n`.
- `runs`: font, size, direction, marked-content ID, `ActualText`, visibility and bounds; `glyphs()` gives each glyph's text and quad.
- `buffers`: every glyph's quad in one `Float32Array` (8 numbers per glyph, in page space) and the glyph text with UTF-16 offsets, ready to transfer from a Web Worker.

Documents open in the richest module loaded: text, then extract, then base. Load the extract or text module before you open or build a document whose text you need; otherwise `page.text()` throws `extract-module-not-loaded`. `doc.canExtractText` tells which module a document is in.

The text module also extracts text. Once it is loaded, Lamina PDF doesn't load the extract module, and documents live in the text module's memory. In a Cloudflare Worker, pass the module yourself: then only a Cloudflare Worker that extracts text bundles it: `import extract from "@laminapdf/wasm/lamina_pdf_extract.wasm"; await loadExtract(extract);`.

```ts
await init({ extract: true });
const doc = Document.open(bytes);
const text = doc.page(0).text({ skipInvisible: true });
console.log(text.text());
```

## Remote and large files

`Document.openRange(source, options)` opens a file whose bytes are fetched range by range, so a remote PDF opens without downloading it whole. Only what the open needs is fetched: the end of the file (the xref), the catalog, the page tree. The document stays synchronous; a call that needs bytes not fetched yet throws `data-pending` with `e.ranges`, and `await doc.ensure(() => …)` fetches them and calls again.

```ts
const doc = await Document.openRange(httpRangeSource("https://example.com/big.pdf"));
const pages = await doc.ensure(() => doc.pages);
const bytes = await doc.ensure(() => doc.save({ mode: "fullRewrite" })); // fetches the rest
```

- **`httpRangeSource(url, { headers, credentials, signal, blockSize })`:** the first request asks for the last 64 KiB with `Range: bytes=-65536`, which also gives the size. Later requests are aligned to 16 KiB blocks and send `If-Range` with the first strong `ETag` (else `Last-Modified`); a different `ETag`, `Last-Modified` or size fails with `source-changed`. A server without either validator can't be checked: serve one. A server that answers `200` (no range support) is read from that one full download. HTTP errors are `fetch-failed` with `data.status`; the URL never appears in an error.
- **CORS:** a `Range` header makes a cross-origin request preflighted. The server must answer with `Access-Control-Allow-Headers: Range` and expose `Content-Range`, `Content-Length` and `ETag` (`Access-Control-Expose-Headers`). It must not compress the response: `Content-Encoding` breaks byte ranges.
- **`callbackSource(size, read)`:** any other random-access input (a `Blob`'s `slice`, a file handle, an IPC call).
- **Options:** `signal` (for the open; aborting rejects and cancels its requests in flight; `ensure(f, { signal })` takes its own), `blockSize`, `readahead`, `budget` (the most bytes the document may hold; more is `source-budget-exceeded`) and `maxRounds`.
- **Off the main thread:** each `ensure` call can take several round trips. `client.openUrl` in a Web Worker hides the loop behind its async API (see below).
- **Large local files in Rust:** the `laminapdf-source` crate's `FileSource` (positioned reads) or, for files nothing changes, `MmapSource`; its rustdoc describes both.

## Off the main thread: `@laminapdf/sdk/worker`

The API is synchronous, so a large file opened on the main thread blocks the page. `@laminapdf/sdk/worker` runs the same API in a Web Worker and gives you async twins of `Document` and its pages.

```ts
import { connect } from "@laminapdf/sdk/worker";

const client = await connect(); // starts the Web Worker, checks its version, calls init() there
await using doc = await client.open(bytes); // copied; { transfer: true } moves it instead
console.log(await doc.pageCount());
const page = await doc.page(0);
const text = await page.text(); // a PageText, rebuilt from the transferred arrays (after client.loadExtract())
const updated = await doc.save({ mode: "incremental" }, { signal, onProgress: (p) => bar(p) });
```

- **Same names, async:** every served member of `Document` and `PageRef` is a method of the same name returning a `Promise`. Getters become methods: `await doc.pageCount()`. A page is a remote handle too.
- **Not served:**
  - `transact`: a callback can't cross to the Web Worker. Use `apply(ops)` instead.
  - `saveJob`: the remote `save` is a stepped job already.
  - `ensure`: the Web Worker fetches a range document's bytes itself.
  - `free`: use `close()` or `await using`.
  - `Builder`: its callbacks (`path`, `tag`) can't cross either. Build on the main thread or in your own Web Worker.

- **Remote files:** `client.openUrl(url, options)` opens a file by range requests in the Web Worker; every call fetches what it needs first. Its options are `httpRangeSource`'s, without a custom `fetch` (a function can't be sent to the Web Worker), and `openRange`'s (`budget`, `readahead`, `maxRounds`). Aborting the call's `signal` also cancels the fetches in flight.
- **Cancellation:** pass `signal` to any call. An aborted call rejects at once with `LaminaPdfError` `cancelled`, whose `cause` is the signal's reason, and a late answer is dropped.
  - **Not started yet:** the call never runs.
  - **`save` / `saveWithReport`:** they run as stepped jobs and stop at the next step. The document stays usable.
  - **Every other call:** one synchronous call in the Web Worker, so it can't be stopped midway (`open` included). `client.terminate()` is the hard stop.
- **Transfers:** `save()` bytes and page text arrays come back transferred. Inputs are copied unless you pass `transfer: true`, which detaches your buffer.
- **Errors:** errors are `LaminaPdfError` on the main thread, with every field. If the Web Worker crashes (an uncaught error, a WebAssembly trap, out of memory), pending calls reject with `worker-crashed` and `client.on("crashed", …)` fires. `client.restart()` starts a new Web Worker. Calls made while it starts wait for it, and the old Web Worker's handles reject with `disposed` (their `close()` does nothing). `terminate()` doesn't fire `crashed`. There's no heartbeat, so a long call is not a crash.
- **Unsendable parameters:** a parameter the Web Worker can't receive (a function) rejects with `invalid-argument`, as do unknown methods. With `transfer: true`, only a buffer the input covers whole moves; a subarray or a `SharedArrayBuffer` view is copied. An error inside the Web Worker that isn't Lamina PDF's is `worker-failed` (internal: a bug to report).
- **`init`:** `connect({ init })` takes what can be sent to a Web Worker: a URL (string or `URL`, resolved against the page), bytes or a `WebAssembly.Module`, alone or as `{ module, text, extract }`. A `Response` can't cross.
- **Version handshake:** on connect, the Web Worker and the client compare their protocol and package versions. A Web Worker from another build rejects `connect()` with `worker-version-mismatch`.
- **Bundlers:** `connect()` contains the literal `new Worker(new URL("./entry.js", import.meta.url), { type: "module" })`, which Vite and webpack bundle with no config. In Vite, keep the package out of pre-bundling (`optimizeDeps: { exclude: ["@laminapdf/sdk"] }`), as for the main entry.
- **CSP or your own hosting:** pass `connect({ worker: () => new Worker(url, { type: "module" }) })`. A factory, so that `restart()` can make a new one.
- **Runtimes:** browsers (Chromium, Firefox, WebKit), and Deno and Bun Web Workers (smoke-tested). Node's `worker_threads` isn't a web `Worker` and isn't supported.
- **Extending the Web Worker:** `@laminapdf/sdk/worker/server`'s `serve({ methods })` serves the same protocol with more methods in the same Web Worker. `@laminapdf/viewer` adds its render methods this way.
  - The page calls them with `client.request(method, params, { priority, key, signal })`.
  - The Web Worker runs the highest `priority` first, and runs calls with the same `key` once. Each caller still gets its own answer: an extension method's `reply(result)` builds one per caller, so each can be given its own transferables.
  - `emit(name, data)` in the Web Worker reaches `client.on("event", …)`.
  - `connect({ onDiscard })` sees the answers of calls already rejected. By default it closes the `ImageBitmap`s in them.
  - `serve({ methods, saveBudget })` sets how many work units each step of a save takes (20,000 by default). A smaller budget answers a cancel sooner and reports progress more often, at some cost in speed.
  - `workerStats()` in the Web Worker reports the calls started, the handles the page holds and the calls queued. A handle the page never frees keeps its document's WebAssembly memory alive, so a growing count finds the `RemoteDocument` an app forgot to close.
  - `Emitter`, from `@laminapdf/sdk/worker`, is the typed event emitter the client uses, for a package that wraps it: `on(event, listener)` returns a function that stops listening, and a listener that throws doesn't stop the others.

## Memory

Documents, builders, pages and images live in WebAssembly memory. `doc.free()` (or `using doc = Document.open(…)`) frees a document at once; `page.finish()` and `builder.finish()` free their own side; the garbage collector frees anything you don't release, through a `FinalizationRegistry`. `wasmMemory()` reports each module's memory size.

## Examples

The [examples](https://docs.laminapdf.com/examples/) are `hello`, `invoice`, `multilingual`, `images-and-vectors`, `links-and-outlines` and `generate-then-edit`, plus `report`, 100 pages with two embedded fonts. Each produces the same bytes as its Rust twin.

A Cloudflare Worker example generates a PDF per request: `/?name=Ada` returns a letter in Helvetica, and `/embedded?name=Ada` the same letter in a font bundled with the Cloudflare Worker. A name the font can't draw gets a 400 with the error as JSON. The [Cloudflare Workers quick start](https://docs.laminapdf.com/quick-start/workers/) shows it and how to run it.

## Working on this package

How the package is built and tested in every runtime is in `CONTRIBUTING.md`, next to this README in the repository.
