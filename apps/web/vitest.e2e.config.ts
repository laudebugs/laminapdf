import { defineConfig } from "vitest/config";

// The reader in a real browser against `wrangler dev`, which serves dist/ as production does (assets
// and _headers): build first. LAMINA_PDF_E2E_BROWSER picks chromium (default), firefox or webkit;
// `cargo xtask pdf-runtimes <browser>` runs it for each. A default export because Vitest reads one.
export default defineConfig({
  test: {
    environment: "node",
    include: ["e2e/*.e2e.test.ts"],
    globalSetup: ["e2e/server.ts"],
    testTimeout: 60_000,
    hookTimeout: 120_000,
    // One browser and one server: the files share them, so they run one after another.
    fileParallelism: false,
  },
});
