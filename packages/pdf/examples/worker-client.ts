// `@laminapdf/pdf/worker`: the sync API's async twin in a Web Worker, so a large file never blocks
// the page. Open, count the pages, save with progress, and abort a save the user cancels. Runs
// in browsers (test/browser/worker.browser.test.ts).
import { isLaminaError } from "@laminapdf/pdf";
import { connect } from "@laminapdf/pdf/worker";

/** What the example saw: the page count, the saved size, and whether the abort was reported. */
export interface WorkerClientResult {
  readonly pages: number;
  readonly savedBytes: number;
  readonly progressEvents: number;
  readonly abortedAsCancelled: boolean;
}

/**
 * Opens `bytes` in a worker, saves them, then starts a second save and aborts it.
 *
 * @param bytes - any PDF; it is copied to the worker (`transfer: true` would move it).
 */
export async function workerClient(bytes: Uint8Array): Promise<WorkerClientResult> {
  const client = await connect();
  try {
    await using doc = await client.open(bytes);
    const pages = await doc.pageCount();

    let progressEvents = 0;
    const saved = await doc.save({ mode: "fullRewrite" }, { onProgress: () => progressEvents++ });

    // A cancel button: abort the signal and the save stops at its next step.
    const controller = new AbortController();
    const saving = doc.save({ mode: "fullRewrite" }, { signal: controller.signal });
    controller.abort(new Error("the user cancelled"));
    let abortedAsCancelled = false;
    try {
      await saving;
    } catch (e) {
      abortedAsCancelled = isLaminaError(e, "cancelled");
    }

    return { pages, savedBytes: saved.byteLength, progressEvents, abortedAsCancelled };
  } finally {
    client.terminate();
  }
}
