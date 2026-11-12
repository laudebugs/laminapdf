// What the handler tests share: fixtures, an in-memory bucket, a Cache API double and a fake clock.
import { readFileSync } from "node:fs";
import type { ManifestCache, ManifestStore } from "../src/cache.ts";
import type { Channel } from "../src/channels.ts";
import type { HandlerDeps, LogEvent } from "../src/handler.ts";
import type { Manifest } from "../src/manifest.ts";
import { bucketFor } from "../src/rollout.ts";

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

export interface Harness {
  readonly deps: HandlerDeps;
  readonly store: MemoryStore;
  readonly cache: MemoryCache;
  readonly logs: LogEvent[];
  /** Moves the fake clock forward. */
  advance(ms: number): void;
  /** Waits for the cache writes handed to `waitUntil`. */
  settle(): Promise<void>;
}

export function harness(): Harness {
  const store = new MemoryStore();
  const cache = new MemoryCache();
  const logs: LogEvent[] = [];
  const pending: Promise<unknown>[] = [];
  let clock = Date.parse("2027-01-20T12:00:00Z");
  return {
    store,
    cache,
    logs,
    deps: {
      stores: { stable: store, beta: store, nightly: store, staging: store },
      cache,
      now: () => clock,
      waitUntil: (work) => pending.push(work),
      log: (entry) => logs.push(entry),
    },
    advance(ms) {
      clock += ms;
    },
    async settle() {
      await Promise.all(pending.splice(0));
    },
  };
}

export function request(path: string, init: { installId?: string; method?: string } = {}): Request {
  const headers = new Headers();
  if (init.installId !== undefined) headers.set("Lamina-Install-Id", init.installId);
  return new Request(`https://updates.laminapdf.com${path}`, { method: init.method ?? "GET", headers });
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

/** The R2 bindings and the compatibility date of one of wrangler.jsonc's Workers. */
export interface WorkerConfig {
  readonly compatibility_date: string;
  readonly r2_buckets: readonly { readonly binding: string; readonly bucket_name: string }[];
}

/** wrangler.jsonc: its top-level Worker (production) and each `env`. Its comments take whole lines. */
export function wranglerConfig(): WorkerConfig & { readonly env: Readonly<Record<string, Partial<WorkerConfig>>> } {
  const text = readFileSync(new URL("../wrangler.jsonc", import.meta.url), "utf8");
  const json = text
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("//"))
    .join("\n");
  return JSON.parse(json);
}
