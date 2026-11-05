import { firefoxProvider } from "@laminapdf/test-utils/firefox";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

// The high-contrast theme in Chromium, Firefox and WebKit: its tokens are CSS system colours, so
// only a browser can say what they resolve to. Firefox gets its own data folder (firefoxProvider
// in @laminapdf/test-utils). A default export because Vitest reads one.
const firefox = firefoxProvider(new URL("./", import.meta.url));

export default defineConfig({
  test: {
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
