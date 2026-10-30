// Range sources (L2c): a document whose bytes arrive range by range. The WASM reads
// synchronously from a sparse source; when bytes are missing it throws `data-pending` with the
// ranges it needs, and the loop here fetches them, supplies them and calls again.
import type { WasmSparseSource } from "@lamina/wasm/base";
import { guard, isLaminaError, type LaminaError, packageError } from "../errors.js";

/** A byte range of the input. */
export interface ByteRange {
  /** First byte. */
  readonly offset: number;
  /** Number of bytes. */
  readonly length: number;
}

/**
 * Random access to a file whose bytes are fetched on demand (a remote file, a `Blob`, a file
 * handle). Matches the viewer's source interface (C3a).
 */
export interface RangeSource {
  /** The file's size in bytes. */
  readonly size: number;
  /**
   * The `length` bytes at `offset` (exactly that many: the file is at least `offset + length`
   * long). Rejects when `signal` aborts.
   */
  read(offset: number, length: number, options?: { readonly signal?: AbortSignal }): Promise<Uint8Array>;
  /** The block size this source prefers fetches aligned to (HTTP: fewer, larger requests). */
  readonly blockSize?: number;
}

/** How a range document fetches. */
export interface RangeOptions {
  /** Aborts fetching: the pending call rejects with the signal's reason. */
  readonly signal?: AbortSignal;
  /** Fetches are widened to whole blocks of this many bytes (default: the source's, else 4096). */
  readonly blockSize?: number;
  /** Extra bytes fetched after each range (default 0). */
  readonly readahead?: number;
  /** Rounds of fetches per call before giving up with the last `data-pending` (default 1024). */
  readonly maxRounds?: number;
  /** The most bytes the document may hold (default: no limit); more is `source-budget-exceeded`. */
  readonly budget?: number;
}

/**
 * A {@link RangeSource} over a caller's read function (a `Blob.slice`, a file handle, an IPC call).
 *
 * @param size - the file's size in bytes.
 * @param read - returns the `length` bytes at `offset`.
 */
export function callbackSource(
  size: number,
  read: (offset: number, length: number, signal?: AbortSignal) => Promise<Uint8Array>,
): RangeSource {
  return { size, read: (offset, length, options) => read(offset, length, options?.signal) };
}

/** The ranges of a `data-pending` error. */
function rangesOf(e: LaminaError): readonly ByteRange[] {
  return e.ranges ?? [];
}

/** @internal What a range document keeps to fetch more. */
export interface RangeState {
  readonly sparse: WasmSparseSource;
  readonly source: RangeSource;
  readonly options: RangeOptions;
}

/**
 * Runs `call` until it stops throwing `data-pending`: fetches the ranges asked for (widened to
 * blocks, minus what the source holds), supplies them and calls again. Gives up with the last
 * `data-pending` after `maxRounds`, or when a round asks again for exactly what the round before
 * asked (no progress).
 *
 * @internal
 */
export async function settle<T>(state: RangeState, call: () => T): Promise<T> {
  const { sparse, source, options } = state;
  const block = options.blockSize ?? source.blockSize ?? 4096;
  const readahead = options.readahead ?? 0;
  const maxRounds = options.maxRounds ?? 1024;
  let previous: string | undefined;
  for (let round = 0; ; round++) {
    let error: LaminaError;
    try {
      return call();
    } catch (e) {
      if (!isLaminaError(e, "data-pending")) throw e;
      error = e;
    }
    const asked = rangesOf(error);
    const key = JSON.stringify(asked);
    const flat = new Float64Array(asked.flatMap((r) => [r.offset, r.offset + r.length]));
    // The planning is lamina-source's (`SparseSource::to_fetch`), shared with the Rust loop.
    const todo = guard(() => sparse.toFetch(flat, block, readahead));
    if (round >= maxRounds || todo.length === 0 || key === previous) throw error;
    options.signal?.throwIfAborted();
    const fetches: Promise<void>[] = [];
    for (let i = 0; i + 1 < todo.length; i += 2) {
      const start = todo[i] ?? 0;
      const end = todo[i + 1] ?? 0;
      fetches.push(
        source
          .read(start, end - start, options.signal === undefined ? {} : { signal: options.signal })
          .then((bytes) => {
            if (bytes.length !== end - start) {
              throw packageError("fetch-failed", "a range source returned a different number of bytes than asked for", {
                data: { reason: "length", expected: end - start, got: bytes.length },
              });
            }
            guard(() => sparse.supply(start, bytes));
          }),
      );
    }
    await Promise.all(fetches);
    previous = key;
  }
}
