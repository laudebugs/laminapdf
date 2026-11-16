// The channel manifests' operations (ADR 0014; apps/updates/README.md): read, check, pause, resume,
// roll out, promote and upload a hand-edited file. It reads and writes the buckets through R2's S3
// API (scripts/r2.ts), with each bucket's token and Access Key ID from the environment, as the
// desktop workflows hold them; `--local` uses the package's own Wrangler on its local R2 instead.
// Every write is checked with the Worker's validator first, and nothing is written when the
// manifest wouldn't change.
//
//   pnpm --filter @laminapdf/updates manifest <command> [--local]
//     show <channel>                         print the manifest (or the empty one)
//     validate <file> [<channel>]            check a file without uploading it
//     put <channel> <file>                   check a hand-edited file and upload it (bridges)
//     pause <channel> | resume <channel>     the kill switch
//     set-rollout <channel> <percent>        move the release to 0–100 % of installs
//     promote <version> --from <channel> --to <channel> --rollout <percent>
//     upload <channel> <dir> --version <v>   the release workflow's files, <dir>/<platform>/<file>,
//                                            to releases/<v>/<platform>/<file> in the channel's bucket
//     publish <channel> <dir> --version <v> --downloads <url> --notes <text> --rollout <percent>
//                                            make the uploaded build the channel's release
//     verify <channel> <version> [--server <url>]   ask the Worker for each platform's update
//     downloads <channel> <dir> --version <v> --downloads <url>
//                                            write downloads/<channel>.json, the website's list of
//                                            the installers (scripts/downloads.ts)
//
// --local works on Wrangler's local R2 (what `pnpm dev` serves) instead of the real buckets; without
// it, the bucket's credentials must be in the environment (scripts/r2.ts names them).
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";
import { CHANNEL_BUCKETS, CHANNELS, type Channel, isChannel } from "../src/channels.ts";
import { type Manifest, parseManifest } from "../src/manifest.ts";
import { type BuiltFolder, downloadsFrom } from "./downloads.ts";
import { emptyManifest, promote, publishRelease, setPaused, setRollout } from "./edits.ts";
import { R2Bucket } from "./r2.ts";
import { releaseFrom } from "./release.ts";

const local = process.argv.includes("--local");
const args = process.argv.slice(2).filter((a) => a !== "--local");

function fail(message: string): never {
  console.error(`manifest: ${message}`);
  process.exit(1);
}

function channelOf(value: string | undefined): Channel {
  if (!isChannel(value)) fail(`expected a channel: ${CHANNELS.join(", ")}`);
  return value;
}

function percentOf(value: string | undefined): number {
  const percent = Number(value);
  if (value === undefined || value.trim() === "" || !Number.isFinite(percent)) fail("expected a percentage, 0 to 100");
  return percent;
}

function option(name: string): string | undefined {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
}

function wrangler(argv: string[]): { status: number; stdout: string; stderr: string } {
  const run = spawnSync("pnpm", ["exec", "wrangler", ...argv], {
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  return { status: run.status ?? 1, stdout: run.stdout, stderr: run.stderr };
}

function bucketOf(channel: Channel): string {
  return CHANNEL_BUCKETS[channel].bucket;
}

function keyOf(channel: Channel): string {
  return `channels/${channel}.json`;
}

function objectPath(channel: Channel): string {
  return `${bucketOf(channel)}/${keyOf(channel)}`;
}

/** Where the manifests are: R2's S3 API, or Wrangler's local R2 with `--local`. */
interface ManifestStore {
  /** The manifest's text, or `undefined` when the channel has none yet. */
  readText(channel: Channel): Promise<string | undefined>;
  /** Stores `text` as the channel's manifest, never cached by the CDN. */
  writeText(channel: Channel, text: string): Promise<void>;
}

const HEADERS = { contentType: "application/json", cacheControl: "no-store" };

const s3Store: ManifestStore = {
  async readText(channel) {
    const bytes = await R2Bucket.open(bucketOf(channel)).getObject(keyOf(channel));
    return bytes === undefined ? undefined : new TextDecoder().decode(bytes);
  },
  async writeText(channel, text) {
    await R2Bucket.open(bucketOf(channel)).putObject(keyOf(channel), text, HEADERS);
  },
};

const localStore: ManifestStore = {
  async readText(channel) {
    const run = wrangler(["r2", "object", "get", objectPath(channel), "--pipe", "--local"]);
    if (run.status === 0) return run.stdout;
    if (/does not exist|not found|NoSuchKey/i.test(run.stderr + run.stdout)) return undefined;
    fail(`couldn't read ${objectPath(channel)}:\n${run.stderr}`);
  },
  async writeText(channel, text) {
    const dir = mkdtempSync(join(tmpdir(), "lamina-manifest-"));
    try {
      const file = join(dir, `${channel}.json`);
      writeFileSync(file, text);
      const run = wrangler([
        "r2",
        "object",
        "put",
        objectPath(channel),
        "--file",
        file,
        "--content-type",
        HEADERS.contentType,
        "--cache-control",
        HEADERS.cacheControl,
        "--local",
      ]);
      if (run.status !== 0) fail(`couldn't write ${objectPath(channel)}:\n${run.stderr}`);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  },
};

const store = local ? localStore : s3Store;

/** The channel's manifest in R2, or the empty one when the channel has none yet. */
async function read(channel: Channel): Promise<Manifest> {
  const text = await store.readText(channel);
  if (text === undefined) return emptyManifest(channel);
  const result = parseManifest(text, channel);
  if (!result.ok) fail(`${objectPath(channel)} is invalid:\n  ${result.errors.join("\n  ")}`);
  return result.manifest;
}

/** Uploads `next` unless it equals `before`; R2 takes about one write a second per key. */
async function write(before: Manifest, next: Manifest): Promise<void> {
  if (JSON.stringify(before) === JSON.stringify(next)) {
    console.log(`${next.channel}: unchanged, nothing written`);
    return;
  }
  await store.writeText(next.channel, `${JSON.stringify(next, null, 2)}\n`);
  console.log(`${next.channel}: written to ${objectPath(next.channel)}; the Worker serves it within 30 s`);
}

function fromFile(path: string | undefined, channel?: Channel): Manifest {
  if (!path) fail("expected a manifest file");
  const result = parseManifest(readFileSync(path, "utf8"), channel);
  if (!result.ok) fail(`${path} is invalid:\n  ${result.errors.join("\n  ")}`);
  return result.manifest;
}

/** Each `<dir>/<platform>/` folder the release workflow downloaded, with its files. */
function builtPlatforms(dir: string | undefined): BuiltFolder[] {
  if (!dir) fail("expected the folder of built files");
  return readdirSync(dir)
    .filter((platform) => statSync(join(dir, platform)).isDirectory())
    .map((platform) => ({
      platform,
      files: readdirSync(join(dir, platform)).filter((file) => statSync(join(dir, platform, file)).isFile()),
      read: (file: string) => readFileSync(join(dir, platform, file), "utf8"),
      size: (file: string) => statSync(join(dir, platform, file)).size,
    }));
}

/** The type a download is served with, by its name. */
function contentType(file: string): string {
  if (file.endsWith(".json")) return "application/json";
  if (file.endsWith(".sig") || file.endsWith(".asc") || file === "SHA256SUMS") return "text/plain; charset=utf-8";
  if (file.endsWith(".gz")) return "application/gzip";
  return "application/octet-stream";
}

/** Puts one file at `key` in `channel`'s bucket: R2's S3 API, or Wrangler's local R2. */
async function putFile(
  channel: Channel,
  key: string,
  file: string,
  headers: { contentType: string; cacheControl: string },
): Promise<void> {
  if (!local) {
    await R2Bucket.open(bucketOf(channel)).putObject(key, { file }, headers);
    return;
  }
  const path = `${bucketOf(channel)}/${key}`;
  const run = wrangler([
    "r2",
    "object",
    "put",
    path,
    "--file",
    file,
    "--content-type",
    headers.contentType,
    "--cache-control",
    headers.cacheControl,
    "--local",
  ]);
  if (run.status !== 0) fail(`couldn't upload ${path}:\n${run.stderr}`);
}

/**
 * Uploads every built file. Versioned paths never change once written, so they are cached for a
 * year; staging's may be overwritten by a later test, so they are never cached (ADR 0014 §1).
 */
async function upload(channel: Channel, dir: string | undefined, version: string | undefined): Promise<void> {
  if (!version) fail("expected --version");
  const cacheControl = channel === "staging" ? "no-store" : "public, max-age=31536000, immutable";
  for (const { platform, files } of builtPlatforms(dir)) {
    for (const file of files) {
      const key = `releases/${version}/${platform}/${file}`;
      await putFile(channel, key, join(dir ?? "", platform, file), { contentType: contentType(file), cacheControl });
      console.log(`uploaded ${bucketOf(channel)}/${key}`);
    }
  }
}

/** Asks the Worker for each platform's update as a fresh install would, and checks the answer. */
async function verify(channel: Channel, version: string | undefined, server: string): Promise<void> {
  if (!version) fail("expected the version to verify");
  const release = (await read(channel)).release;
  if (release?.version !== version) fail(`${channel}'s release is ${release?.version ?? "none"}, not ${version}`);
  const problems: string[] = [];
  for (const [platform, entry] of Object.entries(release.platforms)) {
    const dash = platform.indexOf("-");
    const url = `${server}/${channel}/${platform.slice(0, dash)}/${platform.slice(dash + 1)}/0.0.0`;
    const answer = await fetch(url, { headers: { "Lamina-Install-Id": randomUUID() } });
    const body =
      answer.status === 200 ? ((await answer.json()) as { version?: string; url?: string; signature?: string }) : {};
    if (
      answer.status !== 200 ||
      body.version !== version ||
      body.url !== entry?.url ||
      body.signature !== entry?.signature
    ) {
      problems.push(`${platform}: ${url} answered ${answer.status} ${JSON.stringify(body)}`);
      continue;
    }
    const download = await fetch(entry.url, { method: "HEAD" });
    if (!download.ok) problems.push(`${platform}: ${entry.url} answered ${download.status}`);
    else console.log(`${platform}: ${version} at ${entry.url}`);
  }
  if (problems.length > 0) fail(`the Worker doesn't serve ${version} everywhere:\n  ${problems.join("\n  ")}`);
}

/**
 * Writes `downloads/<channel>.json`, the website's list of the installers, once the Worker serves
 * the release. Cached five minutes: the website reads it when it deploys.
 */
async function writeDownloads(channel: Channel, dir: string | undefined, version: string, downloads: string) {
  const pubDate = new Date().toISOString().replace(/\.\d+Z$/, "Z");
  const list = downloadsFrom({ channel, version, pubDate, downloads }, builtPlatforms(dir));
  const temp = mkdtempSync(join(tmpdir(), "lamina-downloads-"));
  try {
    const file = join(temp, `${channel}.json`);
    writeFileSync(file, `${JSON.stringify(list, null, 2)}\n`);
    const key = `downloads/${channel}.json`;
    await putFile(channel, key, file, { contentType: "application/json", cacheControl: "public, max-age=300" });
    console.log(`${channel}: ${list.files.length} installers listed in ${bucketOf(channel)}/${key}`);
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

async function run(): Promise<void> {
  const [command, first, second] = args;
  switch (command) {
    case "show":
      console.log(JSON.stringify(await read(channelOf(first)), null, 2));
      return;
    case "validate":
      fromFile(first, second === undefined ? undefined : channelOf(second));
      console.log(`${first}: valid`);
      return;
    case "put": {
      const channel = channelOf(first);
      await write(await read(channel), fromFile(second, channel));
      return;
    }
    case "pause":
    case "resume": {
      const before = await read(channelOf(first));
      await write(before, setPaused(before, command === "pause"));
      return;
    }
    case "set-rollout": {
      const before = await read(channelOf(first));
      await write(before, setRollout(before, percentOf(second)));
      return;
    }
    case "promote": {
      const from = await read(channelOf(option("--from")));
      const before = await read(channelOf(option("--to")));
      if (!first) fail("expected a version to promote");
      await write(before, promote(from, before, first, percentOf(option("--rollout"))));
      return;
    }
    case "upload":
      await upload(channelOf(first), second, option("--version"));
      return;
    case "publish": {
      const channel = channelOf(first);
      const downloads = option("--downloads");
      const notes = option("--notes");
      if (!downloads || notes === undefined) fail("expected --downloads and --notes");
      const version = option("--version") ?? fail("expected --version");
      const pubDate = new Date().toISOString().replace(/\.\d+Z$/, "Z");
      const release = releaseFrom({ version, pubDate, notes, downloads }, builtPlatforms(second));
      const before = await read(channel);
      await write(before, publishRelease(before, release, percentOf(option("--rollout"))));
      return;
    }
    case "verify":
      await verify(channelOf(first), second, option("--server") ?? "https://updates.laminapdf.com");
      return;
    case "downloads": {
      const downloads = option("--downloads") ?? fail("expected --downloads");
      const version = option("--version") ?? fail("expected --version");
      await writeDownloads(channelOf(first), second, version, downloads);
      return;
    }
    default:
      fail(
        "commands: show, validate, put, pause, resume, set-rollout, promote, upload, publish, verify, downloads (see scripts/manifest.ts)",
      );
  }
}

run().catch((error: unknown) => {
  fail(error instanceof Error ? error.message : String(error));
});
