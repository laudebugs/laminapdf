// The site header in a real browser, as both sites render it: links in §7.39's order in the tab
// order, the current page in 600 with the accent underline, no sideways scroll at phone width, and
// no axe violation, in the light and the dark theme.

import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "../../src/header.css";
import axe from "axe-core";
import { afterEach, describe, expect, inject, it } from "vitest";
import { page, server, userEvent } from "vitest/browser";
import { wireMenus } from "../../src/menu.mjs";

const HEADERS = inject("headers");

// The page each site draws the header on: the Paper ground and text, no margin.
const ground = document.createElement("style");
ground.textContent = "body { margin: 0; background: var(--paper-ground); color: var(--paper-text); }";
document.head.append(ground);

/** `html` in a <header> on a fresh body, in `theme`. */
function mount(html: string, theme: "light" | "dark"): HTMLElement {
  document.documentElement.dataset.theme = theme;
  document.body.innerHTML = `<header>${html}</header><main><h1>Page</h1></main>`;
  const header = document.querySelector("header");
  if (header === null) throw new Error("no header");
  return header;
}

/** `color` as the browser computes it (`rgb(…)`), the form `box-shadow` reports its colour in. */
function resolvedColor(color: string): string {
  const probe = document.createElement("span");
  probe.style.color = color;
  document.body.append(probe);
  const resolved = getComputedStyle(probe).color;
  probe.remove();
  return resolved;
}

/**
 * Moves the focus on, as a keyboard user does. WebKit on macOS skips links on Tab unless the
 * system's keyboard navigation setting is on, as Safari does; Option+Tab reaches them there.
 */
async function tabForward(): Promise<void> {
  if (server.browser === "webkit") await userEvent.keyboard("{Alt>}{Tab}{/Alt}");
  else await userEvent.tab();
}

afterEach(async () => {
  document.body.innerHTML = "";
  await page.viewport(1280, 800);
});

for (const theme of ["light", "dark"] as const) {
  describe(`the site header, ${theme}`, () => {
    it("tabs through the wordmark, then the links in §7.39's order, then the site's tools", async () => {
      mount(HEADERS.docs, theme);
      // Start from a control just before the header: from nowhere, Firefox's first Tab can stop on
      // the document itself, which would shift every step after it.
      const start = document.createElement("button");
      start.textContent = "Start";
      document.body.prepend(start);
      start.focus();
      const order: string[] = [];
      for (let i = 0; i < 6; i += 1) {
        await tabForward();
        order.push(document.activeElement?.textContent?.trim() ?? "");
      }
      expect(order).toEqual(["Lamina PDF", "Download", "Docs", "How we test", "Privacy", "Search"]);
    });

    it("draws the current page in 600 with the accent underline, and the others plain", () => {
      const header = mount(HEADERS.website, theme);
      const link = (label: string) =>
        [...header.querySelectorAll<HTMLAnchorElement>(".site-nav a")].find((a) => a.textContent?.trim() === label);
      const current = getComputedStyle(link("Privacy") ?? header);
      expect(link("Privacy")?.getAttribute("aria-current")).toBe("page");
      expect(current.fontWeight).toBe("600");
      expect(current.boxShadow).toContain(resolvedColor("var(--paper-accent)"));
      const plain = getComputedStyle(link("Download") ?? header);
      expect(plain.fontWeight).toBe("400");
      expect(plain.boxShadow).toContain(resolvedColor("transparent"));
      expect(link("Download")?.hasAttribute("aria-current")).toBe(false);
    });

    it("fits a phone's width without scrolling sideways", async () => {
      await page.viewport(375, 800);
      mount(HEADERS.website, theme);
      expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(375);
    });

    it("shortens the app link to App and the open-in-new icon on a phone, keeping its name", async () => {
      const app = () => {
        const link = document.querySelector<HTMLAnchorElement>(".site-header-app");
        if (link === null) throw new Error("no app link");
        return link;
      };
      const icon = () => app().querySelector("svg");
      mount(HEADERS.website, theme);
      expect(app().textContent?.trim()).toBe("Open the web app");
      expect(icon() === null ? "none" : getComputedStyle(icon() as SVGElement).display).toBe("none");
      await page.viewport(375, 800);
      // innerText keeps visually hidden text, so check what is drawn: the lead clipped to 1 px,
      // the word capitalised.
      const lead = app().querySelector(".site-header-app-lead")?.getBoundingClientRect();
      expect(lead?.width).toBeLessThanOrEqual(1);
      const word = app().querySelector(".site-header-app-word");
      expect(word === null ? "" : getComputedStyle(word).textTransform).toBe("capitalize");
      expect(getComputedStyle(icon() as SVGElement).display).not.toBe("none");
      await expect.element(page.getByRole("link", { name: "Open the web app" })).toBeVisible();
      const box = app().getBoundingClientRect();
      const iconBox = (icon() as SVGElement).getBoundingClientRect();
      expect(box.right).toBeLessThanOrEqual(375);
      expect(Math.abs(iconBox.top + iconBox.height / 2 - (box.top + box.height / 2))).toBeLessThan(2);
    });

    it("keeps the wordmark, Docs, App and a menu of the other links on one line on a phone", async () => {
      const header = mount(HEADERS.website, theme);
      wireMenus(document);
      const shown = () =>
        [...header.querySelectorAll<HTMLElement>(".site-wordmark, .site-nav a, .site-header-app, .site-menu > summary")]
          .filter((element) => element.getClientRects().length > 0)
          .map((element) => element.getAttribute("aria-label") ?? element.textContent?.trim());
      const menu = header.querySelector<HTMLDetailsElement>("details.site-menu");
      if (menu === null) throw new Error("no menu");
      expect(menu.getClientRects().length).toBe(0);
      for (const width of [320, 375]) {
        await page.viewport(width, 800);
        expect(shown()).toEqual(["Lamina PDF", "Docs", "Open the web app", "More pages"]);
        const tops = [...header.querySelectorAll(".site-wordmark, .site-header-app, .site-menu")].map((element) =>
          element.getBoundingClientRect(),
        );
        expect(Math.max(...tops.map((box) => box.bottom)) - Math.min(...tops.map((box) => box.top))).toBeLessThan(64);
        expect(Math.max(...tops.map((box) => box.right))).toBeLessThanOrEqual(width);
        expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(width);
      }
      const summary = menu.querySelector("summary");
      if (summary === null) throw new Error("no menu button");
      await userEvent.click(summary);
      expect(menu.open).toBe(true);
      const panel = page.getByRole("navigation", { name: "More pages" });
      await expect.element(panel.getByRole("link", { name: "How we test" })).toBeVisible();
      await expect.element(panel.getByRole("link", { name: "Privacy" })).toHaveAttribute("aria-current", "page");
      await userEvent.keyboard("{Escape}");
      expect(menu.open).toBe(false);
      expect(document.activeElement).toBe(summary);
    });

    it("has no axe violation on either site", async () => {
      for (const html of [HEADERS.website, HEADERS.docs]) {
        mount(html, theme);
        const results = await axe.run(document.body, { rules: { region: { enabled: false } } });
        expect(results.violations.map((v) => `${v.id}: ${v.nodes.length}`)).toEqual([]);
      }
    });
  });
}
