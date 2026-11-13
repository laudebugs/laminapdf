---
title: "Save in a Web Worker"
description: "Open and save a PDF in a Web Worker, so a large file never freezes the page, with progress as it saves and a signal that stops the work."
version: "0.2.0"
url: "https://docs.laminapdf.com/recipes/web-worker-client/"
---

# Save in a Web Worker

Open and save a PDF in a Web Worker, so a large file never freezes the page, with progress as it saves and a signal that stops the work.

You'll save a PDF in a Web Worker, reporting progress as it goes and stopping when the user presses Cancel. You need this in a browser app once files get large, because a save on the main thread freezes the page until it ends.

`saveInWorker` starts a Web Worker, opens the bytes there and saves them as a [full rewrite](../../pages/reference/glossary.md#full-rewrite), handing your `onProgress` and `signal` to each call:

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

The promise resolves to the saved file, or rejects with `cancelled` once the signal aborts. Lamina PDF's tests run it in Chromium, Firefox and WebKit.

## How it works

- `connect` starts the Web Worker file that ships with `@laminapdf/sdk` and checks that it comes from the same build as your page.
- `client.open` copies the bytes into the Web Worker and returns a remote document, whose methods return promises. `await using` closes it there when the block ends.
- The Web Worker saves in steps of 20,000 units of work, about one per object in a full rewrite. Between steps it calls `onProgress` with `done`, `total` and `phase`, at most every 50 ms, so a small file saves in one step and reports none.
- Aborting the signal rejects the call at once with `cancelled`, and `error.cause` holds the signal's reason. A save stops at its next step.
- `client.terminate()` in `finally` stops the Web Worker however the save ended, which also stops an open that was still running.

## What to change

- **A progress bar:** in `onProgress`, set a `<progress>` element's `value` to `done` and, when the save knows it, its `max` to `total`.
- **Large files:** pass `transfer: true` to `client.open` to move the buffer instead of copying it, which leaves your copy empty. See [Keep the page responsive](../../pages/guides/keep-the-page-responsive.md#move-bytes-instead-of-copying-them).
- **Several files:** call `connect` once, reuse the client for each file, and terminate it when the page no longer needs it.
- **Edits before the save:** apply them with `await doc.apply(ops)`, then save in the default `auto` mode. It appends them as an [incremental save](../../pages/reference/glossary.md#incremental-save) when the file allows one. See [Save a PDF](../../pages/guides/save-a-pdf.md#choose-a-save-mode).

Next: [a Cloudflare Worker](../../pages/recipes/cloudflare-worker.md) that generates a PDF per request.
