// The channel manifests' operations (ADR 0014; apps/updates/README.md): read, check, pause, resume,
// roll out, promote and upload a hand-edited file. It reads and writes R2 with the package's own
// Wrangler, so it uses the caller's Wrangler login or CLOUDFLARE_API_TOKEN, and the desktop release
// workflow calls it the same way. Every write is checked with the Worker's validator first, and
// nothing is written when the manifest wouldn't change.
//
//   pnpm --filter @laminapdf/updates manifest <command> [--local]
//     show <channel>                         print the manifest (or the empty one)
//     validate <file> [<channel>]            check a file without uploading it
//     put <channel> <file>                   check a hand-edited file and upload it (bridges)
//     pause <channel> | resume <channel>     the kill switch
//     set-rollout <channel> <percent>        move the release to 0–100 % of installs
//     promote <version> --from <channel> --to <channel> --rollout <percent>
//
// --local works on Wrangler's local R2 (what `pnpm dev` serves) instead of the real buckets.
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";
import { CHANNEL_BUCKETS, CHANNELS, type Channel, isChannel } from "../src/channels.ts";
import { type Manifest, parseManifest } from "../src/manifest.ts";
import { emptyManifest, promote, setPaused, setRollout } from "./edits.ts";

const where = process.argv.includes("--local") ? "--local" : "--remote";
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

function objectPath(channel: Channel): string {
  return `${CHANNEL_BUCKETS[channel].bucket}/channels/${channel}.json`;
}

/** The channel's manifest in R2, or the empty one when the channel has none yet. */
function read(channel: Channel): Manifest {
  const run = wrangler(["r2", "object", "get", objectPath(channel), "--pipe", where]);
  if (run.status !== 0) {
    if (/does not exist|not found|NoSuchKey/i.test(run.stderr + run.stdout)) return emptyManifest(channel);
    fail(`couldn't read ${objectPath(channel)}:\n${run.stderr}`);
  }
  const result = parseManifest(run.stdout, channel);
  if (!result.ok) fail(`${objectPath(channel)} is invalid:\n  ${result.errors.join("\n  ")}`);
  return result.manifest;
}

/** Uploads `next` unless it equals `before`; R2 takes about one write a second per key. */
function write(before: Manifest, next: Manifest): void {
  const text = `${JSON.stringify(next, null, 2)}\n`;
  if (JSON.stringify(before) === JSON.stringify(next)) {
    console.log(`${next.channel}: unchanged, nothing written`);
    return;
  }
  const dir = mkdtempSync(join(tmpdir(), "lamina-manifest-"));
  try {
    const file = join(dir, `${next.channel}.json`);
    writeFileSync(file, text);
    const run = wrangler([
      "r2",
      "object",
      "put",
      objectPath(next.channel),
      "--file",
      file,
      "--content-type",
      "application/json",
      "--cache-control",
      "no-store",
      where,
    ]);
    if (run.status !== 0) fail(`couldn't write ${objectPath(next.channel)}:\n${run.stderr}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  console.log(`${next.channel}: written to ${objectPath(next.channel)}; the Worker serves it within 30 s`);
}

function fromFile(path: string | undefined, channel?: Channel): Manifest {
  if (!path) fail("expected a manifest file");
  const result = parseManifest(readFileSync(path, "utf8"), channel);
  if (!result.ok) fail(`${path} is invalid:\n  ${result.errors.join("\n  ")}`);
  return result.manifest;
}

function run(): void {
  const [command, first, second] = args;
  switch (command) {
    case "show":
      console.log(JSON.stringify(read(channelOf(first)), null, 2));
      return;
    case "validate":
      fromFile(first, second === undefined ? undefined : channelOf(second));
      console.log(`${first}: valid`);
      return;
    case "put": {
      const channel = channelOf(first);
      write(read(channel), fromFile(second, channel));
      return;
    }
    case "pause":
    case "resume": {
      const before = read(channelOf(first));
      write(before, setPaused(before, command === "pause"));
      return;
    }
    case "set-rollout": {
      const before = read(channelOf(first));
      write(before, setRollout(before, percentOf(second)));
      return;
    }
    case "promote": {
      const from = read(channelOf(option("--from")));
      const before = read(channelOf(option("--to")));
      if (!first) fail("expected a version to promote");
      write(before, promote(from, before, first, percentOf(option("--rollout"))));
      return;
    }
    default:
      fail("commands: show, validate, put, pause, resume, set-rollout, promote (see scripts/manifest.ts)");
  }
}

try {
  run();
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
