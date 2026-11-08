import process from "node:process";
import { defineConfig, devices } from "@playwright/test";

// The gallery's screenshot, axe and font tests (todo U6f). They run in the pinned Playwright image
// (package.json's `config.playwrightImage`) through `cargo xtask gallery --test`, so screenshots
// match on every machine. Screenshots are Chromium's only; axe runs in all three engines.
// Snapshots are never written by a test run: only `cargo xtask gallery --update` passes
// `--update-snapshots`. A default export because Playwright reads one.
export default defineConfig({
  testDir: "e2e",
  snapshotPathTemplate: "{testDir}/__screenshots__/{arg}{ext}",
  outputDir: process.env.GALLERY_RESULTS ?? "test-results",
  fullyParallel: true,
  forbidOnly: true,
  retries: 0,
  updateSnapshots: "none",
  reporter: [
    ["list"],
    ["html", { outputFolder: `${process.env.GALLERY_RESULTS ?? "test-results"}/report`, open: "never" }],
  ],
  expect: {
    toHaveScreenshot: { maxDiffPixels: 0, animations: "disabled", caret: "hide", scale: "css" },
  },
  use: {
    baseURL: "http://127.0.0.1:4174/",
    locale: "en-GB",
    timezoneId: "UTC",
    viewport: { width: 1280, height: 900 },
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 900 } } },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"], viewport: { width: 1280, height: 900 } },
      testMatch: "a11y.spec.ts",
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"], viewport: { width: 1280, height: 900 } },
      testMatch: "a11y.spec.ts",
    },
  ],
  webServer: {
    command: "pnpm run build && pnpm run preview",
    url: "http://127.0.0.1:4174/",
    reuseExistingServer: false,
    timeout: 180_000,
  },
});
