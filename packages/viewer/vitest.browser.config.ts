import { firefoxProvider } from "@laminapdf/test-utils/browser-config";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

// The viewer's worker and RPC in Chromium, Firefox and WebKit (C3a). The tests import the built
// package (`cargo xtask wasm` builds it), so the worker file is the one an app gets. Firefox gets
// its own data folder (firefoxProvider in @laminapdf/test-utils). A default export because Vitest
// reads one.
const firefox = firefoxProvider(new URL("./", import.meta.url));

export default defineConfig({
  optimizeDeps: { exclude: ["@laminapdf/wasm", "@laminapdf/sdk", "@embedpdf/pdfium", "@laminapdf/viewer"] },
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
