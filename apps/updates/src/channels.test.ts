import { describe, expect, it } from "vitest";
import { wranglerConfig } from "../test/wrangler-config.ts";
import { CHANNEL_BUCKETS, CHANNELS, isChannel } from "./channels.ts";

describe("CHANNEL_BUCKETS", () => {
  const config = wranglerConfig();
  const expected = [...new Map(Object.values(CHANNEL_BUCKETS).map((b) => [b.binding, b.bucket])).entries()]
    .map(([binding, bucket_name]) => ({ binding, bucket_name }))
    .sort((a, b) => a.binding.localeCompare(b.binding));
  const bound = (buckets: readonly { binding: string; bucket_name: string }[] | undefined) =>
    [...(buckets ?? [])].sort((a, b) => a.binding.localeCompare(b.binding));

  it("names every channel", () => {
    expect(Object.keys(CHANNEL_BUCKETS).sort()).toEqual([...CHANNELS].sort());
  });

  it("matches the buckets wrangler.jsonc binds for production", () => {
    expect(bound(config.r2_buckets)).toEqual(expected);
  });

  it.each(Object.keys(config.env))("matches the buckets wrangler.jsonc binds for env.%s", (env) => {
    expect(bound(config.env[env]?.r2_buckets)).toEqual(expected);
  });
});

describe("isChannel", () => {
  it("accepts each channel and nothing else", () => {
    for (const channel of CHANNELS) expect(isChannel(channel)).toBe(true);
    for (const other of ["", "Stable", "canary", 1, null, undefined]) expect(isChannel(other)).toBe(false);
  });
});
