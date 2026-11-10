// The browser projects the render adapters run (Vitest browser mode with Playwright, headless,
// one project per browser: `--project chromium` …).

import { firefoxProvider } from "@laminapdf/test-utils/firefox";
import { retryOnceAndReport } from "@laminapdf/test-utils/flaky";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

/** The RenderBackend contract's browser config for the package at `packageDir`. */
export function renderBrowserConfig(packageDir: URL) {
  return defineConfig({
    test: {
      // One retry, and a pass on it reported as flaky (ADR 0018 §4).
      ...retryOnceAndReport(),
      testTimeout: 60_000,
      projects: [
        {
          extends: true,
          optimizeDeps: { exclude: ["@laminapdf/wasm", "@embedpdf/pdfium"] },
          test: {
            name: "browser",
            include: ["test/browser/*.browser.test.ts"],
            browser: {
              enabled: true,
              headless: true,
              provider: playwright(),
              instances: [
                { browser: "chromium", name: "chromium" },
                { browser: "firefox", name: "firefox", provider: firefoxProvider(packageDir) },
                { browser: "webkit", name: "webkit" },
              ],
            },
          },
        },
      ],
    },
  });
}
