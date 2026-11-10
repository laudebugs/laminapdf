import { firefoxProvider } from "@laminapdf/test-utils/firefox";
import { retryOnceAndReport } from "@laminapdf/test-utils/flaky";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";
import type { BrowserCommand } from "vitest/node";

// The viewer's worker and RPC in Chromium, Firefox and WebKit (C3a). The tests import the built
// package (`cargo xtask wasm` builds it), so the worker file is the one an app gets. Firefox gets
// its own data folder (firefoxProvider in @laminapdf/test-utils). A default export because Vitest
// reads one.
const firefox = firefoxProvider(new URL("./", import.meta.url));

/**
 * Drags the real mouse across the element `selector` names, from 2 px inside its left edge to 2 px
 * inside its right, on its middle line: what a reader does to select a word. `force` skips
 * Playwright's check that the element receives the press, so a layer on top fails the test's
 * assertion (nothing selected) instead of a timeout.
 */
const dragAcross: BrowserCommand<[selector: string]> = async (ctx, selector) => {
  const element = ctx.iframe.locator(selector);
  const box = await element.boundingBox();
  if (box === null) throw new Error(`${selector} has no box`);
  const y = box.height / 2;
  await element.dragTo(element, {
    force: true,
    sourcePosition: { x: 2, y },
    targetPosition: { x: box.width - 2, y },
  });
};

/** A PNG screenshot of the element `selector` names, base64: what the reader sees there. */
const screenshotOf: BrowserCommand<[selector: string]> = async (ctx, selector) =>
  (await ctx.iframe.locator(selector).screenshot()).toString("base64");

export default defineConfig({
  optimizeDeps: { exclude: ["@laminapdf/wasm", "@laminapdf/sdk", "@embedpdf/pdfium", "@laminapdf/viewer"] },
  server: { fs: { allow: ["../.."] } },
  test: {
    ...retryOnceAndReport(),
    testTimeout: 60_000,
    include: ["test/browser/*.browser.test.ts"],
    browser: {
      enabled: true,
      headless: true,
      commands: { dragAcross, screenshotOf },
      provider: playwright(),
      instances: [
        { browser: "chromium", name: "chromium" },
        { browser: "firefox", name: "firefox", provider: firefox },
        { browser: "webkit", name: "webkit" },
      ],
    },
  },
});
