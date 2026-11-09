// Moving through a document in the built reader (todo C6a), in this run's browser, over the corpus's
// outline.pdf (page labels i, A-1, A-2; a nested outline whose items go to pages 1–3, one by a
// named destination and one to a website): the rail's Pages tab names each thumbnail by its label
// and goes to the page clicked; the Outline tab is a tree that goes to an item by click or keyboard
// and marks the section in view; the page box takes a label or a number and refuses a page that
// isn't there; and the rail passes axe. No CSP violation or other origin throughout.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");

/** The page box's value: the label of the page the viewer shows. */
const shownPage = (page: Page) => page.getByRole("textbox", { name: "Page" }).inputValue();

describe(`moving through a document in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** outline.pdf open in a wide window, with the rail open on `tab`; `bypassCSP` lets axe in. */
  async function withRail(tab: "Pages" | "Outline", bypassCSP = false): Promise<Watched> {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
      bypassCSP,
    });
    await chooseFile(watched, "outline.pdf");
    const { page } = watched;
    // The window takes the file's metadata title.
    await page.locator(".web-viewer canvas").first().waitFor();
    await expect.poll(() => page.title()).toBe("Outline fixture · Lamina PDF");
    await page.getByRole("button", { name: "Sidebar" }).click();
    await page.getByRole("tab", { name: tab }).click();
    return watched;
  }

  it("names each thumbnail by its page label and goes to the page clicked", async () => {
    const watched = await withRail("Pages");
    const { page } = watched;
    const names = page.getByRole("listbox", { name: "Pages" }).getByRole("option");
    await expect
      .poll(() => names.evaluateAll((els) => els.map((el) => el.getAttribute("aria-label"))))
      .toEqual(["Page i (1 of 3)", "Page A-1 (2 of 3)", "Page A-2 (3 of 3)"]);
    await page.getByRole("option", { name: "Page A-2 (3 of 3)" }).click();
    await expect.poll(() => shownPage(page)).toBe("A-2");
    await expect
      .poll(() => page.getByRole("option", { name: "Page A-2 (3 of 3)" }).getAttribute("aria-selected"))
      .toBe("true");
    await clean(watched);
  });

  it("goes to an outline item on a click and with the keyboard", async () => {
    const watched = await withRail("Outline");
    const { page } = watched;
    const tree = page.getByRole("treegrid", { name: "Document outline" });
    await tree.getByRole("row", { name: /Appendix/ }).click();
    await expect.poll(() => shownPage(page)).toBe("A-2");
    await tree.getByRole("row", { name: /Introduction/ }).focus();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect.poll(() => shownPage(page)).toBe("A-1");
    await clean(watched);
  });

  it("marks the outline's section at the top of the view", async () => {
    const watched = await withRail("Outline");
    const { page } = watched;
    const current = () => page.locator("[aria-current=location]").textContent();
    await expect.poll(current).toContain("Introduction");
    const input = page.getByRole("textbox", { name: "Page" });
    await input.fill("A-2");
    await input.press("Enter");
    // Appendix and its child Notes both go to A-2's top: the deepest is current, and shows.
    await expect.poll(current).toContain("Notes");
    await clean(watched);
  });

  it("goes to a page by its label or its number, and refuses one that isn't there", async () => {
    const watched = await withRail("Pages");
    const { page } = watched;
    const input = page.getByRole("textbox", { name: "Page" });
    await input.fill("a-1");
    await input.press("Enter");
    await expect.poll(() => shownPage(page)).toBe("A-1");
    await input.fill("3");
    await input.press("Enter");
    await expect.poll(() => shownPage(page)).toBe("A-2");
    await input.fill("9");
    await input.press("Enter");
    await expect.poll(() => page.getByText("There is no page 9.", { exact: false }).first().isVisible()).toBe(true);
    expect(await shownPage(page)).toBe("A-2");
    await clean(watched);
  });

  it("passes axe with the outline open", async () => {
    const watched = await withRail("Outline", true);
    const { page } = watched;
    await page.getByRole("treegrid", { name: "Document outline" }).waitFor();
    const { default: axe } = await import("axe-core");
    await page.addScriptTag({ content: axe.source });
    const serious = await page.evaluate(async () => {
      const run = (
        window as unknown as { axe: { run: (n: Node) => Promise<{ violations: { id: string; impact: string }[] }> } }
      ).axe;
      const { violations } = await run.run(document.body);
      return violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
    });
    expect(serious).toEqual([]);
    await watched.page.close();
  });
});
