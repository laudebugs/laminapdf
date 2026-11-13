// Semantic Versioning 2.0.0 (https://semver.org): parsing and precedence, which is all the update
// service needs. Build metadata (`+…`) is accepted and ignored for precedence (§10). Nightly builds
// carry pre-release identifiers such as `0.1.1-nightly.20261028.42`.

/** A parsed version: three numbers and the pre-release identifiers (empty for a release). */
export interface Version {
  readonly major: number;
  readonly minor: number;
  readonly patch: number;
  readonly prerelease: readonly (number | string)[];
}

// §2 and §9–10: no leading zeros in numbers, identifiers of [0-9A-Za-z-], numeric pre-release
// identifiers without leading zeros.
const SEMVER =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

/** The longest version string accepted, so a request can't make the parser work on megabytes. */
const MAX_LENGTH = 128;

/** Parses `text` as a semantic version, or returns `undefined`. */
export function parseVersion(text: string): Version | undefined {
  if (text.length > MAX_LENGTH) return undefined;
  const m = SEMVER.exec(text);
  if (!m) return undefined;
  const [major, minor, patch] = [m[1], m[2], m[3]].map(Number);
  if (major === undefined || minor === undefined || patch === undefined) return undefined;
  if (![major, minor, patch].every(Number.isSafeInteger)) return undefined;
  const prerelease = m[4] ? m[4].split(".").map((id) => (/^\d+$/.test(id) ? Number(id) : id)) : [];
  if (prerelease.some((id) => typeof id === "number" && !Number.isSafeInteger(id))) return undefined;
  return { major, minor, patch, prerelease };
}

/** Whether `text` is a semantic version. */
export function isVersion(text: string): boolean {
  return parseVersion(text) !== undefined;
}

// §11.4: identifiers compared one by one; numbers below alphanumeric ones; a longer list ranks higher
// when every shared identifier is equal.
function compareIdentifiers(x: number | string, y: number | string): number {
  if (x === y) return 0;
  if (typeof x === "number" && typeof y === "number") return x < y ? -1 : 1;
  if (typeof x === "number") return -1;
  if (typeof y === "number") return 1;
  return x < y ? -1 : 1;
}

function comparePrerelease(a: readonly (number | string)[], b: readonly (number | string)[]): number {
  // A version without pre-release identifiers ranks above one with them.
  if (a.length === 0 || b.length === 0) return b.length - a.length;
  for (const [i, x] of a.entries()) {
    const y = b[i];
    if (y === undefined) break;
    const order = compareIdentifiers(x, y);
    if (order !== 0) return order;
  }
  return a.length - b.length;
}

/** Precedence (§11): negative when `a` < `b`, zero when equal, positive when `a` > `b`. */
function compareVersions(a: Version, b: Version): number {
  for (const key of ["major", "minor", "patch"] as const) {
    if (a[key] !== b[key]) return a[key] < b[key] ? -1 : 1;
  }
  return comparePrerelease(a.prerelease, b.prerelease);
}

/** Compares two version strings; both must parse (callers validate first). */
export function compare(a: string, b: string): number {
  const x = parseVersion(a);
  const y = parseVersion(b);
  if (!x || !y) throw new RangeError("compare() takes two semantic versions");
  return compareVersions(x, y);
}
