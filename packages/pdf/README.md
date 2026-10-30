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
doc.transact((tx) => tx.setInfo({ title: "Hello" }));
const edited = doc.save({ mode: "incremental" }); // the original bytes, then one appended revision
doc.free();
```

Coordinates are page space: points from the top-left of the displayed page, y down; a text run's `at[1]` is its baseline ([ADR 0027](../../docs/decisions/0027-coordinates-units-and-page-indexing.md)). Page indices are 0-based.

## Two modules

| Module | gzip | What it can do |
| --- | ---: | --- |
| base (`init()`) | ≈ 373 KB | Open (passwords, limits, strict mode), read pages and Info, warnings, edit with ops (transactions, undo/redo), save (auto, incremental, full rewrite; save jobs); build vector and image pages with links, outline, page labels, metadata |
| text (`init({ text: true })` or `loadText()`) | ≈ 944 KB | Everything in base, plus fonts (`Font.fromBytes`, the standard 14), shaped text runs and text boxes |

A page that never draws text never downloads the text module. A `Builder` is built in the text module when it is loaded at the time the builder is created (or with `new Builder({ text: true })`); drawing text on a base-module builder throws `text-module-not-loaded`.

## Loading per runtime

`package.json` `exports` resolves one entry per runtime, each with its own loader (no environment sniffing):

| Condition | Entry | How the `.wasm` is loaded |
| --- | --- | --- |
| `workerd` | `dist/entry/workerd.js` | `import wasm from "@lamina/wasm/lamina_base.wasm"`: a `WebAssembly.Module` bundled with the Worker (both modules) |
| `deno`, `browser`, `default` | `dist/index.js` | the glue's `new URL("lamina_base.wasm", import.meta.url)`, fetched and compiled streaming; bundlers emit it as an asset |
| `node` (Node, Bun) | `dist/entry/node.js` | the file of `@lamina/wasm`, read as bytes |

The escape hatch is the input of `init`: a URL or string, a `Response` (or a promise of one), bytes (`ArrayBuffer`, `Uint8Array`) or a compiled `WebAssembly.Module`, for the base module (`init(source)` or `init({ module })`) and the text module (`init({ text: source })` or `loadText(source)`). `init` is idempotent and a failed load can be retried.

**Vite:** exclude the package from dependency pre-bundling, so the `.wasm` URLs stay next to the glue:

```ts
// vite.config.ts
export default { optimizeDeps: { exclude: ["@lamina/pdf"] } };
```

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

`doc.warnings` lists what the reader repaired, as `LaminaWarning` objects of the same shape.

## Memory

Documents, builders, pages and images live in WebAssembly memory. `doc.free()` (or `using doc = Document.open(…)`) frees a document at once; `page.finish()` and `builder.finish()` free their own side; unreleased objects are freed by the glue's `FinalizationRegistry` when collected. `wasmMemory()` reports each module's memory size. The disposed-handle error and the other memory bounds are [L2b](../../docs/todos/ready/L2b-disposal-and-memory.md).

## Examples

[`examples/`](examples) holds the TypeScript twins of the Rust examples (`hello`, `invoice`, `multilingual`, `images-and-vectors`, `links-and-outlines`, `generate-then-edit`); `test/examples.test.ts` checks that each produces the bytes of its Rust golden in [`fixtures/golden/examples/`](../../fixtures/golden/examples).

## Development

```sh
cargo xtask wasm   # builds @lamina/wasm, then this package (tsc), its tests (Vitest: Node, Vite), publint, attw --pack
cargo xtask size   # includes this package's JS (glue + API, min+gzip) against its 20 KB budget
cargo xtask gen codes   # regenerate src/codes.generated.ts from fixtures/codes.json (`check ts` fails when stale)
```
