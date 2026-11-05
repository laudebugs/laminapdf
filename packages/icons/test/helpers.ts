// What the tests share: the icon files read from svg/ and the repository files they compare with.

import { readdirSync, readFileSync } from "node:fs";
import { fromRoot } from "@laminapdf/test-utils/root";

/** Each icon file in svg/ as [name, text], by name. */
export function iconFiles(): [string, string][] {
  const dir = fromRoot("packages/icons/svg/");
  return readdirSync(dir)
    .filter((f) => f.endsWith(".svg"))
    .sort()
    .map((f) => [f.slice(0, -4), readFileSync(new URL(f, dir), "utf8")]);
}

/** aliases.json. */
export function aliases(): Record<string, string> {
  return JSON.parse(readRepo("packages/icons/aliases.json"));
}

/** A repository file as text. */
export function readRepo(path: string): string {
  return readFileSync(fromRoot(path), "utf8");
}
