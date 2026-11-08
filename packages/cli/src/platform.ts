// Which @laminapdf/cli-* package serves this machine. Pure functions over `platforms.json`'s
// entries and a description of the host, so the tests can try every platform.

/** A C library a Linux binary is linked against: glibc's dynamic loader, or musl statically. */
export type Libc = "glibc" | "musl";

/** One platform package, as `platforms.json` lists it. */
export interface Platform {
  /** The npm package that carries the binary, e.g. `@laminapdf/cli-linux-x64-gnu`. */
  readonly package: string;
  /** The Rust target the binary is built for (one of dist-workspace.toml's `targets`). */
  readonly target: string;
  /** `process.platform` of the machines it runs on: `darwin`, `linux` or `win32`. */
  readonly os: string;
  /** `process.arch` of the machines it runs on: `arm64` or `x64`. */
  readonly cpu: string;
  /** Linux only: the C library it needs. */
  readonly libc?: Libc;
}

/** The machine the launcher runs on. */
export interface Host {
  /** `process.platform`. */
  readonly os: string;
  /** `process.arch`. */
  readonly cpu: string;
  /** On Linux, the C library Node itself runs on; absent elsewhere. */
  readonly libc?: Libc;
}

/**
 * The platform packages that can serve `host`, best first.
 *
 * On Linux the package for `host.libc` comes first and the other C library's second: the musl
 * binaries are static, so they run on glibc too, and a glibc machine can still have only the
 * musl package (a lockfile written on Alpine). Elsewhere there is at most one.
 *
 * @param platforms - `platforms.json`'s entries.
 * @param host - the machine to find a binary for.
 * @returns the entries whose `os` and `cpu` match, the one whose `libc` matches first; empty if none.
 */
export function platformsFor(platforms: readonly Platform[], host: Host): Platform[] {
  const matching = platforms.filter((p) => p.os === host.os && p.cpu === host.cpu);
  return [...matching].sort((a, b) => Number(b.libc === host.libc) - Number(a.libc === host.libc));
}

/**
 * The C library a Linux Node runs on, from its diagnostic report: glibc builds report
 * `header.glibcVersionRuntime`, musl builds (Alpine) don't.
 *
 * @param report - `process.report.getReport()`, the report npm reads to pick the package.
 * @returns `glibc` or `musl`.
 */
export function libcFromReport(report: unknown): Libc {
  const header = typeof report === "object" && report !== null && "header" in report ? report.header : undefined;
  const glibc = typeof header === "object" && header !== null && "glibcVersionRuntime" in header;
  return glibc ? "glibc" : "musl";
}

/**
 * The binary's file name inside its platform package's `bin/`.
 *
 * @param platform - the platform package.
 * @returns `laminapdf.exe` on Windows, else `laminapdf`.
 */
export function binaryName(platform: Platform): string {
  return platform.os === "win32" ? "laminapdf.exe" : "laminapdf";
}

/** A host as `platforms.json` names it: `linux arm64 (musl)`. */
function describe(p: Host): string {
  return `${p.os} ${p.cpu}${p.libc === undefined ? "" : ` (${p.libc})`}`;
}

/**
 * What to tell someone whose machine has no platform package.
 *
 * @param platforms - `platforms.json`'s entries.
 * @param host - their machine.
 * @returns the message, naming every supported platform.
 */
export function unsupportedMessage(platforms: readonly Platform[], host: Host): string {
  const supported = platforms.map((p) => `  ${describe(p)}`).join("\n");
  return [
    `laminapdf has no prebuilt binary for ${describe(host)}.`,
    `Supported platforms:\n${supported}`,
    "On another platform, build it from source: cargo install --git https://github.com/laudebugs/laminapdf laminapdf-cli",
  ].join("\n");
}

/**
 * What to tell someone whose platform package is missing: usually an install that left out
 * optional dependencies.
 *
 * @param platform - the package that should have been installed.
 * @returns the message.
 */
export function missingPackageMessage(platform: Platform): string {
  return [
    `laminapdf's binary for this platform, ${platform.package}, is not installed.`,
    "It is an optional dependency of @laminapdf/cli: reinstall without --no-optional or --omit=optional.",
  ].join("\n");
}
