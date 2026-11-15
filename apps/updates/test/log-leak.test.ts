// ADR 0014 §3 and §9: the install ID never reaches a log. Every console method is watched across
// requests that take each path through the handler (served, not served, bridges, pause, a broken
// manifest, R2 failing, bad paths), with the Worker's own logger, `logEvent`, which src/index.ts
// passes too. test/workerd.test.ts checks workerd's output the same way. D7a: no crash report's body
// and no client IP reaches a log either, whatever POST /crash answers.
import { createHash } from "node:crypto";
import { afterEach, describe, expect, it, vi } from "vitest";
import { type HandlerDeps, handle, logEvent } from "../src/handler.ts";
import {
  cohortIds,
  crashFixture,
  crashRequest,
  fixture,
  type Harness,
  harness,
  INSTALL_IDS,
  request,
} from "./helpers.ts";

const METHODS = ["log", "info", "warn", "error", "debug", "trace", "dir", "table"] as const;
const PATHS = [
  "/beta/darwin/aarch64/0.1.3",
  "/beta/darwin/aarch64/0.0.9",
  "/beta/linux/aarch64/0.1.0",
  "/beta/darwin/aarch64/9.9.9",
  "/beta/darwin/arm64/0.1.0",
  "/nope/darwin/aarch64/0.1.0",
  "/healthz",
];

afterEach(() => {
  vi.restoreAllMocks();
});

/** Every line any console method writes from now on. */
function watchConsole(): string[] {
  const written: string[] = [];
  for (const method of METHODS) {
    vi.spyOn(console, method).mockImplementation((...args: unknown[]) => {
      written.push(args.map((a) => (typeof a === "string" ? a : JSON.stringify(a))).join(" "));
    });
  }
  return written;
}

/** Every form of an install ID a log could carry: as sent, lower-cased, and its rollout hashes. */
function forms(id: string, versions: readonly string[]): string[] {
  const lower = id.toLowerCase();
  const hashes = versions.flatMap((v) => {
    const digest = createHash("sha256").update(`${lower}:${v}`).digest();
    return [digest.toString("hex"), digest.toString("base64"), String(digest.readUInt32BE(0))];
  });
  return [id, lower, id.toUpperCase(), lower.replaceAll("-", ""), ...hashes];
}

/** Sends every path with every ID and method, from an empty cache. */
async function sendAll(h: Harness, deps: HandlerDeps, ids: readonly string[]): Promise<void> {
  h.cache.entries.clear();
  for (const path of PATHS) {
    for (const id of ids) {
      for (const method of ["GET", "HEAD", "POST"]) await handle(request(path, { installId: id, method }), deps);
    }
  }
  await h.settle();
}

describe("handle", () => {
  it("never writes the install ID, or a hash of it, to the console", async () => {
    const written = watchConsole();
    const h = harness();
    const deps: HandlerDeps = { ...h.deps, log: logEvent };
    const { inside, outside } = await cohortIds("0.2.0", 10);
    const ids = [inside, outside, ...INSTALL_IDS, inside.toUpperCase()];
    const beta = fixture("beta");

    h.store.put("beta", beta);
    await sendAll(h, deps, ids);
    h.store.put("beta", { ...beta, paused: true });
    await sendAll(h, deps, ids);
    h.store.put("beta", '{"schema":1,"channel":"beta","paused":"no"}');
    await sendAll(h, deps, ids);
    h.deps.stores.beta.get = async () => {
      throw new Error("R2 unavailable");
    };
    await sendAll(h, deps, ids);

    // The broken manifest and the R2 failure did log something, so the check isn't vacuous.
    expect(written.some((line) => line.includes("manifest-invalid"))).toBe(true);
    expect(written.some((line) => line.includes("manifest-unavailable"))).toBe(true);
    const all = written.join("\n");
    for (const form of ids.flatMap((id) => forms(id, ["0.2.0", "0.1.3", "0.1.0"]))) {
      expect(all).not.toContain(form);
    }
  });

  it("never writes a crash report's body or the client's IP to the console", async () => {
    const written = watchConsole();
    const h = harness();
    const deps: HandlerDeps = { ...h.deps, log: logEvent };
    const marker = "CRASHMARKER-q7w8e9";
    const ip = "203.0.113.77";
    const report = { ...crashFixture(), reason: `boom ${marker}` };
    const bodies = [
      JSON.stringify(report),
      JSON.stringify({ ...report, reason: `/Users/${marker}/a.pdf` }),
      JSON.stringify({ ...report, [marker]: true }),
      `{"reason":"${marker}`,
    ];
    const send = async () => {
      for (const body of bodies) {
        await handle(crashRequest(body, { "CF-Connecting-IP": ip }), deps);
        await handle(crashRequest(body, { "CF-Connecting-IP": ip, "Content-Type": "text/plain" }), deps);
      }
    };
    await send();
    // Past the limit, then with R2 failing.
    for (let i = 0; i < 10; i += 1) await send();
    const failing = harness();
    failing.deps.crashStore.put = async () => {
      throw new Error("R2 unavailable");
    };
    await handle(crashRequest(bodies[0] ?? "", { "CF-Connecting-IP": ip }), { ...failing.deps, log: logEvent });

    // The R2 failure did log something, so the check isn't vacuous.
    expect(written.some((line) => line.includes("crash-store-failed"))).toBe(true);
    const all = written.join("\n");
    expect(all).not.toContain(marker);
    expect(all).not.toContain(ip);
  });
});
