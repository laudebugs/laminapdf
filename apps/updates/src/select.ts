// Which release, if any, an install gets (ADR 0014 §2, §4): bridges first, whatever the pause;
// then, unless the channel is paused, `release` for installs in its rollout and `previous` for the
// rest.
import type { Manifest, PlatformEntry, Release } from "./manifest.ts";
import { bucketFor, inRollout } from "./rollout.ts";
import type { PlatformKey } from "./route.ts";
import { compare } from "./semver.ts";

/** The update Tauri's dynamic endpoint returns (https://v2.tauri.app/plugin/updater/). */
export interface Update {
  readonly version: string;
  readonly pub_date: string;
  readonly url: string;
  readonly signature: string;
  readonly notes: string;
}

function updateOf(release: Release, entry: PlatformEntry): Update {
  return {
    version: release.version,
    pub_date: release.pub_date,
    url: entry.url,
    signature: entry.signature,
    notes: release.notes,
  };
}

function bridgeUpdate(manifest: Manifest, platform: PlatformKey, current: string): Update | undefined {
  // Lowest bound first: an install several key rotations behind takes the oldest bridge it needs.
  const bridges = [...(manifest.bridges ?? [])].sort((a, b) => compare(a.below, b.below));
  for (const bridge of bridges) {
    if (compare(current, bridge.below) >= 0) continue;
    const entry = bridge.release.platforms[platform];
    if (entry) return updateOf(bridge.release, entry);
  }
  return undefined;
}

/** `release` itself when it is newer, has the platform, and the install is in its rollout. */
async function rolloutUpdate(
  release: Release | undefined,
  platform: PlatformKey,
  current: string,
  installId: string | undefined,
): Promise<Update | undefined> {
  if (!release || compare(release.version, current) <= 0) return undefined;
  const entry = release.platforms[platform];
  if (!entry) return undefined;
  const bucket = installId === undefined ? undefined : await bucketFor(installId, release.version);
  return inRollout(bucket, release.rollout_percent ?? 100) ? updateOf(release, entry) : undefined;
}

/**
 * The update for an install of `current` on `platform`, or `undefined` (the endpoint's 204).
 * `installId` is the checked, lower-cased header (`installIdOf`); without one only releases at
 * 100 % are served.
 */
export async function selectUpdate(
  manifest: Manifest,
  platform: PlatformKey,
  current: string,
  installId: string | undefined,
): Promise<Update | undefined> {
  const bridge = bridgeUpdate(manifest, platform, current);
  if (bridge) return bridge;
  if (manifest.paused) return undefined;
  const update = await rolloutUpdate(manifest.release, platform, current, installId);
  // `previous` is at 100 %, so the rollout never holds it back.
  return update ?? (await rolloutUpdate(manifest.previous, platform, current, undefined));
}
