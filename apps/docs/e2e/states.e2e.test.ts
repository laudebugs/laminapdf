// The docs' interaction states against `wrangler dev` (e2e/server.ts), in one browser per run
// (LAMINA_PDF_E2E_BROWSER), as a reader meets them (motion plan §3, §6): every clickable element
// shows the hand (the arrow when disabled, the busy arrow while it works) on pages of each kind, at
// a desktop's and a phone's width, with search, the theme switch's list, the phone menu and "Edit
// and run" open; search looks disabled until it has loaded; and under reduced motion Starlight's
// sidebar caret turns at once.

import { cursorCheck } from "@laminapdf/test-utils/cursors";
import { BROWSER, launchBrowser } from "@laminapdf/test-utils/e2e-browser";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const DOCS = new URL("../", import.meta.url);
const origin = inject("origin");

/** Pages of each kind: the hero, a page with tabs and an example, a recipe with a runner, a guide,
    generated reference, the code table. */
const PAGES = [
  "/",
  "/start/first-pdf/",
  "/recipes/hello/",
  "/guides/edit-a-pdf/",
  "/reference/sdk/classes/document/",
  "/reference/codes/",
];
const DESKTOP = { width: 1280, height: 800 };
const PHONE = { width: 390, height: 844 };

let browser: Browser;

beforeAll(async () => {
  browser = await launchBrowser(DOCS);
});

afterAll(async () => {
  await browser?.close();
});

/** `path` in a fresh reader at `viewport`, once it has loaded. */
async function open(
  path: string,
  viewport: { width: number; height: number },
  reducedMotion: "reduce" | "no-preference" = "no-preference",
): Promise<Page> {
  // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
  const context = await browser.newContext({ ignoreHTTPSErrors: true, viewport, reducedMotion });
  const page = await context.newPage();
  await page.goto(new URL(path, origin).href);
  await page.waitForLoadState("load");
  return page;
}

/** The page's clickable elements whose cursor breaks the rule (@laminapdf/test-utils/cursors). */
/** The cursor problems on the page; fails when it checked no control, since covered ones are left out. */
async function problems(page: Page) {
  const { checked, problems } = await page.evaluate(cursorCheck, "");
  expect(checked, "controls the cursor check reached").toBeGreaterThan(0);
  return problems;
}

describe(`the docs' cursors in ${BROWSER}`, () => {
  for (const [name, viewport] of [
    ["a desktop", DESKTOP],
    ["a phone", PHONE],
  ] as const) {
    it(`shows the hand on every clickable element, at ${name}'s width`, async () => {
      for (const path of PAGES) {
        const page = await open(path, viewport);
        expect(await problems(page), path).toEqual([]);
        await page.context().close();
      }
    });
  }

  it("shows the hand in search, the theme switch's list and the runner's bar, the busy arrow while it runs", async () => {
    const page = await open("/recipes/hello/", DESKTOP);
    const search = page.locator("button[data-open-modal]").first();
    await expect.poll(() => search.isEnabled()).toBe(true);
    await search.click();
    await page.locator(".pagefind-ui__search-input").first().fill("encrypted");
    await expect.poll(() => page.locator(".pagefind-ui__result").count(), { timeout: 15_000 }).toBeGreaterThan(0);
    expect(await problems(page), "search").toEqual([]);
    await page.keyboard.press("Escape");

    await page.locator(".header .site-menu-select-button").click();
    expect(await problems(page), "the theme switch").toEqual([]);
    await page.keyboard.press("Escape");

    await page
      .getByRole("button", { name: /Edit and run/ })
      .first()
      .click();
    const run = page.getByRole("button", { name: "Run", exact: true });
    await run.waitFor();
    expect(await problems(page), "the runner").toEqual([]);
    await run.click();
    // While it runs, Run is busy: the check expects the busy arrow there.
    expect(await page.locator(".run-run").getAttribute("aria-busy")).toBe("true");
    expect(await problems(page), "a run").toEqual([]);
    await page.context().close();
  });

  it("shows the hand in the phone menu", async () => {
    const page = await open("/guides/edit-a-pdf/", PHONE);
    await page.locator(".sl-menu-button").click();
    await expect.poll(() => page.locator("#starlight__sidebar").isVisible()).toBe(true);
    expect(await problems(page)).toEqual([]);
    await page.context().close();
  });

  it("draws search as disabled, with the arrow, until it has loaded", async () => {
    const page = await open("/install/", DESKTOP);
    const search = page.locator("button[data-open-modal]").first();
    await expect.poll(() => search.isEnabled()).toBe(true);
    const enabled = await search.evaluate((b) => getComputedStyle(b).color);
    // Starlight enables it once its script has run; set back, it looks as it does before then.
    const token = await search.evaluate((b: HTMLButtonElement) => {
      b.disabled = true;
      const probe = document.createElement("span");
      probe.style.color = "var(--paper-disabled)";
      document.body.append(probe);
      return getComputedStyle(probe).color;
    });
    expect(await search.evaluate((b) => getComputedStyle(b).cursor)).toBe("default");
    // The text fades to the disabled colour as a state does: poll until it lands.
    await expect.poll(() => search.evaluate((b) => getComputedStyle(b).color)).toBe(token);
    expect(token).not.toBe(enabled);
    await page.context().close();
  });
});

describe(`the docs under reduced motion in ${BROWSER}`, () => {
  /** The sidebar's first group caret's turn just after its group is toggled, and once it settles. */
  async function caretTurn(page: Page): Promise<{ first: string; settled: string }> {
    return page.evaluate(async () => {
      const group = document.querySelector<HTMLDetailsElement>(".sidebar-content details");
      const caret = group?.querySelector(".caret");
      if (group == null || caret == null) throw new Error("no sidebar group");
      group.open = !group.open;
      const first = getComputedStyle(caret).transform;
      await new Promise((resolve) => setTimeout(resolve, 400));
      return { first, settled: getComputedStyle(caret).transform };
    });
  }

  it("turns Starlight's sidebar caret at once, where it would otherwise turn over time", async () => {
    const still = await open("/guides/edit-a-pdf/", DESKTOP, "reduce");
    const reduced = await caretTurn(still);
    expect(reduced.first).toBe(reduced.settled);
    await still.context().close();
    // The same toggle without the preference turns over Starlight's 0.2 s, so the check can tell.
    const moving = await open("/guides/edit-a-pdf/", DESKTOP, "no-preference");
    const turning = await caretTurn(moving);
    expect(turning.first).not.toBe(turning.settled);
    await moving.context().close();
  });
});
