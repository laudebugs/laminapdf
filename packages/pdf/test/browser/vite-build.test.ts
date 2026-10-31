// A Vite production build of a tiny app importing @laminapdf/pdf (test/vite-app/), run in Chromium:
// the bundle and both `.wasm` assets served by `vite preview`, the app imported by a real page
// (Playwright). Runs in Node as the `vite-build` project of vitest.browser.config.ts;
// test/vite.test.ts covers the same build with the bundle run in Node.
import { mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { build, type InlineConfig, preview } from "vite";
import { afterAll, describe, expect, it } from "vitest";
import { read } from "../helpers.js";

const app = fileURLToPath(new URL("../vite-app/", import.meta.url));
const pkg = fileURLToPath(new URL("../../", import.meta.url));
const out = mkdtempSync(join(tmpdir(), "lamina-pdf-vite-chromium-"));

const config: InlineConfig = {
  root: app,
  base: "./",
  configFile: false,
  logLevel: "silent",
  // As in test/vite.test.ts: the alias stands in for an install and still resolves through the
  // package's `exports` (the `browser` condition).
  // The subpath maps to its `exports` target (a prefix alias would make it `<pkg>/worker`).
  resolve: {
    alias: [
      { find: /^@lamina\/pdf\/worker$/, replacement: join(pkg, "dist/worker/index.js") },
      { find: /^@lamina\/pdf$/, replacement: pkg },
    ],
  },
  optimizeDeps: { exclude: ["@laminapdf/pdf"], noDiscovery: true },
  build: {
    outDir: out,
    emptyOutDir: true,
    target: "es2022",
    rollupOptions: {
      input: { main: join(app, "main.ts"), worker: join(app, "worker.ts") },
      preserveEntrySignatures: "strict",
      output: { format: "es" },
    },
  },
};

afterAll(() => rmSync(out, { recursive: true, force: true }));

describe("Vite production build in Chromium", () => {
  it("loads both modules from the emitted assets and runs", async () => {
    expect(await runEntry("main")).toEqual([1, 1]);
  });

  it("emits the worker client's worker file, which loads the module from the assets", async () => {
    expect(await runEntry("worker")).toEqual([1, 1]);
  });
});

let built: Promise<unknown> | undefined;

/** Builds the app once, then calls `run(hello.pdf)` of the emitted entry `name` in Chromium. */
async function runEntry(name: string): Promise<unknown> {
  built ??= build(config);
  await built;
  const assets = readdirSync(join(out, "assets"));
  const entry = assets.find((f) => f.startsWith(`${name}-`) && f.endsWith(".js"));
  if (entry === undefined) throw new Error(`the build emitted no ${name}-*.js entry in assets/: ${assets.join(", ")}`);
  const server = await preview({ ...config, preview: { port: 0, host: "127.0.0.1" } });
  const browser = await chromium.launch();
  try {
    const origin = server.resolvedUrls?.local[0] ?? "";
    const page = await browser.newPage();
    await page.goto(origin);
    // A string, not a function: Vitest would rewrite a function's `import()` for Node.
    const src = JSON.stringify(new URL(`assets/${entry}`, origin).href);
    const bytes = JSON.stringify([...read("fixtures/golden/examples/hello.pdf")]);
    return await page.evaluate(`import(${src}).then(({ run }) => run(new Uint8Array(${bytes})))`);
  } finally {
    await browser.close();
    await new Promise<void>((resolve, reject) => server.httpServer.close((e) => (e ? reject(e) : resolve())));
  }
}
