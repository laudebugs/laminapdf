import { defineConfig } from "vitest/config";

// Node: the hash verifier and the RenderBackend contract over the installed pdfium.wasm. The
// browser runs (Chromium, Firefox, WebKit, with the native render goldens) are vitest.browser.config.ts.
// A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"], testTimeout: 60_000 },
});
