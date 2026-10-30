import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

// The browser runs of the B6c matrix: test/browser/*.browser.test.ts in Chromium, Firefox and
// WebKit (Vitest browser mode with Playwright, headless; one project per browser, picked with
// `--project chromium` …), through the `browser` export condition as Vite serves it: the package
// unbundled (excluded from pre-bundling) and each `.wasm` fetched next to its glue. The
// `vite-build` project builds a tiny app with Vite and runs it in a Chromium page
// (test/browser/vite-build.test.ts, in Node). `cargo xtask pdf-runtimes chromium` runs both.
// Browsers come from `playwright install` (PLAYWRIGHT_BROWSERS_PATH in CI).
// A default export because Vitest reads one (the named-exports rule is for package code).
export default defineConfig({
  optimizeDeps: { exclude: ["@lamina/pdf"] },
  test: {
    testTimeout: 60_000,
    projects: [
      {
        extends: true,
        optimizeDeps: { exclude: ["@lamina/pdf"] },
        test: {
          name: "browser",
          include: ["test/browser/*.browser.test.ts"],
          browser: {
            enabled: true,
            headless: true,
            provider: playwright(),
            instances: [
              { browser: "chromium", name: "chromium" },
              { browser: "firefox", name: "firefox" },
              { browser: "webkit", name: "webkit" },
            ],
          },
        },
      },
      {
        extends: true,
        test: { name: "vite-build", environment: "node", include: ["test/browser/vite-build.test.ts"] },
      },
    ],
  },
});
