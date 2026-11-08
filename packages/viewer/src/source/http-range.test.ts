import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { isLaminaPdfError } from "@laminapdf/sdk";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { HttpRangeByteSource } from "./http-range.js";

/** A 1 MiB file whose byte i is i mod 251, so any slice can be checked. */
const FILE = Uint8Array.from({ length: 1024 * 1024 }, (_, i) => i % 251);

/** How the test server answers, switched per test. */
interface Behaviour {
  ranges: boolean;
  etag: string;
  /** Answers every ranged request after the probe with this Content-Range instead. */
  badRange: boolean;
  delayMs: number;
}

let behaviour: Behaviour;
let requests: { range: string | undefined; bytes: number }[] = [];
let server: Server;
let url: string;

function answer(req: IncomingMessage, res: ServerResponse): void {
  const range = req.headers.range;
  const send = () => {
    res.setHeader("ETag", behaviour.etag);
    const m = /^bytes=(\d*)-(\d*)$/.exec(range ?? "");
    if (!behaviour.ranges || m === null) {
      res.writeHead(200, { "Content-Length": FILE.length });
      requests.push({ range, bytes: FILE.length });
      res.end(FILE);
      return;
    }
    const start = m[1] === "" ? FILE.length - Number(m[2]) : Number(m[1]);
    const end = m[1] === "" || m[2] === "" ? FILE.length - 1 : Math.min(FILE.length - 1, Number(m[2]));
    const header =
      behaviour.badRange && m[1] !== ""
        ? `bytes ${start + 1}-${end}/${FILE.length}`
        : `bytes ${start}-${end}/${FILE.length}`;
    res.writeHead(206, { "Content-Range": header, "Content-Length": end - start + 1 });
    requests.push({ range, bytes: end - start + 1 });
    res.end(FILE.subarray(start, end + 1));
  };
  if (behaviour.delayMs > 0) setTimeout(send, behaviour.delayMs);
  else send();
}

/** The global fetch with the options Node's fetch doesn't take dropped, and every init recorded. */
const inits: RequestInit[] = [];
const recording: typeof fetch = (input, init = {}) => {
  inits.push(init);
  const { cache: _cache, mode: _mode, ...rest } = init;
  return fetch(input, rest);
};

beforeAll(async () => {
  server = createServer(answer);
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  url = `http://127.0.0.1:${(server.address() as AddressInfo).port}/file.pdf`;
});

afterAll(() => {
  server.closeAllConnections();
  server.close();
});

beforeEach(() => {
  behaviour = { ranges: true, etag: '"v1"', badRange: false, delayMs: 0 };
  requests = [];
  inits.length = 0;
});

const open = (options: Parameters<typeof HttpRangeByteSource.open>[1] = {}) =>
  HttpRangeByteSource.open(url, { fetch: recording, blockSize: 4096, readahead: 0, ...options });

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
