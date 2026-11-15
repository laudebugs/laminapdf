// The cursor rule over the built reader (motion plan §3, 1 and §6; todo U7d), in this run's browser:
// in each main state (home, an open document with its Pages and Outline tabs, the More menu, View
// settings, Settings, Keyboard shortcuts, Organise's page grid, the password dialog, and a phone's
// width with its bottom bar) every control on screen shows the hand, a disabled one the arrow and a
// pending one the busy arrow, read where a pointer over the control's middle lands
// (@laminapdf/test-utils/cursors). Over page text the I-beam shows while text can be selected, and
// the Link tool's crosshair wins over it.

import { cursorCheck } from "@laminapdf/test-utils/cursors";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const TAGGED = new URL("../../../fixtures/golden/tagged-report.pdf", import.meta.url);
/** The reader's controls that are dragged, not pressed, and show `grab`: the sheet's grabber, a bookmark's grip. */
const DRAG_HANDLES = ".paper-sheet-grabber, .paper-outline-grip";
const LOCKED = new URL("../../../fixtures/encrypted/r4-aes-128-user.pdf", import.meta.url);

/** The controls in `state` whose cursor breaks the rule; fails if the state shows too few to mean much. */
async function wrong(page: Page, state: string, least: number): Promise<string[]> {
  const { checked, problems } = await page.evaluate(cursorCheck, DRAG_HANDLES);
  expect(checked, `controls on screen in ${state}`).toBeGreaterThanOrEqual(least);
  return problems.map((p) => `${state}: ${p.element} shows ${p.cursor}, not ${p.expected}`);
}

/** The cursor a pointer over the middle of the first page text matching `text` shows. */
async function cursorOverText(page: Page, text: string): Promise<string> {
  const span = page.locator(".lamina-text-layer span", { hasText: text }).first();
  await span.waitFor({ state: "attached" });
  return span.evaluate((element) => {
    const box = element.getBoundingClientRect();
    const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
    return hit === null ? "" : getComputedStyle(hit).cursor;
  });
}

describe(`cursors in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("shows the hand on every control in every main state, the arrow when disabled", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
    });
    const { page } = watched;
    const found: string[] = [];
    found.push(...(await wrong(page, "home", 2)));
    await chooseFile(watched, "outline.pdf");
    await page.locator(".web-viewer canvas").first().waitFor();
    // The file asks for its outline at open, which opens the rail; the sidebar button otherwise.
    const pages = page.getByRole("tab", { name: "Pages" });
    await pages.waitFor({ timeout: 5000 }).catch(() => page.getByRole("button", { name: "Sidebar" }).click());
    await page.getByRole("tab", { name: "Pages" }).click();
    await page.locator(".lamina-thumbnail").first().waitFor();
    found.push(...(await wrong(page, "pages", 15)));
    await page.getByRole("tab", { name: "Outline" }).click();
    await page.getByRole("row", { name: /Appendix/ }).waitFor();
    found.push(...(await wrong(page, "outline", 15)));
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menu").waitFor();
    found.push(...(await wrong(page, "more", 5)));
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "View settings" }).click();
    await page.getByRole("dialog", { name: "View" }).waitFor();
    found.push(...(await wrong(page, "view", 5)));
    await page.getByRole("button", { name: "More settings…" }).click();
    await page.getByRole("dialog", { name: "Settings" }).waitFor();
    found.push(...(await wrong(page, "settings", 5)));
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: "Keyboard shortcuts" }).click();
    await page.getByRole("dialog", { name: "Keyboard shortcuts" }).waitFor();
    found.push(...(await wrong(page, "shortcuts", 1)));
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Organise", exact: true }).click();
    await page.locator(".paper-page-tile").first().waitFor();
    found.push(...(await wrong(page, "organise", 5)));
    expect(found).toEqual([]);
    await clean(watched);
  });

  it("shows the hand on the password dialog's controls", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
    });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(LOCKED.pathname);
    await page.getByRole("dialog", { name: "Enter the password" }).waitFor();
    await page.locator(".paper-password-show").waitFor();
    expect(await wrong(page, "password", 3)).toEqual([]);
    await clean(watched);
  });

  it("shows the hand on every control at a phone's width, the bottom bar's included", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 390, height: 844 },
    });
    const { page } = watched;
    const found = await wrong(page, "phone home", 2);
    await chooseFile(watched, "outline.pdf");
    await page.locator(".web-viewer canvas").first().waitFor();
    await page.locator(".paper-bottom-bar-button").first().waitFor();
    found.push(...(await wrong(page, "phone", 5)));
    await page.getByRole("button", { name: "More" }).last().click();
    await page.getByRole("dialog").waitFor();
    found.push(...(await wrong(page, "phone more", 3)));
    expect(found).toEqual([]);
    await clean(watched);
  });

  it("shows the I-beam over page text, and the Link tool's crosshair over it while that tool is on", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
    });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(TAGGED.pathname);
    await page.locator(".web-viewer canvas").first().waitFor();
    expect(await cursorOverText(page, "Widgets")).toBe("text");
    const tool = page.getByRole("button", { name: "Link", exact: true });
    await tool.click();
    await expect.poll(() => cursorOverText(page, "Widgets")).toBe("crosshair");
    await tool.click();
    await expect.poll(() => cursorOverText(page, "Widgets")).toBe("text");
    await clean(watched);
  });
});
