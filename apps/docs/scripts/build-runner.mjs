// Builds the docs' example runner (todo B7e) into public/run/, before `astro build` copies public/
// into dist/: runner/worker.ts as /run/worker.js, with the WebAssembly it loads (@laminapdf/sdk's
// text module, @laminapdf/render-hayro's) and the examples' inputs as hashed files under /run/assets/. The worker needs @laminapdf/wasm's dist/, which only `cargo xtask wasm` builds:
// without it nothing is written, and the pages show their examples without "Edit and run".
import { rm, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const DOCS = new URL("../", import.meta.url);
const ROOT = new URL("../../", DOCS);
const OUT = new URL("public/run/", DOCS);
/** What the worker needs built: without them the runner can't be made. */
const NEEDS = ["packages/wasm/dist/lamina_pdf_text.wasm", "packages/sdk/dist/index.js", "packages/render/dist/index.js", "packages/render-hayro/dist/index.js"];

async function exists(path) {
  return stat(new URL(path, ROOT)).then(
    () => true,
    () => false,
  );
}

await rm(OUT, { recursive: true, force: true });
const missing = [];
for (const path of NEEDS) if (!(await exists(path))) missing.push(path);
if (missing.length > 0) {
  console.log(`docs: no example runner (missing ${missing.join(", ")}; run \`cargo xtask wasm\` first)`);
} else {
  await build({
    configFile: false,
    root: fileURLToPath(DOCS),
    logLevel: "warn",
    // The workspace packages name their WebAssembly by `new URL(…, import.meta.url)`, which
    // pre-bundling would move away from the files.
    optimizeDeps: { exclude: ["@laminapdf/wasm", "@laminapdf/sdk", "@laminapdf/render-hayro"] },
    build: {
      outDir: fileURLToPath(OUT),
      emptyOutDir: true,
      target: "es2022",
      // Every asset a file: the docs' CSP allows no data: URLs for fetch.
      assetsInlineLimit: 0,
      modulePreload: false,
      copyPublicDir: false,
      rollupOptions: {
        input: fileURLToPath(new URL("runner/worker.ts", DOCS)),
        output: { entryFileNames: "worker.js", format: "es" },
      },
    },
  });
  console.log("docs: example runner built into public/run/");
}
