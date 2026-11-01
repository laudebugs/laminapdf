// The viewer has no UI framework inside (C3b): React chrome and the desktop app drive it from
// outside. Every source file is read through Vite (`?raw`), so the check needs no Node API.
import { describe, expect, it } from "vitest";

const sources = import.meta.glob<string>("../src/**/*.ts", { query: "?raw", import: "default", eager: true });

describe("dependencies", () => {
  it("imports no UI framework in src/", () => {
    const framework = /from\s+["'](react|react-dom|preact|vue|svelte|solid-js|@angular\/[^"']+)["']/;
    expect(Object.keys(sources).length).toBeGreaterThan(10);
    expect(
      Object.entries(sources)
        .filter(([, text]) => framework.test(text))
        .map(([path]) => path),
    ).toEqual([]);
  });
});
