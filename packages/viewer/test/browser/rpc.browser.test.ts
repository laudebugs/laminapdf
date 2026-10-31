// The document worker and its RPC in Chromium, Firefox and WebKit (C3a's acceptance criteria).
import { isLaminaError } from "@lamina/pdf";
import { afterEach, describe, expect, it } from "vitest";
import { CORPUS, ENCRYPTED, expected, next, PASSWORD, read, rejection, viewer } from "./helpers.js";

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

describe("cancellation and priorities", () => {
  it("never resolves the 90 of 100 renders aborted before they finish, and runs the 10 kept in priority order", async () => {
    const client = await track(viewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const busy = client.call("__busy", [300]);
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
    expect(await client.call("__renderLog", [])).toEqual(want);
  });

  it("closes the bitmap of a render aborted while it runs, and never resolves it", async () => {
    const client = await track(viewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const controller = new AbortController();
    const started = next(client, "render-start");
    const p = client.render({ page: 0, scale: 3 }, { signal: controller.signal });
    await started;
    controller.abort();
    expect(isAbort(await rejection(p))).toBe(true);
    // The late answer arrives after the next call's (calls run in order).
    await client.call("__renderLog", []);
    expect(client.discardedBitmaps).toBe(1);
  });

  it("starts a new visible render before queued background work", async () => {
    const client = await track(viewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const busy = client.call("__busy", [200]);
    const background = [0.11, 0.12, 0.13].map((scale) => client.render({ page: 0, scale }, { priority: "background" }));
    const visible = client.render({ page: 0, scale: 0.2 }, { priority: "visible" });
    await busy;
    for (const r of await Promise.all([...background, visible])) r.bitmap.close();
    expect(await client.call("__renderLog", [])).toEqual([0.2, 0.11, 0.12, 0.13]);
  });

  it("draws identical renders once and gives each caller its own bitmap", async () => {
    const client = await track(viewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const busy = client.call("__busy", [100]);
    const [a, b] = await Promise.all([client.render({ page: 0, scale: 0.3 }), client.render({ page: 0, scale: 0.3 })]);
    await busy;
    expect(a?.bitmap).not.toBe(b?.bitmap);
    expect(await client.call("__renderLog", [])).toEqual([0.3]);
    a?.bitmap.close();
    b?.bitmap.close();
  });
});

describe("long renders", () => {
  it("doesn't treat a busy worker as a crash", async () => {
    const client = await track(viewer("pdfium", { timeouts: { soft: 5_000, hard: 60_000 } }));
    await client.open(await read("fixtures/corpus/hello.pdf"));
    let crashed = false;
    client.on("crashed", () => {
      crashed = true;
    });
    await client.call("__busy", [1_000]);
    const { bitmap } = await client.render({ page: 0, scale: 0.5 });
    bitmap.close();
    expect(crashed).toBe(false);
  });

  it("emits render-slow past the soft timeout and keeps the worker", async () => {
    const client = await track(viewer("pdfium", { timeouts: { soft: 50, hard: 60_000 } }));
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const slow = next(client, "render-slow");
    const busy = client.call("__busy", [400]);
    const render = client.render({ page: 0, scale: 0.5 });
    expect((await slow).page).toBe(0);
    await busy;
    (await render).bitmap.close();
  });

  it("restarts on stop() without re-sending the blocked render", async () => {
    const client = await track(viewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const restarted = next(client, "restarted");
    const blocked = rejection(client.call("__busy", [5_000]));
    const queued = rejection(client.render({ page: 0, scale: 0.77 }));
    await client.stop();
    expect(isLaminaError(await blocked, "worker-crashed")).toBe(true);
    expect(isLaminaError(await queued, "worker-crashed")).toBe(true);
    expect(await restarted).toEqual({ reason: "cancelled", reopened: true });
    expect(await client.call("__renderLog", [])).toEqual([]);
    (await client.render({ page: 0, scale: 0.5 })).bitmap.close();
  });

  it("emits render-stuck for an encrypted document at the hard cap, and restarts only on stop()", async () => {
    const client = await track(viewer("pdfium", { timeouts: { soft: 20, hard: 100 } }));
    await client.open(await read(ENCRYPTED), { password: PASSWORD });
    let restarted = false;
    client.on("restarted", () => {
      restarted = true;
    });
    const stuck = next(client, "render-stuck");
    const busy = client.call("__busy", [600]);
    const render = client.render({ page: 0, scale: 0.5 });
    expect((await stuck).needsPassword).toBe(true);
    await busy;
    (await render).bitmap.close();
    expect(restarted).toBe(false);
    const required = next(client, "password-required");
    await client.stop();
    await required;
  });
});

describe("crashes", () => {
  it("rejects pending calls, reopens the document and renders again", async () => {
    const client = await track(viewer());
    await client.open(await read("fixtures/corpus/hello.pdf"));
    const restarted = next(client, "restarted");
    const crash = client.call("__crash", []);
    expect(isLaminaError(await rejection(crash), "worker-crashed")).toBe(true);
    expect(await restarted).toEqual({ reason: "error", reopened: true });
    (await client.render({ page: 0, scale: 0.5 })).bitmap.close();
  });

  it("asks for the password again after a crash of an encrypted document", async () => {
    const client = await track(viewer());
    await client.open(await read(ENCRYPTED), { password: PASSWORD });
    const required = next(client, "password-required");
    await rejection(client.call("__crash", []));
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
      const client = await track(viewer());
      const carries = (m: unknown): boolean =>
        m === PASSWORD ||
        (typeof m === "object" &&
          m !== null &&
          !(m instanceof Blob) &&
          !ArrayBuffer.isView(m) &&
          Object.values(m).some(carries));
      await client.open(await read(ENCRYPTED), { password: PASSWORD });
      expect(posted.filter(carries).length).toBe(1);
      expect(JSON.stringify(client.reopenState)).not.toContain(PASSWORD);
      const required = next(client, "password-required");
      await rejection(client.call("__crash", []));
      await required;
      expect(posted.filter(carries).length).toBe(1);
      await client.open(await read(ENCRYPTED), { password: PASSWORD });
      expect(posted.filter(carries).length).toBe(2);
      expect(JSON.stringify(client.reopenState)).not.toContain(PASSWORD);
    } finally {
      Worker.prototype.postMessage = original;
    }
  });
});
