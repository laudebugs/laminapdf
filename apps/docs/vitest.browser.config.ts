/// <reference types="vitest/config" />
// The docs' diagrams in Chromium, Firefox and WebKit: contrast in the light, dark and high-contrast
// themes and with forced colours emulated, the 12 px label floor at desktop, narrowest-wide and
// phone widths, and the image's name and description. Astro's Vite setup compiles Diagram.astro,
// which the global setup renders with the container API in Node and hands to the tests as HTML
// (as @laminapdf/site-kit's header test does). Firefox gets its own data folder (firefoxProvider in
// @laminapdf/test-utils). A default export because Vitest reads one.

import { fileURLToPath } from "node:url";
import { firefoxProvider } from "@laminapdf/test-utils/firefox";
import { retryOnceAndReport } from "@laminapdf/test-utils/flaky";
import { playwright } from "@vitest/browser-playwright";
import { getViteConfig } from "astro/config";
import type { BrowserCommand } from "vitest/node";

// Lets a test turn forced colours on, which a page can't do for itself.
const forcedColors: BrowserCommand<["active" | "none"]> = async (ctx, value) => {
  if (ctx.provider.name !== "playwright") throw new Error("forcedColors needs the Playwright provider");
  await ctx.page.emulateMedia({ forcedColors: value });
};

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
        // Wide enough for a desktop column: a label outside the window has nothing under it.
        viewport: { width: 1280, height: 900 },
        commands: { forcedColors },
        provider: playwright(),
        instances: [
          { browser: "chromium", name: "chromium" },
          { browser: "firefox", name: "firefox", provider: firefoxProvider(new URL("./", import.meta.url)) },
          { browser: "webkit", name: "webkit" },
        ],
      },
    },
  },
  // configFile: false: the docs' own Astro config (Starlight, TypeDoc) is not needed to render a diagram.
  { root: fileURLToPath(new URL("./", import.meta.url)), configFile: false, logLevel: "error" },
);
