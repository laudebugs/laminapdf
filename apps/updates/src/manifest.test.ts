import { readFileSync } from "node:fs";
import { Ajv2020 } from "ajv/dist/2020.js";
import { describe, expect, it } from "vitest";
import { fixture } from "../test/helpers.ts";
import { checkManifest, parseManifest } from "./manifest.ts";

const schema: object = JSON.parse(readFileSync(new URL("../schemas/manifest.schema.json", import.meta.url), "utf8"));
const validate = new Ajv2020({ allErrors: true, multipleOfPrecision: 9 }).compile(schema);

function errorsOf(value: unknown): string[] {
  const checked = checkManifest(value);
  return checked.ok ? [] : checked.errors;
}

const beta = fixture("beta");

// The fixture as plain JSON the cases below may break: every field optional and loosely typed.
interface LoosePlatform {
  url?: string;
  signature?: string;
}
interface LooseRelease {
  version?: string;
  pub_date?: string;
  rollout_percent?: number;
  // The fixture's key, and the one a case adds.
  platforms: { "darwin-aarch64": LoosePlatform; "macos-arm64"?: LoosePlatform } | Record<string, never>;
}
interface Loose {
  schema?: unknown;
  extra?: unknown;
  paused?: unknown;
  release: LooseRelease;
  previous: LooseRelease;
  bridges: [{ below: string; release: LooseRelease }];
}

/** The beta fixture with one change made by `edit`. */
function edited(edit: (m: Loose) => void): unknown {
  const copy = structuredClone(beta) as unknown as Loose;
  edit(copy);
  return copy;
}

describe("checkManifest", () => {
  it.each(["beta", "staging", "nightly"])("accepts the %s fixture", (name) => {
    expect(errorsOf(fixture(name))).toEqual([]);
  });

  it("accepts a channel with no release yet", () => {
    expect(errorsOf({ schema: 1, channel: "stable", paused: false })).toEqual([]);
  });

  it("refuses a manifest stored under another channel's name", () => {
    expect(checkManifest(beta, "stable")).toMatchObject({ ok: false });
  });

  it("reports text that isn't JSON", () => {
    expect(parseManifest("{", "beta")).toEqual({ ok: false, errors: ["/: not JSON"] });
  });

  // Each case breaks one rule; the schema must refuse it too unless `schemaToo` is false (rules
  // that compare two versions are beyond JSON Schema).
  const broken: [string, (m: Loose) => void, boolean][] = [
    ["an unknown schema", (m) => (m.schema = 2), true],
    ["an unknown key", (m) => (m.extra = true), true],
    ["a missing pause flag", (m) => delete m.paused, true],
    ["a release without rollout_percent", (m) => delete m.release.rollout_percent, true],
    ["a rollout over 100", (m) => (m.release.rollout_percent = 101), true],
    ["a rollout finer than 0.01", (m) => (m.release.rollout_percent = 10.005), true],
    ["a previous release below 100 %", (m) => (m.previous.rollout_percent = 50), true],
    ["a version that isn't semver", (m) => (m.release.version = "v0.2.0"), true],
    ["a pub_date that isn't RFC 3339", (m) => (m.release.pub_date = "15 Jan 2027"), true],
    [
      "a platform key Tauri never sends",
      (m) => (m.release.platforms["macos-arm64"] = m.release.platforms["darwin-aarch64"]),
      true,
    ],
    ["an http URL", (m) => (m.release.platforms["darwin-aarch64"].url = "http://example.com/a"), true],
    ["an empty signature", (m) => (m.release.platforms["darwin-aarch64"].signature = " "), true],
    ["a release with no platforms", (m) => (m.release.platforms = {}), true],
    ["a bridge with no platforms", (m) => (m.bridges[0].release.platforms = {}), true],
    ["a bridge platform without url", (m) => delete m.bridges[0].release.platforms["darwin-aarch64"].url, true],
    [
      "a bridge platform without signature",
      (m) => delete m.bridges[0].release.platforms["darwin-aarch64"].signature,
      true,
    ],
    ["a bridge with a rollout", (m) => (m.bridges[0].release.rollout_percent = 100), true],
    ["a bridge whose release is below its bound", (m) => (m.bridges[0].below = "0.1.1"), false],
    ["a previous release newer than the release", (m) => (m.previous.version = "0.3.0"), false],
  ];

  it.each(broken)("refuses %s", (_, edit, schemaToo) => {
    const value = edited(edit);
    expect(errorsOf(value)).not.toEqual([]);
    if (schemaToo) expect(validate(value)).toBe(false);
  });
});

describe("manifest.schema.json", () => {
  it.each(["beta", "staging", "nightly"])("accepts the %s fixture", (name) => {
    expect(validate(fixture(name)), JSON.stringify(validate.errors)).toBe(true);
  });
});
