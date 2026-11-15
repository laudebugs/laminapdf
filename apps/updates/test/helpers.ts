// What the handler tests share: fixtures, an in-memory bucket, a Cache API double, a fake clock, and
// the crash endpoint's bucket and rate limiter.
import { readFileSync } from "node:fs";
import type { ManifestCache, ManifestStore } from "../src/cache.ts";
import type { Channel } from "../src/channels.ts";
import type { CrashLimiter, CrashStore } from "../src/crash-endpoint.ts";
import type { HandlerDeps, LogEvent } from "../src/handler.ts";
import type { Manifest } from "../src/manifest.ts";
import { bucketFor } from "../src/rollout.ts";

/** A manifest from test/fixtures/, by file name without `.json`. */
export function fixture(name: string): Manifest {
  return JSON.parse(readFileSync(new URL(`fixtures/${name}.json`, import.meta.url), "utf8")) as Manifest;
}

/** A bucket held in memory: `put` replaces the object as an R2 write would. */
class MemoryStore implements ManifestStore {
  readonly objects = new Map<string, string>();
  reads = 0;
  put(channel: Channel, manifest: Manifest | string): void {
    this.objects.set(`channels/${channel}.json`, typeof manifest === "string" ? manifest : JSON.stringify(manifest));
  }
  async get(key: string): Promise<{ text(): Promise<string> } | null> {
    this.reads += 1;
    const text = this.objects.get(key);
    return text === undefined ? null : { text: async () => text };
  }
}

/** The crash-reports bucket held in memory: each write's key, text and content type. */
class MemoryCrashStore implements CrashStore {
  readonly objects = new Map<string, { text: string; contentType: string }>();
  async put(key: string, text: string, options: { httpMetadata: { contentType: string } }): Promise<unknown> {
    this.objects.set(key, { text, contentType: options.httpMetadata.contentType });
    return {};
  }
}

/** The Workers rate limiter's simple limit: `limit` calls per key, with no window to pass. */
class FakeLimiter implements CrashLimiter {
  readonly calls = new Map<string, number>();
  readonly #max: number;
  constructor(max: number) {
    this.#max = max;
  }
  async limit({ key }: { key: string }): Promise<{ success: boolean }> {
    const count = (this.calls.get(key) ?? 0) + 1;
    this.calls.set(key, count);
    return { success: count <= this.#max };
  }
}

/** The Cache API's behaviour that matters here: keyed by URL, keeps copies, returns clones. */
class MemoryCache implements ManifestCache {
  readonly entries = new Map<string, Response>();
  async match(request: Request): Promise<Response | undefined> {
    return this.entries.get(request.url)?.clone();
  }
  async put(request: Request, response: Response): Promise<void> {
    this.entries.set(request.url, response.clone());
  }
}

/** The handler's dependencies as test doubles, with the handles a test needs to drive them. */
export interface Harness {
  readonly deps: HandlerDeps;
  readonly store: MemoryStore;
  readonly cache: MemoryCache;
  readonly crashes: MemoryCrashStore;
  readonly limiter: FakeLimiter;
  readonly logs: LogEvent[];
  /** Moves the fake clock forward. */
  advance(ms: number): void;
  /** Waits for the cache writes handed to `waitUntil`. */
  settle(): Promise<void>;
}

/** A fresh {@link Harness}: an empty bucket and cache, no logs, and a clock at 2027-01-20 12:00 UTC. */
export function harness(): Harness {
  const store = new MemoryStore();
  const cache = new MemoryCache();
  const crashes = new MemoryCrashStore();
  // wrangler.jsonc's limit: 10 a minute.
  const limiter = new FakeLimiter(10);
  const logs: LogEvent[] = [];
  const pending: Promise<unknown>[] = [];
  let clock = Date.parse("2027-01-20T12:00:00Z");
  return {
    store,
    cache,
    crashes,
    limiter,
    logs,
    deps: {
      stores: { stable: store, beta: store, nightly: store, staging: store },
      cache,
      now: () => clock,
      waitUntil: (work) => pending.push(work),
      log: (entry) => logs.push(entry),
      crashStore: crashes,
      crashLimit: limiter,
    },
    advance(ms) {
      clock += ms;
    },
    async settle() {
      await Promise.all(pending.splice(0));
    },
  };
}

/** A request to the production host, with the install-ID header when `installId` is given. */
export function request(path: string, init: { installId?: string; method?: string } = {}): Request {
  const headers = new Headers();
  if (init.installId !== undefined) headers.set("Lamina-Install-Id", init.installId);
  return new Request(`https://updates.laminapdf.com${path}`, { method: init.method ?? "GET", headers });
}

/** test/fixtures/crash-report.json, the report the desktop app's tests also read. */
export function crashFixture(): Record<string, unknown> {
  return JSON.parse(readFileSync(new URL("fixtures/crash-report.json", import.meta.url), "utf8"));
}

/** A native crash's report (D7d): the fixture with the stack its minidump gave. */
export function nativeCrashFixture(): Record<string, unknown> {
  return JSON.parse(readFileSync(new URL("fixtures/crash-report-native.json", import.meta.url), "utf8"));
}

/** A `POST /crash` with `body`, as JSON from the given IP unless `headers` says otherwise. */
export function crashRequest(body: BodyInit | null, headers: Record<string, string> = {}): Request {
  return new Request("https://updates.laminapdf.com/crash", {
    method: "POST",
    headers: { "Content-Type": "application/json", "CF-Connecting-IP": "192.0.2.1", ...headers },
    body,
    // Node's fetch needs this for a streamed body.
    ...(body instanceof ReadableStream ? { duplex: "half" } : {}),
  });
}

/** Fixed install IDs, so every run tests the same cohorts. */
export const INSTALL_IDS = [
  "0b7d4c1e-2f3a-4b5c-8d6e-7f8091a2b3c4",
  "1c8e5d2f-3a4b-4c6d-9e7f-8091a2b3c4d5",
  "2d9f6e3a-4b5c-4d7e-af80-91a2b3c4d5e6",
  "3ea07f4b-5c6d-4e8f-b091-a2b3c4d5e6f7",
  "4fb1805c-6d7e-4f90-81a2-b3c4d5e6f708",
  "50c2916d-7e8f-4a01-92b3-c4d5e6f70819",
] as const;

/** One install ID inside a rollout of `percent` for `version` and one outside it. */
export async function cohortIds(version: string, percent: number): Promise<{ inside: string; outside: string }> {
  let inside: string | undefined;
  let outside: string | undefined;
  for (const id of INSTALL_IDS) {
    const bucket = await bucketFor(id, version);
    if (bucket < percent * 100) inside ??= id;
    else outside ??= id;
  }
  // A random UUID lands inside a 10 % rollout one time in ten, so search further when needed.
  for (let i = 0; (!inside || !outside) && i < 10_000; i += 1) {
    const id = crypto.randomUUID();
    if ((await bucketFor(id, version)) < percent * 100) inside ??= id;
    else outside ??= id;
  }
  if (!inside || !outside) throw new Error("no install IDs on both sides of the rollout");
  return { inside, outside };
}
