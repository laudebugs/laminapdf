// The npm packages of the `laminapdf` CLI (todo L4d), from dist's release archives:
//
//   node tools/cli-release/generate.ts --artifacts <dist's output dir> [--out target/cli-npm] [--pack]
//
// writes one folder per platform package whose archive is there (the binary in bin/, the
// licences and third-party notices from the archive, a package.json with `os`, `cpu` and, on
// Linux, `libc` from packages/cli/platforms.json), and cli/, the @laminapdf/cli wrapper, with
// every platform package as an optional dependency at the same version. Build the wrapper first
// (`pnpm --filter @laminapdf/cli build`). With `--pack`, each folder is also packed into packs/,
// with publish.json: the version, its npm dist-tag and the tarballs in publish order, the platform
// packages before the wrapper that depends on them (cli-release.yml's publish-npm reads it). The
// version is the CLI crate's, the workspace's lockstep version (ADR 0017).
import { execFileSync } from "node:child_process";
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve, win32 } from "node:path";
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
export function unpackCommand(
  archive: string,
  dir: string,
  host: string,
  systemRoot = process.env.SystemRoot ?? "C:\\Windows",
): [string, string[]] {
  // Windows' own bsdtar by full path: under Git Bash, PATH finds Git's GNU tar first.
  if (host === "win32") return [win32.join(systemRoot, "System32", "tar.exe"), ["-xf", archive, "-C", dir]];
  if (archive.endsWith(".zip") && host !== "darwin") return ["unzip", ["-q", archive, "-d", dir]];
  return ["tar", ["-xf", archive, "-C", dir]];
}

/** The folder of an unpacked archive that holds its files: dist's top folder, or the root of a flat zip. */
function archiveRoot(unpacked: string, platform: Platform): string {
  const folder = join(unpacked, `laminapdf-cli-${platform.target}`);
  return existsSync(join(folder, binaryName(platform))) ? folder : unpacked;
}

function writePlatform(platform: Platform, archive: string, out: string, version: string): void {
  const unpacked = mkdtempSync(join(tmpdir(), "laminapdf-cli-"));
  try {
    execFileSync(...unpackCommand(archive, unpacked, process.platform));
    const from = archiveRoot(unpacked, platform);
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

/**
 * The npm dist-tag for `version`: its pre-release identifier (`alpha` for `0.2.0-alpha.1`), or
 * `latest` for a release. A pre-release with no word to tag it by (`1.0.0-0.3`) is refused, since
 * tagging it `latest` would hand it to every `npm install`.
 */
export function distTag(version: string): string {
  const match = /^\d+\.\d+\.\d+(?:-([0-9A-Za-z.-]+))?$/.exec(version);
  if (match === null) throw new Error(`not a version: ${version}`);
  if (match[1] === undefined) return "latest";
  const word = /^([a-z]+)/.exec(match[1])?.[1];
  if (word === undefined) throw new Error(`pre-release ${version} names no dist-tag (want e.g. -alpha.1)`);
  return word;
}

/** One package in publish.json: its name and its tarball's file name. */
export interface PackedPackage {
  readonly name: string;
  readonly tarball: string;
}

/** publish.json: the version, its dist-tag, and the packages in publish order (platform packages, then the wrapper). */
export function publishManifest(
  version: string,
  packed: readonly PackedPackage[],
): { version: string; tag: string; packages: PackedPackage[] } {
  const wrapper = packed.filter((p) => p.name === "@laminapdf/cli");
  const platformPackages = packed.filter((p) => p.name !== "@laminapdf/cli");
  return { version, tag: distTag(version), packages: [...platformPackages, ...wrapper] };
}

/** `npm pack`s each folder of `out` into `out/packs` and writes publish.json there. */
function pack(out: string, folders: readonly string[], version: string): void {
  const packs = join(out, "packs");
  mkdirSync(packs, { recursive: true });
  const packed = folders.map((folder) => {
    // Only `global`, on Linux, packs.
    const json = execFileSync("npm", ["pack", "--json", "--pack-destination", packs], {
      cwd: join(out, folder),
      encoding: "utf8",
    });
    const [result] = JSON.parse(json) as { name: string; filename: string }[];
    if (result === undefined) throw new Error(`npm pack wrote nothing for ${folder}`);
    return { name: result.name, tarball: result.filename };
  });
  writeFileSync(join(packs, "publish.json"), `${JSON.stringify(publishManifest(version, packed), null, 2)}\n`);
}

function run(): void {
  const { values } = parseArgs({
    options: { artifacts: { type: "string" }, out: { type: "string" }, pack: { type: "boolean" } },
  });
  if (values.artifacts === undefined) throw new Error("usage: generate.ts --artifacts <dir> [--out <dir>] [--pack]");
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
  if (values.pack === true) {
    const folders = all.filter((p) => written.includes(p.package)).map(folderName);
    pack(out, [...folders, "cli"], version);
  }
  process.stdout.write(`cli-npm: @laminapdf/cli ${version} and ${written.join(", ")} in ${out}\n`);
}

if (import.meta.filename === process.argv[1]) run();
