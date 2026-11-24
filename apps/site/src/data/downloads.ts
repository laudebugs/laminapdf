// The download page's data (/download/, Paper S09): each channel's installers from its
// `downloads/{channel}.json`, which the desktop release workflow writes beside the release
// (apps/updates/scripts/downloads.ts writes the same shape). The page is built from
// src/data/downloads.json, a snapshot: production deploys refresh it first (scripts/downloads.ts,
// in site.yml), and previews and local builds show the committed one. LAMINA_PDF_DOWNLOADS names
// another file, as the e2e build's fixture does. No request leaves the page for the list, so the
// website's `connect-src 'self'` stands.
//
// Which installer suits the visitor is guessed in the browser (src/scripts/download.ts) from
// `visitorFrom`, here so tests can check it in Node. This module runs in the browser too, so the
// snapshot is read by downloads-snapshot.ts.

/** The channels the page offers, in tab order. Stable isn't released yet; nightlies install apart. */
export const CHANNELS = ["beta", "stable", "nightly"] as const;
export type Channel = (typeof CHANNELS)[number];

export type InstallerKind = "dmg" | "nsis" | "msi" | "appimage" | "deb" | "rpm";
const KINDS: readonly InstallerKind[] = ["dmg", "nsis", "msi", "appimage", "deb", "rpm"];

export type Os = "darwin" | "windows" | "linux";
export type Arch = "aarch64" | "x86_64";

/** One installer. */
export interface DownloadFile {
  readonly platform: `${Os}-${Arch}`;
  readonly kind: InstallerKind;
  readonly name: string;
  readonly size: number;
  readonly sha256: string;
  readonly url: string;
  readonly signature?: string;
}

/** A channel's list, schema 1. */
export interface Downloads {
  readonly schema: 1;
  readonly channel: Channel;
  readonly version: string;
  readonly pub_date: string;
  readonly checksums: Readonly<Record<string, { readonly url: string; readonly signature?: string }>>;
  readonly files: readonly DownloadFile[];
}

/** Each channel's list, or `null` before its first release. */
export type Snapshot = Readonly<Record<Channel, Downloads | null>>;

const PLATFORM = /^(darwin|windows|linux)-(aarch64|x86_64)$/;
const HASH = /^[0-9a-f]{64}$/;
/** Each channel's download host, where the release workflow writes its list (ADR 0014 §1). */
export const DOWNLOAD_HOSTS: Readonly<Record<Channel, string>> = {
  beta: "https://downloads.laminapdf.com",
  stable: "https://downloads.laminapdf.com",
  nightly: "https://nightly-downloads.laminapdf.com",
};
const HOSTS = [...new Set(Object.values(DOWNLOAD_HOSTS))].map((host) => `${host}/`);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isDownloadUrl(value: unknown): value is string {
  return typeof value === "string" && HOSTS.some((host) => value.startsWith(host));
}

function isFile(value: unknown): value is DownloadFile {
  if (!isRecord(value)) return false;
  const { platform, kind, name, size, sha256, url, signature } = value;
  return (
    typeof platform === "string" &&
    PLATFORM.test(platform) &&
    KINDS.includes(kind as InstallerKind) &&
    typeof name === "string" &&
    typeof size === "number" &&
    size > 0 &&
    typeof sha256 === "string" &&
    HASH.test(sha256) &&
    isDownloadUrl(url) &&
    (signature === undefined || isDownloadUrl(signature))
  );
}

/**
 * `value` as `channel`'s list, or the reason it isn't one. Every address must be on one of the
 * download hosts, so a bad list can't send a visitor elsewhere.
 */
export function parseDownloads(value: unknown, channel: Channel): Downloads | string {
  if (!isRecord(value)) return "not an object";
  if (value.schema !== 1) return `schema ${String(value.schema)}, not 1`;
  if (value.channel !== channel) return `channel ${String(value.channel)}, not ${channel}`;
  if (typeof value.version !== "string" || typeof value.pub_date !== "string") return "no version or date";
  if (!Array.isArray(value.files) || value.files.length === 0) return "no files";
  const bad = value.files.findIndex((file) => !isFile(file));
  if (bad >= 0) return `file ${bad} is malformed`;
  if (!isRecord(value.checksums)) return "no checksums";
  for (const [platform, sums] of Object.entries(value.checksums)) {
    if (!PLATFORM.test(platform) || !isRecord(sums) || !isDownloadUrl(sums.url)) return `bad checksums for ${platform}`;
  }
  return value as unknown as Downloads;
}

/** Every channel's list from `json`, the snapshot's text; throws on a malformed one. */
export function parseSnapshot(json: string): Snapshot {
  const raw: unknown = JSON.parse(json);
  if (!isRecord(raw)) throw new Error("the downloads snapshot isn't an object");
  const out: Partial<Record<Channel, Downloads | null>> = {};
  for (const channel of CHANNELS) {
    const value = raw[channel] ?? null;
    if (value === null) {
      out[channel] = null;
      continue;
    }
    const parsed = parseDownloads(value, channel);
    if (typeof parsed === "string") throw new Error(`the ${channel} downloads list is invalid: ${parsed}`);
    out[channel] = parsed;
  }
  return out as Snapshot;
}

/** What the browser says about the visitor's computer. */
export interface Visitor {
  /** `undefined` on a phone or tablet, or a system the app doesn't run on. */
  readonly os: Os | undefined;
  /** `undefined` when the browser doesn't say (Safari and Firefox on a Mac always say Intel). */
  readonly arch: Arch | undefined;
  readonly mobile: boolean;
}

/**
 * The visitor from User-Agent Client Hints (`platform`, `architecture`, `mobile`: Chromium only)
 * or else the user agent string. A Mac's user agent says Intel on Apple silicon too, so a Mac is
 * left without an architecture unless the hints give one.
 */
export function visitorFrom(
  hints: { platform?: string; architecture?: string; mobile?: boolean } | undefined,
  userAgent: string,
): Visitor {
  const mobile = hints?.mobile ?? /Android|iPhone|iPad|Mobile/i.test(userAgent);
  if (mobile) return { os: undefined, arch: undefined, mobile: true };
  const os = osFrom(hints?.platform, userAgent);
  return { os, arch: archFrom(hints?.architecture, os, userAgent), mobile: false };
}

/** The system from the hint's platform, else the user agent; `undefined` for ChromeOS and others. */
function osFrom(platform: string | undefined, userAgent: string): Os | undefined {
  const named: Readonly<Record<string, Os>> = { macos: "darwin", windows: "windows", linux: "linux" };
  if (platform !== undefined && platform !== "") return named[platform.toLowerCase()];
  if (/Macintosh|Mac OS X/.test(userAgent)) return "darwin";
  if (/Windows NT/.test(userAgent)) return "windows";
  if (/Linux|X11/.test(userAgent) && !/CrOS/.test(userAgent)) return "linux";
  return undefined;
}

/** The processor from the hint, else the user agent, which on a Mac always says Intel. */
function archFrom(architecture: string | undefined, os: Os | undefined, userAgent: string): Arch | undefined {
  if (architecture === "arm") return "aarch64";
  if (architecture === "x86") return "x86_64";
  if (os === undefined || os === "darwin") return undefined;
  return /aarch64|arm64/i.test(userAgent) ? "aarch64" : "x86_64";
}

/** The installer each system is offered first. */
const FIRST_KIND: Readonly<Record<Os, InstallerKind>> = { darwin: "dmg", windows: "nsis", linux: "appimage" };

/**
 * The platforms whose first installer to offer `visitor`: one, or a Mac's two (Apple silicon, then
 * Intel) when its processor is unknown; none on a phone or an unknown system.
 */
export function offeredPlatforms(visitor: Visitor): string[] {
  const { os, arch } = visitor;
  if (os === undefined) return [];
  const arches: readonly Arch[] = arch === undefined ? ["aarch64", "x86_64"] : [arch];
  return arches.map((a) => `${os}-${a}`);
}

/** The installers a card may offer: each platform's first kind, in table order. */
export function cardFiles(files: readonly DownloadFile[]): DownloadFile[] {
  return tableOrder(files).filter((file) => file.kind === FIRST_KIND[file.platform.split("-")[0] as Os]);
}

/**
 * The systems a channel's release lists but the page doesn't offer yet: macOS joins the beta once
 * Apple has notarised it (todo D7g), and its installers stay off the page until then, so no link
 * reaches one. Nightlies offer macOS: they are unsigned and say so.
 */
export const COMING_SOON: Readonly<Record<Channel, readonly Os[]>> = {
  beta: ["darwin"],
  stable: ["darwin"],
  nightly: [],
};

/** `list` without the files, and checksums, of the systems `channel` holds back. */
export function offered(list: Downloads, channel: Channel): Downloads {
  const held = (platform: string): boolean => COMING_SOON[channel].includes(platform.split("-")[0] as Os);
  return {
    ...list,
    files: list.files.filter((file) => !held(file.platform)),
    checksums: Object.fromEntries(Object.entries(list.checksums).filter(([platform]) => !held(platform))),
  };
}

/**
 * `offered(list, channel)`, or `null` when nothing is left to offer: a list of only held-back
 * systems must not show an empty table.
 */
export function offeredOrNull(list: Downloads | null, channel: Channel): Downloads | null {
  if (list === null) return null;
  const shown = offered(list, channel);
  return shown.files.length === 0 ? null : shown;
}

/** The page's words. */
export const WORDS = {
  title: "Download · Lamina PDF",
  description:
    "Download the Lamina PDF beta for Windows and Linux: read PDFs in tabs and windows, on your own computer, offline. macOS follows.",
  heading: "Lamina PDF for your desktop",
  lead: "Read PDFs on your own computer, in tabs and windows. Files stay on your machine; the app works offline.",
  channel: {
    beta: "Beta",
    stable: "Stable",
    nightly: "Nightly",
  },
  unreleased: {
    beta: "The first beta isn't out yet.",
    stable: "Stable isn't released yet. The beta comes first.",
    nightly: "No nightly has been published yet.",
  },
  notYet: "not released yet",
  // The owner approves this line with the page's other words (todo D7c).
  macSoon:
    "macOS is coming soon: it joins the beta once Apple has notarised the app. The Mac App Store build is a separate install and doesn't come from this page.",
  releaseNotes: "Release notes for",
  // The owner said on 2026-11-24 that the app is free.
  pricing: "Lamina PDF is free.",
  nightlyNote:
    "Nightly builds have the newest changes and aren't signed: macOS and Windows warn before opening them. They install apart from the beta and stable, and update only to newer nightlies.",
  guessed: "Guessed from your browser. Not right? Every download is listed below.",
  chooseBelow: "Choose your system below.",
  phone: "Lamina PDF for desktop runs on macOS, Windows and Linux. On this device, use the web reader.",
  openWebReader: "Open the web reader",
  allDownloads: "All downloads",
  columns: ["System", "File", "Processor", "Size", "SHA-256"] as const,
  copied: "SHA-256 copied",
  copyFailed: "Couldn't copy: select the hash instead",
  checksums: "SHA256SUMS",
  checksumsSignature: "SHA256SUMS.asc",
  signingKey: "the signing key",
  check: {
    heading: "Check your download",
    body: "Compare the file's SHA-256 with the one listed here, and check that this repository's release workflow built it:",
  },
  requirements: {
    heading: "System requirements",
    items: [
      "macOS 13 or later, on Apple silicon or Intel",
      "Windows 11 on x64 or Arm (Windows 10 22H2 is untested)",
      "Ubuntu 22.04 or later, or a supported Fedora, on x64 or Arm",
    ],
  },
  sends: {
    heading: "What the app sends",
    body: "Lamina PDF checks for updates once a day, and you can turn that off in Settings. Crash reports go only if you turn them on. Your documents never leave your computer.",
    privacy: "Privacy policy",
  },
  firstOpen: {
    heading: "Opening it the first time",
    windows:
      "Windows may show “Windows protected your PC”. Lamina PDF is new, so Microsoft SmartScreen hasn't seen many downloads of it yet. Select More info, check the publisher, then select Run anyway.",
    linux:
      "Make the AppImage executable (chmod +x on it) and run it; it updates itself. The .deb and .rpm update with your system's packages. If a window stays blank, choose Settings › Graphics › Compatibility, or start the app with",
    nightly:
      "A nightly opens on macOS only after you allow it in System Settings › Privacy & Security, and on Windows after More info › Run anyway.",
  },
} as const;

/** How the table names each system and processor, and notes an installer. */
export const OS_NAMES: Readonly<Record<Os, string>> = { darwin: "macOS", windows: "Windows", linux: "Linux" };
export const ARCH_NAMES: Readonly<Record<Os, Readonly<Record<Arch, string>>>> = {
  darwin: { aarch64: "Apple silicon", x86_64: "Intel" },
  windows: { aarch64: "Arm", x86_64: "x64" },
  linux: { aarch64: "Arm", x86_64: "x64" },
};
export const KIND_NOTES: Readonly<Partial<Record<InstallerKind, string>>> = {
  nsis: "installer",
  msi: "for managed installs",
  appimage: "updates itself",
  deb: "Debian, Ubuntu",
  rpm: "Fedora",
};

/** "For your Mac with Apple silicon", "For your Windows PC (x64)", …: the card's heading. */
export function forYour(file: DownloadFile): string {
  const [os, arch] = file.platform.split("-") as [Os, Arch];
  if (os === "darwin") return `For your Mac with ${arch === "aarch64" ? "Apple silicon" : "an Intel processor"}`;
  return `For your ${os === "windows" ? "Windows PC" : "Linux computer"} (${ARCH_NAMES[os][arch]})`;
}

/** A size in megabytes, one decimal: "12.3 MB". */
export function megabytes(size: number): string {
  return `${(size / 1_000_000).toFixed(1)} MB`;
}

/** The table's rows: macOS, Windows then Linux; Apple silicon before Intel, x64 before Arm; by kind. */
export function tableOrder(files: readonly DownloadFile[]): DownloadFile[] {
  const rank = (file: DownloadFile): number => {
    const [os, arch] = file.platform.split("-") as [Os, Arch];
    const first: Arch = os === "darwin" ? "aarch64" : "x86_64";
    return ["darwin", "windows", "linux"].indexOf(os) * 2 + (arch === first ? 0 : 1);
  };
  return [...files].sort((a, b) => rank(a) - rank(b) || KINDS.indexOf(a.kind) - KINDS.indexOf(b.kind));
}
