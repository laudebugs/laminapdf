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
//
// --local works on Wrangler's local R2 (what `pnpm dev` serves) instead of the real buckets; without
// it, the bucket's credentials must be in the environment (scripts/r2.ts names them).
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";
import { CHANNEL_BUCKETS, CHANNELS, type Channel, isChannel } from "../src/channels.ts";
import { type Manifest, parseManifest } from "../src/manifest.ts";
import { emptyManifest, promote, setPaused, setRollout } from "./edits.ts";
import { R2 } from "./r2.ts";

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

/** The manifest's text in R2, or `undefined` when the channel has none yet. */
async function readText(channel: Channel): Promise<string | undefined> {
  if (!local) {
    const bytes = await R2.for(bucketOf(channel)).getObject(bucketOf(channel), keyOf(channel));
    return bytes === undefined ? undefined : new TextDecoder().decode(bytes);
  }
  const run = wrangler(["r2", "object", "get", objectPath(channel), "--pipe", "--local"]);
  if (run.status === 0) return run.stdout;
  if (/does not exist|not found|NoSuchKey/i.test(run.stderr + run.stdout)) return undefined;
  fail(`couldn't read ${objectPath(channel)}:\n${run.stderr}`);
}

/** The channel's manifest in R2, or the empty one when the channel has none yet. */
async function read(channel: Channel): Promise<Manifest> {
  const text = await readText(channel);
  if (text === undefined) return emptyManifest(channel);
  const result = parseManifest(text, channel);
  if (!result.ok) fail(`${objectPath(channel)} is invalid:\n  ${result.errors.join("\n  ")}`);
  return result.manifest;
}

/** Stores `text` as `channel`'s manifest, never cached by the CDN. */
async function writeText(channel: Channel, text: string): Promise<void> {
  const headers = { contentType: "application/json", cacheControl: "no-store" };
  if (!local) {
    await R2.for(bucketOf(channel)).putObject(bucketOf(channel), keyOf(channel), text, headers);
    return;
  }
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
      headers.contentType,
      "--cache-control",
      headers.cacheControl,
      "--local",
    ]);
    if (run.status !== 0) fail(`couldn't write ${objectPath(channel)}:\n${run.stderr}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** Uploads `next` unless it equals `before`; R2 takes about one write a second per key. */
async function write(before: Manifest, next: Manifest): Promise<void> {
  if (JSON.stringify(before) === JSON.stringify(next)) {
    console.log(`${next.channel}: unchanged, nothing written`);
    return;
  }
  await writeText(next.channel, `${JSON.stringify(next, null, 2)}\n`);
  console.log(`${next.channel}: written to ${objectPath(next.channel)}; the Worker serves it within 30 s`);
}

function fromFile(path: string | undefined, channel?: Channel): Manifest {
  if (!path) fail("expected a manifest file");
  const result = parseManifest(readFileSync(path, "utf8"), channel);
  if (!result.ok) fail(`${path} is invalid:\n  ${result.errors.join("\n  ")}`);
  return result.manifest;
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
    default:
      fail("commands: show, validate, put, pause, resume, set-rollout, promote (see scripts/manifest.ts)");
  }
}

run().catch((error: unknown) => {
  fail(error instanceof Error ? error.message : String(error));
});
