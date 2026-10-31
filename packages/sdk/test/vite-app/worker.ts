// The worker client in a Vite build (L2a): `connect()`'s literal `new Worker(new URL(…))` must
// make Vite emit the worker file, and the worker must load the `.wasm` from the build's assets.
import { connect } from "@laminapdf/sdk/worker";

/** [page count, 1 if save() returned bytes] of `bytes`, opened through the worker. */
export async function run(bytes: Uint8Array): Promise<[number, number]> {
  const client = await connect();
  try {
    await using doc = await client.open(bytes);
    const pages = await doc.pageCount();
    const saved = await doc.save();
    return [pages, saved.byteLength > 0 ? 1 : 0];
  } finally {
    client.terminate();
  }
}
