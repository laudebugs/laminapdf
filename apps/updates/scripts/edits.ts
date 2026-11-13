// The manifest tool's changes, as functions from one manifest to the next (scripts/manifest.ts reads
// and writes R2 around them). Each result is checked with the Worker's own validator before it is
// written, so the tool can't publish a manifest the Worker would refuse.

import type { Channel } from "../src/channels.ts";
import { checkManifest, type Manifest, type Release } from "../src/manifest.ts";
import { compare } from "../src/semver.ts";

/** The manifest a channel starts with: nothing to serve, not paused. */
export function emptyManifest(channel: Channel): Manifest {
  return { schema: 1, channel, paused: false };
}

function checked(manifest: Manifest): Manifest {
  const result = checkManifest(manifest, manifest.channel);
  if (!result.ok) throw new Error(`the result isn't a valid manifest:\n  ${result.errors.join("\n  ")}`);
  return result.manifest;
}

/** The kill switch on or off. */
export function setPaused(manifest: Manifest, paused: boolean): Manifest {
  return checked({ ...manifest, paused });
}

/** Moves `release` to another share of installs (0–100, steps of 0.01). */
export function setRollout(manifest: Manifest, percent: number): Manifest {
  if (!manifest.release) throw new Error(`${manifest.channel} has no release to roll out`);
  return checked({ ...manifest, release: { ...manifest.release, rollout_percent: percent } });
}

/** The channels whose releases share the releases bucket and downloads.laminapdf.com. */
const PROMOTABLE: readonly Channel[] = ["beta", "stable"];

function releaseNamed(manifest: Manifest, version: string): Release | undefined {
  return [manifest.release, manifest.previous].find((r) => r?.version === version);
}

/**
 * Makes `version` of `from` the release of `to` at `percent`. The release it replaces becomes
 * `previous` when it reached 100 %; otherwise the old `previous` stays (ADR 0014 §2). Only between
 * beta and stable, whose bundles live in the same bucket.
 */
export function promote(from: Manifest, to: Manifest, version: string, percent: number): Manifest {
  if (!PROMOTABLE.includes(from.channel) || !PROMOTABLE.includes(to.channel)) {
    throw new Error("promote works between beta and stable only; nightly and staging bundles live in other buckets");
  }
  const release = releaseNamed(from, version);
  if (!release) throw new Error(`${from.channel} has no release ${version}`);
  const current = to.release;
  if (current && compare(current.version, version) >= 0) {
    throw new Error(`${to.channel} already has ${current.version}, which isn't older than ${version}`);
  }
  const retired = current?.rollout_percent === 100 ? current : to.previous;
  const { previous: _replaced, ...rest } = to;
  return checked({
    ...rest,
    release: { ...release, rollout_percent: percent },
    ...(retired ? { previous: retired } : {}),
  });
}
