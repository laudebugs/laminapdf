// The `browser` condition through Vite (ADR 0016 §2; shareable-libraries §8): with @laminapdf/sdk
// excluded from dependency pre-bundling, the dev server serves the package as it is and the
// `.wasm` next to the glue; a production build emits every module as an asset, and the bundle runs.
import { mkdtempSync, readdirSync, rmSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { build, createServer, type InlineConfig } from "vite";
import { afterAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

const app = fileURLToPath(new URL("./vite-app/", import.meta.url));
const pkg = fileURLToPath(new URL("../", import.meta.url));
const out = mkdtempSync(join(tmpdir(), "lamina-pdf-vite-"));

const config: InlineConfig = {
  root: app,
  base: "./",
  configFile: false,
  logLevel: "silent",
  // The app's package.json would name @laminapdf/sdk as a dependency; the alias stands in for that
  // install and still resolves through the package's `exports` (the `browser` condition).
  resolve: { alias: { "@laminapdf/sdk": pkg } },
  optimizeDeps: { exclude: ["@laminapdf/sdk"], noDiscovery: true },
};

afterAll(() => rmSync(out, { recursive: true, force: true }));

describe("Vite", () => {
  it("serves the package unbundled and the .wasm next to the glue in dev", async () => {
    const server = await createServer({ ...config, server: { port: 0, host: "127.0.0.1" } });
    try {
      await server.listen();
      const main = await server.transformRequest("/main.ts");
      const glue = fileURLToPath(new URL("../../wasm/dist/lamina_base.wasm", import.meta.url));
      const url = server.resolvedUrls?.local[0] ?? "";
      const wasm = await fetch(new URL(`/@fs${glue}`, url));
      expect([
        main?.code.includes("/node_modules/.vite/deps/"),
        main?.code.includes("/packages/sdk/dist/index.js"),
        wasm.headers.get("content-type"),
        new Uint8Array(await wasm.arrayBuffer()).length,
      ]).toEqual([false, true, "application/wasm", read("packages/wasm/dist/lamina_base.wasm").length]);
    } finally {
      await server.close();
    }
  });

  it("emits every module as an asset and the bundle runs", async () => {
    await build({
      ...config,
      build: {
        outDir: out,
        emptyOutDir: true,
        minify: false,
        target: "es2022",
        rollupOptions: { input: join(app, "main.ts"), preserveEntrySignatures: "strict", output: { format: "es" } },
      },
    });
    const assets = readdirSync(join(out, "assets")).filter((f) => f.endsWith(".wasm"));
    // A browser fetches the assets over HTTP; Node's fetch can't read file: URLs, so serve them.
    const realFetch = globalThis.fetch;
    globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = input instanceof URL ? input : new URL(typeof input === "string" ? input : input.url);
      if (url.protocol !== "file:") return realFetch(input, init);
      return new Response((await readFile(url)).slice().buffer, { headers: { "content-type": "application/wasm" } });
    };
    try {
      const entry = readdirSync(join(out, "assets")).find((f) => f.startsWith("main") && f.endsWith(".js"));
      const { run } = (await import(join(out, "assets", entry ?? "main.js"))) as {
        run(bytes: Uint8Array): Promise<readonly number[]>;
      };
      const result = await run(read("fixtures/golden/examples/hello.pdf"));
      expect([assets.map((a) => a.replace(/-[\w-]+\.wasm$/, "")).sort(), result]).toEqual([
        ["lamina_base", "lamina_extract", "lamina_text"],
        [1, 1],
      ]);
    } finally {
      globalThis.fetch = realFetch;
    }
  });
});
