import { defineConfig } from "vitest/config";

// "Edit and run" (todo B7e) in a real browser against `wrangler dev`, which serves dist/ with its
// _headers as production does: build first, after `cargo xtask wasm`. LAMINA_PDF_E2E_BROWSER picks
// chromium (default), firefox or webkit; `cargo xtask pdf-runtimes <browser>` runs it for each.
// A default export because Vitest reads one.
export default defineConfig({
  test: {
    environment: "node",
    include: ["e2e/*.e2e.test.ts"],
    globalSetup: ["e2e/server.ts"],
    testTimeout: 120_000,
    hookTimeout: 120_000,
  },
});
