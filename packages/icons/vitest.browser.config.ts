import { firefoxProvider } from "@laminapdf/test-utils/firefox";
import { retryOnceAndReport } from "@laminapdf/test-utils/flaky";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

// `<Icon>` in React and the gallery's accessibility (axe) in Chromium, Firefox and WebKit. Firefox
// gets its own data folder (firefoxProvider in @laminapdf/test-utils). A default export because
// Vitest reads one.
const firefox = firefoxProvider(new URL("./", import.meta.url));

export default defineConfig({
  test: {
    // One retry, and a pass on it reported as flaky (ADR 0018 §4).
    ...retryOnceAndReport(),
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
