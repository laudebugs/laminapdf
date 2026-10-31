// Runs tests/bench.spec.mjs in Chromium, Firefox and WebKit against the Vite dev server (by
// hand, never in CI): `pnpm bench`, after `pnpm fixtures`.
import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig, devices } from "@playwright/test";

// Firefox keeps its app data in its own folder (as packages/pdf/vitest.browser.config.ts does):
// the default under ~/Library isn't writable in every environment the bench runs in.
const firefoxData = fileURLToPath(new URL("node_modules/.cache/firefox/", import.meta.url));
mkdirSync(firefoxData, { recursive: true });

export default defineConfig({
  testDir: "tests",
  timeout: 30 * 60_000,
  workers: 1,
  reporter: [["list"]],
  use: { baseURL: "http://localhost:5199" },
  webServer: { command: "pnpm exec vite", url: "http://localhost:5199", reuseExistingServer: true, timeout: 120_000 },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    {
      name: "firefox",
      use: {
        ...devices["Desktop Firefox"],
        launchOptions: { env: { ...process.env, MOZ_APP_DATA: firefoxData, MOZ_LOCAL_APP_DATA: firefoxData } },
      },
    },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
});
