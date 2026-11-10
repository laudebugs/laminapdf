/// <reference types="vitest/config" />
// The site header in Chromium, Firefox and WebKit: keyboard order, the current page's underline,
// phone width, light and dark, axe. Astro's Vite setup compiles SiteHeader.astro, which the global
// setup renders with the container API in Node and hands to the tests as HTML. Firefox gets its own
// data folder (firefoxProvider in @laminapdf/test-utils). A default export because Vitest reads one.

import { fileURLToPath } from "node:url";
import { firefoxProvider } from "@laminapdf/test-utils/firefox";
import { retryOnceAndReport } from "@laminapdf/test-utils/flaky";
import { playwright } from "@vitest/browser-playwright";
import { getViteConfig } from "astro/config";

export default getViteConfig(
  {
    test: {
      // One retry, and a pass on it reported as flaky (ADR 0018 §4).
      ...retryOnceAndReport(),
      include: ["test/browser/*.browser.test.ts"],
      globalSetup: ["test/browser/render.ts"],
      browser: {
        enabled: true,
        headless: true,
        provider: playwright(),
        instances: [
          { browser: "chromium", name: "chromium" },
          { browser: "firefox", name: "firefox", provider: firefoxProvider(new URL("./", import.meta.url)) },
          { browser: "webkit", name: "webkit" },
        ],
      },
    },
  },
  { root: fileURLToPath(new URL("./", import.meta.url)), logLevel: "error" },
);
