import { firefoxProvider } from "@laminapdf/test-utils/firefox";
import { retryOnceAndReport } from "@laminapdf/test-utils/flaky";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";
import type { BrowserCommand } from "vitest/node";

// The components in Chromium, Firefox and WebKit: states, keyboard, ARIA, axe, right to left and
// touch sizes. Firefox gets its own data folder (firefoxProvider in @laminapdf/test-utils). A
// default export because Vitest reads one.
const firefox = firefoxProvider(new URL("./", import.meta.url));

// Lets a test make the page prefer reduced motion, which a page can't set for itself.
const reducedMotion: BrowserCommand<["reduce" | null]> = async (ctx, value) => {
  if (ctx.provider.name !== "playwright") throw new Error("reducedMotion needs the Playwright provider");
  await ctx.page.emulateMedia({ reducedMotion: value });
};

export default defineConfig({
  test: {
    // One retry, and a pass on it reported as flaky (ADR 0018 §4).
    ...retryOnceAndReport(),
    include: ["test/browser/*.browser.test.tsx"],
    browser: {
      enabled: true,
      headless: true,
      // Desktop-sized, wider than the phone breakpoint; tests of the phone layout set their own.
      viewport: { width: 1280, height: 800 },
      commands: { reducedMotion },
      provider: playwright(),
      instances: [
        { browser: "chromium", name: "chromium" },
        { browser: "firefox", name: "firefox", provider: firefox },
        { browser: "webkit", name: "webkit" },
      ],
    },
  },
});
