import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The reader, built with the React Compiler through Babel. The document worker is a module worker
// and every WebAssembly file a hashed asset of the same origin (ADR 0009 §2, ADR 0011 §1), so the
// built index.html loads only files from dist/: no inline script or style (the CSP allows none).
// The workspace packages are left out of dependency pre-bundling, which would move their worker
// and WebAssembly URLs away from the files they name. A default export because Vite reads one.
export default defineConfig({
  plugins: [react(), babel({ presets: [reactCompilerPreset()] })],
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
