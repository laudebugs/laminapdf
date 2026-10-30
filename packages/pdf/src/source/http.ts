// httpRangeSource (L2c): a remote PDF read with HTTP Range requests. Errors never carry the URL
// (ADR 0026 §4): it may hold tokens.
import { packageError } from "../errors.js";
import type { RangeSource } from "./range.js";

/** Options for {@link httpRangeSource}. */
export interface HttpRangeOptions {
  /** Extra request headers (authorisation, …). `Range` and `If-Range` are lamina's. */
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
 * xref is at the end). Every later request sends `If-Range` with the first strong `ETag`; a
 * response whose `ETag` or total size differs fails with `source-changed`. A server that ignores
 * `Range` (it answers `200`) is read with that one full download.
 *
 * The server must allow `Range` (across origins: `Access-Control-Allow-Headers: Range` and
 * `Access-Control-Expose-Headers: Content-Range, Content-Length, ETag`) and must not compress
 * the response.
 *
 * @param url - the file.
 * @param options - headers, credentials, signal, block size.
 * @returns the source, once the first request has answered.
 * @throws {@link LaminaError} `fetch-failed` (with `data.status`), `source-changed`.
 */
export async function httpRangeSource(url: string | URL, options: HttpRangeOptions = {}): Promise<RangeSource> {
  const doFetch = options.fetch ?? fetch;
  // A whole number of bytes, at least 1 (a fraction would make a malformed Range header).
  const tail = Number.isFinite(options.tail) ? Math.max(1, Math.floor(options.tail ?? 0)) : 65_536;
  const request = async (range: string, signal: AbortSignal | undefined, ifRange?: string): Promise<Response> => {
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

  const first = await request(`bytes=-${tail}`, options.signal);
  if (first.status === 200) {
    // No range support: this response is the whole file.
    const whole = new Uint8Array(await first.arrayBuffer());
    return {
      size: whole.length,
      read: async (offset, length) => whole.slice(offset, offset + length),
    };
  }
  if (first.status === 416 && /^bytes \*\/0$/.test(first.headers.get("Content-Range")?.trim() ?? "")) {
    // An empty file has no last bytes to send: open it as empty (lamina then says what it is).
    await first.body?.cancel();
    return { size: 0, read: async () => new Uint8Array() };
  }
  if (first.status !== 206) {
    await first.body?.cancel();
    throw failed("status", first.status);
  }
  const range = contentRange(first.headers.get("Content-Range"));
  if (range === undefined || range.end !== range.size - 1) throw failed("content-range", 206);
  const size = range.size;
  const tailStart = range.start;
  const tailBytes = new Uint8Array(await first.arrayBuffer());
  if (tailBytes.length !== size - tailStart) throw failed("content-range", 206);
  const etag = first.headers.get("ETag") ?? undefined;
  // If-Range needs a strong validator; a weak one is only compared.
  const ifRange = etag !== undefined && !etag.startsWith("W/") ? etag : undefined;
  // Set once a ranged request is answered 200: the whole file, downloaded once and shared by
  // every read (reads already in flight cancel their own bodies and wait for it).
  let whole: Promise<Uint8Array> | undefined;

  /** The bytes `offset..end` from the network (or from the whole file, once there is one). */
  const fetchRange = async (offset: number, end: number, signal: AbortSignal | undefined): Promise<Uint8Array> => {
    if (whole !== undefined) return (await whole).slice(offset, end);
    const res = await request(`bytes=${offset}-${end - 1}`, signal, ifRange);
    const tag = res.headers.get("ETag") ?? undefined;
    if (etag !== undefined && tag !== undefined && tag !== etag) {
      await res.body?.cancel();
      throw changed();
    }
    if (res.status === 200) {
      // The server ignored Range (or If-Range found the file changed and sent it whole).
      if (etag !== undefined && tag === undefined) {
        await res.body?.cancel();
        throw changed();
      }
      if (whole === undefined) {
        whole = res.arrayBuffer().then((buffer) => {
          const bytes = new Uint8Array(buffer);
          if (bytes.length !== size) throw changed();
          return bytes;
        });
      } else {
        await res.body?.cancel();
      }
      return (await whole).slice(offset, end);
    }
    if (res.status !== 206) {
      await res.body?.cancel();
      throw failed("status", res.status);
    }
    const got = contentRange(res.headers.get("Content-Range"));
    if (got !== undefined && got.size !== size) {
      await res.body?.cancel();
      throw changed();
    }
    if (got === undefined || got.start !== offset || got.end !== end - 1) {
      await res.body?.cancel();
      throw failed("content-range", 206);
    }
    const bytes = new Uint8Array(await res.arrayBuffer());
    if (bytes.length !== end - offset) throw failed("content-range", 206);
    return bytes;
  };

  const read = async (offset: number, length: number, opts?: { readonly signal?: AbortSignal }) => {
    const end = offset + length;
    if (offset >= tailStart) return tailBytes.slice(offset - tailStart, end - tailStart);
    if (end <= tailStart) return fetchRange(offset, end, opts?.signal);
    // Straddles the tail the probe brought: fetch only the part before it.
    const out = new Uint8Array(length);
    out.set(await fetchRange(offset, tailStart, opts?.signal));
    out.set(tailBytes.subarray(0, end - tailStart), tailStart - offset);
    return out;
  };

  return { size, read, blockSize: options.blockSize ?? 16_384 };
}
