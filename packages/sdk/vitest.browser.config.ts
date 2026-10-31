import { mkdirSync } from "node:fs";
import { env } from "node:process";
import { fileURLToPath } from "node:url";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

// Firefox (Playwright's build, 155 and later) reads the user's Firefox data folder
// (~/Library/Application Support/Firefox on macOS) even with its own profile, and fails with
// "Could not find profile folder." where it can't read that folder (macOS refuses it on the
// self-hosted runners and in sandboxed shells). MOZ_APP_DATA and MOZ_LOCAL_APP_DATA point it at a
// folder of its own instead, in this checkout (so runners sharing a machine don't share it).
const firefoxData = fileURLToPath(new URL("node_modules/.cache/lamina-firefox/", import.meta.url));
mkdirSync(firefoxData, { recursive: true });
const firefox = playwright({
  launchOptions: { env: { ...env, MOZ_APP_DATA: firefoxData, MOZ_LOCAL_APP_DATA: firefoxData } },
});

// The browser runs of the B6c matrix: test/browser/*.browser.test.ts in Chromium, Firefox and
// WebKit (Vitest browser mode with Playwright, headless; one project per browser, picked with
// `--project chromium` …), through the `browser` export condition as Vite serves it: the package
// unbundled (excluded from pre-bundling) and each `.wasm` fetched next to its glue. The
// `vite-build` project builds a tiny app with Vite and runs it in a Chromium page
// (test/browser/vite-build.test.ts, in Node). `cargo xtask pdf-runtimes chromium` runs both.
// Browsers come from `playwright install` (PLAYWRIGHT_BROWSERS_PATH in CI).
// A default export because Vitest reads one (the named-exports rule is for package code).
export default defineConfig({
  test: {
    testTimeout: 60_000,
    projects: [
      {
        extends: true,
        optimizeDeps: { exclude: ["@laminapdf/sdk"] },
        test: {
          name: "browser",
          include: ["test/browser/*.browser.test.ts"],
          browser: {
            enabled: true,
            headless: true,
            provider: playwright(),
            instances: [
              { browser: "chromium", name: "chromium" },
              { browser: "firefox", name: "firefox", provider: firefox },
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
