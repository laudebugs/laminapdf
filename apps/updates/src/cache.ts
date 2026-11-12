// The manifest cache (ADR 0014 §4–5): one layer only, the Cache API, for 30 seconds. R2 reads are
// strongly consistent, so a `paused` change reaches every data centre within 30 s plus one read,
// inside the kill switch's 60 s. Never add a second layer (an in-isolate memo, a KV copy) without
// taking its time out of this TTL: two stacked layers can serve a stale manifest for their sum.
import { type Checked, parseManifest } from "./manifest.ts";
import type { Channel } from "./route.ts";

/** How long a cached manifest is used, in milliseconds. */
export const MANIFEST_TTL_MS = 30_000;

/** What the Worker reads a manifest from: an R2 bucket binding satisfies it. */
export interface ManifestStore {
  get(key: string): Promise<{ text(): Promise<string> } | null>;
}

/** The cache the manifest is kept in: the Workers Cache API (`caches.default`) satisfies it. */
export interface ManifestCache {
  match(request: Request): Promise<Response | undefined>;
  put(request: Request, response: Response): Promise<void>;
}

export interface CacheDeps {
  readonly cache: ManifestCache;
  /** The current time in milliseconds (`Date.now`); tests pass a fake clock. */
  readonly now: () => number;
  /** Keeps the cache write alive after the response (`ctx.waitUntil`). */
  readonly waitUntil: (work: Promise<unknown>) => void;
}

export type Loaded = { readonly state: "absent" } | (Checked & { readonly state: "read" });

// When the copy was read from R2. The Worker checks it itself, so the 30 s holds even where the
// Cache API keeps an entry longer than its max-age, and tests can drive it with a fake clock.
const FETCHED_AT = "Lamina-Fetched-At";
// Whether the channel had a manifest. A channel without one is cached too, so an unseeded channel
// costs one R2 read per data centre every 30 s, not one per request.
const PRESENT = "Lamina-Manifest";

/** The synthetic cache key for a channel's manifest, on the Worker's own host. */
function manifestKey(origin: string, channel: Channel): Request {
  return new Request(`${origin}/__manifest/${channel}.json`);
}

/** Reads `channels/{channel}.json` through the cache, then checks it. */
export async function loadManifest(
  store: ManifestStore,
  channel: Channel,
  origin: string,
  deps: CacheDeps,
): Promise<Loaded> {
  const key = manifestKey(origin, channel);
  const hit = await deps.cache.match(key);
  if (hit) {
    const fetchedAt = Number(hit.headers.get(FETCHED_AT));
    if (Number.isFinite(fetchedAt) && deps.now() - fetchedAt < MANIFEST_TTL_MS) {
      return toLoaded(hit.headers.get(PRESENT) === "yes" ? await hit.text() : null, channel);
    }
  }
  const object = await store.get(`channels/${channel}.json`);
  const text = object ? await object.text() : null;
  const copy = new Response(text ?? "", {
    headers: {
      "Cache-Control": `max-age=${MANIFEST_TTL_MS / 1000}`,
      "Content-Type": "application/json",
      [FETCHED_AT]: String(deps.now()),
      [PRESENT]: text === null ? "no" : "yes",
    },
  });
  deps.waitUntil(deps.cache.put(key, copy));
  return toLoaded(text, channel);
}

function toLoaded(text: string | null, channel: Channel): Loaded {
  if (text === null) return { state: "absent" };
  return { state: "read", ...parseManifest(text, channel) };
}
