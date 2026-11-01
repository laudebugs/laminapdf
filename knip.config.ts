// knip (Q2u): unused files, exports and dependencies across the packages and the docs site. Each
// package's entries are its `exports`, each `dist/**.js` mapped back to the `src/**.ts` it is built
// from, plus what no `exports` can name: worker files started with `new URL(…)`, tests, Vitest
// configs, examples and benches. Run by `cargo xtask check ts` (`pnpm run check:unused`).
import { existsSync, readdirSync, readFileSync } from "node:fs";
import type { KnipConfig } from "knip";

/** Every `.js` file an `exports` map points at (each subpath and condition). */
function exportedFiles(exports: unknown): string[] {
  if (typeof exports === "string") return exports.endsWith(".js") ? [exports] : [];
  if (exports === null || typeof exports !== "object") return [];
  return Object.values(exports).flatMap(exportedFiles);
}

/** A package's entries: its exports' sources, then what runs without being exported. */
function entries(dir: string): string[] {
  const manifest = JSON.parse(readFileSync(`packages/${dir}/package.json`, "utf8")) as { exports?: unknown };
  const sources = exportedFiles(manifest.exports).map((f) => f.replace(/^\.\/dist\//, "src/").replace(/\.js$/, ".ts"));
  const optional = ["src/worker/entry.ts", "test", "examples", "bench", "scripts"].filter((p) =>
    existsSync(`packages/${dir}/${p}`),
  );
  const globs: Record<string, string> = {
    "src/worker/entry.ts": "src/worker/entry.ts",
    test: "test/**/*.ts",
    examples: "examples/**/*.{ts,mjs}",
    bench: "bench/**/*.mjs",
    scripts: "scripts/**/*.mjs",
  };
  const vitest = readdirSync(`packages/${dir}`).some((f) => /^vitest\..*config\.ts$/.test(f))
    ? ["vitest.*config.ts"]
    : [];
  return [...new Set(sources), ...optional.map((p) => globs[p] as string), ...vitest];
}

/** @laminapdf/wasm is left out: wasm-bindgen generates its code, with nothing of ours to find unused. */
const GENERATED = new Set(["wasm"]);

const packages = readdirSync("packages", { withFileTypes: true })
  .filter((d) => d.isDirectory() && !GENERATED.has(d.name))
  .map((d) => d.name);

const config: KnipConfig = {
  workspaces: {
    // The root has no code of its own: its tools (tools/, docs/ research) are run by xtask.
    ".": { entry: [], project: [] },
    ...Object.fromEntries(
      packages.map((dir) => [
        `packages/${dir}`,
        {
          entry: entries(dir),
          project: ["src/**/*.ts", "test/**/*.ts", "examples/**/*.{ts,mjs}"].filter((g) =>
            existsSync(`packages/${dir}/${g.split("/")[0]}`),
          ),
        },
      ]),
    ),
    "apps/docs": {},
  },
  ignore: ["**/src/generated/**"],
  ignoreWorkspaces: ["packages/wasm"],
  ignoreDependencies: [
    // Run by `cargo xtask pdf-runtimes` as `pnpm exec deno` and `pnpm exec bun`, which knip can't see.
    "deno",
    "bun",
  ],
};

export default config;
