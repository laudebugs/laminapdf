// The three entry files (src/index.ts for `browser`/`deno`/`default`, src/entry/node.ts and
// src/entry/workerd.ts) are one module bound to three loaders: each needs its own file, so every
// export keeps its doc comment for the runtime it serves. This test keeps them the same apart from
// their header, their `locate` import and the phrase saying where the base module comes from.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const ENTRIES = ["src/index.ts", "src/entry/node.ts", "src/entry/workerd.ts"];

/** The entry with what may differ replaced by placeholders. */
function normalised(path: string): string {
  const text = readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
  const body = text.slice(text.indexOf("\nimport ") + 1);
  return body
    .replaceAll('from "../', 'from "./')
    .replace(/^import \{ locate \} from "[^"]+";$/m, "import { locate } from LOADER;")
    .replace(/Loads the base WebAssembly module \([^)]*\)/, "Loads the base WebAssembly module (WHERE)");
}

describe("the entry files", () => {
  it("differ only in their header, their loader and where the base module comes from", () => {
    const [first, ...rest] = ENTRIES.map(normalised);
    for (const [i, other] of rest.entries()) expect(other, ENTRIES[i + 1]).toBe(first);
  });
});
