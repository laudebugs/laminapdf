// `laminapdf` from npm: find this platform's @laminapdf/cli-* package, which npm installed as an
// optional dependency (its `os`, `cpu` and `libc` fields skip the others), and run its binary
// with our arguments, standard streams and exit status. No install script downloads anything.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import process from "node:process";
import {
  binaryName,
  type Host,
  libcFromReport,
  missingPackageMessage,
  type Platform,
  platformFor,
  unsupportedMessage,
} from "./platform.js";

/** The general-error exit code of ADR 0031, for a launcher that couldn't start the binary. */
const FAILURE = 1;

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(FAILURE);
}

function host(): Host {
  if (process.platform !== "linux") return { os: process.platform, cpu: process.arch };
  return { os: process.platform, cpu: process.arch, libc: libcFromReport(process.report.getReport()) };
}

/** The binary's path, if the platform package is installed. */
function binaryPath(platform: Platform): string | undefined {
  try {
    const manifest = createRequire(import.meta.url).resolve(`${platform.package}/package.json`);
    return join(dirname(manifest), "bin", binaryName(platform));
  } catch {
    return undefined;
  }
}

/** Runs the binary and exits as it did; a signal that ended it ends this process too. */
export function main(): void {
  const platforms = JSON.parse(readFileSync(new URL("../platforms.json", import.meta.url), "utf8")) as Platform[];
  const here = host();
  const platform = platformFor(platforms, here) ?? fail(unsupportedMessage(platforms, here));
  const binary = binaryPath(platform) ?? fail(missingPackageMessage(platform));
  const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
  if (result.error !== undefined) fail(`laminapdf: could not run ${binary}: ${result.error.message}`);
  if (result.signal !== null) process.kill(process.pid, result.signal);
  process.exit(result.status ?? FAILURE);
}
