# C3a — `@laminapdf/viewer` part 1: document worker and typed RPC

| Field | Value |
| --- | --- |
| ID | `C3a` |
| Status | `done` |
| Plan | [web reader §6 C3. Viewer core](../../plans/web-app.md#c3-viewer-core) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B6b](./B6b-lamina-pdf-package.md), [L2a](./L2a-worker-client.md), [C2a](./C2a-render-adrs.md), [C2c](./C2c-render-packages.md) |
| Blocks | [C3b](./C3b-viewer-virtualiser-and-zoom.md), [E10i](../ready/E10i-platform-signers.md) |
| Dependency notes | [C2c](../done/C2c-render-packages.md) (web render adapters); [C2a](../done/C2a-render-adrs.md) (ADR 0009 RPC requirements); library **B6** (`@laminapdf/sdk` `Document` in the worker; [pdf-library B6](../../plans/pdf-library.md#b6-wasm-and-laminapdfsdk)) |
| ADRs | Implements 0009 (typed worker RPC) and 0004 (backend selection). Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits`, warning codes surfaced to the UI), [0005](../../decisions/0005-document-model.md) (`ByteSource`) |
| Branch | `feat/c3a-viewer-worker-rpc` |
| PR title | `feat(viewer): add the document worker and typed rpc with priorities and cancellation` |
| Size | L (600–1,500 lines incl. tests) |
| PR | [#132](https://github.com/laudebugs/lamina/pull/132) |

## Goal

Create `packages/viewer` and its foundation: one **document worker** per open document that hosts `lamina-wasm` (via `@laminapdf/sdk`) and a `RenderBackend` adapter, and a **hand-written typed RPC** between the main thread and that worker, with request IDs, a method union, priority queues, transfer lists, `AbortSignal` cancellation and automatic restart after a crash. Everything later in the viewer (virtualiser, text layer, find, thumbnails, print) talks to the document only through this RPC, so its shape and cancellation semantics are the most important thing to get right in Round C.

## Context

- **Delivered by [L2a](../done/L2a-worker-client.md):** `@laminapdf/sdk/worker` (`packages/sdk/src/worker/`) already has the envelopes (`protocol.ts`: `call`/`cancel`/`result`/`progress`/`crashed`, `PROTOCOL_VERSION` plus the package version in a hello/welcome handshake), monotonic request IDs, a handle table, `AbortSignal` cancellation (a queued call never runs; `save` is stepped and stops at the next step; late answers dropped), throttled progress events, transfers, ADR 0026 error rebuilding, and crash detection with `restart()`. This todo adds the viewer's concerns on top: render messages, priorities, coalescing, timeouts, the password re-prompt and the re-open policy after a crash.
- ADR 0009 (from [C2a](../done/C2a-render-adrs.md)) — RPC requirements, single-threaded baseline, no Comlink.
- [Research: the web client](../../research/architecture/app-and-package-architecture.md#the-web-client-is-react-chrome-around-an-imperative-viewer-core) — "fast scrolling cancels stale render jobs"; shell owns threads.
- [web-frontend notes](../../research/architecture/notes/web-frontend.md) §2 (render path (b): rasterise in WASM, `createImageBitmap`, transfer) and §5 (custom RPC, COOP/COEP) and §6 (stream `File.slice()`; avoid double copies; OPFS).
- [C1b result](../done/C1b-pdfium-wasm-spike.md) — chosen input strategy and instantiate timings.
- [web-app.md §4 functional 1–2, 10](../../plans/web-app.md#functional) and [privacy and security](../../plans/web-app.md#privacy-and-security) (hostile files crash the worker, not the page; password stays in worker memory).
- [TypeScript guidelines](../../guidelines/typescript.md) — `@laminapdf/viewer` is a browser UI package (DOM globals allowed), still ESM-only, named exports, Vitest browser mode.

## Scope

**In scope**

- `packages/viewer` scaffold (`@laminapdf/viewer`, `private: true`, ESM, Vitest browser mode + Playwright, nested `biome.json` that allows DOM globals with a reason).
- `src/rpc/protocol.ts`: method union, request/response/event envelopes, error codes (shared with `fixtures/codes.json`), versioned protocol constant.
- `src/rpc/client.ts` (main thread) and `src/worker/server.ts` (worker): request IDs, promises, `AbortSignal` → `cancel` message, transfer lists, events (progress, warnings), per-call slow/hard timeouts for long renders (no heartbeat), restart with re-open on crash.
- `src/worker/scheduler.ts`: priority queues (`visible` > `prefetch` > `thumbnail` > `background` (search, text)), coalescing of duplicate requests, cancellation before start and between render steps.
- `src/worker/document.ts`: open from a `ByteSource` (`File`/`Blob` via `slice()`, `ArrayBuffer`, later HTTP range in C4c), password, `Limits`, warnings → events; page count and page sizes (all boxes and rotation) up front for layout.
- `src/worker/render.ts`: render request → backend → `ImageData` → `createImageBitmap` → transfer; backend selected at open (`pdfium` default, `hayro` optional).
- `ByteSource` abstraction in TS (`read(offset, length, signal)`, `size`) shared with C4c's HTTP range source.

**Out of scope** (and which todo covers it)

- Virtualiser, canvas pool, tiles, zoom → [C3b](../done/C3b-viewer-virtualiser-and-zoom.md).
- Performance harness → [C3c](../ready/C3c-viewer-perf-harness.md).
- HTTP range `ByteSource` and password UI → [C4c](../ready/C4c-remote-files-encryption-privacy.md).
- Text runs, struct tree, search RPC methods → [C5a](../ready/C5a-text-layer-and-selection.md), [C5b](../ready/C5b-find-in-document.md) (the protocol leaves room for them).
- Threaded (SharedArrayBuffer) variant (ADR 0009: later).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Scaffold `packages/viewer` (`package.json` with `exports`, `files`, `sideEffects: false`, `engines`; `tsconfig.json` extending the base with `lib: ["ES2022","DOM","WebWorker"]` split per entry; `vitest.config.ts` browser mode with Playwright providers chromium/firefox/webkit).
- [x] Define the protocol: `type Request = { id: number; method: "open" | "close" | "pageInfo" | "render" | "cancel" | …; params; priority }`, `Response = { id; ok: true; result } | { id; ok: false; error: { code; message } }`, `Event = { kind: "warning" | "progress" | "crashed"; … }`. Types derive params/results per method (a `Methods` map) so `client.call("render", params)` is fully typed.
- [x] Client: monotonically increasing IDs; `call(method, params, { priority, signal, transfer })`; on abort send `cancel {id}` and reject with `DOMException("AbortError")`; a response for a cancelled ID is dropped and any transferred `ImageBitmap` is `close()`d (**cancelled renders never paint**).
- [x] Worker server: message loop, dispatch, per-method handlers, catch-all error mapping to codes; never `postMessage` an `Error` object (structured clone loses fields) — send codes.
- [x] Scheduler: one active render at a time per worker (synchronous WASM), priority queue, coalesce identical `(page, scale, rect, flags)` requests, check cancellation before starting and after each backend step; expose queue depth for the perf harness.
- [x] Crash handling — **no heartbeat**: the worker runs one synchronous WASM call at a time and cannot answer anything during a long render, so silence is not a crash signal. A crash is only: the worker's `error` or `messageerror` event, or the server reporting a WASM trap (`RuntimeError`, out-of-memory) after which it marks the module poisoned. Then: reject all pending with `worker-crashed`, emit `crashed`, spawn a new worker, re-open the document from the same `ByteSource`, resume — but do **not** automatically re-issue the request that was running when it crashed (that page shows "Couldn't render this page — Retry"), so a hostile page can't cause a crash loop.
- [x] Long renders: each `render` call has a **soft timeout** (default 10 s, configurable) after which the client emits `render-slow { page }` so the UI can offer "This page is taking a long time — Stop", and a **hard cap** (default 120 s). Only a user Stop or the hard cap terminates the worker (`worker.terminate()`), then the same restart path as a crash runs with `reason: "cancelled" | "hard-timeout"`, without re-issuing that render. For an **encrypted** document, terminating means re-entering the password, so the hard cap does not terminate automatically: the client emits `render-stuck { page, needsPassword: true }` and the UI warns ("Stopping will ask for the password again") and terminates only on the user's confirmation. The main thread **never caches the password**: it lives only in the worker that received it, so it dies with a crashed worker. Re-opening an encrypted document after a crash therefore returns `needs-password` and the client emits a `password-required` event so the UI re-prompts (C4c wires the dialog); unencrypted documents re-open without user action.
- [x] Open: `@laminapdf/sdk` `Document.open` and backend open in parallel (both from the same `ByteSource`); surface ADR 0003 warnings (e.g. repaired xref) as events; return page count and sizes.
- [x] Backend loading: `init({ wasmUrl })` for each module, started as soon as the worker is created (before a file is chosen) so WASM downloads in parallel with file selection ([§4 target](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward)).
- [x] Minimal `examples/rpc-demo.html` (served by Vitest/Vite dev) that opens a file and draws page 1 to a canvas — enough for manual checks before C3b.
- [x] Wire into `cargo xtask check ts`.

## Acceptance criteria

- [x] The viewer's worker protocol builds on `@laminapdf/sdk/worker` ([L2a](../done/L2a-worker-client.md)): render and viewer messages extend it rather than defining a second RPC ([shareable-libraries §8](../../plans/shareable-libraries.md#8-changes-to-existing-todos))
- [x] Typed: calling an unknown method or passing wrong params fails `tsc` (type tests with `expectTypeOf`).
- [x] Opening `fixtures/corpus/*.pdf` returns page count and sizes matching `fixtures/corpus/expected.json` in Chromium, Firefox and WebKit.
- [x] Cancellation: a test issues 100 render requests and aborts 90 before they finish; no aborted request resolves, every transferred bitmap for an aborted ID is closed, and the 10 kept ones resolve in priority order.
- [x] Priority: with a queue of `background` work, a new `visible` request starts next (asserted via scheduler events).
- [x] Slow render is not a crash: a test-only `__busy(ms)` method blocks the worker synchronously for longer than any idle interval but under the soft timeout; no `crashed` event, no restart, and the call resolves. Above the soft timeout, `render-slow` fires and the worker keeps running; a simulated user Stop terminates and restarts it with `reason: "cancelled"` and the blocked request is not re-issued; with an encrypted document, reaching the hard cap emits `render-stuck` with `needsPassword: true` and does not terminate until confirmed (timeouts shortened in tests via config, no wall-clock assertions beyond ordering).
- [x] Crash: a test-only `__crash` method kills the worker; pending calls reject with `worker-crashed`, a new worker re-opens the document and the next render succeeds; the page (main thread) never throws.
- [x] Crash with an encrypted document: after `__crash`, the re-open yields a `password-required` event (no automatic re-open with a remembered password); supplying the password again via a fresh `open` renders the next page.
- [x] A hostile fixture that exceeds `Limits` yields an error code, not a hang (runner timeout per ADR 0018, no wall-clock assertions).
- [x] The password never appears in any `postMessage` except the `open` call made with the user-entered password (exactly one per user entry, including across a crash/re-prompt cycle), and no main-thread object (client, RPC state) retains it after that call returns (asserted by wrapping `postMessage` in tests and by inspecting the client state after open and after a crash).

## Outcome

What shipped, and where it differs from the plan above:

- **One protocol:** `@laminapdf/sdk/worker` gained an extension point instead of a second RPC.
  - `src/worker/server.ts`'s `serve({ methods })` (the `./worker/server` subpath) serves the L2a protocol with more methods. `entry.ts` is `serve()` alone.
  - Requests carry an optional `priority` and coalescing `key` (`PROTOCOL_VERSION` 2). `emit()` posts extension events. `WorkerClient.request()` calls extension methods, and `connect({ onDiscard })` sees late answers (by default it closes their `ImageBitmap`s).
  - The viewer's worker (`packages/viewer/src/worker/entry.ts`) is `serve({ methods })` with its methods, and `ViewerClient` calls them through `request()`.
- **Scheduler:** the queue is generic and lives in the shared server, not in `packages/viewer/src/worker/scheduler.ts`.
  - It reads every pending message before each call (a synchronous call otherwise finishes before the next message is read, and nothing is ever queued), runs the highest priority first, FIFO within one, and coalesces equal keys.
  - An extension method's `reply` builds each coalesced caller's answer (one `ImageBitmap` each).
  - The viewer's half is `PRIORITY` and `renderKey` in `src/protocol.ts`. Queue depth is `__stats().queued`.
- **Cancellation:** the viewer rejects with `AbortError`, as ADR 0009 §3.5 says. `@laminapdf/sdk/worker` keeps its `cancelled` LaminaError (L2a), and the viewer maps between them.
- **Open:**
  - The worker reads the whole source (a Blob in 4 MiB `slice()`s), then opens lamina's Document and the backend's RenderDocument from the same bytes. They open in sequence, not in parallel: both are synchronous in one worker.
  - Page sizes are `PageInfo`'s displayed width and height, rotation, ID and label. It has no other boxes yet.
  - `needsPassword` is whether the file fails to open without its password (`password-required`), so a password given to a plain file doesn't block a reopen.
  - Each open has an `id`. Renders name it (in their coalescing key, too), so a render queued for a replaced document fails with `disposed`. A failed open leaves the previous document open.
- **Backends:** the PDFium `.wasm` URL is an option (`backend: { backend: "pdfium", wasm }`), because a library can't resolve a dependency's asset without the app's bundler. Hayro loads its own.
- **Restart:**
  - After a crash or `stop()`, the client restarts the worker, reloads the backend and reopens the document. The UI re-requesting its visible tiles is C3b's. Nothing pending is re-sent.
  - A document that needed a password isn't reopened: `password-required` fires.
- **Not done:** the lamina `RemoteDocument` methods (L2a's `open`/`openUrl`) are served by the same worker, but `ViewerClient` doesn't expose them; C5a/C5b add text and search through the same `call`. An extra `render-start` event exists for C3c's harness.
- **Versioning:** `@laminapdf/viewer` starts at `0.1.0-alpha.0`, in `.changeset/config.json`'s fixed group (added after Rex's lockstep follow-up, #131, made `version sync` read the group from that file).
- **CI:** `cargo xtask wasm` runs the viewer's `:wasm` scripts (build, typecheck, the Node type tests). `cargo xtask pdf-runtimes chromium|firefox|webkit` runs its browser suite after the render adapters'.

## Post-merge checks

None.

## Tests

Vitest browser-mode tests in `packages/viewer/test/rpc/*.test.ts` (protocol types, client/server round-trip, cancellation, priorities, crash/restart, warnings), using `fixtures/corpus/` files fetched by the test server; unit tests for the scheduler in Node-free browser mode. Encrypted open uses `fixtures/corpus/enc.pdf` and B2's encrypted fixtures.

## Docs to update

`packages/viewer/README.md` (architecture diagram of client/worker/scheduler, protocol table); `CLAUDE.md` layout (`packages/viewer`); tick "Create `packages/viewer` with the typed RPC…" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Synchronous WASM can't be interrupted mid-render**: cancellation granularity is per request (or per progressive step if PDFium's progressive API is exposed); keep tile sizes small (C3b) so latency stays bounded.
- **Two engines opening the same `ByteSource`**: random reads from both double I/O on remote sources; C4c adds a block cache — design the `ByteSource` interface now so a cache can wrap it.
- **Module workers + WASM in Firefox/Safari** with Vite 8/Rolldown (`new Worker(new URL(...), { type: "module" })`) is unverified ([web-frontend notes §8](../../research/architecture/notes/web-frontend.md)); test all three engines early.
- One worker per document vs a shared worker pool: ADR 0009 decides; this PR implements per-document.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
