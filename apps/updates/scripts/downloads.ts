// A channel's downloads list, `downloads/{channel}.json` beside its releases in the channel's
// bucket: each installer a person downloads, with its size, its SHA-256 from the build's
// SHA256SUMS and its address, for the website's download page (apps/site/src/data/downloads.ts
// reads the same shape). scripts/manifest.ts's `downloads` command writes it once the update
// Worker serves the release; these functions decide, so tests can check them without R2.

import type { PlatformKey } from "../src/route.ts";
import { isPlatformKey } from "../src/route.ts";
import type { PlatformFiles } from "./release.ts";

/** What a person installs from, by file name; the updater's own bundles aren't listed. */
export type InstallerKind = "dmg" | "nsis" | "msi" | "appimage" | "deb" | "rpm";

const KINDS: readonly (readonly [InstallerKind, (name: string) => boolean])[] = [
  ["dmg", (name) => name.endsWith(".dmg")],
  ["nsis", (name) => name.endsWith("-setup.exe")],
  ["msi", (name) => name.endsWith(".msi")],
  ["appimage", (name) => name.endsWith(".AppImage")],
  ["deb", (name) => name.endsWith(".deb")],
  ["rpm", (name) => name.endsWith(".rpm")],
];

/** The kind of installer `name` is, or `undefined` for every other file. */
export function installerKind(name: string): InstallerKind | undefined {
  return KINDS.find(([, matches]) => matches(name))?.[0];
}

/** One installer in the list. */
interface DownloadFile {
  readonly platform: PlatformKey;
  readonly kind: InstallerKind;
  readonly name: string;
  readonly size: number;
  readonly sha256: string;
  readonly url: string;
  /** The detached GPG signature's address (Linux release builds), when there is one. */
  readonly signature?: string;
}

/** A channel's downloads list, version 1. */
export interface Downloads {
  readonly schema: 1;
  readonly channel: string;
  readonly version: string;
  /** RFC 3339. */
  readonly pub_date: string;
  /** Each platform's SHA256SUMS (and its `.asc`, when signed). */
  readonly checksums: Readonly<Partial<Record<PlatformKey, { readonly url: string; readonly signature?: string }>>>;
  readonly files: readonly DownloadFile[];
}

/** One platform's built folder, as `manifest downloads` reads it: release.ts's, with sizes. */
export interface BuiltFolder extends PlatformFiles {
  readonly size: (file: string) => number;
}

/** What the list is of. */
export interface DownloadsInput {
  readonly channel: string;
  readonly version: string;
  readonly pubDate: string;
  /** The bucket's download address, such as https://downloads.laminapdf.com. */
  readonly downloads: string;
}

/** SHA256SUMS' hashes by file name: `<hex>  <name>`, or `<hex> *<name>` in binary mode. */
export function parseSums(text: string): ReadonlyMap<string, string> {
  const sums = new Map<string, string>();
  for (const line of text.split("\n")) {
    const match = /^([0-9a-f]{64}) [ *](.+)$/.exec(line.trimEnd());
    if (match?.[1] !== undefined && match[2] !== undefined) sums.set(match[2], match[1]);
  }
  return sums;
}

/**
 * The downloads list for `input` from the built folders. Every installer must be in its folder's
 * SHA256SUMS, and every folder must hold one: a list that couldn't be checked stops the publish.
 */
export function downloadsFrom(input: DownloadsInput, built: readonly BuiltFolder[]): Downloads {
  const files: DownloadFile[] = [];
  const checksums: Partial<Record<PlatformKey, { url: string; signature?: string }>> = {};
  for (const folder of built) {
    const { platform } = folder;
    if (!isPlatformKey(platform)) throw new Error(`${platform} isn't a platform key such as darwin-aarch64`);
    if (!folder.files.includes("SHA256SUMS")) throw new Error(`${platform} has no SHA256SUMS`);
    const sums = parseSums(folder.read("SHA256SUMS"));
    const at = (file: string) =>
      `${input.downloads}/${["releases", input.version, platform, file].map(encodeURIComponent).join("/")}`;
    const signed = (file: string) => (folder.files.includes(`${file}.asc`) ? { signature: at(`${file}.asc`) } : {});
    checksums[platform] = { url: at("SHA256SUMS"), ...signed("SHA256SUMS") };
    for (const name of [...folder.files].sort()) {
      const kind = installerKind(name);
      if (kind === undefined) continue;
      const sha256 = sums.get(name);
      if (sha256 === undefined) throw new Error(`${platform}: ${name} isn't in SHA256SUMS`);
      files.push({ platform, kind, name, size: folder.size(name), sha256, url: at(name), ...signed(name) });
    }
  }
  if (files.length === 0) throw new Error("no installer was built");
  return {
    schema: 1,
    channel: input.channel,
    version: input.version,
    pub_date: input.pubDate,
    checksums,
    files,
  };
}
