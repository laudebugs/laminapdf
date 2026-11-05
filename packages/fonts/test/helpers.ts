// What the tests share: the lock, the package's own files, and fonts.css parsed into faces.

import { readFileSync } from "node:fs";
import { fromRoot } from "@laminapdf/test-utils/root";

/** One output of tools/fonts/subset.py, as fonts.lock.json records it. */
interface Output {
  readonly sha256: string;
  readonly bytes: number;
  readonly family: string;
  readonly style: string;
  readonly weight: string;
  readonly coverage: string;
}

/** fonts.lock.json. */
export interface Lock {
  readonly tools: Record<string, string>;
  readonly sources: readonly {
    readonly id: string;
    readonly package: string;
    readonly version: string;
    readonly reservedFontName: string | null;
    readonly files: Record<string, string>;
  }[];
  readonly outputs: Record<string, Output>;
  readonly fallback: Record<string, string | number>;
}

/** A path inside packages/fonts. */
export function packagePath(path: string): URL {
  return fromRoot(`packages/fonts/${path}`);
}

/** fonts.lock.json, parsed. */
export function lock(): Lock {
  return JSON.parse(readFileSync(packagePath("fonts.lock.json"), "utf8"));
}

/** One `@font-face` rule of fonts.css, its descriptors by name. */
export type Face = Record<string, string>;

/** fonts.css's `@font-face` rules. */
export function faces(): Face[] {
  const css = readFileSync(packagePath("fonts.css"), "utf8");
  return [...css.matchAll(/@font-face\s*\{([^}]*)\}/g)].map(([, body = ""]) =>
    Object.fromEntries(
      body
        .split(";")
        .map((line) => line.trim())
        .filter(Boolean)
        .map((line) => {
          const colon = line.indexOf(":");
          return [line.slice(0, colon).trim(), line.slice(colon + 1).trim()];
        }),
    ),
  );
}

/** The code points of a `U+…` range list (fonts.css' unicode-range or the lock's coverage). */
export function codePoints(ranges: string): Set<number> {
  const points = new Set<number>();
  for (const part of ranges.split(",")) {
    const [start = "", end = start] = part.trim().replace(/^U\+/, "").split("-");
    for (let p = Number.parseInt(start, 16); p <= Number.parseInt(end, 16); p++) points.add(p);
  }
  return points;
}
