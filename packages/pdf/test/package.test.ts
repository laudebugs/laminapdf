// Packaging: every export condition's entry exports the same API, and src/ uses no Node-only API
// outside the Node loader (typescript.md §portability; Biome's noNodejsModules checks the imports).
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { describe, expect, it } from "vitest";

const pkg = fileURLToPath(new URL("../", import.meta.url));

/** The names an entry exports, as esbuild sees them (the workerd entry can't load in Node). */
async function exportsOf(entry: string): Promise<string[]> {
  const result = await build({
    entryPoints: [join(pkg, entry)],
    bundle: true,
    write: false,
    metafile: true,
    format: "esm",
    platform: "neutral",
    external: ["@lamina/wasm/*", "node:*"],
    logLevel: "silent",
  });
  return Object.values(result.metafile.outputs)
    .flatMap((o) => o.exports)
    .sort();
}

function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((d) =>
    d.isDirectory() ? sources(join(dir, d.name)) : d.name.endsWith(".ts") ? [join(dir, d.name)] : [],
  );
}

describe("package", () => {
  it("exports the same names from the browser, node and workerd entries", async () => {
    const [browser, node, workerd] = await Promise.all(
      ["dist/index.js", "dist/entry/node.js", "dist/entry/workerd.js"].map(exportsOf),
    );
    expect([node, workerd]).toEqual([browser, browser]);
  });

  it("uses no Node-only API in src/ except the Node loader", () => {
    const banned = /\b(process|Buffer|require|__dirname|__filename)\b|from "node:|import\("node:/;
    const offenders = sources(join(pkg, "src"))
      .filter((f) => !f.endsWith(join("loader", "node.ts")))
      .filter((f) => banned.test(readFileSync(f, "utf8").replace(/\/\/.*$/gm, "")));
    expect(offenders).toEqual([]);
  });
});
