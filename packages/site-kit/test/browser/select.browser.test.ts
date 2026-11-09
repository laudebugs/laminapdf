// The sites' select in a real browser, as Select.astro renders it: its name from the label, shown
// or hidden; chosen with the keyboard and the pointer, the chevron letting presses through; the
// field's colours and the focus ring from the Paper tokens; no inline style; and no axe violation,
// in the light and the dark theme.

import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "../../src/select.css";
import axe from "axe-core";
import { afterEach, describe, expect, inject, it } from "vitest";
import { page, userEvent } from "vitest/browser";

const SELECTS = inject("selects");

const ground = document.createElement("style");
ground.textContent = "body { margin: 0; padding: 16px; background: var(--paper-ground); color: var(--paper-text); }";
document.head.append(ground);

/** `html` on a fresh body in `theme`, and its <select>. */
function mount(html: string, theme: "light" | "dark"): HTMLSelectElement {
  document.documentElement.dataset.theme = theme;
  document.body.innerHTML = `<main><h1>Page</h1>${html}</main>`;
  const select = document.querySelector("select");
  if (select === null) throw new Error("no select");
  return select;
}

/** `value` as the browser computes a colour (`rgb(…)`). */
function resolvedColor(value: string): string {
  const probe = document.createElement("span");
  probe.style.color = value;
  document.body.append(probe);
  const resolved = getComputedStyle(probe).color;
  probe.remove();
  return resolved;
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("the select's markup", () => {
  it("carries no inline style, which the website's checks refuse", () => {
    for (const html of [SELECTS.shown, SELECTS.hidden]) expect(html).not.toMatch(/\sstyle=/);
  });
});

for (const theme of ["light", "dark"] as const) {
  describe(`the select, ${theme}`, () => {
    it("takes its name from the label, shown or hidden", async () => {
      mount(SELECTS.shown, theme);
      await expect.element(page.getByRole("combobox", { name: "Theme" })).toBeVisible();
      await expect.element(page.getByText("Theme")).toBeVisible();
      mount(SELECTS.hidden, theme);
      await expect.element(page.getByRole("combobox", { name: "Select theme" })).toBeVisible();
      const label = document.querySelector(".site-select-label");
      expect(label?.getBoundingClientRect().width).toBeLessThanOrEqual(1);
    });

    it("starts on the chosen option and changes with the keyboard", async () => {
      const select = mount(SELECTS.hidden, theme);
      expect(select.value).toBe("auto");
      const changes: string[] = [];
      select.addEventListener("change", () => changes.push(select.value));
      await userEvent.tab();
      expect(document.activeElement).toBe(select);
      await userEvent.selectOptions(select, "light");
      expect(select.value).toBe("light");
      expect(changes).toEqual(["light"]);
    });

    it("shows the focus ring when the keyboard reaches it", async () => {
      const select = mount(SELECTS.hidden, theme);
      await userEvent.tab();
      const style = getComputedStyle(select);
      expect(style.outlineStyle).toBe("solid");
      expect(style.outlineColor).toBe(resolvedColor("var(--paper-focus)"));
    });

    it("lets a press on the chevron reach the select", () => {
      mount(SELECTS.hidden, theme);
      const chevron = document.querySelector(".site-select-chevron");
      if (chevron === null) throw new Error("no chevron");
      expect(chevron.getAttribute("aria-hidden")).toBe("true");
      const box = chevron.getBoundingClientRect();
      expect(document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2)?.tagName).toBe("SELECT");
    });

    it("draws the Paper field: its fill, text, edge and height", () => {
      const select = mount(SELECTS.hidden, theme);
      const style = getComputedStyle(select);
      expect(style.backgroundColor).toBe(resolvedColor("var(--paper-field)"));
      expect(style.color).toBe(resolvedColor("var(--paper-text)"));
      expect(style.borderTopColor).toBe(resolvedColor("var(--paper-control-border)"));
      expect(select.getBoundingClientRect().height).toBeGreaterThanOrEqual(32);
    });

    it("has no axe violation", async () => {
      for (const html of [SELECTS.shown, SELECTS.hidden]) {
        mount(html, theme);
        const results = await axe.run(document.body, { rules: { region: { enabled: false } } });
        expect(results.violations.map((v) => `${v.id}: ${v.nodes.length}`)).toEqual([]);
      }
    });
  });
}
