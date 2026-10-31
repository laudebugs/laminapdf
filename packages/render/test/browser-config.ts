// The browser projects both adapters run (Vitest browser mode with Playwright, headless, one
// project per browser: `--project chromium` …), so the Firefox workaround lives once.
import { mkdirSync } from "node:fs";
import { env } from "node:process";
import { fileURLToPath } from "node:url";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

/**
 * Firefox (Playwright's build, 155 and later) reads the user's Firefox data folder even with its
 * own profile, and fails with "Could not find profile folder." where it can't (macOS refuses it
 * on the self-hosted runners). MOZ_APP_DATA and MOZ_LOCAL_APP_DATA point it at a folder of its
 * own, in the package's node_modules (so runners sharing a machine don't share it). The same
 * workaround is in @laminapdf/pdf's vitest.browser.config.ts.
 */
function firefox(packageDir: URL) {
  const data = fileURLToPath(new URL("node_modules/.cache/lamina-firefox/", packageDir));
  mkdirSync(data, { recursive: true });
  return playwright({ launchOptions: { env: { ...env, MOZ_APP_DATA: data, MOZ_LOCAL_APP_DATA: data } } });
}

/** The RenderBackend contract's browser config for the package at `packageDir`. */
export function renderBrowserConfig(packageDir: URL) {
  return defineConfig({
    test: {
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
                { browser: "firefox", name: "firefox", provider: firefox(packageDir) },
                { browser: "webkit", name: "webkit" },
              ],
            },
          },
        },
      ],
    },
  });
}
