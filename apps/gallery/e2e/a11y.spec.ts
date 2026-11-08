// axe on every view in every theme, in Chromium, Firefox and WebKit (todo U6f): zero violations.

import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { THEMES } from "../src/routes.ts";
import { open, VIEWS } from "./views.ts";

for (const view of VIEWS) {
  for (const theme of THEMES) {
    test(`${view.name} in ${theme} has no axe violations`, async ({ page }) => {
      await open(page, view, { theme });
      // `region` asks that all content sit in landmarks; overlays portal to the body by design, so
      // it would only ever flag them. On overlay views a modal hides the page behind it, and with
      // it the gallery's own `main` and `h1`, which those two rules look for: they check the
      // gallery's page, not the component in front of it.
      const pageRules = view.state === "" ? [] : ["landmark-one-main", "page-has-heading-one"];
      const results = await new AxeBuilder({ page }).disableRules(["region", ...pageRules]).analyze();
      const found = results.violations.map(
        (v) => `${v.id}: ${v.help} (${v.nodes.map((n) => n.target.join(" ")).join(", ")})`,
      );
      expect(found).toEqual([]);
    });
  }
}
