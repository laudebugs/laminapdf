// The npm packages of the `laminapdf` CLI (todo L4d), from dist's release archives:
//
//   node tools/cli-release/generate.ts --artifacts <dist's output dir> [--out target/cli-npm]
//
// writes one folder per platform package whose archive is there (the binary in bin/, the
// licences and third-party notices from the archive, a package.json with `os`, `cpu` and, on
// Linux, `libc` from packages/cli/platforms.json), and cli/, the @laminapdf/cli wrapper, with
// every platform package as an optional dependency at the same version. Build the wrapper first
// (`pnpm --filter @laminapdf/cli build`); then `npm pack` each folder. The version is the CLI
// crate's, the workspace's lockstep version (ADR 0017).
import { execFileSync } from "node:child_process";
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import process from "node:process";
import { parseArgs } from "node:util";
import { binaryName, type Platform } from "../../packages/cli/src/platform.ts";

const ROOT = resolve(import.meta.dirname, "../..");
const WRAPPER = join(ROOT, "packages/cli");
/** What each platform package ships besides its binary; dist puts all three in every archive. */
const LEGAL = ["LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md"];

/** `packages/cli/platforms.json`. */
export function platforms(): Platform[] {
  return JSON.parse(readFileSync(join(WRAPPER, "platforms.json"), "utf8")) as Platform[];
}

/** The workspace's version, which `laminapdf-cli` inherits (`version.workspace = true`). */
export function workspaceVersion(): string {
  const manifest = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
  const section = manifest.split("[workspace.package]")[1] ?? "";
  const version = /^version = "([^"]+)"$/m.exec(section)?.[1];
  if (version === undefined) throw new Error("Cargo.toml has no [workspace.package] version");
  return version;
}

/** The folder a platform package is written to: its name after `@laminapdf/`. */
export function folderName(platform: Platform): string {
  return platform.package.replace("@laminapdf/", "");
}

/** A platform package's package.json. */
export function platformManifest(platform: Platform, version: string): Record<string, unknown> {
  return {
    name: platform.package,
    version,
    description: `The laminapdf command-line tool's binary for ${platform.target}. Install @laminapdf/cli, which picks it.`,
    license: "MIT OR Apache-2.0",
    homepage: "https://laminapdf.com",
    repository: { type: "git", url: "git+https://github.com/laudebugs/laminapdf.git", directory: "packages/cli" },
    os: [platform.os],
    cpu: [platform.cpu],
    ...(platform.libc === undefined ? {} : { libc: [platform.libc] }),
    files: ["bin", ...LEGAL],
    preferUnplugged: true,
    publishConfig: { access: "public" },
  };
}

/** The wrapper's package.json: the workspace one at `version`, publishable, with the platforms. */
export function wrapperManifest(base: Record<string, unknown>, all: Platform[], version: string) {
  const { private: _private, devDependencies: _dev, scripts: _scripts, ...rest } = base;
  const optionalDependencies = Object.fromEntries(all.map((p) => [p.package, version]));
  return { ...rest, version, optionalDependencies };
}

/** The archive dist wrote for `platform` in `dir`, if any. */
function archiveFor(dir: string, platform: Platform): string | undefined {
  const stem = join(dir, `laminapdf-cli-${platform.target}`);
  return [`${stem}.tar.xz`, `${stem}.zip`].find((path) => existsSync(path));
}

/**
 * The command that unpacks `archive` into `dir` on `host` (a `process.platform`): GNU tar, the
 * Linux runner's, reads no zip, so a Windows `.zip` goes to unzip there; bsdtar (macOS,
 * Windows) reads both.
 */
export function unpackCommand(archive: string, dir: string, host: string): [string, string[]] {
  if (archive.endsWith(".zip") && host !== "darwin" && host !== "win32") {
    return ["unzip", ["-q", archive, "-d", dir]];
  }
  return ["tar", ["-xf", archive, "-C", dir]];
}

function writePlatform(platform: Platform, archive: string, out: string, version: string): void {
  const unpacked = mkdtempSync(join(tmpdir(), "laminapdf-cli-"));
  try {
    execFileSync(...unpackCommand(archive, unpacked, process.platform));
    const from = join(unpacked, `laminapdf-cli-${platform.target}`);
    const dir = join(out, folderName(platform));
    mkdirSync(join(dir, "bin"), { recursive: true });
    cpSync(join(from, binaryName(platform)), join(dir, "bin", binaryName(platform)));
    chmodSync(join(dir, "bin", binaryName(platform)), 0o755);
    for (const file of LEGAL) cpSync(join(from, file), join(dir, file));
    writeFileSync(join(dir, "package.json"), `${JSON.stringify(platformManifest(platform, version), null, 2)}\n`);
    writeFileSync(
      join(dir, "README.md"),
      `# ${platform.package}\n\nThe \`laminapdf\` binary for ${platform.target}. Install [@laminapdf/cli](https://www.npmjs.com/package/@laminapdf/cli) instead: it depends on this package and runs the binary.\n`,
    );
  } finally {
    rmSync(unpacked, { recursive: true, force: true });
  }
}

function writeWrapper(out: string, all: Platform[], version: string): void {
  const dir = join(out, "cli");
  if (!existsSync(join(WRAPPER, "dist/launcher.js"))) {
    throw new Error("packages/cli is not built: run `pnpm --filter @laminapdf/cli build` first");
  }
  for (const file of ["bin", "dist", "platforms.json", "README.md"])
    cpSync(join(WRAPPER, file), join(dir, file), { recursive: true });
  for (const file of ["LICENSE-MIT", "LICENSE-APACHE"]) cpSync(join(ROOT, file), join(dir, file));
  const base = JSON.parse(readFileSync(join(WRAPPER, "package.json"), "utf8")) as Record<string, unknown>;
  writeFileSync(join(dir, "package.json"), `${JSON.stringify(wrapperManifest(base, all, version), null, 2)}\n`);
}

function run(): void {
  const { values } = parseArgs({ options: { artifacts: { type: "string" }, out: { type: "string" } } });
  if (values.artifacts === undefined) throw new Error("usage: generate.ts --artifacts <dir> [--out <dir>]");
  const out = resolve(values.out ?? join(ROOT, "target/cli-npm"));
  const version = workspaceVersion();
  const all = platforms();
  rmSync(out, { recursive: true, force: true });
  const written = all.flatMap((platform) => {
    const archive = archiveFor(values.artifacts ?? "", platform);
    if (archive === undefined) return [];
    writePlatform(platform, archive, out, version);
    return [platform.package];
  });
  if (written.length === 0) throw new Error(`no laminapdf-cli-<target> archive in ${values.artifacts}`);
  writeWrapper(out, all, version);
  process.stdout.write(`cli-npm: @laminapdf/cli ${version} and ${written.join(", ")} in ${out}\n`);
}

if (import.meta.filename === process.argv[1]) run();
