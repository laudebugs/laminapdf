// The site header in a real browser, as both sites render it: links in §7.39's order in the tab
// order, the current page in 600 with the accent underline, no sideways scroll at phone width, and
// no axe violation, in the light and the dark theme.

import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "../../src/header.css";
import axe from "axe-core";
import { afterEach, describe, expect, inject, it } from "vitest";
import { page, server, userEvent } from "vitest/browser";

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
      const order: string[] = [];
      for (let i = 0; i < 5; i += 1) {
        await tabForward();
        order.push(document.activeElement?.textContent?.trim() ?? "");
      }
      expect(order).toEqual(["Lamina PDF", "Download", "Docs", "Privacy", "Search"]);
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

    it("has no axe violation on either site", async () => {
      for (const html of [HEADERS.website, HEADERS.docs]) {
        mount(html, theme);
        const results = await axe.run(document.body, { rules: { region: { enabled: false } } });
        expect(results.violations.map((v) => `${v.id}: ${v.nodes.length}`)).toEqual([]);
      }
    });
  });
}
