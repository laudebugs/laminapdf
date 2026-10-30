// Range documents (L2c): httpRangeSource against a local server that logs requests,
// callbackSource, Document.openRange and doc.ensure.
import { readdirSync } from "node:fs";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { callbackSource, Document, httpRangeSource, init, isLaminaError, type LaminaError } from "@lamina/pdf";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(() => init());

/** A PDF of `pages` pages with a ~40 KiB content stream each, streams first, xref at the end. */
function largePdf(pages = 32): { bytes: Uint8Array; contents: [number, number][] } {
  const enc = new TextEncoder();
  const parts: Uint8Array[] = [];
  let at = 0;
  const offsets: number[] = [];
  const contents: [number, number][] = [];
  const push = (s: string | Uint8Array) => {
    const b = typeof s === "string" ? enc.encode(s) : s;
    parts.push(b);
    at += b.length;
  };
  push("%PDF-1.7\n");
  // Objects 1..pages: content streams; pages+1..2*pages: page dicts; then Pages, Catalog.
  for (let i = 0; i < pages; i++) {
    const line = `BT /F1 12 Tf 72 720 Td (page ${i}) Tj ET\n`;
    const data = line.repeat(Math.floor((40 * 1024) / line.length));
    offsets.push(at);
    const start = at;
    push(`${i + 1} 0 obj\n<< /Length ${data.length} >>\nstream\n${data}\nendstream\nendobj\n`);
    contents.push([start, at]);
  }
  const pagesNum = 2 * pages + 1;
  for (let i = 0; i < pages; i++) {
    offsets.push(at);
    push(`${pages + i + 1} 0 obj\n<< /Type /Page /Parent ${pagesNum} 0 R /Contents ${i + 1} 0 R >>\nendobj\n`);
  }
  const kids = Array.from({ length: pages }, (_, i) => `${pages + i + 1} 0 R`).join(" ");
  offsets.push(at);
  push(`${pagesNum} 0 obj\n<< /Type /Pages /Kids [${kids}] /Count ${pages} /MediaBox [0 0 612 792] >>\nendobj\n`);
  offsets.push(at);
  push(`${pagesNum + 1} 0 obj\n<< /Type /Catalog /Pages ${pagesNum} 0 R >>\nendobj\n`);
  const xref = at;
  const size = offsets.length + 1;
  push(`xref\n0 ${size}\n0000000000 65535 f \n`);
  for (const o of offsets) push(`${String(o).padStart(10, "0")} 00000 n \n`);
  push(`trailer\n<< /Size ${size} /Root ${pagesNum + 1} 0 R >>\nstartxref\n${xref}\n%%EOF\n`);
  const bytes = new Uint8Array(at);
  let o = 0;
  for (const p of parts) {
    bytes.set(p, o);
    o += p.length;
  }
  return { bytes, contents };
}

interface Logged {
  range: string | undefined;
  status: number;
  start: number;
  end: number;
  closed: boolean;
}

/** A local file server: Range on or off, an ETag that can change, an optional delay. */
class FileServer {
  readonly log: Logged[] = [];
  bytes: Uint8Array = new Uint8Array();
  ranges = true;
  /** Answer only the first ranged request with 206, later ones with 200 (a CDN that does that). */
  rangesOnlyFirst = false;
  etag = '"v1"';
  delayMs = 0;
  #server: Server;
  url = "";

  constructor() {
    this.#server = createServer((req, res) => this.#handle(req, res));
  }

  async start(): Promise<void> {
    await new Promise<void>((resolve) => this.#server.listen(0, "127.0.0.1", resolve));
    this.url = `http://127.0.0.1:${(this.#server.address() as AddressInfo).port}/file.pdf`;
  }

  async stop(): Promise<void> {
    this.#server.closeAllConnections();
    await new Promise<void>((resolve) => this.#server.close(() => resolve()));
  }

  serve(bytes: Uint8Array): this {
    this.bytes = bytes;
    this.log.length = 0;
    this.ranges = true;
    this.rangesOnlyFirst = false;
    this.etag = '"v1"';
    this.delayMs = 0;
    return this;
  }

  /** Bytes sent in 206 responses. */
  get sent(): number {
    return this.log.filter((l) => l.status === 206).reduce((n, l) => n + (l.end - l.start), 0);
  }

  #handle(req: IncomingMessage, res: ServerResponse): void {
    const size = this.bytes.length;
    const header = req.headers.range;
    const entry: Logged = { range: header, status: 0, start: 0, end: 0, closed: false };
    this.log.push(entry);
    req.on("close", () => {
      entry.closed = true;
    });
    const send = () => {
      if (res.destroyed) return;
      res.setHeader("ETag", this.etag);
      res.setHeader("Accept-Ranges", "bytes");
      const ifRange = req.headers["if-range"];
      const m = header === undefined ? null : /^bytes=(\d*)-(\d*)$/.exec(header);
      const later = this.rangesOnlyFirst && this.log.length > 1;
      if (size === 0 && m !== null) {
        entry.status = 416;
        res.writeHead(416, { "Content-Range": "bytes */0" });
        res.end();
        return;
      }
      if (!this.ranges || later || m === null || (ifRange !== undefined && ifRange !== this.etag)) {
        entry.status = 200;
        entry.end = size;
        res.writeHead(200, { "Content-Length": size });
        res.end(this.bytes);
        return;
      }
      let start: number;
      let end: number;
      if (m[1] === "") {
        start = Math.max(0, size - Number(m[2]));
        end = size;
      } else {
        start = Number(m[1]);
        end = m[2] === "" ? size : Math.min(size, Number(m[2]) + 1);
      }
      Object.assign(entry, { status: 206, start, end });
      res.writeHead(206, { "Content-Range": `bytes ${start}-${end - 1}/${size}`, "Content-Length": end - start });
      res.end(this.bytes.subarray(start, end));
    };
    if (this.delayMs > 0) setTimeout(send, this.delayMs);
    else send();
  }
}

const server = new FileServer();
beforeAll(() => server.start());
afterAll(() => server.stop());

/** What a reader can tell about `doc`, through `ensure` when it is a range document. */
async function facts(doc: Document): Promise<unknown[]> {
  const pages = await doc.ensure(() => doc.pages);
  const info = await doc.ensure(() => doc.info);
  const saved = await doc.ensure(() => doc.save({ mode: "fullRewrite" }));
  return [pages, info, Array.from(saved)];
}

const corpus = readdirSync(new URL("../../../fixtures/corpus/", import.meta.url))
  .filter((f) => f.endsWith(".pdf"))
  .sort();

describe("Document.openRange over HTTP", () => {
  it.each(corpus)("reads %s like Document.open", async (name) => {
    const bytes = read(`fixtures/corpus/${name}`);
    server.serve(bytes);
    let expected: unknown;
    try {
      expected = await facts(Document.open(bytes));
    } catch (e) {
      expected = (e as LaminaError).code;
    }
    let ours: unknown;
    try {
      ours = await facts(await Document.openRange(httpRangeSource(server.url)));
    } catch (e) {
      ours = (e as LaminaError).code;
    }
    expect(ours).toEqual(expected);
  });

  it("fetches less than a large file to open it and list its pages, and nothing of the page contents", async () => {
    const { bytes, contents } = largePdf();
    server.serve(bytes);
    const doc = await Document.openRange(httpRangeSource(server.url));
    expect(await doc.ensure(() => doc.pageCount)).toBe(32);
    expect(await doc.ensure(() => doc.pageInfo(20).width)).toBe(612);
    expect(server.sent).toBeLessThan(bytes.length / 2);
    // Open reads the tail (the first request) and the head window. Every other request is for
    // the xref, the catalog, the page tree and the page objects: none reaches into a content
    // stream's data beyond the blocks at its ends.
    const others = server.log.filter((l, i) => i > 0 && l.start !== 0);
    expect(others.length).toBeGreaterThan(0);
    for (const l of others) {
      for (const [start, end] of contents) {
        // One 16 KiB block of slack at each end: blocks holding the neighbouring headers.
        const overlap = Math.min(end - 16_384, l.end) - Math.max(start + 16_384, l.start);
        expect(overlap, `${l.range} vs content ${start}-${end}`).toBeLessThanOrEqual(0);
      }
    }
  });

  it("matches Document.open on a large file, saving it whole through ensure", async () => {
    const { bytes } = largePdf();
    server.serve(bytes);
    const ours = await facts(await Document.openRange(httpRangeSource(server.url)));
    expect(ours).toEqual(await facts(Document.open(bytes)));
  });

  it("asks for the tail first", async () => {
    const { bytes } = largePdf();
    server.serve(bytes);
    await Document.openRange(httpRangeSource(server.url));
    expect(server.log[0]?.range).toBe("bytes=-65536");
  });

  it("opens with one full download when the server ignores Range", async () => {
    const { bytes } = largePdf(4);
    server.serve(bytes).ranges = false;
    const doc = await Document.openRange(httpRangeSource(server.url));
    expect(await doc.ensure(() => doc.pageCount)).toBe(4);
    await doc.ensure(() => doc.save({ mode: "fullRewrite" }));
    expect(server.log.map((l) => l.status)).toEqual([200]);
  });

  it("downloads the file once when ranged requests after the first are answered 200", async () => {
    const { bytes } = largePdf();
    server.serve(bytes).rangesOnlyFirst = true;
    const doc = await Document.openRange(httpRangeSource(server.url));
    await doc.ensure(() => doc.save({ mode: "fullRewrite" }));
    // Reads in flight when the first 200 arrived cancel their bodies; later reads use the download.
    expect(server.log.slice(1).every((l) => l.status === 200)).toBe(true);
    expect(server.log.length).toBeLessThan(10);
  });

  it("never fetches over the network what the tail probe already brought", async () => {
    const { bytes } = largePdf();
    server.serve(bytes);
    const doc = await Document.openRange(httpRangeSource(server.url));
    await doc.ensure(() => doc.save({ mode: "fullRewrite" }));
    const tailStart = bytes.length - 65_536;
    for (const l of server.log.slice(1)) expect(l.end, l.range).toBeLessThanOrEqual(tailStart);
  });

  it("opens an empty remote file as empty (the reader then says it isn't a PDF)", async () => {
    server.serve(new Uint8Array());
    const e = await Document.openRange(httpRangeSource(server.url)).catch((x: unknown) => x);
    expect(isLaminaError(e) && e.code !== "fetch-failed").toBe(true);
    expect(server.log.map((l) => l.status)).toEqual([416]);
  });

  it("fails with source-changed when the ETag changes mid-read", async () => {
    const { bytes } = largePdf();
    server.serve(bytes);
    const doc = await Document.openRange(httpRangeSource(server.url));
    server.etag = '"v2"';
    const e = await doc.ensure(() => doc.save({ mode: "fullRewrite" })).catch((x: unknown) => x);
    expect(isLaminaError(e, "source-changed")).toBe(true);
    expect(JSON.stringify(e)).not.toContain(server.url);
  });

  it("fails with fetch-failed and the status, never the URL, for an error response", async () => {
    const e = await httpRangeSource(`${server.url}?token=secret`, {
      fetch: async () => new Response("nope", { status: 403 }),
    }).catch((x: unknown) => x);
    expect(isLaminaError(e, "fetch-failed") && [e.data, e.message.includes("secret")]).toEqual([
      { reason: "status", status: 403 },
      false,
    ]);
  });

  it("rejects when the signal aborts and leaves no request running", async () => {
    const { bytes } = largePdf();
    server.serve(bytes);
    const source = await httpRangeSource(server.url);
    server.delayMs = 5_000;
    const controller = new AbortController();
    const opening = Document.openRange(source, { signal: controller.signal });
    await new Promise((r) => setTimeout(r, 50));
    controller.abort(new Error("stop"));
    await expect(opening).rejects.toThrow("stop");
    await new Promise((r) => setTimeout(r, 50));
    expect(server.log.length).toBeGreaterThan(1);
    expect(server.log.every((l) => l.closed)).toBe(true);
  });
});

describe("doc.ensure", () => {
  it("makes a lazy call that needs more bytes work, and the bare call throws data-pending with ranges", async () => {
    const { bytes } = largePdf();
    server.serve(bytes);
    const doc = await Document.openRange(httpRangeSource(server.url));
    let bare: unknown;
    try {
      doc.save({ mode: "fullRewrite" });
    } catch (e) {
      bare = e;
    }
    expect(isLaminaError(bare, "data-pending") && (bare.ranges?.length ?? 0) > 0).toBe(true);
    const saved = await doc.ensure(() => doc.save({ mode: "fullRewrite" }));
    expect(saved.length).toBeGreaterThan(bytes.length / 2);
  });

  it("just runs the call on an ordinary document", async () => {
    const doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    expect(await doc.ensure(() => doc.pageCount)).toBe(1);
  });
});

describe("callbackSource", () => {
  it("opens through a caller's read function", async () => {
    const { bytes } = largePdf(4);
    const asked: number[] = [];
    const source = callbackSource(bytes.length, async (offset, length) => {
      asked.push(length);
      return bytes.slice(offset, offset + length);
    });
    const doc = await Document.openRange(source, { blockSize: 0 });
    expect(await doc.ensure(() => doc.pageCount)).toBe(4);
    expect(asked.reduce((a, b) => a + b, 0)).toBeLessThan(bytes.length);
  });

  it("fails with fetch-failed when a read returns the wrong number of bytes", async () => {
    const { bytes } = largePdf(4);
    const source = callbackSource(bytes.length, async (o, n) => bytes.slice(o, o + n - 1));
    const e = await Document.openRange(source).catch((x: unknown) => x);
    expect(isLaminaError(e, "fetch-failed") && e.data?.reason).toBe("length");
  });

  it("fails with source-budget-exceeded past the budget", async () => {
    const { bytes } = largePdf(4);
    const source = callbackSource(bytes.length, async (o, n) => bytes.slice(o, o + n));
    const e = await Document.openRange(source, { budget: 1_000 }).catch((x: unknown) => x);
    expect(isLaminaError(e, "source-budget-exceeded")).toBe(true);
  });
});
