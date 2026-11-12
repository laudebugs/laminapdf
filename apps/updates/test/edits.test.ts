import { describe, expect, it } from "vitest";
import { emptyManifest, promote, setPaused, setRollout } from "../scripts/edits.ts";
import type { Manifest } from "../src/manifest.ts";
import { fixture } from "./helpers.ts";

const beta = fixture("beta");

describe("the manifest tool's edits", () => {
  it("pauses and resumes a channel", () => {
    expect(setPaused(beta, true).paused).toBe(true);
    expect(setPaused(setPaused(beta, true), false)).toEqual(beta);
  });

  it("moves the rollout", () => {
    expect(setRollout(beta, 50).release?.rollout_percent).toBe(50);
  });

  it("refuses a rollout the Worker would refuse", () => {
    expect(() => setRollout(beta, 101)).toThrow(/rollout_percent/);
    expect(() => setRollout(beta, 12.345)).toThrow(/steps of 0.01/);
  });

  it("refuses to roll out a channel with no release", () => {
    expect(() => setRollout(emptyManifest("stable"), 10)).toThrow(/no release/);
  });

  it("promotes a beta release into an empty stable channel", () => {
    const stable = promote(beta, emptyManifest("stable"), "0.2.0", 1);
    expect(stable.channel).toBe("stable");
    expect(stable.release).toEqual({ ...beta.release, rollout_percent: 1 });
    expect(stable.previous).toBeUndefined();
  });

  const release013 = beta.previous;
  if (!release013) throw new Error("the beta fixture has a previous release");

  it("retires a fully rolled-out release to previous", () => {
    const stable: Manifest = { ...emptyManifest("stable"), release: release013 };
    const next = promote(beta, stable, "0.2.0", 5);
    expect(next.previous?.version).toBe("0.1.3");
    expect(next.release?.rollout_percent).toBe(5);
  });

  it("keeps the old previous when the replaced release never reached everyone", () => {
    const partial: Manifest = {
      ...emptyManifest("stable"),
      release: { ...release013, version: "0.1.9", rollout_percent: 40 },
      previous: release013,
    };
    const next = promote(beta, partial, "0.2.0", 1);
    expect(next.previous?.version).toBe("0.1.3");
  });

  it("refuses to promote an older or equal version", () => {
    const stable = promote(beta, emptyManifest("stable"), "0.2.0", 1);
    expect(() => promote(beta, stable, "0.2.0", 1)).toThrow(/isn't older/);
  });

  it("refuses to promote out of the nightly or staging buckets", () => {
    expect(() => promote(fixture("nightly"), emptyManifest("stable"), "0.2.1-nightly.20270116.7", 1)).toThrow(
      /beta and stable only/,
    );
  });

  it("refuses an unknown version", () => {
    expect(() => promote(beta, emptyManifest("stable"), "9.9.9", 1)).toThrow(/no release 9.9.9/);
  });
});
