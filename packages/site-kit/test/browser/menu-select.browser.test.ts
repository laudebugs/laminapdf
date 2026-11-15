// The sites' select in a real browser, as MenuSelect.astro renders it and menu-select.ts drives it:
// its name from the label, shown or hidden; the chosen value's icon and label in the button; the
// list opened, moved through and chosen from with the keyboard (the focus staying on the button)
// and with the pointer; a click outside closing it; the Paper colours and the focus ring; no
// inline style; and no axe violation, open or closed, in the light and the dark theme.

import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "../../src/menu-select.css";
import axe from "axe-core";
import { afterEach, describe, expect, inject, it } from "vitest";
import { page, userEvent } from "vitest/browser";
import { defineMenuSelect, type SiteMenuSelect } from "../../src/menu-select.ts";

const SELECTS = inject("selects");
defineMenuSelect();

const ground = document.createElement("style");
ground.textContent = "body { margin: 0; padding: 16px; background: var(--paper-ground); color: var(--paper-text); }";
document.head.append(ground);

/** `html` on a fresh body in `theme`, and its select. */
function mount(html: string, theme: "light" | "dark"): SiteMenuSelect {
  document.documentElement.dataset.theme = theme;
  document.body.innerHTML = "<main><h1>Page</h1></main>";
  // Parsed in a template, whose scripts never run, and the rendered module script dropped: this
  // file defines the element.
  const template = document.createElement("template");
  template.innerHTML = html;
  for (const script of template.content.querySelectorAll("script")) script.remove();
  document.querySelector("main")?.append(template.content);
  const select = document.querySelector<SiteMenuSelect>("site-menu-select");
  if (select === null) throw new Error("no select");
  return select;
}

function button(): HTMLButtonElement {
  const found = document.querySelector<HTMLButtonElement>(".site-menu-select-button");
  if (found === null) throw new Error("no button");
  return found;
}

/** The label the button shows: the other options' labels are in it too, hidden. */
function shown(): string | undefined {
  return button().querySelector("[data-for]:not([hidden])")?.textContent?.trim();
}

/** The text of the option aria-activedescendant points at. */
function active(): string | undefined {
  const id = button().getAttribute("aria-activedescendant");
  return id ? document.getElementById(id)?.textContent?.trim() : undefined;
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

  it("names the button as served, before its script runs", () => {
    expect(SELECTS.hidden).toMatch(/role="combobox"[^>]*aria-label="Select theme"/);
  });
});

for (const theme of ["light", "dark"] as const) {
  describe(`the select, ${theme}`, () => {
    it("takes its name from the label, shown or hidden, and shows the value with its icon", async () => {
      mount(SELECTS.shown, theme);
      await expect.element(page.getByRole("combobox", { name: "Theme" })).toBeVisible();
      await expect.element(page.getByText("Theme")).toBeVisible();
      mount(SELECTS.hidden, theme);
      await expect.element(page.getByRole("combobox", { name: "Select theme" })).toBeVisible();
      expect(document.querySelector(".site-menu-select-label")?.getBoundingClientRect().width).toBeLessThanOrEqual(1);
      expect(shown()).toBe("Auto");
      expect([...button().querySelectorAll("svg")].filter((svg) => svg.getBoundingClientRect().width > 0).length).toBe(
        2, // the value's icon and the chevron
      );
    });

    it("opens, moves and chooses with the keyboard, the focus staying on the button", async () => {
      const select = mount(SELECTS.hidden, theme);
      const changes: string[] = [];
      select.addEventListener("change", () => changes.push(select.value));
      await userEvent.tab();
      expect(document.activeElement).toBe(button());
      await userEvent.keyboard("{ArrowDown}");
      expect(button().getAttribute("aria-expanded")).toBe("true");
      expect(active()).toBe("Auto");
      await userEvent.keyboard("{Home}");
      expect(active()).toBe("Light");
      await userEvent.keyboard("{ArrowDown}");
      expect(active()).toBe("Dark");
      await userEvent.keyboard("{Enter}");
      expect(button().getAttribute("aria-expanded")).toBe("false");
      expect(select.value).toBe("dark");
      expect(shown()).toBe("Dark");
      expect(changes).toEqual(["dark"]);
      expect(document.activeElement).toBe(button());

      await userEvent.keyboard("l");
      expect(active()).toBe("Light");
      await userEvent.keyboard("{Escape}");
      expect(select.value).toBe("dark");
      expect(changes).toEqual(["dark"]);
    });

    it("opens with Space and chooses with Space, the key's release clicking nothing", async () => {
      const select = mount(SELECTS.hidden, theme);
      await userEvent.tab();
      await userEvent.keyboard(" ");
      expect(button().getAttribute("aria-expanded")).toBe("true");
      await userEvent.keyboard("{ArrowUp}");
      await userEvent.keyboard(" ");
      expect(button().getAttribute("aria-expanded")).toBe("false");
      expect(select.value).toBe("dark");
    });

    it("chooses on Tab only the keyboard's option, not one the pointer passed over", async () => {
      const select = mount(SELECTS.hidden, theme);
      await userEvent.tab();
      await userEvent.keyboard("{ArrowDown}");
      await userEvent.hover(page.getByRole("option", { name: "Light" }));
      await userEvent.tab();
      expect(select.value).toBe("auto");
    });

    it("focuses the button when its shown label is pressed", async () => {
      mount(SELECTS.shown, theme);
      await userEvent.click(page.getByText("Theme"));
      expect(document.activeElement).toBe(button());
    });

    it("chooses with the pointer and closes on a click outside", async () => {
      const select = mount(SELECTS.hidden, theme);
      await userEvent.click(button());
      await userEvent.click(page.getByRole("option", { name: "Light" }));
      expect(select.value).toBe("light");
      expect(document.querySelector<HTMLElement>("[role=listbox]")?.hidden).toBe(true);
      await userEvent.click(button());
      expect(document.querySelector<HTMLElement>("[role=listbox]")?.hidden).toBe(false);
      await userEvent.click(page.getByRole("heading", { name: "Page" }));
      expect(document.querySelector<HTMLElement>("[role=listbox]")?.hidden).toBe(true);
      expect(select.value).toBe("light");
    });

    it("marks the chosen option, with an icon on every option", async () => {
      mount(SELECTS.hidden, theme);
      await userEvent.click(button());
      const options = [...document.querySelectorAll("[role=option]")];
      expect(options.map((o) => o.getAttribute("aria-selected"))).toEqual(["false", "false", "true"]);
      for (const option of options) expect(option.querySelector("svg.site-menu-select-icon")).not.toBeNull();
    });

    it("shows the focus ring when the keyboard reaches it", async () => {
      mount(SELECTS.hidden, theme);
      await userEvent.tab();
      const style = getComputedStyle(button());
      expect(style.outlineStyle).toBe("solid");
      expect(style.outlineColor).toBe(resolvedColor("var(--paper-focus)"));
    });

    it("draws the Paper field and popup", async () => {
      mount(SELECTS.hidden, theme);
      const style = getComputedStyle(button());
      expect(style.backgroundColor).toBe(resolvedColor("var(--paper-field)"));
      expect(style.borderTopColor).toBe(resolvedColor("var(--paper-control-border)"));
      expect(button().getBoundingClientRect().height).toBeGreaterThanOrEqual(32);
      await userEvent.click(button());
      const list = document.querySelector("[role=listbox]");
      if (list === null) throw new Error("no list");
      expect(getComputedStyle(list).backgroundColor).toBe(resolvedColor("var(--paper-surface)"));
      expect(list.getBoundingClientRect().top).toBeGreaterThan(button().getBoundingClientRect().bottom);
    });

    it("has no axe violation, closed or open", async () => {
      for (const html of [SELECTS.shown, SELECTS.hidden]) {
        mount(html, theme);
        const closed = await axe.run(document.body, { rules: { region: { enabled: false } } });
        expect(closed.violations.map((v) => `${v.id}: ${v.nodes.length}`)).toEqual([]);
        await userEvent.click(button());
        const open = await axe.run(document.body, { rules: { region: { enabled: false } } });
        expect(open.violations.map((v) => `${v.id}: ${v.nodes.length}`)).toEqual([]);
      }
    });
  });
}
