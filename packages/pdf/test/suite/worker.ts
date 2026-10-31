// The worker client's smoke test for runtimes outside Vitest's browser mode (Deno and Bun web
// Workers, L2a): connect, open, page count, save.
import { connect } from "@laminapdf/pdf/worker";

/** Throws unless `bytes` (hello.pdf) opens and saves through a worker. */
export async function workerSmoke(bytes: Uint8Array): Promise<void> {
  const client = await connect();
  try {
    await using doc = await client.open(bytes);
    const pages = await doc.pageCount();
    if (pages !== 1) throw new Error(`expected 1 page, got ${pages}`);
    const saved = await doc.save();
    if (!(saved instanceof Uint8Array) || saved.byteLength === 0) throw new Error("save() returned no bytes");
  } finally {
    client.terminate();
  }
}
