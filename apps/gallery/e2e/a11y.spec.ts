// axe on every view in every theme, in Chromium, Firefox and WebKit (todo U6f): zero violations.

import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { THEMES } from "../src/routes.ts";
import { open, VIEWS } from "./views.ts";

for (const view of VIEWS) {
  for (const theme of THEMES) {
    test(`${view.name} in ${theme} has no axe violations`, async ({ page }) => {
      await open(page, view, { theme });
      const results = await new AxeBuilder({ page }).analyze();
      const found = results.violations.map(
        (v) => `${v.id}: ${v.help} (${v.nodes.map((n) => n.target.join(" ")).join(", ")})`,
      );
      expect(found).toEqual([]);
    });
  }
}
