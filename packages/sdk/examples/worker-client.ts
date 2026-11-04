// Saving a PDF in a Web Worker, so a large file never blocks the page: `@laminapdf/sdk/worker`
// runs the same API there, and each method returns a Promise. The save reports its progress step
// by step, and aborting the signal (a Cancel button, say) stops it.
import { type CallOptions, connect } from "@laminapdf/sdk/worker";

/** What the save reports to and listens to. */
export type SaveInWorkerOptions = Pick<CallOptions, "onProgress" | "signal">;

/**
 * Opens `bytes` in a Web Worker and saves them there as a full rewrite.
 *
 * @param bytes - any PDF; it is copied to the worker (`transfer: true` would move it).
 * @param options - `onProgress` is called after each step of the save; aborting `signal` stops
 *   the save at its next step, and the promise rejects with a `cancelled` error.
 * @returns the saved file.
 */
export async function saveInWorker(bytes: Uint8Array, options: SaveInWorkerOptions = {}): Promise<Uint8Array> {
  const client = await connect();
  try {
    await using doc = await client.open(bytes);
    return await doc.save({ mode: "fullRewrite" }, options);
  } finally {
    client.terminate();
  }
}
