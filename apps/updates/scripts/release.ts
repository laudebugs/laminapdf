// A release as the desktop release workflow publishes it: which built file each platform
// updates from, and the manifest entry that names it. scripts/manifest.ts's `publish` reads the
// files; these functions decide, so tests can check them without R2.

import type { Release } from "../src/manifest.ts";
import { isPlatformKey, type PlatformKey } from "../src/route.ts";

/** What the Tauri updater installs on each OS (`createUpdaterArtifacts`, Tauri 2). */
const UPDATER_SUFFIX: Readonly<Record<string, string>> = {
  darwin: ".app.tar.gz",
  windows: "-setup.exe",
  linux: ".AppImage",
};

/** The file `platform` updates from among `files`, or undefined when there is none. */
export function updaterBundle(platform: PlatformKey, files: readonly string[]): string | undefined {
  const suffix = UPDATER_SUFFIX[platform.slice(0, platform.indexOf("-"))];
  const found = files.filter((file) => suffix !== undefined && file.endsWith(suffix));
  if (found.length > 1) throw new Error(`${platform} has more than one updater bundle: ${found.join(", ")}`);
  return found[0];
}

/** One platform's built files: the folder's name and the names of the files in it. */
export interface PlatformFiles {
  readonly platform: string;
  readonly files: readonly string[];
  /** Reads the text of one of `files`. */
  readonly read: (file: string) => string;
}

/** What `publish` was told about the release. */
export interface ReleaseInput {
  readonly version: string;
  /** RFC 3339. */
  readonly pubDate: string;
  readonly notes: string;
  /** The bucket's download address, such as https://nightly-downloads.laminapdf.com. */
  readonly downloads: string;
}

/**
 * The manifest's release for `input`, from each platform's updater bundle and its `.sig`. Every
 * platform must have both: a build that made no updater bundle stops the publish.
 */
export function releaseFrom(input: ReleaseInput, built: readonly PlatformFiles[]): Release {
  const platforms: Partial<Record<PlatformKey, { url: string; signature: string }>> = {};
  for (const { platform, files, read } of built) {
    if (!isPlatformKey(platform)) throw new Error(`${platform} isn't a platform key such as darwin-aarch64`);
    const bundle = updaterBundle(platform, files);
    if (bundle === undefined) throw new Error(`${platform} has no updater bundle`);
    if (!files.includes(`${bundle}.sig`)) throw new Error(`${platform}: ${bundle} has no .sig`);
    const path = ["releases", input.version, platform, bundle].map(encodeURIComponent).join("/");
    platforms[platform] = { url: `${input.downloads}/${path}`, signature: read(`${bundle}.sig`).trim() };
  }
  if (Object.keys(platforms).length === 0) throw new Error("no platform was built");
  return { version: input.version, pub_date: input.pubDate, notes: input.notes, platforms };
}
