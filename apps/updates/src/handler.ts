// The update endpoint (ADR 0014 §4): `GET|HEAD /{channel}/{target}/{arch}/{current_version}` answers
// 204 (no update) or 200 with Tauri's dynamic JSON; `/healthz` answers 200; anything else 404.
// The `Lamina-Install-Id` header is read here and nowhere else: it is checked, lower-cased, hashed
// for the rollout and dropped. It never goes into a log, an error or a subrequest (the Worker makes
// none: R2 is read through its binding).
import { type CacheDeps, loadManifest, type ManifestStore } from "./cache.ts";
import { installIdOf } from "./rollout.ts";
import { type Channel, route } from "./route.ts";
import { selectUpdate } from "./select.ts";

/** What the Worker logs: an event name and fields that never include a request header. */
export type LogEvent =
  | { readonly event: "manifest-invalid"; readonly channel: Channel; readonly errors: readonly string[] }
  | { readonly event: "manifest-unavailable"; readonly channel: Channel };

export interface HandlerDeps extends CacheDeps {
  /** Each channel's bucket (ADR 0014 §5). */
  readonly stores: Readonly<Record<Channel, ManifestStore>>;
  readonly log: (entry: LogEvent) => void;
}

const INSTALL_ID_HEADER = "Lamina-Install-Id";

// Every answer depends on the install-ID header, so no cache between the app and the Worker may keep
// one.
const NO_STORE = { "Cache-Control": "no-store" } as const;

function answer(request: Request, status: number, body?: string, type?: string): Response {
  const headers: Record<string, string> = { ...NO_STORE };
  if (type) headers["Content-Type"] = type;
  if (status === 405) headers.Allow = "GET, HEAD";
  return new Response(request.method === "HEAD" || status === 204 ? null : (body ?? null), { status, headers });
}

export async function handle(request: Request, deps: HandlerDeps): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") return answer(request, 405);
  const url = new URL(request.url);
  const target = route(url.pathname);
  switch (target.kind) {
    case "health":
      return answer(request, 200, "ok\n", "text/plain; charset=utf-8");
    case "not-found":
      return answer(request, 404);
    case "bad-request":
      return answer(request, 400);
    case "update":
      break;
  }
  const installId = installIdOf(request.headers.get(INSTALL_ID_HEADER));
  let loaded: Awaited<ReturnType<typeof loadManifest>>;
  try {
    loaded = await loadManifest(deps.stores[target.channel], target.channel, url.origin, deps);
  } catch {
    deps.log({ event: "manifest-unavailable", channel: target.channel });
    return answer(request, 503);
  }
  if (loaded.state === "absent") return answer(request, 204);
  if (!loaded.ok) {
    // A broken manifest offers nothing rather than failing every app's check.
    deps.log({ event: "manifest-invalid", channel: target.channel, errors: loaded.errors });
    return answer(request, 204);
  }
  const update = await selectUpdate(loaded.manifest, target.platform, target.current, installId);
  if (!update) return answer(request, 204);
  return answer(request, 200, JSON.stringify(update), "application/json");
}
