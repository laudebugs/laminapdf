import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

/** Every module the file imports, following relative imports, as [module, specifier] pairs. */
function imports(url: URL, seen = new Set<string>()): [string, string][] {
  if (seen.has(url.href)) return [];
  seen.add(url.href);
  const text = readFileSync(url, "utf8");
  const out: [string, string][] = [];
  // `from "x"`, a side-effect `import "x"` and a dynamic `import("x")` all name a module.
  for (const m of text.matchAll(/\b(?:from|import)\s*\(?\s*"([^"]+)"/g)) {
    const spec = m[1] ?? "";
    out.push([url.pathname.replace(/.*\/packages\/icons\//, ""), spec]);
    if (spec.startsWith(".")) out.push(...imports(new URL(spec.replace(/\.js$/, ".ts"), url), seen));
  }
  return out;
}

describe("the root entry", () => {
  it("imports nothing outside the package, React least of all", () => {
    const outside = imports(new URL("../src/index.ts", import.meta.url)).filter(([, spec]) => !spec.startsWith("."));
    expect(outside).toEqual([]);
  });

  it("keeps React to the react entry", () => {
    const react = imports(new URL("../src/react/index.ts", import.meta.url)).filter(
      ([, spec]) => !spec.startsWith("."),
    );
    expect(react).toEqual([["src/react/index.ts", "react"]]);
  });
});
