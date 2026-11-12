// The update endpoint's path: `/{channel}/{target}/{arch}/{current_version}`, the variables Tauri's
// updater fills in (https://v2.tauri.app/plugin/updater/). Anything else is `/healthz` or 404.
import { type Channel, isChannel } from "./channels.ts";
import { isOneOf } from "./one-of.ts";
import { isVersion } from "./semver.ts";

/** Tauri's `{{target}}` values. */
const TARGETS = ["linux", "windows", "darwin"] as const;
/** Tauri's `{{arch}}` values. */
const ARCHES = ["x86_64", "aarch64", "i686", "armv7"] as const;

/** A manifest's platform key, `{target}-{arch}`, such as `darwin-aarch64`. */
export type PlatformKey = `${(typeof TARGETS)[number]}-${(typeof ARCHES)[number]}`;

/**
 * What a request path asks for: an update check (with its parsed segments), the health check, an
 * unknown path (404) or a known channel with segments the updater never sends (400).
 */
export type Route =
  | { readonly kind: "update"; readonly channel: Channel; readonly platform: PlatformKey; readonly current: string }
  | { readonly kind: "health" }
  | { readonly kind: "not-found" }
  | { readonly kind: "bad-request" };

/** Whether `key` is a `{target}-{arch}` pair the updater can send. */
export function isPlatformKey(key: string): key is PlatformKey {
  const dash = key.indexOf("-");
  return dash > 0 && isOneOf(TARGETS, key.slice(0, dash)) && isOneOf(ARCHES, key.slice(dash + 1));
}

/** Reads a request path. An unknown channel is 404; a known channel with bad segments is 400. */
export function route(pathname: string): Route {
  if (pathname === "/healthz") return { kind: "health" };
  const parts = pathname.split("/");
  // "/a/b/c/d" splits into ["", "a", "b", "c", "d"].
  if (parts.length !== 5 || parts[0] !== "") return { kind: "not-found" };
  const [, channel = "", target = "", arch = "", current = ""] = parts;
  if (!isChannel(channel)) return { kind: "not-found" };
  if (!isOneOf(TARGETS, target) || !isOneOf(ARCHES, arch)) return { kind: "bad-request" };
  // Tauri sends the version without a leading "v".
  if (!isVersion(current)) return { kind: "bad-request" };
  return { kind: "update", channel, platform: `${target}-${arch}`, current };
}
