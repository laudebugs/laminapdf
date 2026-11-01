// What the document worker reads a Blob through: `@laminapdf/sdk`'s RangeSource, so the viewer and
// Lamina PDF share one interface for a document's bytes (the HTTP range source is one too).
import { callbackSource, type RangeSource } from "@laminapdf/sdk";

/** A Blob (or File) as a RangeSource: each read is a `slice()`, so the file is never read whole for one range. */
export function blobSource(blob: Blob): RangeSource {
  return callbackSource(blob.size, async (offset, length, signal) => {
    signal?.throwIfAborted();
    const bytes = new Uint8Array(await blob.slice(offset, offset + length).arrayBuffer());
    signal?.throwIfAborted();
    return bytes;
  });
}

/** Reads past this many bytes at a time (a 1 GiB file is read in 4 MiB slices). */
const CHUNK = 4 * 1024 * 1024;

/**
 * Every byte of `source`, in chunks. Both engines need the whole file in memory in v0.1 (PDFium's
 * reads are synchronous).
 */
export async function readAll(source: RangeSource, signal?: AbortSignal): Promise<Uint8Array> {
  const out = new Uint8Array(source.size);
  for (let offset = 0; offset < source.size; offset += CHUNK) {
    const chunk = await source.read(offset, Math.min(CHUNK, source.size - offset), signal ? { signal } : {});
    out.set(chunk, offset);
  }
  return out;
}
