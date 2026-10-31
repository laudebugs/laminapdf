# @laminapdf/viewer

The web reader's viewer core ([ADR 0009](../../docs/decisions/0009-web-client-architecture.md)): framework-free, with React chrome around it later. Part 1 ([C3a](../../docs/todos/done/C3a-viewer-worker-rpc.md)) is the **document worker** and its **typed RPC**. The virtualiser, tiles and zoom are [C3b](../../docs/todos/ready/C3b-viewer-virtualiser-and-zoom.md). Private, ESM-only.

```ts
import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { ViewerClient } from "@laminapdf/viewer";

const viewer = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
const doc = await viewer.open(file); // a File/Blob, or bytes
const { bitmap } = await viewer.render({ page: 0, scale: devicePixelRatio }, { priority: "visible", signal });
ctx.drawImage(bitmap, 0, 0);
bitmap.close();
```

## Architecture

```text
 main thread                                   document worker (one per open document)
┌────────────────────────────┐   postMessage   ┌──────────────────────────────────────────┐
│ ViewerClient                │ ─────────────▶ │ @laminapdf/sdk/worker/server  serve({ … })   │
│  call(method, params, opts) │  call / cancel │  handshake · priority queue · coalescing  │
│  render timers (slow/stuck) │                │  cancellation · ADR 0026 errors · traps   │
│  restart + reopen on crash  │ ◀───────────── │   ├─ viewer methods (src/worker/)         │
│  AbortError on abort        │ result / event │   │   openDocument → lamina Document +    │
│ @laminapdf/sdk/worker client   │  (transferred  │   │                  RenderDocument       │
│  request() · onDiscard      │   ImageBitmap) │   │   render → backend → ImageBitmap      │
└────────────────────────────┘                 │   └─ lamina's own methods (open, …)       │
                                               └──────────────────────────────────────────┘
```

There is one protocol, not two: the viewer's worker is `@laminapdf/sdk/worker`'s server with the viewer's methods added (`serve({ methods })`), and the client calls them through `WorkerClient.request` (ADR 0009 §3, [L2a](../../docs/todos/done/L2a-worker-client.md)).

## Methods

| Method | Params | Result | Priority |
| --- | --- | --- | --- |
| `initBackend` | `{ backend: "pdfium" \| "hayro", wasm? }` | `{ name, version }` | document. Sent by `create()`, before a file is chosen, so the WebAssembly downloads in parallel with file selection. |
| `openDocument` | `source` (`Blob`, `ArrayBuffer`, `Uint8Array`), `{ password?, limits?, renderLimits? }` | `{ pageCount, pages, needsPassword, backend }` | document |
| `closeDocument` | — | — | document |
| `pageInfo` | `index` | `PageInfo` (displayed size in points, rotation, ID, label) | document |
| `render` | ADR 0004 `RenderRequest` | `{ bitmap: ImageBitmap, width, height }` (transferred) | `visible` > `prefetch` > `background` |
| `__busy`, `__renderLog`, `__crash` | test-only | | |

`client.call(method, params, { priority, signal })` is typed by the `Methods` map: an unknown method or wrong params fails `tsc` (`test/types.test.ts`).

**Events:**

| Event | Source | Data | Meaning |
| --- | --- | --- | --- |
| `warning` | worker | ADR 0003 warning | e.g. a repaired xref |
| `render-start` | worker | `{ page, scale }` | for the perf harness |
| `render-slow` | page | `{ id, page }` | the soft timeout passed, counted from `render-start`: waiting in the queue isn't slow |
| `render-stuck` | page | `{ id, page, needsPassword }` | an encrypted document's render passed the hard cap |
| `crashed` | page | `{ reason }` | the worker crashed |
| `restarted` | page | `{ reason, reopened }` | a new worker serves again |
| `password-required` | page | — | the new worker can't reopen without the password |
| `restart-failed` | page | `{ reason, error }` | a restart failed (for example, the backend wouldn't load again) |

## Semantics

- **Priorities and coalescing:** the worker runs one call at a time, the highest priority first, FIFO within one.
  - Before each call it reads every message already sent, so a burst of renders and cancels is ordered and pruned before anything runs.
  - Renders with an equal normalised `RenderRequest` (`renderKey`: page, scale, clip, rotation, flags, background) run once, and each caller gets its own bitmap.
- **Cancellation:** `signal` rejects the call at once with a `DOMException` named `AbortError` (ADR 0009 §3.5).
  - A queued render is dropped.
  - A running one finishes, because synchronous WebAssembly can't be interrupted. Its late answer's bitmap is closed unpainted (`onDiscard`).
  - (`@laminapdf/sdk/worker` itself rejects with a `cancelled` LaminaError; the viewer maps it to `AbortError`.)
- **Timeouts:** the page keeps the timers, because a worker inside a render can't post anything.
  - Defaults are `soft` 10 s (`render-slow`) and `hard` 120 s.
  - At the hard cap the client calls `stop("hard-timeout")`. For a document that needed a password, it emits `render-stuck` instead and waits for the user's `stop()`.
- **Crashes:** a crash is the worker's `error` or `messageerror`, or a WebAssembly trap it reports, never silence.
  - Pending calls reject with `worker-crashed`.
  - A new worker reopens the document from the same source, but nothing that was pending is re-sent. The UI offers Retry, so a page that crashes the engine can't loop.
  - `stop()` takes the same path with `reason: "cancelled"`.
- **Secrets:** the password crosses to the worker once, in `openDocument`, and is kept only there. The page keeps the source and the other options.
  - After a crash of an encrypted document, `password-required` fires and the caller opens again.
  - The tests count every `postMessage` that carries the password.
- **Sources:**
  - A `Blob`/`File` is sent by reference, and the worker reads it in 4 MiB `slice()`s.
  - Bytes are copied, and kept on the page for a reopen.
  - `ByteSource` (`size`, `read(offset, length, signal)`) is the interface C4c's HTTP range source and block cache build on.

## Tests

- `test/browser/rpc.browser.test.ts` runs in Chromium, Firefox and WebKit, over the built package: `cargo xtask pdf-runtimes chromium|firefox|webkit`, or `pnpm exec vitest run -c vitest.browser.config.ts` after `cargo xtask wasm`. It covers:
  - the corpus page counts and sizes against `fixtures/corpus/expected.json`;
  - hayro;
  - limits;
  - 100 renders with 90 aborted;
  - a render aborted mid-run;
  - priorities and coalescing;
  - slow, stuck and stopped renders;
  - crashes, including an encrypted file;
  - the password count.
- `test/types.test.ts` (Node) holds the type tests.
- `examples/rpc-demo.html` is a manual check: run `pnpm --filter @laminapdf/viewer exec vite examples`, pick a PDF, and page 1 is drawn.
