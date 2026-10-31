// The viewer's ByteSource (C3a): random reads of a document's bytes, in the worker. C4c adds an
// HTTP range source and a block cache that wraps any source, so both engines' reads of the same
// bytes are fetched once; the interface is small for that reason.
import type { SourceInit } from "./protocol.js";

/** A document's bytes, read in ranges. */
export interface ByteSource {
  readonly size: number;
  /** `length` bytes at `offset` (fewer only at the end). Rejects when `signal` aborts. */
  read(offset: number, length: number, signal?: AbortSignal): Promise<Uint8Array>;
}

/** A Blob (or File) source: each read is a `slice()`, so the file is never read whole for one range. */
export function blobSource(blob: Blob): ByteSource {
  return {
    size: blob.size,
    async read(offset, length, signal) {
      signal?.throwIfAborted();
      const bytes = new Uint8Array(await blob.slice(offset, offset + length).arrayBuffer());
      signal?.throwIfAborted();
      return bytes;
    },
  };
}

/** An in-memory source. */
export function bufferSource(bytes: Uint8Array): ByteSource {
  return {
    size: bytes.byteLength,
    async read(offset, length, signal) {
      signal?.throwIfAborted();
      return bytes.subarray(offset, Math.min(bytes.byteLength, offset + length));
    },
  };
}

/** The source for what `open` was given. */
export function sourceOf(init: SourceInit): ByteSource {
  if (init instanceof Blob) return blobSource(init);
  return bufferSource(init instanceof Uint8Array ? init : new Uint8Array(init));
}

/** Reads past this many bytes at a time (a 1 GiB file is read in 4 MiB slices). */
const CHUNK = 4 * 1024 * 1024;

/**
 * Every byte of `source`, in chunks. Both engines need the whole file in memory in v0.1 (PDFium's
 * reads are synchronous); C4c reads ranges on demand instead.
 */
export async function readAll(source: ByteSource, signal?: AbortSignal): Promise<Uint8Array> {
  const out = new Uint8Array(source.size);
  for (let offset = 0; offset < source.size; offset += CHUNK) {
    const chunk = await source.read(offset, Math.min(CHUNK, source.size - offset), signal);
    out.set(chunk, offset);
  }
  return out;
}
