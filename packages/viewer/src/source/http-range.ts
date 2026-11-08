// A remote PDF read in the document worker with HTTP Range requests: `@laminapdf/sdk`'s
// httpRangeSource (tail-first probe, If-Range and ETag checks, the 200 fallback) with what a
// reader needs on top:
//
// - privacy: every request omits credentials and the referrer (ADR 0011 §3), and by default skips
//   the HTTP cache, so the file is not kept on disk after the tab closes;
// - a block cache that the render backend reads synchronously (`readSync`, `RenderSource`): PDFium
//   asks for bytes while it draws, and a miss makes the render throw `data-pending`, after which
//   the worker fills the ranges asked for and draws again;
// - readahead, a byte budget (least recently used blocks go first) and progress;
// - a cap on the whole download a server without range support makes, until large files can be
//   streamed to OPFS.

import type { ByteRange, RenderSource } from "@laminapdf/render";
import { httpRangeSource, isLaminaPdfError, LaminaPdfError, type RangeSource } from "@laminapdf/sdk";
// Only docsUrl: the package has no side effects, so the explain pages stay out of the worker.
import { docsUrl } from "@laminapdf/sdk/explain";

/** Options for {@link HttpRangeByteSource.open}. */
export interface HttpRangeByteSourceOptions {
  /** Requests are aligned to blocks of this many bytes (default 65 536). */
  readonly blockSize?: number;
  /** Whole blocks fetched after each range asked for (default 1). */
  readonly readahead?: number;
  /** The most bytes the cache keeps; the least recently used blocks go first (default 64 MiB). */
  readonly cacheBytes?: number;
  /**
   * The largest file downloaded whole when the server ignores `Range` (default 256 MiB); above
   * it the open fails with `source-budget-exceeded`.
   */
  readonly maxWholeBytes?: number;
  /** Skip the browser's HTTP cache (default true). */
  readonly noStore?: boolean;
  /** Called after each fetch with the bytes fetched so far and the file's size. */
  readonly onProgress?: (progress: { readonly loaded: number; readonly total: number }) => void;
  /** Aborts the probe. */
  readonly signal?: AbortSignal;
  /** The `fetch` to use (default: the global one). */
  readonly fetch?: typeof fetch;
}

const DEFAULTS = { blockSize: 65_536, readahead: 1, cacheBytes: 64 * 1024 * 1024, maxWholeBytes: 256 * 1024 * 1024 };

/**
 * The fetch every request of the source goes through: no cookies or credentials, no referrer, no
 * HTTP cache unless asked, CORS mode. A full answer larger than `maxWholeBytes` is refused before
 * its body is read; `onWhole` hears that the server sent the whole file.
 */
function privateFetch(base: typeof fetch, noStore: boolean, maxWholeBytes: number, onWhole: () => void): typeof fetch {
  return async (input, init) => {
    const response = await base(input, {
      ...init,
      credentials: "omit",
      referrerPolicy: "no-referrer",
      mode: "cors",
      ...(noStore ? { cache: "no-store" } : {}),
    });
    if (response.status === 200) {
      const length = Number(response.headers.get("Content-Length") ?? Number.NaN);
      if (Number.isFinite(length) && length > maxWholeBytes) {
        await response.body?.cancel();
        throw tooLarge(length, maxWholeBytes);
      }
      onWhole();
    }
    return response;
  };
}

function tooLarge(size: number, budget: number): LaminaPdfError {
  return new LaminaPdfError({
    code: "source-budget-exceeded",
    kind: "limit",
    message: "the server doesn't send parts of the file, and the whole file is larger than the reader downloads",
    data: { reason: "whole-download", size, budget },
    docsUrl: docsUrl("source-budget-exceeded"),
  });
}

/** httpRangeSource reports a throwing fetch as `fetch-failed`; the cap's own error is its cause. */
function unwrapCap(e: unknown): unknown {
  return isLaminaPdfError(e, "fetch-failed") && isLaminaPdfError(e.cause, "source-budget-exceeded") ? e.cause : e;
}

/** A remote file as a `RangeSource` (for Lamina PDF) and a `RenderSource` (for the render backend). */
export class HttpRangeByteSource implements RangeSource, RenderSource {
  readonly size: number;
  readonly blockSize: number;
  readonly #inner: RangeSource;
  readonly #readahead: number;
  readonly #cacheBytes: number;
  readonly #onProgress: HttpRangeByteSourceOptions["onProgress"];
  /** Cached blocks by index, least recently used first (a Map keeps insertion order). */
  readonly #blocks = new Map<number, Uint8Array>();
  #cached = 0;
  /** Blocks being fetched, so two fills never fetch one block twice. */
  readonly #inFlight = new Map<number, Promise<void>>();
  /** The whole file, when the server sent it whole. */
  #whole: Uint8Array | undefined;
  #loaded = 0;

  private constructor(inner: RangeSource, options: HttpRangeByteSourceOptions) {
    this.#inner = inner;
    this.size = inner.size;
    this.blockSize = Math.max(1, Math.floor(options.blockSize ?? DEFAULTS.blockSize));
    this.#readahead = Math.max(0, Math.floor(options.readahead ?? DEFAULTS.readahead));
    this.#cacheBytes = options.cacheBytes ?? DEFAULTS.cacheBytes;
    this.#onProgress = options.onProgress;
  }

  /**
   * Probes `url` (its last 64 KiB: the size, and the part a reader reads first) and returns the
   * source. A server that ignores `Range` sends the whole file, which is kept.
   *
   * @throws LaminaPdfError `fetch-failed` (`data.reason` `network` covers a refused CORS check,
   *   which the browser doesn't tell apart from being offline), `source-changed`,
   *   `source-budget-exceeded`, or the signal's reason.
   */
  static async open(url: string, options: HttpRangeByteSourceOptions = {}): Promise<HttpRangeByteSource> {
    let whole = false;
    const fetcher = privateFetch(
      options.fetch ?? fetch,
      options.noStore ?? true,
      options.maxWholeBytes ?? DEFAULTS.maxWholeBytes,
      () => {
        whole = true;
      },
    );
    const inner = await httpRangeSource(url, {
      fetch: fetcher,
      blockSize: options.blockSize ?? DEFAULTS.blockSize,
      ...(options.signal === undefined ? {} : { signal: options.signal }),
    }).catch((e: unknown) => {
      throw unwrapCap(e);
    });
    const source = new HttpRangeByteSource(inner, options);
    if (whole) {
      // The probe's answer was the file: httpRangeSource holds it, and this reads it back once.
      source.#whole = await inner.read(0, inner.size);
      source.#progress(inner.size);
    } else {
      source.#progress(Math.min(inner.size, 65_536));
    }
    return source;
  }

  /** The whole file, when the server sent it whole (then every read is answered from it). */
  get whole(): Uint8Array | undefined {
    return this.#whole;
  }

  /** Bytes read from the remote file so far (the probe's included), re-reads of evicted blocks too. */
  get loaded(): number {
    return this.#loaded;
  }

  /** The `length` bytes at `offset` if every block is cached, else undefined (`RenderSource`). */
  readSync(offset: number, length: number): Uint8Array | undefined {
    const end = Math.min(this.size, offset + length);
    if (offset < 0 || end < offset) return undefined;
    if (this.#whole !== undefined) return this.#whole.subarray(offset, end);
    const first = Math.floor(offset / this.blockSize);
    const last = Math.floor((end - 1) / this.blockSize);
    if (end === offset) return new Uint8Array();
    if (first === last) {
      const block = this.#touch(first);
      if (block === undefined) return undefined;
      const at = first * this.blockSize;
      return block.subarray(offset - at, end - at);
    }
    const out = new Uint8Array(end - offset);
    for (let index = first; index <= last; index++) {
      const block = this.#touch(index);
      if (block === undefined) return undefined;
      const at = index * this.blockSize;
      const from = Math.max(offset, at) - at;
      const to = Math.min(end, at + block.length) - at;
      out.set(block.subarray(from, to), at + from - offset);
    }
    return out;
  }

  /**
   * Fetches the blocks `ranges` touch (and the readahead after each) that aren't cached: the
   * ranges of a render backend's `data-pending`.
   */
  async fill(ranges: readonly ByteRange[], signal?: AbortSignal): Promise<void> {
    if (this.#whole !== undefined) return;
    const wanted = this.#blocksFor(ranges);
    const waits: Promise<void>[] = [];
    let run: number[] = [];
    const flush = () => {
      if (run.length > 0) waits.push(this.#fetchRun(run, signal));
      run = [];
    };
    for (const index of wanted) {
      const pending = this.#inFlight.get(index);
      if (pending !== undefined || this.#blocks.has(index)) {
        flush();
        if (pending !== undefined) waits.push(pending);
        continue;
      }
      if (run.length > 0 && run[run.length - 1] !== index - 1) flush();
      run.push(index);
    }
    flush();
    await Promise.all(waits);
    // Keep what this fill asked for: evicting it now would make the caller miss again.
    this.#evict(new Set(wanted));
  }

  /** The blocks `ranges` touch, with the readahead after each, in order and once each. */
  #blocksFor(ranges: readonly ByteRange[]): number[] {
    const wanted = new Set<number>();
    const lastBlock = Math.max(0, Math.ceil(this.size / this.blockSize) - 1);
    for (const [offset, length] of ranges) {
      if (length <= 0) continue;
      const first = Math.floor(offset / this.blockSize);
      const last = Math.min(lastBlock, Math.floor((offset + length - 1) / this.blockSize) + this.#readahead);
      for (let index = first; index <= last; index++) wanted.add(index);
    }
    return [...wanted].sort((a, b) => a - b);
  }

  /** `RangeSource.read`: fills, then reads from the cache. */
  async read(offset: number, length: number, options?: { readonly signal?: AbortSignal }): Promise<Uint8Array> {
    const cached = this.readSync(offset, length);
    if (cached !== undefined && cached.length === length) return cached.slice();
    await this.fill([[offset, length]], options?.signal);
    const bytes = this.readSync(offset, length);
    if (bytes === undefined) throw new Error("a range read missed blocks it had just fetched");
    return bytes.slice();
  }

  /** Fetches the consecutive blocks `run` in one request. */
  #fetchRun(run: readonly number[], signal: AbortSignal | undefined): Promise<void> {
    const first = run[0] ?? 0;
    const last = run[run.length - 1] ?? first;
    const start = first * this.blockSize;
    const end = Math.min(this.size, (last + 1) * this.blockSize);
    const fetching = this.#inner.read(start, end - start, signal === undefined ? {} : { signal }).then(
      (bytes) => {
        for (const index of run) {
          const at = index * this.blockSize - start;
          this.#store(index, bytes.slice(at, Math.min(bytes.length, at + this.blockSize)));
        }
        this.#progress(bytes.length);
      },
      (e: unknown) => {
        throw unwrapCap(e);
      },
    );
    const done = fetching.finally(() => {
      for (const index of run) if (this.#inFlight.get(index) === done) this.#inFlight.delete(index);
    });
    for (const index of run) this.#inFlight.set(index, done);
    return done;
  }

  #store(index: number, block: Uint8Array): void {
    const old = this.#blocks.get(index);
    if (old !== undefined) this.#cached -= old.length;
    this.#blocks.delete(index);
    this.#blocks.set(index, block);
    this.#cached += block.length;
  }

  /** Block `index`, marked as used now; undefined when it isn't cached. */
  #touch(index: number): Uint8Array | undefined {
    const block = this.#blocks.get(index);
    if (block !== undefined) {
      this.#blocks.delete(index);
      this.#blocks.set(index, block);
    }
    return block;
  }

  /** Drops the least recently used blocks over the budget, never one in `keep`. */
  #evict(keep: ReadonlySet<number>): void {
    for (const [index, block] of this.#blocks) {
      if (this.#cached <= this.#cacheBytes) return;
      if (keep.has(index)) continue;
      this.#blocks.delete(index);
      this.#cached -= block.length;
    }
  }

  #progress(bytes: number): void {
    this.#loaded += bytes;
    this.#onProgress?.({ loaded: this.#loaded, total: this.size });
  }
}
