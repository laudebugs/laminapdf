// The Organise mode in the desktop window (todo E2n, Paper O06, O09 and O10), as a person uses it:
// the page grid's moves, turns, duplicates, deletes, blank pages and reversal; Page setup's crop,
// resize, trim box, landscape and labels; each saved in place, then read back from the file by
// pdf.js and, for the boxes, by the engine once the file is open again. On signed files, a change
// to a signed page asks first, and one a certification forbids shows the banner; neither changes
// anything until the person says so.
import { copyFileSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { platform } from "node:process";
import { $, $$, browser, expect } from "@wdio/globals";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import { Key } from "webdriverio";
import { fixture, invoke, menuClick, openAndWait } from "../app.ts";
import { workingCopies } from "../paths.ts";

const dir = join(workingCopies, "organise");
const MOD = platform === "darwin" ? Key.Command : Key.Ctrl;
/** The grid deletes with Backspace on macOS, Delete elsewhere, as its menu says. */
const DELETE = platform === "darwin" ? Key.Backspace : Key.Delete;

/** A fresh copy of fixture `parts`, to edit and save. */
function copyOf(...parts: string[]): string {
  const path = join(dir, parts.at(-1) ?? "copy.pdf");
  copyFileSync(fixture("fixtures", ...parts), path);
  return path;
}

/** What pdf.js reads of each page of `path`: its first line of text, its `/Rotate`, and the labels. */
async function readByPdfJs(path: string) {
  const task = getDocument({ data: new Uint8Array(readFileSync(path)), verbosity: 0 });
  const doc = await task.promise;
  try {
    const pages = [];
    for (let number = 1; number <= doc.numPages; number++) {
      const page = await doc.getPage(number);
      const items = (await page.getTextContent()).items as { str?: string }[];
      const [x0, y0, x1, y1] = page.view;
      pages.push({
        text: items
          .map((i) => i.str ?? "")
          .join("")
          .trim(),
        rotate: page.rotate,
        size: [Math.round((x1 ?? 0) - (x0 ?? 0)), Math.round((y1 ?? 0) - (y0 ?? 0))],
        crop: page.view,
      });
    }
    return { pages, labels: await doc.getPageLabels() };
  } finally {
    await task.destroy();
  }
}

/** Opens `path` and switches to Organise, waiting for its tiles. */
async function organise(path: string, pages: number): Promise<void> {
  await openAndWait(path);
  await $("aria/Organise").click();
  await browser.waitUntil(async () => (await $$(".paper-page-tile").length) === pages, {
    timeoutMsg: `the grid didn't show ${pages} tiles`,
  });
}

/**
 * Selects tile `index` alone, clicked away from its corner badge. A person's click also focuses the
 * tile, so the grid's keys reach it; the driver's click selects without focusing, so the page focuses
 * it as the click would.
 */
async function pick(index: number): Promise<void> {
  const tile = (await $$(".paper-page-tile"))[index];
  if (tile === undefined) throw new Error(`no tile ${index}`);
  await tile.click({ x: -20, y: -30 });
  await browser.execute(
    (i) => globalThis.document.querySelectorAll<HTMLElement>(".paper-page-tile")[i]?.focus(),
    index,
  );
}

/** The tiles' page IDs, in order. */
async function order(): Promise<string[]> {
  return browser.execute(() =>
    [...globalThis.document.querySelectorAll<HTMLElement>(".paper-page-tile")].map((t) => t.dataset.pageId ?? ""),
  );
}

/** The tiles' captions: each page's label or number. */
async function captions(): Promise<string[]> {
  return browser.execute(() =>
    [...globalThis.document.querySelectorAll(".paper-page-tile-number")].map((t) => t.textContent ?? ""),
  );
}

/** How many steps the edit session holds unsaved. */
async function unsaved(): Promise<number> {
  return browser.execute(() => globalThis.__laminaE2e?.unsaved() ?? 0);
}

/** Waits until `count` edits are unsaved. */
async function waitUnsaved(count: number, why: string): Promise<void> {
  await browser.waitUntil(async () => (await unsaved()) === count, { timeoutMsg: `${why}: not ${count} unsaved` });
}

/** Chooses `item` in the menu behind button `menu`. */
async function choose(menu: string, item: string): Promise<void> {
  await $(`aria/${menu}`).click();
  const entry = $(`//*[@role="menuitem"][contains(normalize-space(.), ${JSON.stringify(item)})]`);
  await entry.waitForDisplayed({ timeoutMsg: `no ${item} in ${menu}` });
  await entry.click();
}

/**
 * Chooses `option` in the open dialog's list labelled `label`, a native select. WebKit's driver
 * finds neither the list by its label nor its options, so the page chooses it as a person's choice
 * would: the value, then the change event.
 */
async function chooseIn(label: string, option: string): Promise<void> {
  const chosen = await browser.execute(
    (name, text) => {
      // Named by its aria-label, or by a <label for>.
      const labels = [...globalThis.document.querySelectorAll('[role="dialog"] label')];
      const control = labels.find((l) => l.textContent?.trim() === name)?.getAttribute("for");
      const list =
        globalThis.document.querySelector(`[role="dialog"] select[aria-label="${name}"]`) ??
        (control === null || control === undefined ? null : globalThis.document.getElementById(control));
      if (!(list instanceof HTMLSelectElement)) return false;
      const found = [...list.options].find((o) => o.textContent?.trim() === text);
      if (found === undefined) return false;
      list.value = found.value;
      list.dispatchEvent(new Event("change", { bubbles: true }));
      return true;
    },
    label,
    option,
  );
  if (!chosen) throw new Error(`no ${option} in the list ${label}`);
}

/** The open dialog headed `name`. */
async function dialog(name: string) {
  const found = $(`//*[@role="dialog"][.//*[self::h1 or self::h2][normalize-space(.)=${JSON.stringify(name)}]]`);
  await found.waitForDisplayed({ timeoutMsg: `the ${name} dialog didn't open` });
  return found;
}

/** Saves in place and waits until nothing is unsaved. */
async function save(file: string): Promise<void> {
  await menuClick("cmd:file.save");
  await browser.waitUntil(async () => (await browser.getTitle()) === `${file} — Lamina PDF`, {
    timeout: 15_000,
    timeoutMsg: `${file} wasn't saved`,
  });
}

/**
 * Closes every tab, answering Close without saving for one with edits, so each scenario starts at
 * the start screen: the run closes the tabs before a file's first scenario only.
 */
async function closeEveryTab(): Promise<void> {
  await browser.keys(Key.Escape);
  for (let tries = 0; tries < 20; tries++) {
    if (await $("button=Choose a file").isExisting()) return;
    await menuClick("cmd:tab.close");
    const ask = $("[role=alertdialog]");
    if (await ask.isExisting()) await ask.$("button=Close without saving").click();
    await browser.pause(150);
  }
  throw new Error("the window didn't go back to its start screen");
}

describe("the Organise mode", () => {
  before(() => {
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
  });

  afterEach(closeEveryTab);

  it("[cap C.01 desktop round-trip] [cap C.03 desktop round-trip] [cap C.04 desktop round-trip] [cap C.08 desktop round-trip] [cap C.13 desktop round-trip] moves, turns, duplicates, deletes, inserts and reverses pages, saved in place", async () => {
    const path = copyOf("golden", "examples", "flowing_report.pdf");
    const before = await readByPdfJs(path);
    await organise(path, 13);
    const ids = await order();

    // Turn page 1 a quarter clockwise.
    await pick(0);
    await browser.keys("r");
    await waitUnsaved(1, "the turn");

    // Duplicate it, then delete the copy.
    await choose("More page actions", "Duplicate");
    await browser.waitUntil(async () => (await order()).length === 14, { timeoutMsg: "no copy of page 1" });
    await pick(1);
    await browser.keys(DELETE);
    await browser.waitUntil(async () => (await order()).length === 13, { timeoutMsg: "the copy wasn't deleted" });

    // A blank page at the end, then page 1 one place later.
    await choose("Insert", "Insert blank pages");
    await (await dialog("Insert blank pages")).$("button=Insert").click();
    await browser.waitUntil(async () => (await order()).length === 14, { timeoutMsg: "no blank page" });
    await pick(0);
    await browser.keys([MOD, Key.Shift, Key.ArrowRight]);
    await browser.waitUntil(async () => (await order())[1] === ids[0], { timeoutMsg: "page 1 didn't move" });

    // Every page reversed.
    await choose("More page actions", "Select all");
    await choose("More page actions", "Reverse");
    await browser.waitUntil(async () => (await order())[13] === ids[1], { timeoutMsg: "the pages weren't reversed" });
    await waitUnsaved(6, "the six steps");

    await save("flowing_report.pdf");
    const after = await readByPdfJs(path);
    // Blank, pages 13 to 3, page 1 turned, page 2.
    expect(after.pages.map((p) => p.text)).toEqual([
      "",
      ...before.pages
        .slice(2)
        .reverse()
        .map((p) => p.text),
      before.pages[0]?.text,
      before.pages[1]?.text,
    ]);
    expect(after.pages.map((p) => p.rotate)).toEqual([...Array(12).fill(0), 90, 0]);
  });

  it("[cap C.09 desktop round-trip] [cap C.10 desktop round-trip] [cap C.11 desktop round-trip] [cap C.12 desktop round-trip] [cap I.10 desktop round-trip] crops, resizes, sets boxes, turns and labels pages through Page setup, saved in place", async () => {
    const path = copyOf("golden", "examples", "merge_documents.pdf");
    await organise(path, 6);

    // Crop page 1 from the top.
    await pick(0);
    await choose("Page setup", "Crop pages…");
    const crop = await dialog("Crop pages");
    await crop.$("aria/Top").setValue("1");
    await crop.$("button=Crop page").click();
    await waitUnsaved(1, "the crop");

    // Resize page 2 to A4.
    await pick(1);
    await choose("Page setup", "Resize pages…");
    const resize = await dialog("Resize pages");
    await chooseIn("New size", "A4");
    await resize.$("button=Resize page").click();
    await waitUnsaved(2, "the resize");

    // Page 3's own trim box, half an inch in from each edge.
    await pick(2);
    await choose("Page setup", "Page boxes…");
    const boxes = await dialog("Page boxes");
    // WebKit's driver can't click the hidden checkbox, so the page clicks it, as a person's click would.
    await browser.execute(() => {
      const trim = [...globalThis.document.querySelectorAll('[role="dialog"] fieldset')].find(
        (set) => set.querySelector("legend")?.textContent === "Trim box",
      );
      trim?.querySelector<HTMLInputElement>('input[type="checkbox"]')?.click();
    });
    for (const [side, value] of [
      ["left", "36"],
      ["bottom", "36"],
      ["right", "576"],
      ["top", "756"],
    ] as const) {
      await boxes.$(`aria/Trim box, ${side}`).setValue(value);
    }
    await boxes.$("button=Apply boxes").click();
    await waitUnsaved(3, "the trim box");

    // Page 4 landscape.
    await pick(3);
    await choose("Page setup", "Make landscape");
    await waitUnsaved(4, "the turn to landscape");

    // Labels: i, ii, then 1, 2, 3 from page 3, shown on the tiles at once.
    await choose("Page setup", "Page labels…");
    const labels = await dialog("Page labels");
    await chooseIn("Range 1, style", "i, ii, iii");
    await labels.$("button=Add range").click();
    await labels.$("aria/Range 2, from page").setValue("3");
    await labels.$("button=Apply labels").click();
    await waitUnsaved(5, "the labels");
    await browser.waitUntil(async () => (await captions()).join(" ") === "i ii 1 2 3 4", {
      timeoutMsg: "the tiles don't show the new labels",
    });

    await save("merge_documents.pdf");
    const saved = await readByPdfJs(path);
    expect(saved.labels).toEqual(["i", "ii", "1", "2", "3", "4"]);
    expect(saved.pages[0]?.crop[3]).toBeLessThan(792 - 1);
    expect(saved.pages[1]?.size).toEqual([595, 842]);
    // pdf.js gives the box before /Rotate: turned a quarter, the portrait paper shows landscape.
    const [paperWidth = 0, paperHeight = 0] = saved.pages[3]?.size ?? [];
    expect([saved.pages[3]?.rotate, saved.pages[3]?.rotate === 90 ? paperHeight > paperWidth : false]).toEqual([
      90,
      true,
    ]);

    // The trim box, read by the engine from the saved file, opened again.
    await menuClick("cmd:tab.close");
    await $("button=Choose a file").waitForExist();
    await openAndWait(path);
    const doc = await browser.execute(() => globalThis.__laminaE2e?.doc());
    const read = JSON.parse((await invoke("page_boxes", { doc, index: 2 })) as string) as {
      trim: number[];
      own: { trim: boolean };
    };
    expect([read.trim, read.own.trim]).toEqual([[36, 36, 576, 756], true]);
  });

  it("asks before turning a signed page, and changes nothing when the person cancels", async () => {
    const path = copyOf("signed", "approval.pdf");
    await organise(path, 1);
    await pick(0);
    await browser.keys("r");
    const ask = $('//*[@role="alertdialog"]');
    await ask.waitForDisplayed({ timeoutMsg: "no question about the signed page" });
    await ask.$("button=Cancel").click();
    await ask.waitForDisplayed({ reverse: true });
    expect(await unsaved()).toBe(0);
  });

  it("shows the banner when a change would invalidate the certification, and changes nothing until Apply anyway…", async () => {
    const path = copyOf("signed", "certified-p1.pdf");
    await organise(path, 1);
    await pick(0);
    await browser.keys("r");
    const banner = $('//*[@role="alert"][contains(., "would invalidate the certification")]');
    await banner.waitForDisplayed({ timeoutMsg: "no certification banner" });
    expect(await unsaved()).toBe(0);
    // No Save a copy on the desktop yet: the banner offers what the window can do.
    expect(await banner.$("button=Save a copy…").isExisting()).toBe(false);
    await banner.$("button=Apply anyway…").click();
    await waitUnsaved(1, "Apply anyway…");
  });
});
