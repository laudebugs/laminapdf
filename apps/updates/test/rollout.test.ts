import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { BUCKETS, bucketFor, inRollout, installIdOf } from "../src/rollout.ts";
import { INSTALL_IDS } from "./helpers.ts";

describe("installIdOf", () => {
  it("lower-cases a UUID v4", () => {
    expect(installIdOf("0B7D4C1E-2F3A-4B5C-8D6E-7F8091A2B3C4")).toBe("0b7d4c1e-2f3a-4b5c-8d6e-7f8091a2b3c4");
  });

  it.each([
    ["missing", null],
    ["empty", ""],
    ["not a UUID", "install-1"],
    ["version 1", "0b7d4c1e-2f3a-1b5c-8d6e-7f8091a2b3c4"],
    ["wrong variant", "0b7d4c1e-2f3a-4b5c-cd6e-7f8091a2b3c4"],
    ["braces", "{0b7d4c1e-2f3a-4b5c-8d6e-7f8091a2b3c4}"],
    ["padded", " 0b7d4c1e-2f3a-4b5c-8d6e-7f8091a2b3c4"],
  ])("treats a %s header as absent", (_, header) => {
    expect(installIdOf(header)).toBeUndefined();
  });
});

describe("bucketFor", () => {
  it("is u32_be(SHA-256(id ‖ ':' ‖ version)[0..4]) mod 10 000", async () => {
    const [id] = INSTALL_IDS;
    const digest = createHash("sha256").update(`${id}:0.2.0`).digest();
    expect(await bucketFor(id, "0.2.0")).toBe(digest.readUInt32BE(0) % 10_000);
  });

  it("is stable for one ID and version", async () => {
    for (const id of INSTALL_IDS) {
      expect(await bucketFor(id, "0.2.0")).toBe(await bucketFor(id, "0.2.0"));
    }
  });

  it("reshuffles cohorts between versions", async () => {
    const a = await Promise.all(INSTALL_IDS.map((id) => bucketFor(id, "0.2.0")));
    const b = await Promise.all(INSTALL_IDS.map((id) => bucketFor(id, "0.3.0")));
    expect(a).not.toEqual(b);
  });

  it("puts 10 % ± 0.5 % of 100 000 random installs in a 10 % rollout", async () => {
    const total = 100_000;
    let inside = 0;
    for (let i = 0; i < total; i += 1) {
      const id = installIdOf(crypto.randomUUID());
      if (id === undefined) throw new Error("crypto.randomUUID() gave something that isn't a UUID v4");
      const bucket = await bucketFor(id, "0.2.0");
      expect(bucket).toBeGreaterThanOrEqual(0);
      expect(bucket).toBeLessThan(BUCKETS);
      if (inRollout(bucket, 10)) inside += 1;
    }
    expect(Math.abs(inside / total - 0.1)).toBeLessThanOrEqual(0.005);
  });
});

describe("inRollout", () => {
  it("lets everyone in at 100 %, with or without an ID", () => {
    expect(inRollout(9_999, 100)).toBe(true);
    expect(inRollout(undefined, 100)).toBe(true);
  });

  it("keeps installs without an ID out of a partial rollout", () => {
    expect(inRollout(undefined, 99.99)).toBe(false);
  });

  it("admits buckets below percent × 100", () => {
    expect(inRollout(999, 10)).toBe(true);
    expect(inRollout(1_000, 10)).toBe(false);
    expect(inRollout(0, 0.01)).toBe(true);
    expect(inRollout(1, 0.01)).toBe(false);
    expect(inRollout(0, 0)).toBe(false);
  });
});
