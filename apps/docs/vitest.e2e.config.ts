import { retryOnceAndReport } from "@laminapdf/test-utils/flaky";
import { defineConfig } from "vitest/config";

// "Edit and run" (todo B7e) and the theme switch (W1g) in a real browser against `wrangler dev`, which
// serves dist/ with its _headers as production does: build first, after `cargo xtask wasm`. LAMINA_PDF_E2E_BROWSER picks
// chromium (default), firefox or webkit; `cargo xtask pdf-runtimes <browser>` runs it for each.
// A default export because Vitest reads one.
export default defineConfig({
  test: {
    // One retry, and a pass on it reported as flaky (ADR 0018 §4).
    ...retryOnceAndReport(),
    environment: "node",
    include: ["e2e/*.e2e.test.ts"],
    globalSetup: ["e2e/server.ts"],
    testTimeout: 120_000,
    hookTimeout: 120_000,
  },
});
