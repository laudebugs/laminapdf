// The bench page and its module worker. Cross-origin isolated (COOP/COEP), so Chromium's
// performance.measureUserAgentSpecificMemory() works; out/fixtures/ is served at /fixtures/.
import { defineConfig } from "vite";

const isolated = { "Cross-Origin-Opener-Policy": "same-origin", "Cross-Origin-Embedder-Policy": "require-corp" };

export default defineConfig({
  publicDir: "out/public",
  server: { port: 5199, strictPort: true, headers: isolated },
  preview: { port: 5199, strictPort: true, headers: isolated },
  optimizeDeps: { exclude: ["@lamina/pdf", "@lamina/wasm", "@embedpdf/pdfium"] },
  worker: { format: "es" },
});
