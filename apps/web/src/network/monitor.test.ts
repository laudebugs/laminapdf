import type { NetworkRecord } from "@laminapdf/viewer";
import { describe, expect, it } from "vitest";
import { classifyRequest, purposeOf } from "./classify.ts";
import { NetworkMonitor, summarize } from "./monitor.ts";
import { networkReport } from "./report.ts";

const APP = "https://app.laminapdf.com";
const FILE = "https://files.example.org/reports/q3.pdf";

const record = (url: string, at: number, extra: Partial<NetworkRecord> = {}): NetworkRecord => ({
  url,
  method: "GET",
  initiator: "fetch",
  bytes: 100,
  failed: false,
  source: "worker",
  at,
  ...extra,
});

describe("classifyRequest", () => {
  it("puts the reader's own files in the app group", () => {
    expect(classifyRequest(`${APP}/assets/pdfium-abc.wasm`, { appOrigin: APP })).toBe("app");
    expect(classifyRequest(`${APP}/`, { appOrigin: APP, documentUrl: FILE })).toBe("app");
  });

  it("puts the open file's host in the document group", () => {
    expect(classifyRequest("https://files.example.org/other.pdf", { appOrigin: APP, documentUrl: FILE })).toBe(
      "document",
    );
  });

  it("calls every other origin unexpected, the file's host too when no file from it is open", () => {
    expect(classifyRequest("https://cdn.example.net/x.js", { appOrigin: APP, documentUrl: FILE })).toBe("unexpected");
    expect(classifyRequest(FILE, { appOrigin: APP })).toBe("unexpected");
  });

  it("names what an app file was for", () => {
    expect(purposeOf(`${APP}/assets/a.wasm`, "fetch", "app")).toBe("webassembly");
    expect(purposeOf(`${APP}/assets/index-x.js`, "script", "app")).toBe("script");
    expect(purposeOf(`${APP}/assets/index-x.css`, "link", "app")).toBe("style");
    expect(purposeOf(`${APP}/assets/sans.woff2`, "css", "app")).toBe("font");
    expect(purposeOf(`${APP}/`, "navigation", "app")).toBe("page");
    expect(purposeOf(FILE, "fetch", "document")).toBe("document");
  });
});

describe("NetworkMonitor", () => {
  it("merges the worker's events and its log without counting a request twice", () => {
    const monitor = new NetworkMonitor();
    const a = record(`${APP}/assets/a.wasm`, 1);
    const b = record(`${APP}/assets/b.wasm`, 2);
    monitor.workerRequest("w1", 1, b);
    monitor.workerLog({ worker: "w1", requests: [a, b], violations: [] });
    monitor.workerRequest("w1", 0, a);
    expect(monitor.getState().requests).toEqual([b, a]);
    expect(monitor.getState().workerFeed).toBe(true);
  });

  it("keeps a restarted worker's records apart from the first one's", () => {
    const monitor = new NetworkMonitor();
    monitor.workerRequest("w1", 0, record(`${APP}/a`, 1));
    monitor.workerRequest("w2", 0, record(`${APP}/a`, 2));
    expect(monitor.getState().requests).toHaveLength(2);
  });

  it("tells subscribers of each change", () => {
    const monitor = new NetworkMonitor();
    let calls = 0;
    monitor.subscribe(() => {
      calls += 1;
    });
    monitor.pageRequest(record(`${APP}/`, 1, { source: "page" }));
    monitor.documentOpening(5, undefined);
    expect(calls).toBe(2);
  });
});

describe("summarize", () => {
  it("counts no document traffic for a local file, only the app's files", () => {
    const monitor = new NetworkMonitor();
    monitor.pageRequest(record(`${APP}/`, 1, { initiator: "navigation", source: "page" }));
    monitor.documentOpening(10, undefined);
    monitor.workerRequest("w", 0, record(`${APP}/assets/pdfium.wasm`, 11));
    const summary = summarize(monitor.getState(), APP);
    expect(summary).toMatchObject({ total: 2, documentRequests: 0, appRequests: 2, unexpected: [] });
    expect(summary.appRows.map((row) => row.purpose).sort()).toEqual(["page", "webassembly"]);
  });

  it("counts a linked file's range requests since it opened, by origin with their bytes", () => {
    const monitor = new NetworkMonitor();
    monitor.documentOpening(10, FILE);
    for (let i = 0; i < 3; i++) monitor.workerRequest("w", i, record(FILE, 11 + i, { bytes: 65536 }));
    const summary = summarize(monitor.getState(), APP);
    expect(summary.documentRequests).toBe(3);
    expect(summary.documentRows).toEqual([
      { origin: "https://files.example.org", group: "document", requests: 3, bytes: 196608 },
    ]);
  });

  it("leaves out an earlier file's traffic, and shows an unexpected site whenever it was asked", () => {
    const monitor = new NetworkMonitor();
    monitor.documentOpening(0, FILE);
    monitor.workerRequest("w", 0, record(FILE, 1));
    monitor.pageRequest(record("https://tracker.example/pixel.gif", 2, { initiator: "img", source: "page" }));
    monitor.documentOpening(10, undefined);
    const summary = summarize(monitor.getState(), APP);
    expect(summary.documentRequests).toBe(1);
    expect(summary.unexpected).toEqual(["https://tracker.example"]);
  });

  it("leaves out a call the policy blocked, which sent nothing", () => {
    const monitor = new NetworkMonitor();
    monitor.documentOpening(0, undefined);
    monitor.pageRequest(record("http://127.0.0.1:9/x", 1, { source: "page", failed: true }));
    monitor.pageViolation({ directive: "connect-src", blockedUri: "http://127.0.0.1:9/x", source: "page", at: 1 });
    const summary = summarize(monitor.getState(), APP);
    expect(summary).toMatchObject({ total: 0, documentRequests: 0 });
    expect(summary.blocked).toHaveLength(1);
  });

  it("ignores URLs without a network origin", () => {
    const monitor = new NetworkMonitor();
    monitor.pageRequest(record("data:text/plain,x", 1, { source: "page" }));
    expect(summarize(monitor.getState(), APP).total).toBe(0);
  });
});

describe("networkReport", () => {
  it("lists every origin, every kind of app file and every blocked request", () => {
    const monitor = new NetworkMonitor();
    monitor.documentOpening(10, FILE);
    monitor.workerRequest("w", 0, record(FILE, 11));
    monitor.pageViolation({ directive: "img-src", blockedUri: "https://x.example/a.png", source: "page", at: 12 });
    const report = networkReport(summarize(monitor.getState(), APP), "09:12");
    expect(report).toContain("since 09:12");
    expect(report).toContain("https://files.example.org: 1 requests, 100 bytes");
    expect(report).toContain("img-src: https://x.example/a.png (page)");
  });
});
