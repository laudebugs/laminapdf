import { defineConfig } from "vitest/config";

// The deployed reader at LAMINA_PDF_SMOKE_URL, in Chromium (e2e/smoke.test.ts): what each deploy
// checks once it is live (.github/workflows/web-deploy.yml). Nothing is built or served here;
// e2e/live.ts waits for the host to serve the app first. A default export because Vitest reads one.
export default defineConfig({
  test: {
    environment: "node",
    include: ["e2e/smoke.test.ts"],
    globalSetup: ["e2e/live.ts"],
    testTimeout: 60_000,
    hookTimeout: 60_000,
    // A deploy can take a few seconds to reach every Cloudflare location: one retry.
    retry: 1,
  },
});
