// Screenshots of every view (todo U6f), in Chromium only: each theme, the touch density at phone
// width, every page appearance for page-related routes, forced colours, and right to left. They
// are compared pixel for pixel with e2e/__screenshots__/; `cargo xtask gallery --update` is the only
// way to change those files. A failure leaves the expected, actual and diff images in the results.

import { expect, test } from "@playwright/test";
import { APPEARANCES, THEMES } from "../src/routes.ts";
import { open, VIEWS } from "./views.ts";

test.skip(({ browserName }) => browserName !== "chromium", "screenshots are Chromium's only");

for (const view of VIEWS) {
  test.describe(view.name, () => {
    for (const theme of THEMES) {
      test(`theme ${theme}`, async ({ page }) => {
        await open(page, view, { theme });
        await expect(page).toHaveScreenshot(`${view.name}--${theme}.png`, { fullPage: true });
      });
    }

    test("touch density at 390 px", async ({ page }) => {
      await page.setViewportSize({ width: 390, height: 844 });
      await open(page, view, { density: "touch" });
      await expect(page).toHaveScreenshot(`${view.name}--touch.png`, { fullPage: true });
    });

    test("forced colours", async ({ page }) => {
      await page.emulateMedia({ forcedColors: "active" });
      await open(page, view);
      await expect(page).toHaveScreenshot(`${view.name}--forced-colors.png`, { fullPage: true });
    });

    test("right to left", async ({ page }) => {
      await open(page, view, { dir: "rtl" });
      await expect(page).toHaveScreenshot(`${view.name}--rtl.png`, { fullPage: true });
    });

    if (view.pageRelated) {
      for (const appearance of APPEARANCES.filter((a) => a !== "original")) {
        test(`page appearance ${appearance}`, async ({ page }) => {
          await open(page, view, { appearance });
          await expect(page).toHaveScreenshot(`${view.name}--page-${appearance}.png`, { fullPage: true });
        });
      }
    }
  });
}
