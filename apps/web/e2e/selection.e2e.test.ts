// The selection surfaces in the built app (todo U2a), in this run's browser: a mouse selection
// gets the selection toolbar once the mouse is up (never during the drag), beside the selected
// text and never over it; Shift+F10 reaches it and Escape dismisses it; Copy writes the selection
// by paragraph to the clipboard; Search for
// opens find with the selected words; the palette has the selection's commands. On a phone
// profile, a touch selection swaps the bottom bar for the selection's bar, clear of the selected
// text, and no toolbar floats over the page (the OS's own menu goes there).

import { type Browser, type BrowserType, chromium, devices, type Page, webkit } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const FIXTURE = new URL("../../../fixtures/reading-order/two-columns-ordered.pdf", import.meta.url).pathname;

type Rect = { left: number; top: number; right: number; bottom: number };
const overlaps = (a: Rect, b: Rect) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

async function open(watched: Watched): Promise<void> {
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(FIXTURE);
  const spans = page.locator('.lamina-page[data-page-index="0"] .lamina-text-layer span');
  await spans.nth(3).waitFor({ timeout: 30_000 });
  // The view settles (its zoom and first scroll) before a drag starts.
  let last = "";
  await expect
    .poll(
      async () => {
        const now = JSON.stringify([
          await page.locator(".lamina-viewer").evaluate((v) => v.scrollTop),
          await spans.nth(1).boundingBox(),
        ]);
        const still = now === last;
        last = now;
        return still;
      },
      { timeout: 20_000, intervals: [300] },
    )
    .toBe(true);
}

/** Page 1's spans wholly in view, by index, top to bottom. */
function visibleSpans(page: Page): Promise<number[]> {
  return page.evaluate(() => {
    const viewer = (document.querySelector(".lamina-viewer") as HTMLElement).getBoundingClientRect();
    const spans = [...document.querySelectorAll('.lamina-page[data-page-index="0"] .lamina-text-layer span')];
    return spans.flatMap((span, k) => {
      const r = span.getBoundingClientRect();
      const inside =
        r.top > viewer.top + 4 && r.bottom < viewer.bottom - 4 && r.left > viewer.left && r.right < viewer.right;
      return inside ? [k] : [];
    });
  });
}

/** The selection's rectangles now. */
function selectionRects(page: Page): Promise<Rect[]> {
  return page.evaluate(() => {
    const selection = document.getSelection();
    if (selection === null || selection.rangeCount === 0) return [];
    return [...selection.getRangeAt(0).getClientRects()]
      .filter((r) => r.width > 0 && r.height > 0)
      .map((r) => ({ left: r.left, top: r.top, right: r.right, bottom: r.bottom }));
  });
}

/**
 * Drags the real mouse from the `from`th to the `to`th span in view on page 1, checking `during`
 * before releasing.
 */
async function drag(page: Page, from: number, to: number, during?: () => Promise<void>): Promise<void> {
  const visible = await visibleSpans(page);
  const spans = page.locator('.lamina-page[data-page-index="0"] .lamina-text-layer span');
  [from, to] = [visible[from] as number, visible[to] as number];
  const a = (await spans.nth(from).boundingBox()) as { x: number; y: number; width: number; height: number };
  const b = (await spans.nth(to).boundingBox()) as { x: number; y: number; width: number; height: number };
  await page.mouse.move(a.x + 1, a.y + a.height / 2);
  await page.mouse.down();
  await page.mouse.move(b.x + b.width - 1, b.y + b.height / 2, { steps: 8 });
  await during?.();
  await page.mouse.up();
}

describe(`the selection toolbar in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  const start = async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
    });
    await open(watched);
    return watched;
  };

  it("shows after the drag, beside the selection and never over it", async () => {
    const watched = await start();
    const { page } = watched;
    const toolbar = page.getByRole("toolbar", { name: "Text selection" });
    await drag(page, 1, 3, async () => {
      expect(await toolbar.count()).toBe(0);
    });
    await toolbar.waitFor();
    const box = (await toolbar.boundingBox()) as { x: number; y: number; width: number; height: number };
    const bar = { left: box.x, top: box.y, right: box.x + box.width, bottom: box.y + box.height };
    const rects = await selectionRects(page);
    expect(rects.length).toBeGreaterThan(0);
    for (const r of rects) expect(overlaps(bar, r), JSON.stringify({ bar, r })).toBe(false);
    for (const name of ["Copy", "Copy exact", "Search for"]) {
      await toolbar.getByRole("button", { name, exact: true }).waitFor();
    }
    expect(await watched.violations()).toEqual([]);
    await clean(watched);
  });

  it("takes the focus on Shift+F10 and goes on Escape", async () => {
    const watched = await start();
    const { page } = watched;
    await drag(page, 1, 2);
    const toolbar = page.getByRole("toolbar", { name: "Text selection" });
    await toolbar.waitFor();
    await page.keyboard.press("Shift+F10");
    await expect.poll(() => toolbar.evaluate((el) => el.contains(document.activeElement))).toBe(true);
    await page.keyboard.press("Escape");
    await expect.poll(() => toolbar.count()).toBe(0);
    await clean(watched);
  });

  it("copies by paragraph from Copy, with no line break inside a paragraph", async () => {
    const watched = await start();
    const { page } = watched;
    await drag(page, 1, 5);
    // The app's Permissions-Policy keeps the page from reading the clipboard: what the copy wrote
    // is read in the copy event itself, after the viewer's handler.
    await page.evaluate(() => {
      window.addEventListener("copy", (e) => {
        Object.assign(window, { copied: (e as ClipboardEvent).clipboardData?.getData("text/plain") ?? "" });
      });
    });
    const toolbar = page.getByRole("toolbar", { name: "Text selection" });
    await toolbar.getByRole("button", { name: "Copy", exact: true }).click();
    const copied = await page.evaluate(() => (window as unknown as { copied?: string }).copied ?? "");
    const selected = await page.evaluate(() => (document.getSelection()?.toString() ?? "").replace(/\s+/g, " ").trim());
    expect(copied.length).toBeGreaterThan(0);
    for (const paragraph of copied.split("\n\n")) expect(paragraph).not.toContain("\n");
    // The browser's own selection text runs words together across lines; copy puts a space there.
    expect(copied.replace(/\s/g, "")).toBe(selected.replace(/\s/g, ""));
    await clean(watched);
  });

  it("opens find with the selected words from Search for", async () => {
    const watched = await start();
    const { page } = watched;
    await drag(page, 1, 1);
    const toolbar = page.getByRole("toolbar", { name: "Text selection" });
    const selected = await page.evaluate(() => (document.getSelection()?.toString() ?? "").replace(/\s+/g, " ").trim());
    await toolbar.getByRole("button", { name: "Search for" }).click();
    const field = page.getByRole("search").getByRole("textbox");
    await expect.poll(() => field.inputValue()).toBe(selected);
    await clean(watched);
  });

  it("lists the selection's commands in the palette", async () => {
    const watched = await start();
    const { page } = watched;
    await page.locator(".lamina-viewer").focus();
    await page.keyboard.press(BROWSER === "webkit" ? "Meta+K" : "Control+K");
    const palette = page.getByRole("dialog");
    for (const name of ["Copy exact", "Select by column", "Show or hide the selection toolbar"]) {
      await palette.getByRole("combobox").fill(name);
      await palette
        .getByRole("option", { name: new RegExp(name) })
        .first()
        .waitFor();
    }
    await clean(watched);
  });
});

/** The phone profiles each engine has (Firefox has no mobile emulation). */
const PHONES: Record<string, { readonly engine: BrowserType; readonly device: string }> = {
  chromium: { engine: chromium, device: "Pixel 8" },
  webkit: { engine: webkit, device: "iPhone 15" },
};

describe.skipIf(PHONES[BROWSER] === undefined)(`the selection bar on a phone in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    const phone = PHONES[BROWSER] as { readonly engine: BrowserType };
    browser = await phone.engine.launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("replaces the bottom bar while text is selected by touch, clear of the selected text", async () => {
    const phone = PHONES[BROWSER] as { device: string };
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ...devices[phone.device],
      ignoreHTTPSErrors: true,
    });
    await open(watched);
    const { page } = watched;
    // A long press selects in the OS, which emulation can't do: a touch on the text, then the
    // selection a long press would make.
    await page.evaluate(() => {
      const span = document.querySelectorAll(
        '.lamina-page[data-page-index="0"] .lamina-text-layer span',
      )[2] as HTMLElement;
      const r = span.getBoundingClientRect();
      const at = {
        clientX: r.left + 2,
        clientY: r.top + r.height / 2,
        pointerType: "touch",
        pointerId: 3,
        bubbles: true,
      };
      span.dispatchEvent(new PointerEvent("pointerdown", at));
      span.dispatchEvent(new PointerEvent("pointerup", at));
      document.dispatchEvent(new PointerEvent("pointerup", at));
      document.getSelection()?.selectAllChildren(span);
    });
    const bar = page.getByRole("toolbar", { name: "Selection actions" });
    await bar.waitFor();
    expect(await page.getByRole("toolbar", { name: "Document" }).count()).toBe(0);
    expect(await page.getByRole("toolbar", { name: "Text selection" }).count()).toBe(0);
    await bar.getByRole("button", { name: "Copy" }).waitFor();
    const box = (await bar.boundingBox()) as { x: number; y: number; width: number; height: number };
    const area = { left: box.x, top: box.y, right: box.x + box.width, bottom: box.y + box.height };
    for (const r of await selectionRects(page)) expect(overlaps(area, r), JSON.stringify({ area, r })).toBe(false);
    // The selection gone, the bottom bar is back.
    await page.evaluate(() => document.getSelection()?.removeAllRanges());
    await page.getByRole("toolbar", { name: "Document" }).waitFor();
    await clean(watched);
  });
});
