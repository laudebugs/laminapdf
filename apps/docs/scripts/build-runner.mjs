// Builds the docs' example runner (todo B7e) into public/run/, before `astro build` copies public/
// into dist/: runner/worker.ts as /run/worker.js, with the WebAssembly it loads (@laminapdf/sdk's
// text module, @laminapdf/render-hayro's) and the examples' inputs as hashed files under /run/assets/;
// and runner/types-worker.ts as /run/types.js, the editor's TypeScript (todo Q4t), with the
// declarations it reads bundled in (`virtual:lamina-types`). Both need @laminapdf/wasm's dist/, which only `cargo xtask wasm` builds:
// without it nothing is written, and the pages show their examples without "Edit and run".
import { execFileSync } from "node:child_process";
import { readdir, readFile, rm, stat } from "node:fs/promises";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const DOCS = new URL("../", import.meta.url);
const ROOT = new URL("../../", DOCS);
const OUT = new URL("public/run/", DOCS);
/** What the worker needs built: without them the runner can't be made. */
const NEEDS = [
  "packages/wasm/dist/lamina_pdf_text.wasm",
  "packages/sdk/dist/index.js",
  "packages/render/dist/index.js",
  "packages/render-hayro/dist/index.js",
];

/**
 * The packages whose declarations the editor's TypeScript reads, by folder from the repository
 * root: the SDK, and the WebAssembly bindings its declarations import.
 */
const TYPED_PACKAGES = ["packages/sdk", "packages/wasm"];

/** The ES libraries the examples are typed against (types-worker.ts's `lib`); each brings those it references. */
const LIBRARIES = ["es2022", "esnext.disposable"];

/**
 * Every `.d.ts` file under `dir`, by its path from `dir`.
 *
 * @param {URL} dir a folder URL, ending in `/`
 * @returns {Promise<string[]>}
 */
async function declarations(dir) {
  const entries = await readdir(dir, { recursive: true, withFileTypes: true });
  return entries
    .filter((entry) => entry.isFile() && entry.name.endsWith(".d.ts"))
    .map((entry) => relative(fileURLToPath(dir), join(entry.parentPath, entry.name)).replaceAll(sep, "/"));
}

/**
 * The files the editor's TypeScript reads, by their path in its file system: the ES libraries in
 * /lib/, and each typed package's package.json and declarations in /node_modules/<name>/, so it
 * resolves the SDK's imports as a project would.
 *
 * @returns {Promise<Record<string, string>>}
 */
async function typeFiles() {
  /** @type {Record<string, string>} */
  const files = {};
  const lib = new URL("node_modules/typescript/lib/", DOCS);
  const queue = [...LIBRARIES];
  for (let name = queue.pop(); name !== undefined; name = queue.pop()) {
    const file = `lib.${name}.d.ts`;
    if (`/lib/${file}` in files) continue;
    const text = await readFile(new URL(file, lib), "utf8");
    files[`/lib/${file}`] = text;
    for (const [, referenced] of text.matchAll(/<reference lib="([^"]+)"/g)) if (referenced) queue.push(referenced);
  }
  for (const folder of TYPED_PACKAGES) {
    const dir = new URL(`${folder}/`, ROOT);
    const manifest = await readFile(new URL("package.json", dir), "utf8");
    const { name } = JSON.parse(manifest);
    files[`/node_modules/${name}/package.json`] = manifest;
    for (const path of await declarations(new URL("dist/", dir))) {
      files[`/node_modules/${name}/dist/${path}`] = await readFile(new URL(`dist/${path}`, dir), "utf8");
    }
  }
  return files;
}

/** `virtual:lamina-types`: {@link typeFiles} as a module whose default export is that record. */
const TYPES_MODULE = "virtual:lamina-types";
/** @returns {import("vite").Plugin} */
function laminaTypes() {
  return {
    name: "lamina-types",
    resolveId: (id) => (id === TYPES_MODULE ? `\0${TYPES_MODULE}` : undefined),
    load: async (id) =>
      id === `\0${TYPES_MODULE}` ? `export default ${JSON.stringify(await typeFiles())};` : undefined,
  };
}

/** @param {string} path from the repository root */
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
  // The worker, which the site's typecheck leaves out because it needs these packages built.
  execFileSync(fileURLToPath(new URL("node_modules/.bin/tsc", DOCS)), ["--noEmit", "-p", "runner/tsconfig.json"], {
    cwd: DOCS,
    stdio: "inherit",
  });
  await build({
    configFile: false,
    root: fileURLToPath(DOCS),
    logLevel: "warn",
    // Served at /run/, so its assets are /run/assets/….
    base: "/run/",
    // The workspace packages name their WebAssembly by `new URL(…, import.meta.url)`, which
    // pre-bundling would move away from the files.
    optimizeDeps: { exclude: ["@laminapdf/wasm", "@laminapdf/sdk", "@laminapdf/render-hayro"] },
    plugins: [laminaTypes()],
    build: {
      outDir: fileURLToPath(OUT),
      emptyOutDir: true,
      target: "es2022",
      // Every asset a file: the docs' CSP allows no data: URLs for fetch.
      assetsInlineLimit: 0,
      modulePreload: false,
      copyPublicDir: false,
      // TypeScript alone is about 3.5 MB before compression; it loads only on the reader's first edit.
      chunkSizeWarningLimit: 6000,
      rollupOptions: {
        input: {
          worker: fileURLToPath(new URL("runner/worker.ts", DOCS)),
          types: fileURLToPath(new URL("runner/types-worker.ts", DOCS)),
        },
        output: { entryFileNames: "[name].js", format: "es" },
      },
    },
  });
  console.log("docs: example runner built into public/run/");
}
