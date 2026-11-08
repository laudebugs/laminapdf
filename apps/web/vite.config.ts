import { readFile } from "node:fs/promises";
import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

/**
 * The website's icons, linked by index.html as the website and the docs site link them. The files
 * live in the website's `public/` (its `scripts/share-image.mjs` draws the PNGs from the SVG); this
 * plugin serves them in development and copies them into the build, so all three sites show the
 * same icon from one set of files.
 */
export const WEBSITE_ICONS = ["favicon.svg", "favicon-32.png", "apple-touch-icon.png"] as const;

function websiteIcons(): Plugin {
  const from = (name: string) => new URL(`../site/public/${name}`, import.meta.url);
  return {
    name: "lamina-website-icons",
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        const name = WEBSITE_ICONS.find((icon) => req.url === `/${icon}`);
        if (name === undefined) return next();
        res.setHeader("Content-Type", name.endsWith(".svg") ? "image/svg+xml" : "image/png");
        res.end(await readFile(from(name)));
      });
    },
    async generateBundle() {
      for (const name of WEBSITE_ICONS) {
        this.emitFile({ type: "asset", fileName: name, source: await readFile(from(name)) });
      }
    },
  };
}

// The reader, built with the React Compiler through Babel. The document worker is a module worker
// and every WebAssembly file a hashed asset of the same origin (ADR 0009 §2, ADR 0011 §1), so the
// built index.html loads only files from dist/: no inline script or style (the CSP allows none).
// The workspace packages are left out of dependency pre-bundling, which would move their worker
// and WebAssembly URLs away from the files they name. A default export because Vite reads one.
export default defineConfig({
  plugins: [react(), babel({ presets: [reactCompilerPreset()] }), websiteIcons()],
  optimizeDeps: {
    exclude: ["@laminapdf/wasm", "@laminapdf/sdk", "@embedpdf/pdfium", "@laminapdf/viewer", "@laminapdf/render-hayro"],
  },
  worker: { format: "es" },
  build: {
    target: "es2022",
    // The polyfill is an inline script; every supported browser has modulepreload.
    modulePreload: { polyfill: false },
    // Every asset is a file, so the CSP needs no data: fonts or inline styles.
    assetsInlineLimit: 0,
  },
  server: { fs: { allow: ["../.."] } },
});
