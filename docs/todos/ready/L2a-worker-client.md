# L2a — `@lamina/pdf/worker`: the async twin of the sync API, with typed RPC, version handshake, `AbortSignal`, transfer lists and progress

| Field | Value |
| --- | --- |
| ID | `L2a` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L2](../../plans/shareable-libraries.md#l2-typescript-hardening), [§5 design summary](../../plans/shareable-libraries.md#5-design-summary) |
| Round / milestone | Round L2 (shareable libraries) — before C3a |
| Depends on | [B6b](../done/B6b-lamina-pdf-package.md), [L1a](../done/L1a-contract-adrs.md), [L1b](../ongoing/L1b-error-contract-codegen.md) |
| Blocks | [C3a](./C3a-viewer-worker-rpc.md) |
| ADRs | Follows 0028 (jobs, `CancelToken`, async only at the edges) and 0026 (errors cross as JSON, not `Error` objects), both from [L1a](../done/L1a-contract-adrs.md); [0002](../../decisions/0002-binding-strategy.md) §1 (sync core after `init()`); 0016 (export conditions, from [B6a](../done/B6a-lamina-wasm-and-adr-0016.md)); informs proposed 0009 (typed worker RPC, [C2a](./C2a-render-adrs.md)) |
| Branch | `feat/l2a-worker-client` |
| PR title | `feat(pdf): @lamina/pdf/worker async client with typed RPC, version handshake and AbortSignal` |
| Size | L (≈ 600–900 lines of TS + tests) |
| PR | — |

## Goal

Let web apps keep lamina off the main thread without learning a second API. `@lamina/pdf/worker` runs the sync `@lamina/pdf` API inside a Web Worker. It exposes the same method names returning Promises, typed by a `Promisified<T>` mapped type derived from the sync interfaces, so the two never drift. It does a version handshake on connect, takes `AbortSignal` and `onProgress`, and transfers buffers instead of cloning them. `@lamina/viewer` ([C3a](./C3a-viewer-worker-rpc.md)) builds on it, so lamina has one worker protocol, not two ([report: the worker client is the async twin](../../research/libraries/shareable-libraries.md#the-worker-client-is-the-async-twin-of-the-sync-api-and-it-should-be-shared-with-the-viewer)).

## Context

- **Range documents ([L2c](../done/L2c-range-byte-source.md)):** run `Document.openRange` and `doc.ensure` inside the worker, so a remote handle's calls stay plain async on the caller's side (the fetch-and-retry loop is hidden in the worker). L2c merged before this todo, so the wiring is this todo's.
- [Report: the worker client](../../research/libraries/shareable-libraries.md#the-worker-client-is-the-async-twin-of-the-sync-api-and-it-should-be-shared-with-the-viewer): the PDF.js version-mismatch lesson, unpdf's inlined worker, Vite's literal `new URL()` inside `new Worker()`, the `{ worker }` escape hatch.
- [TypeScript note §3](../../research/libraries/notes/typescript-web-dx.md#3-worker-offloading): the API sketch, the handshake, and why progress is sent as fire-and-forget messages instead of Comlink callbacks.
- [TypeScript note §4](../../research/libraries/notes/typescript-web-dx.md#4-tree-shaking-subpath-exports-multiple-wasm-modules): `Promisified<T>` keeps sync and async in lockstep.
- [Report: cancellation and progress](../../research/libraries/shareable-libraries.md#cancellation-and-progress-belong-in-laminas-api-not-the-bindings): a busy sync wasm call cannot see a JS `cancel()`. Work must be chunked with `job.step()`, or the worker terminated.
- [C3a](./C3a-viewer-worker-rpc.md): the viewer's planned RPC (request IDs, priorities, crash restart, no Comlink, never `postMessage` an `Error`). Its scope now builds on this package.
- [TypeScript guideline](../../guidelines/typescript.md): ESM-only, named exports, no Node-only APIs in shared packages.
- [ADR 0018](../../decisions/0018-test-strategy.md#2-ci-lanes): no wall-clock assertions in the PR lanes.

## Scope

**In scope**

- **`./worker` subpath** in `packages/pdf`:
  - `connect(options?)` returns a `WorkerClient`. Options: `{ worker?, signal?, init? }`, where `init` passes the wasm URL or `WebAssembly.Module`.
  - The package contains the literal `new Worker(new URL("./worker.js", import.meta.url), { type: "module" })`, so Vite and webpack bundle the worker with no config. `{ worker }` accepts a caller-made `Worker` for CSP-restricted pages.
  - The worker entry (`src/worker/entry.ts` → `dist/worker.js`) calls `init()` and serves the API.
- **Typed RPC** (`src/worker/protocol.ts`):
  - Envelopes: `Request { id, method, handle?, params }`, `Response { id, ok, result | error }`, `Event { kind: "progress" | "warning" | "crashed", id? }`.
  - A `Methods` map from the sync interfaces gives fully typed calls.
  - `Promisified<T>` / `Remote<T>` mapped types turn each sync method into `(...args) => Promise<R>` and each handle-returning method into a remote handle.
- **Handles:** documents, pages and builders live in the worker and are referenced by ID. Remote handles implement `Symbol.asyncDispose` (and `close()`), which frees the worker-side object.
- **Version handshake:** on connect the worker replies `{ protocol, version }`. A mismatch with the client's build rejects `connect()` with a `LaminaError` (new code, e.g. `worker-version-mismatch`, added via [L1b](../ongoing/L1b-error-contract-codegen.md)'s process).
- **Cancellation:** each call accepts `signal`. Abort sends `cancel { id }`. The worker trips the job's `CancelToken` (ADR 0028) and checks it between `step()` calls. A call that cannot be stepped is documented as not interruptible, with `client.terminate()` as the hard stop. The client rejects aborted calls promptly and drops late responses.
- **Transfers:** outputs (`save()` bytes, rendered pixels) are always transferred. Inputs are transferred when the caller passes `{ transfer: true }`; otherwise they are copied (see Risks).
- **Progress:** `onProgress` per call, sent as throttled event messages, never as a callback proxy.
- **Errors:** the worker serialises errors to ADR 0026 JSON and the client rebuilds `LaminaError` (structured clone loses fields on `Error` objects). Warnings arrive as events and on results.
- **Crash detection:**
  - The worker's `error`/`messageerror` event, or a reported wasm trap, rejects all pending calls with `worker-crashed` and emits `crashed`.
  - `client.restart()` respawns the worker.
  - Automatic re-open and retry policy stays with the caller ([C3a](./C3a-viewer-worker-rpc.md) for the viewer).
  - Silence is not a crash: no heartbeat.
- Runtimes: browsers (Chromium, Firefox, WebKit), Deno and Bun web Workers.

**Out of scope** (and which todo covers it)

- Viewer priorities, coalescing, render scheduling, soft/hard render timeouts, password re-prompt: [C3a](./C3a-viewer-worker-rpc.md) (built on this client).
- In-thread disposal, memory caps and trap recovery: [L2b](./L2b-disposal-and-memory.md).
- Range loading through the worker: [L2c](../done/L2c-range-byte-source.md) (the worker runs its fetch-and-retry loop).
- Node `worker_threads` adapter and a worker pool: later, if a use case appears (see Risks).
- SharedArrayBuffer and threaded wasm: excluded by ADR 0002 and proposed ADR 0009.

## Tasks

- [ ] Read the report section, the TS note §3–§4, C3a, ADR 0028 and B6b's `src/` on `main`
- [ ] Write the public types first (`src/worker/index.ts`: `connect`, `WorkerClient`, `RemoteDocument`, `RemotePage`, `Promisified<T>`, `CallOptions { signal?, onProgress?, transfer? }`) and review them in the PR
- [ ] `protocol.ts`:
  - [ ] Envelopes and a `PROTOCOL_VERSION` constant
  - [ ] `Methods` derived from the sync classes (no hand-maintained list where a mapped type can do it)
  - [ ] Error and warning payloads in ADR 0026 shape
- [ ] `client.ts`:
  - [ ] Monotonic IDs; a pending map; abort handling; late-response dropping
  - [ ] Transfer lists; progress dispatch
  - [ ] Crash detection; `restart()`, `terminate()`
- [ ] `entry.ts` (worker):
  - [ ] Handshake; `init()`
  - [ ] A handle table; dispatch to the sync API
  - [ ] Job stepping with `CancelToken` and a throttled progress callback for long methods (`save`, and later render and extract)
  - [ ] Error serialisation
- [ ] Remote handles with `Symbol.asyncDispose` (use `Symbol.asyncDispose ?? Symbol.for("Symbol.asyncDispose")`, without mutating globals)
- [ ] `package.json`: add the `./worker` export and the worker file to `files`; check publint and `attw --pack` still pass; include `worker.js` in `cargo xtask size`
- [ ] Vitest browser mode (Playwright: Chromium, Firefox, WebKit) in `packages/pdf`, if [B6c](../done/B6c-runtime-matrix.md) has not set it up yet; Deno and Bun smoke tests through B6c's runners, if present
- [ ] A worker example in `packages/pdf/examples/worker-client/` (open, page count, save, abort), run as a test
- [ ] README section: when to use the worker client, bundler notes (Vite `optimizeDeps.exclude`), CSP and `{ worker }`

## Acceptance criteria

- [ ] For every public method of the sync `Document` (and `Page`, `Builder`), the remote handle has a method of the same name returning a `Promise` of the same result type (type test with `expectTypeOf` over the mapped type). Adding a sync method without worker support fails `tsc`
- [ ] Opening every file in `fixtures/corpus` through the client gives the same page count, warnings and `save()` bytes as the sync API, in Chromium, Firefox and WebKit
- [ ] A client built from a different `version` or `PROTOCOL_VERSION` than the worker rejects `connect()` with the mismatch code. Nothing else is sent
- [ ] Abort before start: the request never runs (asserted via a worker-side counter). Abort during a stepped job: the job stops at the next step, the promise rejects, and the handle stays usable
- [ ] `save()` output arrives as a transferred buffer (the worker-side buffer is detached); with `{ transfer: true }` the caller's input buffer is detached; without it, the input is unchanged
- [ ] Errors from the worker are `instanceof LaminaError` on the main thread, with all ADR 0026 fields intact
- [ ] A test-only `__crash` method makes pending calls reject with `worker-crashed` and emits `crashed`; `restart()` then serves new calls
- [ ] Full lane (not PR lanes, per ADR 0018): opening a generated 50 MB file through the client in Chromium produces no main-thread long task over 50 ms (`PerformanceObserver` `longtask`), recorded in the job summary

## Post-merge checks

- [ ] The full-lane 50 MB long-task check has run on `main` and its result is linked in the done todo

## Tests

- `packages/pdf/test/worker/*.test.ts` in Vitest browser mode: types, round-trip parity over `fixtures/corpus`, handshake, abort before and during a job, transfers, error rebuilding, crash and restart, async disposal (the worker's handle table returns to empty).
- Type tests for `Promisified<T>` and the `Methods` map.
- Full-lane long-task test with a 50 MB file generated in the test (many pages or a large image stream); no fixture committed.

## Docs to update

- [ ] [shareable-libraries §9 L2](../../plans/shareable-libraries.md#9-to-dos): tick the worker-client to-do
- [ ] `packages/pdf/README.md` (worker client section); `CLAUDE.md` if the layout or commands change
- [ ] [C3a](./C3a-viewer-worker-rpc.md): note in its Context which parts this PR delivered (protocol, cancellation, crash detection), so C3a only adds viewer concerns

## Risks and open questions

- **Transfer by default?** Transferring the input detaches the caller's buffer, which surprises users who reuse it. Copying a 50 MB file costs memory and time. Recommendation: copy by default, `{ transfer: true }` opt-in, and say so in TSDoc. The owner decides before the API is published.
- **Abort rejection value:** web convention rejects with `signal.reason` (an `AbortError` `DOMException`); ADR 0026 has a `cancelled` code. Recommendation: reject with a `LaminaError` of code `cancelled` whose `cause` is `signal.reason`. Record the choice in the README.
- **Not every call is steppable** until B5c's `Job` covers it. A long `open()` cannot be cancelled mid-call; `terminate()` is the only hard stop. List per method which ones are steppable.
- **Workers in Deno and Bun** support module workers, but the bundled `new URL()` path may differ. Unverified; test early.
- **Node:** `worker_threads` is not a web `Worker`. Leaving Node out keeps `src/` free of Node APIs. An adapter can come later.
- **Vite pre-bundling** breaks `new URL(..., import.meta.url)` unless the package is excluded from `optimizeDeps` ([report: per-condition loaders](../../research/libraries/shareable-libraries.md#one-wasm-file-resolved-per-export-condition-with-an-escape-hatch)). Test it with a Vite fixture app, and document the fix.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
