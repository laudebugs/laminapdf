// httpRangeSource: a remote PDF read with HTTP Range requests. Errors never carry the URL
// (ADR 0026 §4): it may hold tokens.
import { packageError } from "../errors.js";
import { type RangeSource, WHOLE } from "./range.js";

/**
 * Options for {@link httpRangeSource}.
 *
 * @since 0.2.0
 */
export interface HttpRangeOptions {
  /** Extra request headers (authorisation, …). `Range` and `If-Range` are Lamina PDF's. */
  readonly headers?: Readonly<Record<string, string>>;
  /** `fetch`'s credentials mode. */
  readonly credentials?: RequestCredentials;
  /** Aborts the probe; reads take their own signal. */
  readonly signal?: AbortSignal;
  /** Fetches are aligned to blocks of this many bytes (default 16 384: each object a reader
   * checks costs one block, so larger blocks fetch more of the file). */
  readonly blockSize?: number;
  /** The first request asks for this many bytes at the end of the file (default 65 536). */
  readonly tail?: number;
  /** The `fetch` to use (default: the global one). */
  readonly fetch?: typeof fetch;
}

/** `bytes start-end/size` → its numbers. */
function contentRange(header: string | null): { start: number; end: number; size: number } | undefined {
  const m = /^bytes (\d+)-(\d+)\/(\d+)$/.exec(header?.trim() ?? "");
  if (m === null) return undefined;
  return { start: Number(m[1]), end: Number(m[2]), size: Number(m[3]) };
}

function failed(reason: "network" | "status" | "content-range", status?: number, cause?: unknown) {
  const e = packageError("fetch-failed", "fetching a range of the remote file failed", {
    data: status === undefined ? { reason } : { reason, status },
  });
  if (cause !== undefined) Object.defineProperty(e, "cause", { value: cause });
  return e;
}

function changed() {
  return packageError("source-changed", "the remote file changed while it was being read");
}

/**
 * A {@link RangeSource} over a URL, for `Document.openRange`. The first request asks for the
 * last `tail` bytes, which gives the size and the part of the file a reader needs first (the
 * xref is at the end). Every later request sends `If-Range` with the first strong `ETag` (else
 * its `Last-Modified`); a response whose `ETag`, `Last-Modified` or total size differs fails
 * with `source-changed`. A server that ignores
 * `Range` (it answers `200`) is read with that one full download.
 *
 * The server must allow `Range` (across origins: `Access-Control-Allow-Headers: Range, If-Range` and
 * `Access-Control-Expose-Headers: Content-Range, Content-Length, ETag`) and must not compress
 * the response.
 *
 * @param url - the file.
 * @param options - headers, credentials, signal, block size.
 * @returns the source, once the first request has answered.
 * @throws {@link LaminaPdfError} `fetch-failed` (with `data.status`), `source-changed`.
 *
 * @since 0.2.0
 */
export async function httpRangeSource(url: string | URL, options: HttpRangeOptions = {}): Promise<RangeSource> {
  // A whole number of bytes, at least 1 (a fraction would make a malformed Range header).
  const tail = Number.isFinite(options.tail) ? Math.max(1, Math.floor(options.tail ?? 0)) : 65_536;
  const request = rangeRequester(url, options);
  const first = await request(`bytes=-${tail}`, options.signal);
  if (first.status === 200) {
    // No range support: this response is the whole file.
    const whole = new Uint8Array(await first.arrayBuffer());
    // Tagged, so Document.openRange opens these bytes directly instead of copying them again.
    return {
      size: whole.length,
      read: async (offset, length) => whole.slice(offset, offset + length),
      [WHOLE]: whole,
    } as RangeSource;
  }
  if (first.status === 416 && /^bytes \*\/0$/.test(first.headers.get("Content-Range")?.trim() ?? "")) {
    // An empty file has no last bytes to send: open it as empty (Lamina PDF then says what it is).
    await first.body?.cancel();
    return { size: 0, read: async () => new Uint8Array() };
  }
  const file = new RangedFile(request, await readProbe(first));
  return {
    size: file.size,
    read: (offset, length, opts) => file.read(offset, length, opts?.signal),
    blockSize: options.blockSize ?? 16_384,
  };
}

/** Cancels `res`'s body (nothing will read it) and rejects with `error`. */
async function discard(res: Response, error: Error): Promise<never> {
  await res.body?.cancel();
  throw error;
}

/** Sends one ranged request, with `If-Range` when given. */
type RangeRequest = (range: string, signal: AbortSignal | undefined, ifRange?: string) => Promise<Response>;

function rangeRequester(url: string | URL, options: HttpRangeOptions): RangeRequest {
  const doFetch = options.fetch ?? fetch;
  return async (range, signal, ifRange) => {
    const headers = new Headers(options.headers);
    headers.set("Range", range);
    if (ifRange !== undefined) headers.set("If-Range", ifRange);
    try {
      return await doFetch(url, {
        headers,
        ...(options.credentials === undefined ? {} : { credentials: options.credentials }),
        ...(signal === undefined ? {} : { signal }),
      });
    } catch (e) {
      if (signal?.aborted) throw signal.reason;
      throw failed("network", undefined, e);
    }
  };
}

/** What the first ranged answer told us: the size, the tail it sent and the file's version. */
interface Probe {
  readonly size: number;
  readonly tailStart: number;
  readonly tailBytes: Uint8Array;
  readonly etag: string | undefined;
  readonly lastModified: string | undefined;
}

/** Checks the first answer is a `206` with the file's last bytes, and reads them. */
async function readProbe(first: Response): Promise<Probe> {
  if (first.status !== 206) {
    return discard(first, failed("status", first.status));
  }
  const range = contentRange(first.headers.get("Content-Range"));
  if (range === undefined || range.end !== range.size - 1) {
    return discard(first, failed("content-range", 206));
  }
  const tailBytes = new Uint8Array(await first.arrayBuffer());
  if (tailBytes.length !== range.size - range.start) throw failed("content-range", 206);
  return {
    size: range.size,
    tailStart: range.start,
    tailBytes,
    etag: first.headers.get("ETag") ?? undefined,
    lastModified: first.headers.get("Last-Modified") ?? undefined,
  };
}

/** The reads after the probe: from its tail, by ranged requests, or from one whole download. */
class RangedFile {
  readonly #request: RangeRequest;
  readonly #probe: Probe;
  // If-Range needs a strong ETag, else a Last-Modified date; a weak ETag is only compared.
  readonly #ifRange: string | undefined;
  // Set once a ranged request is answered 200: the whole file, downloaded once and shared by
  // every read (reads already in flight cancel their own bodies and wait for it).
  #whole: Promise<Uint8Array> | undefined;

  constructor(request: RangeRequest, probe: Probe) {
    this.#request = request;
    this.#probe = probe;
    const { etag, lastModified } = probe;
    this.#ifRange = etag !== undefined && !etag.startsWith("W/") ? etag : lastModified;
  }

  get size(): number {
    return this.#probe.size;
  }

  async read(offset: number, length: number, signal: AbortSignal | undefined): Promise<Uint8Array> {
    const { tailStart, tailBytes } = this.#probe;
    const end = offset + length;
    if (offset >= tailStart) return tailBytes.slice(offset - tailStart, end - tailStart);
    if (end <= tailStart) return this.#fetchRange(offset, end, signal);
    // Straddles the tail the probe brought: fetch only the part before it.
    const out = new Uint8Array(length);
    out.set(await this.#fetchRange(offset, tailStart, signal));
    out.set(tailBytes.subarray(0, end - tailStart), tailStart - offset);
    return out;
  }

  /** Whether `res` shows a different version than the first response (ETag, then Last-Modified). */
  #differs(res: Response): boolean {
    const { etag, lastModified } = this.#probe;
    const tag = res.headers.get("ETag") ?? undefined;
    if (etag !== undefined && tag !== undefined) return tag !== etag;
    const modified = res.headers.get("Last-Modified") ?? undefined;
    return lastModified !== undefined && modified !== undefined && modified !== lastModified;
  }

  /** The bytes `offset..end` from the network (or from the whole file, once there is one). */
  async #fetchRange(offset: number, end: number, signal: AbortSignal | undefined): Promise<Uint8Array> {
    if (this.#whole !== undefined) return (await this.#whole).slice(offset, end);
    const res = await this.#request(`bytes=${offset}-${end - 1}`, signal, this.#ifRange);
    if (this.#differs(res)) {
      return discard(res, changed());
    }
    if (res.status === 200) return this.#fromWhole(res, offset, end);
    return this.#fromPartial(res, offset, end);
  }

  /** The server ignored Range (or If-Range found the file changed and sent it whole). */
  async #fromWhole(res: Response, offset: number, end: number): Promise<Uint8Array> {
    if (this.#probe.etag !== undefined && res.headers.get("ETag") === null) {
      return discard(res, changed());
    }
    let whole = this.#whole;
    if (whole === undefined) whole = this.#download(res);
    else await res.body?.cancel();
    return (await whole).slice(offset, end);
  }

  #download(res: Response): Promise<Uint8Array> {
    const download = res.arrayBuffer().then((buffer) => {
      const bytes = new Uint8Array(buffer);
      if (bytes.length !== this.#probe.size) throw changed();
      return bytes;
    });
    this.#whole = download;
    // A failed or aborted download (its read's signal) must not stick: the next read that
    // gets a 200 starts a new one.
    download.catch(() => {
      if (this.#whole === download) this.#whole = undefined;
    });
    return download;
  }

  /** A `206` with exactly the bytes asked for, from the same file. */
  async #fromPartial(res: Response, offset: number, end: number): Promise<Uint8Array> {
    if (res.status !== 206) {
      return discard(res, failed("status", res.status));
    }
    const got = contentRange(res.headers.get("Content-Range"));
    if (got !== undefined && got.size !== this.#probe.size) {
      return discard(res, changed());
    }
    if (got === undefined || got.start !== offset || got.end !== end - 1) {
      return discard(res, failed("content-range", 206));
    }
    const bytes = new Uint8Array(await res.arrayBuffer());
    if (bytes.length !== end - offset) throw failed("content-range", 206);
    return bytes;
  }
}
