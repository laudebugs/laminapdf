/// <reference types="vitest/config" />
// The site header and the select in Chromium, Firefox and WebKit: keyboard order, the current
// page's underline, phone width, light and dark, cursors and the pressed state, axe. Astro's Vite setup compiles SiteHeader.astro, which the global
// setup renders with the container API in Node and hands to the tests as HTML. Firefox gets its own
// data folder (firefoxProvider in @laminapdf/test-utils). A default export because Vitest reads one.

import { fileURLToPath } from "node:url";
import { firefoxProvider } from "@laminapdf/test-utils/firefox";
import { retryOnceAndReport } from "@laminapdf/test-utils/flaky";
import { playwright } from "@vitest/browser-playwright";
import { getViteConfig } from "astro/config";
import type { BrowserCommand } from "vitest/node";

/**
 * Holds the real mouse button down on the element `selector` names, 4 px in from its start on its
 * middle line, off a link's text so nothing is dragged: a page can't press its own elements, and
 * :active needs a real press. `release` lets go away from it.
 */
const press: BrowserCommand<[selector: string]> = async (ctx, selector) => {
  await ctx.iframe
    .locator(selector)
    .first()
    .hover({ position: { x: 4, y: 8 } });
  await ctx.page.mouse.down();
};

const release: BrowserCommand<[]> = async (ctx) => {
  await ctx.page.mouse.move(0, 0);
  await ctx.page.mouse.up();
};

/** Makes the page prefer reduced motion or not, which a page can't set for itself. */
const reducedMotion: BrowserCommand<["reduce" | "no-preference"]> = async (ctx, value) => {
  await ctx.page.emulateMedia({ reducedMotion: value });
};

export default getViteConfig(
  {
    test: {
      ...retryOnceAndReport(),
      include: ["test/browser/*.browser.test.ts"],
      globalSetup: ["test/browser/render.ts"],
      browser: {
        enabled: true,
        headless: true,
        commands: { press, release, reducedMotion },
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
