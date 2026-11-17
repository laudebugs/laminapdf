// Organise mode in the built reader (todo E2h, Paper O06), in this run's browser: the flowing report's
// thirteen pages in the grid, then a drag (Chromium, whose drags Playwright drives natively), a
// keyboard move, Move to page…, R to turn a page, Duplicate and Reverse from More page actions,
// Delete, and Insert ▾ › Blank pages…, each one undo step. Saved (Save downloads where the browser
// can't write files) and read back by Lamina PDF's SDK, the file's pages are the grid's, in its
// order, with the turned page at 90°. No CSP violation or other origin throughout.

import { readFileSync } from "node:fs";
import { Builder, Document, init } from "@laminapdf/sdk";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious } from "./reader.ts";

const origin = inject("origin");
const FILE = new URL("../../../fixtures/golden/examples/flowing_report.pdf", import.meta.url);
const SIGNED = new URL("../../../fixtures/signed/approval.pdf", import.meta.url);
const CERTIFIED = new URL("../../../fixtures/signed/certified-p1.pdf", import.meta.url);
const CERTIFIED_OUTLINE = new URL("../../../fixtures/signed/certified-p2-outline.pdf", import.meta.url);
const SIGNED_PAGE_2 = new URL("../../../fixtures/signed/approval-organised.pdf", import.meta.url);
const USAGE_RIGHTS = new URL("../../../fixtures/signed/usage-rights.pdf", import.meta.url);

/** No pickers: Save becomes Download, in every engine, so the saved bytes are the download's. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/**
 * Where a tile is pressed: off its centre, since React Aria takes a press at an item's exact centre
 * for a screen reader's (TalkBack's) and starts a keyboard drag.
 */
const OFF_CENTRE = { x: 20, y: 30 };

/** A file of `count` A4 pages, each with a shape that moves from page to page. */
function manyPages(count: number): Buffer {
  using b = new Builder();
  for (let i = 0; i < count; i++) {
    const page = b.page("A4");
    page
      .surface()
      .fillColor([(i % 10) / 10, 0.4, 0.6])
      .path((p) => p.rect([56 + (i % 20) * 20, 56 + (i % 30) * 20, 160, 100]))
      .fill();
    page.finish();
  }
  using doc = b.finish();
  return Buffer.from(doc.save());
}

/** The grid's page IDs, in order. */
function order(page: Page): Promise<string[]> {
  return page
    .locator(".paper-page-tile")
    .evaluateAll((tiles) => tiles.map((t) => (t as HTMLElement).dataset.pageId ?? ""));
}

/** Waits until the top bar counts `count` unsaved changes. */
async function unsaved(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe(text);
}

/** Opens `file` in the reader and switches to Organise. */
async function organise(page: Page, file: URL): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Organise", exact: true }).click();
  await page.locator(".paper-page-tile").first().waitFor();
}

/** Selects the tile at `index` alone. */
async function pick(page: Page, index: number): Promise<void> {
  await page.locator(".paper-page-tile").nth(index).click({ position: OFF_CENTRE });
}

/** Drags the tile at `from` past the right edge of the tile at `to`, as a mouse does. */
async function drag(page: Page, from: number, to: number): Promise<void> {
  const a = await page.locator(".paper-page-tile").nth(from).boundingBox();
  const b = await page.locator(".paper-page-tile").nth(to).boundingBox();
  if (a === null || b === null) throw new Error("a tile isn't shown");
  await page.mouse.move(a.x + OFF_CENTRE.x, a.y + OFF_CENTRE.y);
  await page.mouse.down();
  await page.mouse.move(a.x + OFF_CENTRE.x + 10, a.y + OFF_CENTRE.y, { steps: 4 });
  await page.mouse.move(b.x + b.width - 6, b.y + b.height / 2, { steps: 12 });
  await page.mouse.up();
}

/** `ids` with the item at `from` taken out and put back at `to`. */
function moved(ids: readonly string[], from: number, to: number): string[] {
  const next = [...ids];
  const [id] = next.splice(from, 1);
  next.splice(to, 0, id as string);
  return next;
}

describe(`organising pages in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init({ extract: true });
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap C.01 web round-trip] [cap C.03 web round-trip] [cap C.04 web round-trip] [cap C.08 web round-trip] [cap C.13 web round-trip] moves, turns, duplicates, reverses, deletes and inserts pages", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
    });
    const { page } = watched;
    await organise(page, FILE);
    await page.locator(".paper-page-tile").nth(12).waitFor();
    let ids = await order(page);
    expect(ids).toHaveLength(13);
    let steps = 0;

    // Page 2 goes after page 4: by a drag where Playwright drives one, else by Move to page….
    await pick(page, 1);
    if (BROWSER === "chromium") await drag(page, 1, 3);
    else {
      await page.getByRole("button", { name: "Move to page…" }).click();
      const dialog = page.getByRole("dialog");
      await dialog.getByLabel("Page", { exact: true }).fill("4");
      await expect.poll(() => dialog.getByText("New order: 1, 3, 4, 2,").count()).toBe(1);
      await dialog.getByRole("button", { name: "Move", exact: true }).click();
    }
    ids = moved(ids, 1, 3);
    await unsaved(page, ++steps);
    await expect.poll(() => order(page)).toEqual(ids);

    // The moved page goes one back with the keyboard, then to the start with Move to page…. Its
    // tile takes focus first: after Move to page…, focus is back on the dialog's button.
    await pick(page, 3);
    await page.keyboard.press("ControlOrMeta+Shift+ArrowLeft");
    ids = moved(ids, 3, 2);
    await unsaved(page, ++steps);
    await expect.poll(() => order(page)).toEqual(ids);
    await page.getByRole("button", { name: "Move to page…" }).click();
    const move = page.getByRole("dialog");
    await move.getByText("At the start").click();
    await move.getByRole("button", { name: "Move", exact: true }).click();
    ids = moved(ids, 2, 0);
    await unsaved(page, ++steps);
    await expect.poll(() => order(page)).toEqual(ids);
    await expect.poll(() => page.getByRole("status").filter({ hasText: "to the start" }).count()).toBe(1);

    // R turns the selected page a quarter clockwise (the dialog gave focus back to its button).
    const turned = ids[0] as string;
    await pick(page, 0);
    await page.keyboard.press("r");
    await unsaved(page, ++steps);

    // Duplicate puts a copy after the page; Delete takes the copy away again.
    await page.getByRole("button", { name: "More page actions" }).click();
    await page.getByRole("menuitem", { name: /Duplicate/ }).click();
    await unsaved(page, ++steps);
    await expect.poll(async () => (await order(page)).length).toBe(14);
    await pick(page, 1);
    await page.keyboard.press("Delete");
    await unsaved(page, ++steps);
    await expect.poll(() => order(page)).toEqual(ids);

    // Two blank US Letter pages after page 1, then every page in reverse.
    await pick(page, 0);
    await page.getByRole("button", { name: "Insert", exact: true }).click();
    await page.getByRole("menuitem", { name: /Insert blank pages/ }).click();
    const blank = page.getByRole("dialog");
    await blank.getByLabel("Number of pages").fill("2");
    await blank.getByText("US Letter").click();
    await blank.getByRole("button", { name: "Insert", exact: true }).click();
    await unsaved(page, ++steps);
    await expect.poll(async () => (await order(page)).length).toBe(15);
    const inserted = (await order(page)).slice(1, 3);
    expect(inserted.some((id) => ids.includes(id))).toBe(false);
    await page.getByRole("button", { name: "More page actions" }).click();
    await page.getByRole("menuitem", { name: /Select all/ }).click();
    await page.getByRole("button", { name: "More page actions" }).click();
    await page.getByRole("menuitem", { name: /Reverse/ }).click();
    await unsaved(page, ++steps);
    const final = [ids[0] as string, ...inserted, ...ids.slice(1)].reverse();
    await expect.poll(() => order(page)).toEqual(final);

    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const saved = new Uint8Array(readFileSync(await (await download).path()));
    using doc = Document.open(saved);
    const pages = doc.pages;
    expect(pages.map((p) => p.id)).toEqual(final);
    expect(pages.find((p) => p.id === turned)?.rotation).toBe(90);
    for (const id of inserted) expect(pages.find((p) => p.id === id)).toMatchObject({ width: 612, height: 792 });
    await clean(watched);
  });

  it("asks before turning a signed page, and says a certified file's pages can't change", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
    });
    const { page } = watched;
    await organise(page, SIGNED);
    await pick(page, 0);
    await page.keyboard.press("r");
    const ask = page.getByRole("alertdialog");
    await expect.poll(() => ask.getByText("Page 1 holds a signature").count()).toBe(1);
    await ask.getByRole("button", { name: "Cancel" }).click();
    await expect.poll(() => ask.count()).toBe(0);
    expect(await page.locator(".paper-top-bar-save-state").textContent()).not.toMatch(/unsaved/);

    await clean(watched);

    const again = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
    });
    await organise(again.page, CERTIFIED);
    await pick(again.page, 0);
    await again.page.keyboard.press("r");
    const notice = again.page.getByRole("alert").filter({ hasText: "would invalidate the certification" });
    await expect.poll(() => notice.count()).toBe(1);
    expect(await again.page.locator(".paper-top-bar-save-state").textContent()).not.toMatch(/unsaved/);
    await clean(again);
  });

  // A signed file under a page op: the verdict before it, Apply anyway…, and the save that removes what it breaks.
  it("refuses a move on a certified file until Apply anyway…, then saves it without the certification", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
    });
    const { page } = watched;
    await organise(page, CERTIFIED_OUTLINE);
    const before = await order(page);
    await pick(page, 2);
    await page.keyboard.press("ControlOrMeta+Shift+ArrowLeft");
    const notice = page.getByRole("alert").filter({ hasText: "would invalidate the certification" });
    await expect.poll(() => notice.count()).toBe(1);
    expect(await order(page)).toEqual(before);
    expect(await page.locator(".paper-top-bar-save-state").textContent()).not.toMatch(/unsaved/);

    await notice.getByRole("button", { name: "Apply anyway…" }).click();
    await unsaved(page, 1);
    const moved = [before[0], before[2], before[1]];
    await expect.poll(() => order(page)).toEqual(moved);
    await page.getByText("Saving will invalidate the certification.").waitFor();

    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const ask = page.getByRole("alertdialog", { name: "Remove the certification and save?" });
    await ask.getByRole("button", { name: "Remove signature and save" }).click();
    using doc = Document.open(new Uint8Array(readFileSync(await (await download).path())));
    expect({
      order: doc.pages.map((p) => p.id),
      signatures: doc.plan().signatures.length,
      bookmarks: doc.outline().length,
    }).toEqual({ order: moved, signatures: 0, bookmarks: 3 });
    await clean(watched);
  });

  it("moves an approval-signed file's signed page without asking, and keeps the signature", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
    });
    const { page } = watched;
    await organise(page, SIGNED_PAGE_2);
    await pick(page, 1);
    await page.keyboard.press("ControlOrMeta+Shift+ArrowRight");
    await unsaved(page, 1);
    expect(await page.getByRole("alertdialog").count()).toBe(0);
    await page
      .getByText("Signed file. Saving adds your changes after the signature; the signature stays valid.")
      .waitFor();
    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const saved = new Uint8Array(readFileSync(await (await download).path()));
    const original = new Uint8Array(readFileSync(SIGNED_PAGE_2));
    using doc = Document.open(saved);
    expect({
      afterTheSignedBytes: original.every((b, i) => saved[i] === b),
      signatures: doc.plan().signatures.length,
    }).toEqual({ afterTheSignedBytes: true, signatures: 1 });
    await clean(watched);
  });

  it("says turning a page would invalidate a file's usage rights", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
    });
    const { page } = watched;
    await organise(page, USAGE_RIGHTS);
    await pick(page, 0);
    await page.keyboard.press("r");
    const notice = page.getByRole("alert").filter({ hasText: "would invalidate the usage rights" });
    await expect.poll(() => notice.count()).toBe(1);
    expect(await page.getByRole("alertdialog").count()).toBe(0);
    await clean(watched);
  });

  it("reorders with the keyboard alone, and passes axe in the grid and its dialogs", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
      bypassCSP: true,
    });
    const { page } = watched;
    await organise(page, FILE);
    const before = await order(page);
    expect(await serious(page)).toEqual([]);

    // Focus goes into the grid on page 1; the arrow keys choose page 3 and ⇧ + the arrows widen
    // the choice to page 4; ⌘⇧→ (Ctrl+Shift+→) moves both one on.
    await page.locator(".paper-page-tile").first().focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
    await expect.poll(() => page.locator(".paper-organise-note").textContent()).toBe("2 pages selected");
    await page.keyboard.press("ControlOrMeta+Shift+ArrowRight");
    await unsaved(page, 1);
    const after = [...before.slice(0, 2), before[4], before[2], before[3], ...before.slice(5)];
    await expect.poll(() => order(page)).toEqual(after);
    // Focus stays with the moved pages, so the next move takes them on again.
    await page.keyboard.press("ControlOrMeta+Shift+ArrowLeft");
    await unsaved(page, 2);
    await expect.poll(() => order(page)).toEqual(before);
    expect(await serious(page)).toEqual([]);

    await page.keyboard.press("Shift+F10");
    await page.getByRole("menu").waitFor();
    // A menu item's shortcut chip describes it, which axe's experimental
    // label-content-name-mismatch doesn't allow for (as in a11y.e2e.test.ts's palette).
    expect(await serious(page, ["label-content-name-mismatch"])).toEqual([]);
    await page.keyboard.press("Escape");
    // Enter on one of the menu's items runs it (it isn't the grid's Enter, which opens a page).
    await page.keyboard.press("Shift+F10");
    await page.getByRole("menuitem", { name: "Insert blank page after" }).waitFor();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await unsaved(page, 3);
    await expect.poll(async () => (await order(page)).length).toBe(before.length + 1);
    expect(await page.locator(".paper-organise-mode").count()).toBe(1);
    await page.getByRole("button", { name: "Move to page…" }).click();
    await page.getByRole("dialog").waitFor();
    expect(await serious(page)).toEqual([]);
    await page.keyboard.press("Escape");
    await clean(watched);
  });

  // The C3 harness's rule (web-app §4): no task over 50 ms while the page answers a gesture, here
  // from the drag's start to its drop. The edit's answer, which re-renders the reader around the
  // grid, gets twice that, for slower machines than the one the 50 ms was measured on. Long tasks
  // are Chromium's alone.
  it.skipIf(BROWSER !== "chromium")("drags a page across 200 thumbnails with no long task", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
    });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles({ name: "200-pages.pdf", mimeType: "application/pdf", buffer: manyPages(200) });
    await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
    await page.getByRole("button", { name: "Organise", exact: true }).click();
    // Only the tiles near the view are in the page (the grid is virtualized).
    await page.locator(".paper-page-tile").nth(8).waitFor();
    const before = await order(page);
    await pick(page, 0);
    // Let the first thumbnails draw, then watch for long tasks through the drag and its edit.
    await page.waitForTimeout(1500);
    await page.evaluate(() => {
      const log: { tasks: [number, number][]; drag: number[] } = { tasks: [], drag: [] };
      (window as unknown as { __long: typeof log }).__long = log;
      new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) log.tasks.push([entry.startTime, entry.duration]);
      }).observe({ type: "longtask" });
      for (const type of ["dragstart", "drop"])
        document.addEventListener(type, () => log.drag.push(performance.now()), true);
    });
    await drag(page, 0, 8);
    await unsaved(page, 1);
    await expect.poll(async () => (await order(page))[8]).toBe(before[0]);
    await page.waitForTimeout(500);
    const {
      tasks,
      drag: [start = 0, drop = Number.POSITIVE_INFINITY],
    } = await page.evaluate(
      () => (window as unknown as { __long: { tasks: [number, number][]; drag: number[] } }).__long,
    );
    const during = tasks.filter(([at, length]) => at + length > start && at < drop);
    expect(during).toEqual([]);
    expect(tasks.filter(([, length]) => length > 100)).toEqual([]);
    await clean(watched);
  });
});
