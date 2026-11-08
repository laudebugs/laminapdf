import { isLaminaPdfError } from "@laminapdf/sdk";
import { beforeEach, describe, expect, it } from "vitest";
import { HttpRangeByteSource } from "./http-range.js";

/** A 1 MiB file whose byte i is i mod 251, so any slice can be checked. */
const FILE = Uint8Array.from({ length: 1024 * 1024 }, (_, i) => i % 251);

/** How the test server answers, switched per test. */
interface Behaviour {
  ranges: boolean;
  etag: string;
  /** Answers ranged requests after the probe with a Content-Range one byte off. */
  badRange: boolean;
  delayMs: number;
}

let behaviour: Behaviour;
let requests: { range: string | undefined; bytes: number }[] = [];
/** Every request's options, as the source passed them. */
const inits: RequestInit[] = [];

/** The server's answer to a request with header `range`. */
function answer(range: string | undefined): Response {
  const headers: Record<string, string> = { ETag: behaviour.etag };
  const m = /^bytes=(\d*)-(\d*)$/.exec(range ?? "");
  if (!behaviour.ranges || m === null) {
    requests.push({ range, bytes: FILE.length });
    return new Response(FILE.slice(), { status: 200, headers: { ...headers, "Content-Length": `${FILE.length}` } });
  }
  const start = m[1] === "" ? FILE.length - Number(m[2]) : Number(m[1]);
  const end = m[1] === "" || m[2] === "" ? FILE.length - 1 : Math.min(FILE.length - 1, Number(m[2]));
  const shift = behaviour.badRange && m[1] !== "" ? 1 : 0;
  headers["Content-Range"] = `bytes ${start + shift}-${end}/${FILE.length}`;
  requests.push({ range, bytes: end - start + 1 });
  return new Response(FILE.slice(start, end + 1), { status: 206, headers });
}

/** A fetch that answers as a file server would, after `delayMs`, and records every request. */
const server: typeof fetch = (_input, init = {}) => {
  inits.push(init);
  const range = new Headers(init.headers).get("Range") ?? undefined;
  const { signal } = init;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => resolve(answer(range)), behaviour.delayMs);
    signal?.addEventListener("abort", () => {
      clearTimeout(timer);
      reject(signal.reason);
    });
  });
};

const url = "https://files.example/file.pdf";

beforeEach(() => {
  behaviour = { ranges: true, etag: '"v1"', badRange: false, delayMs: 0 };
  requests = [];
  inits.length = 0;
});

const open = (options: Parameters<typeof HttpRangeByteSource.open>[1] = {}) =>
  HttpRangeByteSource.open(url, { fetch: server, blockSize: 4096, readahead: 0, ...options });

describe("HttpRangeByteSource", () => {
  it("fetches only the blocks a read touches, then answers it synchronously", async () => {
    const source = await open();
    expect(source.size).toBe(FILE.length);
    expect(source.readSync(10_000, 100)).toBeUndefined();
    await source.fill([[10_000, 100]]);
    expect(source.readSync(10_000, 100)).toEqual(FILE.subarray(10_000, 10_100));
    // The probe, then one block.
    expect(requests.map((r) => r.range)).toEqual(["bytes=-65536", "bytes=8192-12287"]);
  });

  it("reads across blocks and from the probe's tail", async () => {
    const source = await open();
    expect(await source.read(4000, 200)).toEqual(FILE.subarray(4000, 4200));
    const tail = FILE.length - 100;
    expect(await source.read(tail, 100)).toEqual(FILE.subarray(tail));
  });

  it("sends no credentials and no referrer, and skips the HTTP cache", async () => {
    const source = await open();
    await source.fill([[0, 1]]);
    for (const init of inits) {
      expect(init).toMatchObject({
        credentials: "omit",
        referrerPolicy: "no-referrer",
        mode: "cors",
        cache: "no-store",
      });
    }
  });

  it("fetches the readahead after each range", async () => {
    const source = await open({ readahead: 2 });
    await source.fill([[0, 10]]);
    expect(requests.at(-1)?.range).toBe("bytes=0-12287");
    expect(source.readSync(8192, 4096)).toEqual(FILE.subarray(8192, 12_288));
  });

  it("fetches a block once when two fills ask for it at the same time", async () => {
    behaviour.delayMs = 20;
    const source = await open();
    await Promise.all([source.fill([[0, 10]]), source.fill([[5, 10]])]);
    expect(requests.filter((r) => r.range === "bytes=0-4095")).toHaveLength(1);
  });

  it("evicts the least recently used blocks over its budget, never those just asked for", async () => {
    const source = await open({ cacheBytes: 8192 });
    await source.fill([[0, 1]]);
    await source.fill([[4096, 1]]);
    source.readSync(0, 1);
    await source.fill([[8192, 1]]);
    expect(source.readSync(0, 1)).toBeDefined();
    expect(source.readSync(4096, 1)).toBeUndefined();
    expect(source.readSync(8192, 1)).toBeDefined();
  });

  it("downloads the file whole when the server ignores Range", async () => {
    behaviour.ranges = false;
    const source = await open();
    expect(source.whole).toEqual(FILE);
    expect(source.readSync(123, 4)).toEqual(FILE.subarray(123, 127));
    expect(requests).toHaveLength(1);
  });

  it("refuses a whole download larger than its cap before reading it", async () => {
    behaviour.ranges = false;
    const error = await open({ maxWholeBytes: 1000 }).catch((e: unknown) => e);
    expect(isLaminaPdfError(error, "source-budget-exceeded")).toBe(true);
  });

  it("fails on a Content-Range that isn't the range asked for", async () => {
    const source = await open();
    behaviour.badRange = true;
    const error = await source.fill([[0, 1]]).catch((e: unknown) => e);
    expect(isLaminaPdfError(error, "fetch-failed")).toBe(true);
  });

  it("fails with source-changed when the file changes between reads", async () => {
    const source = await open();
    behaviour.etag = '"v2"';
    const error = await source.fill([[0, 1]]).catch((e: unknown) => e);
    expect(isLaminaPdfError(error, "source-changed")).toBe(true);
  });

  it("stops a slow fetch when its signal aborts", async () => {
    const source = await open();
    behaviour.delayMs = 5_000;
    const controller = new AbortController();
    const filling = source.fill([[0, 1]], controller.signal);
    controller.abort(new Error("stop"));
    await expect(filling).rejects.toThrow("stop");
  });

  it("reports the bytes read so far", async () => {
    const seen: number[] = [];
    const source = await open({ onProgress: ({ loaded, total }) => seen.push(loaded / total) });
    await source.fill([[0, 1]]);
    expect(seen).toEqual([65_536 / FILE.length, (65_536 + 4096) / FILE.length]);
  });
});
