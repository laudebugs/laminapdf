import { mkdirSync } from "node:fs";
import { env } from "node:process";
import { fileURLToPath } from "node:url";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

// The viewer's worker and RPC in Chromium, Firefox and WebKit (C3a). The tests import the built
// package (`cargo xtask wasm` builds it), so the worker file is the one an app gets. Firefox gets
// its own data folder, as in packages/render/test/browser-config.ts. A default export because
// Vitest reads one.
const data = fileURLToPath(new URL("node_modules/.cache/lamina-firefox/", import.meta.url));
mkdirSync(data, { recursive: true });
const firefox = playwright({ launchOptions: { env: { ...env, MOZ_APP_DATA: data, MOZ_LOCAL_APP_DATA: data } } });

export default defineConfig({
  optimizeDeps: { exclude: ["@lamina/wasm", "@lamina/pdf", "@embedpdf/pdfium", "@lamina/viewer"] },
  server: { fs: { allow: ["../.."] } },
  test: {
    testTimeout: 60_000,
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
});
