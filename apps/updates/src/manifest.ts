// A channel's manifest, `channels/{channel}.json` in its bucket (ADR 0014 §5). The Worker and the
// manifest tooling (scripts/manifest.ts) both validate with this module; schemas/manifest.schema.json
// describes the same shape for other tools, and a test holds the two together.
import { CHANNELS, type Channel, isChannel } from "./channels.ts";
import { isPlatformKey, type PlatformKey } from "./route.ts";
import { compare, isVersion } from "./semver.ts";

/** One platform's update: the bundle's address and the contents of its `.sig` file. */
export interface PlatformEntry {
  readonly url: string;
  readonly signature: string;
}

/** A release as the manifest lists it. */
export interface Release {
  readonly version: string;
  /** RFC 3339, as Tauri's `pub_date`. */
  readonly pub_date: string;
  readonly notes: string;
  /** 0–100, in steps of 0.01. Required on `release`; `previous` must be 100; bridges have none. */
  readonly rollout_percent?: number;
  readonly platforms: Readonly<Partial<Record<PlatformKey, PlatformEntry>>>;
}

/** A release served to every install below `below`, whatever the rollout and the pause. */
interface Bridge {
  readonly below: string;
  readonly release: Release;
}

/** A channel's manifest: what the Worker serves from, `channels/{channel}.json` in its bucket. */
export interface Manifest {
  readonly schema: 1;
  readonly channel: Channel;
  /** The kill switch: no `release` or `previous` is served; bridges still are. */
  readonly paused: boolean;
  readonly release?: Release;
  readonly previous?: Release;
  readonly bridges?: readonly Bridge[];
}

/** The result of checking a manifest: the manifest, or every problem found, each with its JSON path. */
export type Checked =
  | { readonly ok: true; readonly manifest: Manifest }
  | { readonly ok: false; readonly errors: string[] };

type Json = Record<string, unknown>;
/** How a release's `rollout_percent` is checked: required, fixed at 100, or not allowed. */
type Rollout = "required" | "hundred" | "forbidden";

const RFC3339 = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;
const TOP_KEYS = ["schema", "channel", "paused", "release", "previous", "bridges"];
const RELEASE_KEYS = ["version", "pub_date", "notes", "rollout_percent", "platforms"];

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isHttpsUrl(value: unknown): boolean {
  if (typeof value !== "string") return false;
  try {
    return new URL(value).protocol === "https:";
  } catch {
    return false;
  }
}

function isSemver(value: unknown): value is string {
  return typeof value === "string" && isVersion(value);
}

function unknownKeys(value: Json, allowed: readonly string[], at: string, errors: string[]): void {
  for (const key of Object.keys(value)) {
    if (!allowed.includes(key)) errors.push(`${at}/${key}: unknown key`);
  }
}

function isPercent(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isFinite(value) &&
    value >= 0 &&
    value <= 100 &&
    // Steps of 0.01: the rollout has 10 000 buckets.
    Math.abs(value * 100 - Math.round(value * 100)) < 1e-9
  );
}

function rolloutProblem(percent: unknown, rollout: Rollout): string | undefined {
  if (rollout === "forbidden") {
    return percent === undefined ? undefined : "a bridge applies to everyone below it, so it has none";
  }
  if (percent === undefined) return rollout === "required" ? "required" : undefined;
  if (!isPercent(percent)) return "must be 0 to 100 in steps of 0.01";
  return rollout === "hundred" && percent !== 100 ? "the previous release must be at 100" : undefined;
}

function checkPlatform(key: string, entry: unknown, at: string, errors: string[]): void {
  const p = `${at}/platforms/${key}`;
  if (!isPlatformKey(key)) errors.push(`${p}: not a {target}-{arch} key`);
  if (!isObject(entry)) {
    errors.push(`${p}: must be an object with url and signature`);
    return;
  }
  unknownKeys(entry, ["url", "signature"], p, errors);
  if (!isHttpsUrl(entry.url)) errors.push(`${p}/url: must be an https URL`);
  if (typeof entry.signature !== "string" || entry.signature.trim() === "") {
    errors.push(`${p}/signature: must be the .sig file's contents`);
  }
}

function checkRelease(value: unknown, at: string, rollout: Rollout, errors: string[]): void {
  if (!isObject(value)) {
    errors.push(`${at}: must be an object`);
    return;
  }
  unknownKeys(value, RELEASE_KEYS, at, errors);
  if (!isSemver(value.version)) errors.push(`${at}/version: must be a semantic version`);
  const date = value.pub_date;
  if (typeof date !== "string" || !RFC3339.test(date) || Number.isNaN(Date.parse(date))) {
    errors.push(`${at}/pub_date: must be an RFC 3339 date-time`);
  }
  if (typeof value.notes !== "string") errors.push(`${at}/notes: must be a string`);
  const problem = rolloutProblem(value.rollout_percent, rollout);
  if (problem) errors.push(`${at}/rollout_percent: ${problem}`);
  if (!isObject(value.platforms) || Object.keys(value.platforms).length === 0) {
    errors.push(`${at}/platforms: must list at least one platform`);
    return;
  }
  for (const [key, entry] of Object.entries(value.platforms)) checkPlatform(key, entry, at, errors);
}

function versionOf(value: unknown): string | undefined {
  return isObject(value) && isSemver(value.version) ? value.version : undefined;
}

function checkBridge(bridge: unknown, at: string, errors: string[]): void {
  if (!isObject(bridge)) {
    errors.push(`${at}: must be an object`);
    return;
  }
  unknownKeys(bridge, ["below", "release"], at, errors);
  if (!isSemver(bridge.below)) errors.push(`${at}/below: must be a semantic version`);
  checkRelease(bridge.release, `${at}/release`, "forbidden", errors);
  const version = versionOf(bridge.release);
  if (version && isSemver(bridge.below) && compare(version, bridge.below) < 0) {
    errors.push(`${at}/release/version: must be at least ${at}/below`);
  }
}

function checkHeader(value: Json, channel: Channel | undefined, errors: string[]): void {
  unknownKeys(value, TOP_KEYS, "", errors);
  if (value.schema !== 1) errors.push("/schema: must be 1");
  if (!isChannel(value.channel)) {
    errors.push(`/channel: must be one of ${CHANNELS.join(", ")}`);
  } else if (channel !== undefined && value.channel !== channel) {
    errors.push(`/channel: is ${value.channel}, but the file is ${channel}'s`);
  }
  if (typeof value.paused !== "boolean") errors.push("/paused: must be true or false");
}

function checkReleases(value: Json, errors: string[]): void {
  if (value.release !== undefined) checkRelease(value.release, "/release", "required", errors);
  if (value.previous !== undefined) checkRelease(value.previous, "/previous", "hundred", errors);
  const release = versionOf(value.release);
  const previous = versionOf(value.previous);
  if (release && previous && compare(previous, release) >= 0) {
    errors.push("/previous/version: must be older than /release/version");
  }
  if (value.bridges === undefined) return;
  if (!Array.isArray(value.bridges)) {
    errors.push("/bridges: must be a list");
    return;
  }
  value.bridges.forEach((bridge: unknown, i: number) => {
    checkBridge(bridge, `/bridges/${i}`, errors);
  });
}

/** Checks a parsed manifest; `channel`, when given, must match the manifest's own. */
export function checkManifest(value: unknown, channel?: Channel): Checked {
  if (!isObject(value)) return { ok: false, errors: ["/: must be an object"] };
  const errors: string[] = [];
  checkHeader(value, channel, errors);
  checkReleases(value, errors);
  // Every field the type names was checked above, so the value is a Manifest; src/manifest.test.ts
  // holds this validator to schemas/manifest.schema.json, case by case.
  return errors.length === 0 ? { ok: true, manifest: value as unknown as Manifest } : { ok: false, errors };
}

/** Parses and checks a manifest file's text. */
export function parseManifest(text: string, channel?: Channel): Checked {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return { ok: false, errors: ["/: not JSON"] };
  }
  return checkManifest(value, channel);
}
