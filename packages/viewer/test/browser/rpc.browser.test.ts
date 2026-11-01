// The document worker and its RPC in Chromium, Firefox and WebKit (C3a's acceptance criteria).
import { isLaminaError, type LaminaError } from "@laminapdf/sdk";
import { connect } from "@laminapdf/sdk/worker";
import { afterEach, describe, expect, it } from "vitest";
import { CORPUS, ENCRYPTED, expected, hook, next, PASSWORD, read, rejection, testViewer, viewer } from "./helpers.js";

const clients: { terminate(): void }[] = [];
async function track<T extends { terminate(): void }>(p: Promise<T>): Promise<T> {
  const c = await p;
  clients.push(c);
  return c;
}
afterEach(() => {
  for (const c of clients.splice(0)) c.terminate();
});

function isAbort(e: unknown): boolean {
  return e instanceof DOMException && e.name === "AbortError";
}

describe("open", () => {
  it("returns page counts and sizes matching expected.json for the corpus", async () => {
    const facts = await expected();
    const client = await track(viewer());
    let checked = 0;
    for (const path of CORPUS) {
      const name = path.split("/").pop() ?? "";
      const fact = facts[name];
      if (fact === undefined) continue;
      const opened = await client.open(await read(path));
      expect(opened.pageCount, name).toBe(fact.page_count);
      const [x0, y0, x1, y1] = fact.first_page_mediabox as [number, number, number, number];
      const page = opened.pages[0];
      const [w, h] = page !== undefined && page.rotation % 180 !== 0 ? [y1 - y0, x1 - x0] : [x1 - x0, y1 - y0];
      expect([page?.width, page?.height], name).toEqual([w, h]);
      checked++;
    }
    expect(checked).toBeGreaterThan(10);
  });

  it("opens with hayro as well, and renders", async () => {
    const client = await track(viewer("hayro"));
    const opened = await client.open(await read("fixtures/corpus/hello.pdf"));
    expect(opened.backend.name).toBe("hayro");
    const { bitmap } = await client.render({ page: 0, scale: 0.5 });
    expect(bitmap.width).toBeGreaterThan(0);
    bitmap.close();
  });

  it("fails a file over the limits with limit-exceeded, not a hang", async () => {
    const client = await track(viewer());
    const e = await rejection(client.open(await read("fixtures/corpus/hello.pdf"), { limits: { maxNestingDepth: 1 } }));
    expect(isLaminaError(e, "limit-exceeded")).toBe(true);
  });
});

describe("errors", () => {
  it("keeps a render data-pending's ranges across the worker", async () => {
    const client = await track(
      connect({
        worker: () => new Worker(new URL("./render-error.worker.ts", import.meta.url), { type: "module" }),
      }),
    );
    const e = await rejection(client.request("pending", []));
    expect(isLaminaError(e, "data-pending")).toBe(true);
    expect((e as LaminaError).ranges).toEqual([
      { offset: 0, length: 1024 },
      { offset: 4096, length: 512 },
    ]);
  });
});

describe("cancellation and priorities", () => {
  it("never resolves the 90 of 100 renders aborted before they finish, and runs the 10 kept in priority order", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const busy = hook(client, "__busy", [300]);
    const priorities = ["background", "prefetch", "visible"] as const;
    const kept: { scale: number; priority: (typeof priorities)[number] }[] = [];
    const calls = Array.from({ length: 100 }, (_, i) => {
      const scale = 0.1 + i / 1000;
      const controller = new AbortController();
      const priority = priorities[i % 3] ?? "visible";
      const keep = i % 10 === 0;
      if (keep) kept.push({ scale, priority });
      const p = client.render({ page: 0, scale }, { priority, signal: controller.signal });
      if (!keep) controller.abort();
      return { keep, p };
    });
    // Settled at once, so an aborted call's rejection is never unhandled while we wait.
    const settled = Promise.allSettled(calls.map((c) => c.p));
    await busy;
    const results = await settled;
    for (const [i, r] of results.entries()) {
      if (calls[i]?.keep) {
        expect(r.status).toBe("fulfilled");
        if (r.status === "fulfilled") r.value.bitmap.close();
      } else {
        expect(r.status === "rejected" && isAbort(r.reason)).toBe(true);
      }
    }
    const order = { visible: 0, prefetch: 1, background: 2 };
    const want = [...kept].sort((a, b) => order[a.priority] - order[b.priority]).map((k) => k.scale);
    expect(await hook(client, "__renderLog", [])).toEqual(want);
  });

  it("closes the bitmap of a render aborted while it runs, and never resolves it", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    // Counts the page's ImageBitmap closes: only the late answer's bitmap is closed here.
    const close = ImageBitmap.prototype.close;
    let closed = 0;
    ImageBitmap.prototype.close = function (this: ImageBitmap) {
      closed++;
      close.call(this);
    };
    try {
      const controller = new AbortController();
      const started = next(client, "render-start");
      const p = client.render({ page: 0, scale: 3 }, { signal: controller.signal });
      await started;
      controller.abort();
      expect(isAbort(await rejection(p))).toBe(true);
      // The late answer arrives after the next call's (calls run in order).
      await hook(client, "__renderLog", []);
      expect(closed).toBe(1);
    } finally {
      ImageBitmap.prototype.close = close;
    }
  });

  it("starts a new visible render before queued background work", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const busy = hook(client, "__busy", [200]);
    const background = [0.11, 0.12, 0.13].map((scale) => client.render({ page: 0, scale }, { priority: "background" }));
    const visible = client.render({ page: 0, scale: 0.2 }, { priority: "visible" });
    await busy;
    for (const r of await Promise.all([...background, visible])) r.bitmap.close();
    expect(await hook(client, "__renderLog", [])).toEqual([0.2, 0.11, 0.12, 0.13]);
  });

  it("answers a same-key render sent after the first one started", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const started = next(client, "render-start");
    const first = client.render({ page: 0, scale: 2 });
    await started;
    const second = client.render({ page: 0, scale: 2 });
    for (const r of await Promise.all([first, second])) r.bitmap.close();
  });

  it("drops a shared job back to its remaining caller's priority when the urgent one cancels", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const busy = hook(client, "__busy", [200]);
    const background = client.render({ page: 0, scale: 0.11 }, { priority: "background" });
    const controller = new AbortController();
    const urgent = rejection(
      client.render({ page: 0, scale: 0.11 }, { priority: "visible", signal: controller.signal }),
    );
    const prefetch = client.render({ page: 0, scale: 0.2 }, { priority: "prefetch" });
    controller.abort();
    await busy;
    expect(isAbort(await urgent)).toBe(true);
    for (const r of await Promise.all([background, prefetch])) r.bitmap.close();
    expect(await hook(client, "__renderLog", [])).toEqual([0.2, 0.11]);
  });

  it("draws identical renders once and gives each caller its own bitmap", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const busy = hook(client, "__busy", [100]);
    const [a, b] = await Promise.all([client.render({ page: 0, scale: 0.3 }), client.render({ page: 0, scale: 0.3 })]);
    await busy;
    expect(a?.bitmap).not.toBe(b?.bitmap);
    expect(await hook(client, "__renderLog", [])).toEqual([0.3]);
    a?.bitmap.close();
    b?.bitmap.close();
  });
});

describe("long renders", () => {
  it("doesn't treat a busy worker as a crash", async () => {
    const client = await track(testViewer("pdfium", { timeouts: { soft: 5_000, hard: 60_000 } }));
    await client.open(await read("fixtures/corpus/hello.pdf"));
    let crashed = false;
    client.on("crashed", () => {
      crashed = true;
    });
    await hook(client, "__busy", [1_000]);
    const { bitmap } = await client.render({ page: 0, scale: 0.5 });
    bitmap.close();
    expect(crashed).toBe(false);
  });

  it("emits render-slow past the soft timeout and keeps the worker", async () => {
    const client = await track(testViewer("pdfium", { timeouts: { soft: 1, hard: 60_000 } }));
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const slow = next(client, "render-slow");
    const render = client.render({ page: 0, scale: 4 });
    expect((await slow).page).toBe(0);
    (await render).bitmap.close();
  });

  it("doesn't count time waiting in the queue against a render", async () => {
    const client = await track(testViewer("pdfium", { timeouts: { soft: 300, hard: 60_000 } }));
    await client.open(await read("fixtures/corpus/hello.pdf"));
    let slow = false;
    client.on("render-slow", () => {
      slow = true;
    });
    const busy = hook(client, "__busy", [600]);
    const render = client.render({ page: 0, scale: 0.2 });
    await busy;
    (await render).bitmap.close();
    expect(slow).toBe(false);
  });

  it("restarts on stop() without re-sending the blocked render", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const restarted = next(client, "restarted");
    const blocked = rejection(hook(client, "__busy", [5_000]));
    const queued = rejection(client.render({ page: 0, scale: 0.77 }));
    await client.stop();
    expect(isLaminaError(await blocked, "worker-crashed")).toBe(true);
    expect(isLaminaError(await queued, "worker-crashed")).toBe(true);
    expect(await restarted).toEqual({ reason: "cancelled", reopened: true });
    expect(await hook(client, "__renderLog", [])).toEqual([]);
    (await client.render({ page: 0, scale: 0.5 })).bitmap.close();
  });

  it("emits render-stuck for an encrypted document at the hard cap, and restarts only on stop()", async () => {
    const client = await track(testViewer("pdfium", { timeouts: { soft: 1, hard: 2 } }));
    await client.open(await read(ENCRYPTED), { password: PASSWORD });
    let restarted = false;
    client.on("restarted", () => {
      restarted = true;
    });
    const stuck = next(client, "render-stuck");
    const render = client.render({ page: 0, scale: 4 });
    expect((await stuck).needsPassword).toBe(true);
    (await render).bitmap.close();
    expect(restarted).toBe(false);
    const required = next(client, "password-required");
    await client.stop();
    await required;
  });
});

describe("documents", () => {
  it("keeps the open document when another fails to open", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    await rejection(client.open(new TextEncoder().encode("not a pdf")));
    (await client.render({ page: 0, scale: 0.5 })).bitmap.close();
  });

  it("fails a render queued for a document that another open replaced", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const busy = hook(client, "__busy", [200]);
    const stale = rejection(client.render({ page: 0, scale: 0.5 }, { priority: "background" }));
    await client.open(await read("fixtures/corpus/fpdf2.pdf"));
    await busy;
    expect(isLaminaError(await stale, "disposed")).toBe(true);
  });

  it("keeps the open document when an open is aborted while it runs", async () => {
    const client = await track(testViewer());
    const first = await client.open(await read("fixtures/corpus/hello.pdf"));
    const controller = new AbortController();
    const opening = rejection(
      client.open(await read("fixtures/corpus/text-example.pdf"), {}, { signal: controller.signal }),
    );
    // The cancel crosses while the worker opens synchronously.
    queueMicrotask(() => controller.abort());
    expect(isAbort(await opening)).toBe(true);
    expect(client.document?.id).toBe(first.id);
    (await client.render({ page: 0, scale: 0.5 })).bitmap.close();
  });

  it("renders the reopened document when asked during a restart", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    await client.open(await read("fixtures/corpus/fpdf2.pdf"));
    const restarted = next(client, "restarted");
    await rejection(hook(client, "__crash", []));
    const render = client.render({ page: 0, scale: 0.5 });
    await restarted;
    (await render).bitmap.close();
  });

  it("only reports needsPassword for a file that can't open without it", async () => {
    const client = await track(testViewer());
    expect((await client.open(await read("fixtures/corpus/hello.pdf"), { password: "unused" })).needsPassword).toBe(
      false,
    );
    expect((await client.open(await read(ENCRYPTED), { password: PASSWORD })).needsPassword).toBe(true);
  });
});

describe("crashes", () => {
  it("rejects pending calls, reopens the document and renders again", async () => {
    const client = await track(testViewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const restarted = next(client, "restarted");
    const crash = hook(client, "__crash", []);
    expect(isLaminaError(await rejection(crash), "worker-crashed")).toBe(true);
    expect(await restarted).toEqual({ reason: "error", reopened: true });
    (await client.render({ page: 0, scale: 0.5 })).bitmap.close();
  });

  it("asks for the password again after a crash of an encrypted document", async () => {
    const client = await track(testViewer());
    await client.open(await read(ENCRYPTED), { password: PASSWORD });
    const required = next(client, "password-required");
    await rejection(hook(client, "__crash", []));
    await required;
    expect(client.document).toBeUndefined();
    await client.open(await read(ENCRYPTED), { password: PASSWORD });
    (await client.render({ page: 0, scale: 0.5 })).bitmap.close();
  });
});

describe("secrets", () => {
  it("posts the password only in the open calls the user made, and keeps it nowhere on the page", async () => {
    const posted: unknown[] = [];
    const original = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (this: Worker, message: unknown, ...rest: unknown[]) {
      posted.push(message);
      return (original as (...a: unknown[]) => void).call(this, message, ...rest);
    } as typeof Worker.prototype.postMessage;
    try {
      const client = await track(testViewer());
      const carries = (m: unknown): boolean =>
        m === PASSWORD ||
        (typeof m === "object" &&
          m !== null &&
          !(m instanceof Blob) &&
          !ArrayBuffer.isView(m) &&
          Object.values(m).some(carries));
      await client.open(await read(ENCRYPTED), { password: PASSWORD });
      expect(posted.filter(carries).length).toBe(1);
      const required = next(client, "password-required");
      await rejection(hook(client, "__crash", []));
      await required;
      expect(posted.filter(carries).length).toBe(1);
      await client.open(await read(ENCRYPTED), { password: PASSWORD });
      expect(posted.filter(carries).length).toBe(2);
    } finally {
      Worker.prototype.postMessage = original;
    }
  });
});
