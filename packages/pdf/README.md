# `@lamina/pdf`

Create, read, edit and save PDFs from TypeScript. lamina's Rust core runs as WebAssembly ([`@lamina/wasm`](../wasm)); after one `await init()`, the API is synchronous and the same in browsers, Node, Deno, Bun and Cloudflare Workers ([ADR 0016](../../docs/decisions/0016-package-design.md)). ESM only, private until the first release (P1/P2).

## Quick start

```ts
import { Builder, Document, Font, init } from "@lamina/pdf";

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

Coordinates are page space: points from the top-left of the displayed page, y down; a text run's `at[1]` is its baseline ([ADR 0027](../../docs/decisions/0027-coordinates-units-and-page-indexing.md)). Page indices are 0-based.

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
| `workerd` | `dist/entry/workerd.js` | `import wasm from "@lamina/wasm/lamina_base.wasm"`: a `WebAssembly.Module` bundled with the Worker (both modules) |
| `deno`, `browser`, `default` | `dist/index.js` | the glue's `new URL("lamina_base.wasm", import.meta.url)`, fetched and compiled streaming; bundlers emit it as an asset |
| `node` (Node, Bun) | `dist/entry/node.js` | the file of `@lamina/wasm`, read as bytes |

The escape hatch is the input of `init`: a URL or string, a `Response` (or a promise of one), bytes (`ArrayBuffer`, `Uint8Array`) or a compiled `WebAssembly.Module`, for the base module (`init(source)` or `init({ module })`) and the text module (`init({ text: source })` or `loadText(source)`). `init` is idempotent and a failed load can be retried; naming a different source after a module is loaded (or loading) throws `already-initialized`.

**Vite:** exclude the package from dependency pre-bundling, so the `.wasm` URLs stay next to the glue:

```ts
// vite.config.ts
export default { optimizeDeps: { exclude: ["@lamina/pdf"] } };
```

## Passwords, fonts and images

- A password is tried as given, then (after `wrong-password`) in its NFKC form, so R2–R4 passwords reach the core unchanged and R5/R6 ones match SASLprep's normalisation ([ADR 0016](../../docs/decisions/0016-package-design.md) §7).
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

Every error is a `LaminaError` with the [ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md) §1 fields: `code` (from [`fixtures/codes.json`](../../fixtures/codes.json); the `ErrorCode` union is generated from it by `cargo xtask gen codes`), `kind`, `message`, `hint`, `didYouMean`, `location`, `data` (the only place document text appears) and `docsUrl` (the code's page in [`docs/errors/`](../../docs/errors/README.md)). `toJSON()` gives the plain object back.

```ts
try {
  Document.open(bytes, { password });
} catch (e) {
  if (isLaminaError(e, "wrong-password")) askAgain();
  else throw e;
}
```

`isLaminaError(e, code)` narrows: inside the `if`, `e.code` has the type `"wrong-password"`. `LaminaError<C>` is generic over the `ErrorCode` union, so a `switch (e.code)` with a `never` default is checked for exhaustiveness, and stops compiling when a new code arrives until you handle it. `toJSON()` validates against the `lamina.error/1` JSON Schema, shipped as `@lamina/pdf/schemas/error.schema.json`.

`doc.warnings` lists what the reader repaired, as `LaminaWarning` objects of the same shape (without `didYouMean`), typed by `WarningCode`.

`@lamina/pdf/explain` is a separate entry, kept out of the main bundle: `explain(code)` returns the code's page as Markdown (what happened, why, how to fix, an example, the `data` fields), with `KINDS`, `HINTS` and `docsUrl(code)` for every code.

```ts
import { explain } from "@lamina/pdf/explain";
if (isLaminaError(e)) console.error(e.message, e.hint, explain(e.code));
```

## Extracting text

`page.text()` returns a page's text: `lines` (in content order, trimmed), `text()` (the lines joined with `\n`), `runs` (font, size, direction, marked-content ID, `ActualText`, visibility, bounds; `glyphs()` gives each glyph's text and quad) and `buffers` (every glyph's quad in one `Float32Array`, 8 numbers per glyph in ADR 0027 page space, and the glyph text with UTF-16 offsets, to transfer from a worker). Documents open in the richest module loaded (text, then extract, then base): load the extract or text module before opening (or building) a document whose text you need, or `page.text()` throws `extract-module-not-loaded`; `doc.canExtractText` tells which module a document is in. With the text module loaded, the extract module isn't loaded (the text module extracts too), and documents live in the text module's memory. In Cloudflare Workers, pass the module yourself so other Workers don't bundle it: `import extract from "@lamina/wasm/lamina_extract.wasm"; await loadExtract(extract);`.

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
- **Workers:** each `ensure` call can take several round trips. The worker client (`client.openUrl`, [L2a](../../docs/todos/done/L2a-worker-client.md)) hides the loop behind its async API.
- **Large local files in Rust:** `lamina-source`'s `FileSource` (positioned reads) or, for files nothing changes, `MmapSource` ([its README](../../crates/lamina-source/README.md)).

## Off the main thread: `@lamina/pdf/worker`

The API is synchronous, so a large file opened on the main thread blocks the page. `@lamina/pdf/worker` runs the same API in a Web Worker and gives you async twins of `Document` and its pages.

```ts
import { connect } from "@lamina/pdf/worker";

const client = await connect(); // starts the worker, checks its version, calls init() there
await using doc = await client.open(bytes); // copied; { transfer: true } moves it instead
console.log(await doc.pageCount());
const page = await doc.page(0);
const text = await page.text(); // a PageText, rebuilt from the transferred arrays (after client.loadExtract())
const updated = await doc.save({ mode: "incremental" }, { signal, onProgress: (p) => bar(p) });
```

- **Same names, async:** every served member of `Document` and `PageRef` is a method of the same name returning a `Promise`. Getters become methods: `await doc.pageCount()`. A page is a remote handle too.
- **Not served:**
  - `transact`: a callback can't cross to a worker. Use `apply(ops)` instead.
  - `saveJob`: the remote `save` is a stepped job already.
  - `ensure`: the worker fetches a range document's bytes itself.
  - `free`: use `close()` or `await using`.
  - `Builder`: its callbacks (`path`, `tag`) can't cross either. Build on the main thread or in your own worker.

  `tsc` fails if a new sync method is neither served nor excluded (`src/worker/support.ts`).
- **Remote files:** `client.openUrl(url, options)` opens a file by range requests in the worker; every call fetches what it needs first. Its options are `httpRangeSource`'s, without a custom `fetch` (a function can't be sent to the worker).
- **Cancellation:** pass `signal` to any call. An aborted call rejects at once with `LaminaError` `cancelled`, whose `cause` is the signal's reason, and a late answer is dropped.
  - **Not started yet:** the call never runs.
  - **`save` / `saveWithReport`:** they run as stepped jobs and stop at the next step. The document stays usable.
  - **Every other call:** one synchronous call in the worker, so it can't be stopped midway (`open` included). `client.terminate()` is the hard stop.
- **Transfers:** `save()` bytes and page text arrays come back transferred. Inputs are copied unless you pass `transfer: true`, which detaches your buffer.
- **Errors:** errors are `LaminaError` on the main thread, with every ADR 0026 field. If the worker crashes (an uncaught error, a WebAssembly trap, out of memory), pending calls reject with `worker-crashed` and `client.on("crashed", …)` fires. `client.restart()` starts a new worker. Calls made while it starts wait for it, and the old worker's handles reject with `disposed` (their `close()` does nothing). `terminate()` doesn't fire `crashed`. There's no heartbeat, so a long call is not a crash.
- **Unsendable parameters:** a parameter the worker can't receive (a function, an untransferable buffer with `transfer: true`) rejects with `invalid-argument`, as do unknown methods.
- **Version handshake:** on connect, the worker and the client compare their protocol and package versions. A worker from another build rejects `connect()` with `worker-version-mismatch` (PDF.js's version-mismatch lesson).
- **Bundlers:** `connect()` contains the literal `new Worker(new URL("./entry.js", import.meta.url), { type: "module" })`, which Vite and webpack bundle with no config (`test/browser/vite-build.test.ts` builds and runs one). In Vite, keep the package out of pre-bundling (`optimizeDeps: { exclude: ["@lamina/pdf"] }`), as for the main entry.
- **CSP or your own hosting:** pass `connect({ worker: () => new Worker(url, { type: "module" }) })`. A factory, so that `restart()` can make a new one.
- **Runtimes:** browsers (Chromium, Firefox, WebKit), and Deno and Bun web Workers (smoke-tested). Node's `worker_threads` isn't a web `Worker` and isn't supported.

## Memory

Documents, builders, pages and images live in WebAssembly memory. `doc.free()` (or `using doc = Document.open(…)`) frees a document at once; `page.finish()` and `builder.finish()` free their own side; unreleased objects are freed by the glue's `FinalizationRegistry` when collected. `wasmMemory()` reports each module's memory size. The disposed-handle error and the other memory bounds are [L2b](../../docs/todos/ready/L2b-disposal-and-memory.md).

## Examples

[`examples/`](examples) holds the TypeScript twins of the Rust examples (`hello`, `invoice`, `multilingual`, `images-and-vectors`, `links-and-outlines`, `generate-then-edit`), each producing the bytes of its Rust golden in [`fixtures/golden/examples/`](../../fixtures/golden/examples), and `report` (100 pages, two embedded fonts: the performance case of [pdf-library §4](../../docs/plans/pdf-library.md#non-functional), checked by its SHA-256 instead of a golden).

[`examples/worker/`](examples/worker) is a Cloudflare Worker that generates a PDF per request (`/?name=Ada`: a letter in Helvetica, or a 400 with the error as JSON for a name it can't draw; `/report`: the report). From this folder:

```sh
pnpm exec wrangler dev -c examples/worker/wrangler.toml   # then open http://localhost:8787/?name=Ada
```

## Runtimes

The one example list, [`test/suite/examples.ts`](test/suite/examples.ts) (no Node API, no test framework; `runSuite(runtime, read, register)` registers it with a runner's test function), runs in every runtime the package supports, each comparing every example with its golden byte for byte and the report with its hash ([B6c](../../docs/todos/done/B6c-runtime-matrix.md)):

| Runtime | Runner | Export condition |
| --- | --- | --- |
| Node 22, 24, 26 | Vitest, [`vitest.node.config.ts`](vitest.node.config.ts): project `node` (`test/*.test.ts`, also the Vite tests; what `cargo xtask wasm` runs) and project `measure` ([`test/node/measure.test.ts`](test/node/measure.test.ts)) | `node` |
| Chromium, Firefox, WebKit | Vitest browser mode with Playwright, [`vitest.browser.config.ts`](vitest.browser.config.ts) (`test/browser/`; fixtures fetched as Vite assets); its `vite-build` project runs a Vite production build of [`test/vite-app/`](test/vite-app) in a Chromium page | `browser` |
| Deno | `deno test` over [`test/deno/`](test/deno) | `deno` |
| Bun | `bun test` over [`test/bun/`](test/bun) | `node` |
| workerd | Vitest in Node, [`vitest.workers.config.ts`](vitest.workers.config.ts): wrangler's test harness runs [`test/workerd/worker.ts`](test/workerd/worker.ts) and the Worker example in workerd, and the test compares their responses | `workerd` |

Each also prints the performance smoke's figures, measured first in a fresh module instance (Node: its own file; elsewhere the first test of the file, or the first request to a fresh Worker), so the first hello is cold and the memory is the hello and report runs' alone: hello after `init()` (first call and median of 20), the report (median of 3) and the WebAssembly memory after it. They are recorded, never asserted: `cargo xtask pdf-runtimes` collects them into a table against the §4 targets (≤ 5 ms, ≤ 500 ms, < 64 MB in a Worker; the job summary in CI) and `target/xtask/pdf-runtimes/measurements.json`; the nightly lane appends them to the benchmark history.

```sh
cargo xtask wasm                          # first: builds packages/wasm/dist/ and this package's dist/
cargo xtask pdf-runtimes                  # every runtime, in turn
cargo xtask pdf-runtimes chromium deno    # some of them (node, chromium, firefox, webkit, deno, bun, workerd)
```

`pdf-runtimes node` uses the `node` on `PATH`. Deno, Bun, wrangler (with workerd) and Playwright are dev dependencies of this package, so `pnpm install` brings them; `pdf-runtimes` runs `playwright install <browser>` before a browser (into Playwright's per-user cache, or `PLAYWRIGHT_BROWSERS_PATH`). By hand, from this folder:

```sh
pnpm exec vitest run -c vitest.node.config.ts --project measure   # then --project node
pnpm exec vitest run -c vitest.browser.config.ts --project chromium --project vite-build   # or firefox, webkit
pnpm exec deno test --sloppy-imports --no-check --allow-read test/deno/
pnpm exec bun test ./test/bun/
pnpm exec vitest run -c vitest.workers.config.ts
```

CI: the fast lane runs Node 22 and 26 and Chromium on every PR (Node 24, `.nvmrc`, is the `wasm` job's own run), the full lane Firefox, WebKit, Deno, Bun and workerd, and the nightly lane all of them with the figures recorded.

## Development

```sh
cargo xtask wasm   # builds @lamina/wasm, then this package (tsc), its tests (Vitest: Node, Vite), publint, attw --pack
cargo xtask pdf-runtimes   # the example suite in every runtime (see Runtimes)
cargo xtask size   # includes this package's JS (glue + API, min+gzip) against its 20 KB budget
cargo xtask gen codes   # regenerate src/generated/ and schemas/ from fixtures/codes.json (`check docs` fails when stale)
```
