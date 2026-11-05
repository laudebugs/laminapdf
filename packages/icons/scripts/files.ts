// The package's icon files, read once for the generator, the lint script and the tests.

import { readdirSync, readFileSync } from "node:fs";

const dir = new URL("../svg/", import.meta.url);

/** Each file in svg/ as [name, text], sorted by name. */
export function iconFiles(): [string, string][] {
  return readdirSync(dir)
    .filter((f) => f.endsWith(".svg"))
    .map((f): [string, string] => [f.slice(0, -4), readFileSync(new URL(f, dir), "utf8")])
    .sort(([a], [b]) => (a < b ? -1 : 1));
}

/** aliases.json: a second name for an icon, mapped to the icon's own name. */
export function aliases(): Record<string, string> {
  return JSON.parse(readFileSync(new URL("../aliases.json", import.meta.url), "utf8"));
}
