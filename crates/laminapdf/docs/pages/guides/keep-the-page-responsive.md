---
title: "Keep the page responsive"
description: "Run Lamina PDF in a Web Worker with @laminapdf/sdk/worker, so a large file never freezes the page."
version: "0.3.0"
url: "https://docs.laminapdf.com/guides/keep-the-page-responsive/"
---

# Keep the page responsive

Run Lamina PDF in a Web Worker with @laminapdf/sdk/worker, so a large file never freezes the page.

You'll run Lamina PDF in a Web Worker, so opening or saving a large file never freezes your page. You need this in a browser app once files get big: on the main thread, a [full rewrite](../../pages/reference/glossary.md#full-rewrite) of a 50 MB file blocks the page for about 200 ms.

`@laminapdf/sdk` is synchronous after `init()`. `@laminapdf/sdk/worker` runs the same API in a Web Worker, where you call the same methods and each returns a `Promise`. This example opens a file in a Web Worker and saves it there. The save reports its progress, and aborting the signal stops the work, for example when the user presses Cancel:

`packages/sdk/examples/worker-client.ts`:

```ts
// Saving a PDF in a Web Worker, so a large file never blocks the page: `@laminapdf/sdk/worker`
// runs the same API there, and each method returns a Promise. The save reports its progress step
// by step, and aborting the signal (a Cancel button, say) stops the work.
import { type CallOptions, connect } from "@laminapdf/sdk/worker";

/** What the save reports to and listens to. */
export type SaveInWorkerOptions = Pick<CallOptions, "onProgress" | "signal">;

/**
 * Opens `bytes` in a Web Worker and saves them there as a full rewrite.
 *
 * @param bytes - any PDF; it is copied to the worker (`transfer: true` would move it).
 * @param options - `onProgress` is called after each step of the save. Aborting `signal` stops
 *   the work wherever it is (starting the Web Worker, opening, or at the save's next step), and
 *   the promise rejects with a `cancelled` error.
 * @returns the saved file.
 */
export async function saveInWorker(bytes: Uint8Array, options: SaveInWorkerOptions = {}): Promise<Uint8Array> {
  const client = await connect(options);
  try {
    await using doc = await client.open(bytes, options);
    return await doc.save({ mode: "fullRewrite" }, options);
  } finally {
    client.terminate();
  }
}
```

## Call the same methods, awaited

Getters become methods: `await doc.pageCount()`. `doc.page(i)` returns a remote page, and `page.text()` returns a `PageText`, once you've called `client.loadExtract()`.

The Web Worker leaves out a few members, each for a reason:

- **`transact`:** its callback can't cross to the Web Worker, so pass the edits to `apply(ops)` instead.
- **`saveJob`:** the Web Worker's `save` already works in steps.
- **`ensure`:** the Web Worker fetches the byte ranges a call needs by itself.
- **`free`:** call `doc.close()`, or use `await using`, instead.
- **`Builder`:** build on the main thread, or in a Web Worker of your own.

## Cancel a call

The Web Worker runs one call at a time, so a call waits while another runs. Pass an `AbortSignal` as `signal` to any call. An aborted call rejects with `cancelled`, and `error.cause` holds the signal's reason. What happens to the work depends on where it was:

- A call that hasn't started never runs.
- `save` and `saveWithReport` stop at their next step, and the document stays usable.
- Any other call runs to the end in the Web Worker, because it is one synchronous call, though the byte fetches of a file opened with `openUrl` stop. `client.terminate()` stops it at once.

*Each call crosses to the Web Worker as a message; an abort skips a call not started and stops a running save.*

Your page's client sends each call, such as `await doc.pageCount()`, to the Web Worker as a message. There `@laminapdf/sdk` runs one call at a time, and each answer comes back as a Promise; saved bytes and page text come back transferred rather than copied. When a call's `AbortSignal` aborts, the call rejects with `cancelled`. A call not started never runs, and a running `save` or `saveWithReport` stops at its next step. Any other call runs to the end, though the byte fetches of a file opened with `openUrl` stop.

## Move bytes instead of copying them

Saved bytes and page text come back transferred, not copied. Your input is copied unless you pass `transfer: true`, which moves it and leaves your buffer empty.

## Recover from a crash

If the Web Worker dies, pending calls reject with `worker-crashed`, and `client.on("crashed", …)` fires. `client.restart()` starts a fresh Web Worker, and you open your documents again.

`connect()` also checks that the Web Worker comes from the same `@laminapdf/sdk` build as your page. It rejects with `worker-version-mismatch` otherwise.

## Bundlers and a content security policy

Vite and webpack bundle the Web Worker with no setup. In Vite, exclude `@laminapdf/sdk` from `optimizeDeps`, as for the main entry. To host the Web Worker's file yourself, for example under a strict content security policy, pass `connect({ worker: () => new Worker(url, { type: "module" }) })`.

Next: browse the [recipes](../../pages/recipes.md), with the PDF each one makes, or the [troubleshooting](../../pages/troubleshooting.md) page when something goes wrong.
