// axe on every view in every theme, in Chromium, Firefox and WebKit (todo U6f): zero violations.

import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, test } from "@playwright/test";
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
      // A route's grid shows a component's states side by side, so a landmark the app has once
      // (reader mode's section) repeats there with the same name. A `landmark-unique` finding
      // counts only when the landmark it repeats sits in the same sample, or both sit outside the
      // grid: a repeat within one component. One axe run per page; one per sample was too slow.
      const results = await new AxeBuilder({ page }).disableRules(["region", ...pageRules]).analyze();
      const violations = [];
      for (const violation of results.violations) {
        if (violation.id !== "landmark-unique") {
          violations.push(violation);
          continue;
        }
        const nodes = [];
        for (const node of violation.nodes) {
          const related = node.any.flatMap((check) => (check.relatedNodes ?? []).map((r) => r.target));
          if (await sameSample(page, node.target, related)) nodes.push(node);
        }
        if (nodes.length > 0) violations.push({ ...violation, nodes });
      }
      const found = violations.map((v) => `${v.id}: ${v.help} (${v.nodes.map((n) => n.target.join(" ")).join(", ")})`);
      expect(found).toEqual([]);
    });
  }
}

/**
 * Whether the landmark at `target` and any landmark in `related` sit in the same sample of the
 * gallery's grid, or both outside it. Targets are axe's selectors (one per frame).
 */
async function sameSample(page: Page, target: unknown, related: unknown[]): Promise<boolean> {
  return page.evaluate(
    ([one, others]) => {
      const sample = (selector: unknown) => {
        const path = Array.isArray(selector) ? selector : [selector];
        const element = document.querySelector(String(path[path.length - 1]));
        return element?.closest(".g-grid > .g-cell") ?? null;
      };
      const mine = sample(one);
      return others.length === 0 || others.some((other) => sample(other) === mine);
    },
    [target, related] as const,
  );
}
