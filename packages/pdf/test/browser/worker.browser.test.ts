// `@lamina/pdf/worker` in a browser (Chromium, Firefox, WebKit; L2a): the remote API matches
// the sync one on the corpus, the handshake refuses another build, cancellation before and
// during a stepped save, transfers, rebuilt LaminaErrors, crash and restart, async disposal.
import { Document, init, isLaminaError, LaminaError, loadText } from "@lamina/pdf";
import { connect, type RemoteDocument, WorkerClient } from "@lamina/pdf/worker";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { report } from "../../examples/report.js";
import { workerClient } from "../../examples/worker-client.js";
import { REPORT_INPUTS } from "../suite/examples.js";

const urls = import.meta.glob<string>(["../../../../fixtures/corpus/*.pdf", "../../../../fixtures/fonts/*.{ttf,otf}"], {
  query: "?url",
  import: "default",
  eager: true,
});

async function read(path: string): Promise<Uint8Array> {
  const url = urls[`../../../../${path}`];
  if (url === undefined) throw new Error(`${path} is not served`);
  return new Uint8Array(await (await fetch(url)).arrayBuffer());
}

async function rejection(p: Promise<unknown>): Promise<unknown> {
  try {
    await p;
  } catch (e) {
    return e;
  }
  throw new Error("expected a rejection");
}

let client: WorkerClient;

beforeAll(async () => {
  await init();
  client = await connect();
});

afterAll(() => client.terminate());

describe("the worker client", () => {
  it("opens every corpus file with the sync API's page count, warnings and save() bytes", async () => {
    const files = Object.keys(urls)
      .filter((k) => k.includes("/corpus/"))
      .map((k) => k.replace("../../../../", ""));
    expect(files.length).toBeGreaterThan(10);
    for (const file of files) {
      const bytes = await read(file);
      let sync: Document;
      try {
        sync = Document.open(bytes);
      } catch (e) {
        // A file the sync API refuses must be refused the same way through the worker.
        const remote = await rejection(client.open(bytes));
        expect(isLaminaError(remote) && remote.code, file).toBe((e as LaminaError).code);
        continue;
      }
      await using doc = await client.open(bytes);
      expect(await doc.pageCount(), file).toBe(sync.pageCount);
      expect(await doc.warnings(), file).toEqual(sync.warnings);
      expect(await doc.save(), file).toEqual(sync.save());
      sync.free();
    }
  });

  it("runs the worker-client example", async () => {
    const result = await workerClient(await read("fixtures/corpus/hello.pdf"));
    expect(result).toMatchObject({ pages: 1, abortedAsCancelled: true });
    expect(result.savedBytes).toBeGreaterThan(0);
  });

  it("returns remote pages and rebuilds page text", async () => {
    await client.loadExtract();
    await using doc = await client.open(await read("fixtures/corpus/text-example.pdf"));
    const page = await doc.page(0);
    expect(await page.index()).toBe(0);
    const text = await page.text();
    expect(text.text().length).toBeGreaterThan(0);
    expect(text.glyphCount).toBeGreaterThan(0);
  });

  it("opens a remote file by range requests in the worker (openUrl)", async () => {
    const path = "fixtures/corpus/text-example.pdf";
    const url = new URL(urls[`../../../../${path}`] ?? "", globalThis.location.href).href;
    const sync = Document.open(await read(path));
    await using doc = await client.openUrl(url);
    expect(await doc.pageCount()).toBe(sync.pageCount);
    expect(await doc.info()).toEqual(sync.info);
    sync.free();
  });

  it("rebuilds worker errors as LaminaError with every ADR 0026 field", async () => {
    const e = await rejection(client.open(new TextEncoder().encode("not a pdf")));
    expect(e).toBeInstanceOf(LaminaError);
    const error = e as LaminaError;
    expect([typeof error.code, typeof error.kind, typeof error.docsUrl]).toEqual(["string", "string", "string"]);
  });

  it("transfers save() output and, with transfer: true, the input", async () => {
    const bytes = await read("fixtures/corpus/hello.pdf");
    const kept = bytes.slice();
    await using copied = await client.open(kept);
    expect(kept.byteLength).toBe(bytes.byteLength);
    const moved = bytes.slice();
    await using transferred = await client.open(moved, { transfer: true });
    expect(moved.byteLength).toBe(0);
    const saved = await transferred.save();
    expect(saved.byteLength).toBeGreaterThan(0);
    expect(await copied.pageCount()).toBe(1);
  });

  it("never runs a call aborted before it starts", async () => {
    const before = await stats();
    const controller = new AbortController();
    controller.abort();
    const e = await rejection(client.open(await read("fixtures/corpus/hello.pdf"), { signal: controller.signal }));
    expect(isLaminaError(e, "cancelled")).toBe(true);
    expect((await stats()).started).toBe(before.started + 1);
  });

  it("stops a stepped save when aborted, and the document stays usable", async () => {
    // The 100-page report: a full rewrite of it takes several save steps.
    await loadText();
    const bytes = report(await read(REPORT_INPUTS[0]), await read(REPORT_INPUTS[1]));
    await using doc = await client.open(bytes);
    await internal("__budget", [50]); // many small steps, so the abort lands between two
    await doc.apply([{ op: "setInfo", args: { patch: { title: "changed" } } }]);
    const controller = new AbortController();
    const saving = doc.save(
      { mode: "fullRewrite" },
      { signal: controller.signal, onProgress: () => controller.abort() },
    );
    const e = await rejection(saving);
    expect(isLaminaError(e, "cancelled")).toBe(true);
    await internal("__budget", [0]);
    expect((await doc.save({ mode: "fullRewrite" })).byteLength).toBeGreaterThan(0);
  });

  it("frees handles on async disposal", async () => {
    const before = (await stats()).handles;
    {
      await using doc = await client.open(await read("fixtures/corpus/hello.pdf"));
      await doc.page(0);
      expect((await stats()).handles).toBe(before + 2);
    }
    expect((await stats()).handles).toBe(before);
  });

  it("refuses a worker from another build, sending nothing else", async () => {
    const source = `self.onmessage = (e) => { if (e.data.kind === "hello") self.postMessage({ kind: "welcome", worker: { protocol: 999, version: "0.0.0-other" } }); else self.postMessage({ kind: "unexpected" }); };`;
    const worker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
    const e = await rejection(connect({ worker }));
    expect(isLaminaError(e, "worker-version-mismatch")).toBe(true);
    // docs/errors/worker-version-mismatch.md's data fields.
    expect(Object.keys((e as LaminaError).data ?? {}).sort()).toEqual(["actual", "expected"]);
  });

  it("rejects pending calls with worker-crashed on a crash, and restart() serves again", async () => {
    const crashes: string[] = [];
    client.on("crashed", (reason) => crashes.push(reason));
    const pending = internal("__crash");
    const e = await rejection(pending);
    expect(isLaminaError(e, "worker-crashed")).toBe(true);
    expect(Object.keys((e as LaminaError).data ?? {})).toEqual(["reason"]);
    expect(crashes.length).toBe(1);
    await client.restart();
    await using doc: RemoteDocument = await client.open(await read("fixtures/corpus/hello.pdf"));
    expect(await doc.pageCount()).toBe(1);
  });
});

describe("the worker client's lifecycle", () => {
  it("makes handles of a stopped worker disposed, and their close() a no-op", async () => {
    const old = await client.open(await read("fixtures/corpus/hello.pdf"));
    await client.restart();
    await using fresh = await client.open(await read("fixtures/corpus/hello.pdf"));
    const e = await rejection(old.pageCount());
    expect(isLaminaError(e, "disposed")).toBe(true);
    await old.close();
    expect(await fresh.pageCount()).toBe(1);
  });

  it("serves a call made while restart() is still starting", async () => {
    const restarting = client.restart();
    const opened = client.open(await read("fixtures/corpus/hello.pdf"));
    await restarting;
    await using doc = await opened;
    expect(await doc.pageCount()).toBe(1);
  });

  it("rejects calls after a failed restart instead of hanging", async () => {
    const controller = new AbortController();
    controller.abort();
    expect(isLaminaError(await rejection(client.restart(controller.signal)), "cancelled")).toBe(true);
    expect(isLaminaError(await rejection(client.loadText()), "worker-crashed")).toBe(true);
    await client.restart();
  });

  it("doesn't fire crashed on terminate()", async () => {
    const crashes: string[] = [];
    const off = client.on("crashed", (reason) => crashes.push(reason));
    client.terminate();
    await client.restart();
    off();
    expect(crashes).toEqual([]);
  });

  it("rejects a parameter that can't be sent with invalid-argument, and keeps serving", async () => {
    const e = await rejection(client.openUrl("https://example.invalid/a.pdf", { headers: { x: (() => {}) as never } }));
    expect(isLaminaError(e, "invalid-argument")).toBe(true);
    await using doc = await client.open(await read("fixtures/corpus/hello.pdf"));
    expect(await doc.pageCount()).toBe(1);
  });

  it("closes twice without error", async () => {
    const doc = await client.open(await read("fixtures/corpus/hello.pdf"));
    await doc.close();
    await doc.close();
  });

  it("frees the document of an openUrl aborted while it fetches", async () => {
    const before = (await stats()).handles;
    const path = "fixtures/corpus/text-example.pdf";
    const url = new URL(urls[`../../../../${path}`] ?? "", globalThis.location.href).href;
    const controller = new AbortController();
    const opening = client.openUrl(url, { signal: controller.signal });
    setTimeout(() => controller.abort(), 0);
    expect(isLaminaError(await rejection(opening), "cancelled")).toBe(true);
    // Calls run one at a time, so this answers after the aborted open has finished.
    expect((await stats()).handles).toBe(before);
  });
});

describe("the worker client, second review", () => {
  it("passes openRange's budget through openUrl", async () => {
    const path = "fixtures/corpus/text-example.pdf";
    const url = new URL(urls[`../../../../${path}`] ?? "", globalThis.location.href).href;
    const e = await rejection(client.openUrl(url, { budget: 1024, blockSize: 1024 }));
    expect(isLaminaError(e, "source-budget-exceeded")).toBe(true);
  });

  it("reuses one handle per page", async () => {
    const before = (await stats()).handles;
    await using doc = await client.open(await read("fixtures/corpus/hello.pdf"));
    await doc.page(0);
    await doc.page(0);
    expect((await stats()).handles).toBe(before + 2);
  });

  it("copies, not detaches, a subarray passed with transfer: true", async () => {
    const bytes = await read("fixtures/corpus/hello.pdf");
    const big = new Uint8Array(bytes.byteLength + 16);
    big.set(bytes, 16);
    await using doc = await client.open(big.subarray(16), { transfer: true });
    expect(big.byteLength).toBe(bytes.byteLength + 16);
    expect(await doc.pageCount()).toBe(1);
  });

  it("rejects an init the worker can't receive with invalid-argument", async () => {
    const e = await rejection(connect({ init: { module: new Response("x") as never } }));
    expect(isLaminaError(e, "invalid-argument")).toBe(true);
  });

  it("ends a worker stuck in its handshake on terminate(), and an aborted call waiting for it", async () => {
    const silent = new Worker(
      URL.createObjectURL(new Blob(["self.onmessage = () => {};"], { type: "text/javascript" })),
    );
    const stuck = new WorkerClient({ worker: silent });
    const starting = stuck.start();
    const controller = new AbortController();
    const waiting = stuck.loadText({ signal: controller.signal });
    controller.abort();
    expect(isLaminaError(await rejection(waiting), "cancelled")).toBe(true);
    stuck.terminate();
    expect(isLaminaError(await rejection(starting), "worker-crashed")).toBe(true);
  });

  it("leaves no cancel behind for calls that already finished", async () => {
    const controller = new AbortController();
    await using doc = await client.open(await read("fixtures/corpus/hello.pdf"));
    await doc.pageCount({ signal: controller.signal });
    controller.abort();
    expect((await stats()).queued).toBe(0);
  });
});

/** A test-only method of the worker (`__stats`, `__budget`, `__crash`). */
function internal(method: string, params: unknown[] = []): Promise<unknown> {
  return (client as unknown as { call: (h: number, m: string, p: unknown[]) => Promise<unknown> }).call(
    0,
    method,
    params,
  );
}

async function stats(): Promise<{ started: number; handles: number; queued: number }> {
  return (await internal("__stats")) as { started: number; handles: number; queued: number };
}
