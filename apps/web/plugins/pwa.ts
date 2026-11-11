// The reader as an installable app: the web app manifest, written from the Paper tokens so
// its colours match the reader's, and the service worker, compiled from src/sw/service-worker.ts
// once the build has written dist/, with the list of files to install and a version that changes
// whenever any of them does. Hand-written rather than Workbox: the worker is small, and serving a
// cached response as stored keeps the security headers, which a rebuilt response would lose.

import { createHash } from "node:crypto";
import { readdir, readFile, writeFile } from "node:fs/promises";
import { join, relative, resolve, sep } from "node:path";
import { THEMES } from "@laminapdf/tokens";
import { type Plugin, transformWithOxc } from "vite";

/** The app's name, as the browser's install prompt and the OS show it. */
const APP_NAME = "Lamina PDF";

/** The web app manifest (W3C Web App Manifest; `file_handlers` from the File Handling API). */
export function manifest(): Record<string, unknown> {
  const light = THEMES.light.teal;
  return {
    id: "/",
    name: APP_NAME,
    short_name: APP_NAME,
    description: "Read PDFs in your browser. Lamina PDF opens them on this device and uploads nothing.",
    start_url: "/",
    scope: "/",
    display: "standalone",
    background_color: light.ground,
    theme_color: light.surface,
    icons: [
      { src: "/icon-192.png", sizes: "192x192", type: "image/png", purpose: "any" },
      { src: "/icon-512.png", sizes: "512x512", type: "image/png", purpose: "any" },
      { src: "/icon-maskable-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
    ],
    // An installed copy registers as a PDF handler where the browser supports it (Chromium).
    file_handlers: [{ action: "/", accept: { "application/pdf": [".pdf"] } }],
  };
}

/**
 * Files left out of the precache, cached the first time they are fetched instead: the hayro
 * backend (the #debug switch) and the text module, which the reader never loads. The worker
 * itself, the server's header rules and robots.txt aren't files the page fetches.
 */
const LAZY = [/hayro/, /lamina_pdf_text/, /\.map$/];
const NEVER = new Set(["_headers", "robots.txt", "sw.js"]);

/** The paths the worker installs, from the build's files (relative, `/`-separated): `/` for the shell. */
export function precacheList(files: readonly string[]): string[] {
  return files
    .filter((file) => !NEVER.has(file) && !LAZY.some((pattern) => pattern.test(file)))
    .map((file) => (file === "index.html" ? "/" : `/${file}`))
    .sort();
}

/** A version for `contents` (path → bytes): any change to a precached file changes it. */
export function precacheVersion(contents: ReadonlyMap<string, Uint8Array>): string {
  const hash = createHash("sha256");
  for (const [path, bytes] of [...contents].sort(([a], [b]) => a.localeCompare(b))) {
    hash.update(path).update("\0").update(createHash("sha256").update(bytes).digest("hex")).update("\n");
  }
  return hash.digest("hex").slice(0, 16);
}

async function filesUnder(root: string, dir = root): Promise<string[]> {
  const out: string[] = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...(await filesUnder(root, path)));
    else out.push(relative(root, path).split(sep).join("/"));
  }
  return out;
}

/** The manifest during the build, and dist/sw.js once the build is written. */
export function pwa(): Plugin {
  let outDir = "dist";
  return {
    name: "lamina-pwa",
    apply: "build",
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir);
    },
    generateBundle() {
      this.emitFile({
        type: "asset",
        fileName: "manifest.webmanifest",
        source: `${JSON.stringify(manifest(), null, 2)}\n`,
      });
    },
    async closeBundle() {
      const list = precacheList(await filesUnder(outDir));
      const contents = new Map<string, Uint8Array>();
      for (const path of list) {
        contents.set(path, await readFile(join(outDir, path === "/" ? "index.html" : path.slice(1))));
      }
      const source = await readFile(new URL("../src/sw/service-worker.ts", import.meta.url), "utf8");
      const { code } = await transformWithOxc(source, "service-worker.ts", { lang: "ts" });
      const prelude =
        `const __LAMINA_PRECACHE__ = ${JSON.stringify(list)};\n` +
        `const __LAMINA_VERSION__ = ${JSON.stringify(precacheVersion(contents))};\n`;
      await writeFile(join(outDir, "sw.js"), prelude + code);
    },
  };
}
