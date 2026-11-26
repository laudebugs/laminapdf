// Pages from other files in the desktop window (Paper O07), as a person does them: Insert pages
// from a file, Replace pages and Merge documents, the file chosen in the native dialog (the e2e build queues it),
// previewed, and applied as one step; then saved in place and read back from the file by pdf.js.
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import { Key } from "webdriverio";
import { fixture, invoke, menuClick, openAndWait, queuePick } from "../app.ts";
import { workingCopies } from "../paths.ts";

const dir = join(workingCopies, "organise-documents");

/** A fresh copy of fixture `parts`, to edit and save. */
function copyOf(...parts: string[]): string {
  const path = join(dir, parts.at(-1) ?? "copy.pdf");
  copyFileSync(fixture("fixtures", ...parts), path);
  return path;
}

/** What pdf.js reads of each page of `path`: its text. */
async function textsOf(path: string): Promise<string[]> {
  const task = getDocument({ data: new Uint8Array(readFileSync(path)), verbosity: 0 });
  const doc = await task.promise;
  try {
    const texts: string[] = [];
    for (let number = 1; number <= doc.numPages; number++) {
      const items = (await (await doc.getPage(number)).getTextContent()).items as { str?: string }[];
      texts.push(
        items
          .map((i) => i.str ?? "")
          .join("")
          .trim(),
      );
    }
    return texts;
  } finally {
    await task.destroy();
  }
}

/** The shown document's page IDs in order, as the engine reports them. */
async function order(): Promise<string[]> {
  return browser.execute(() => [...(globalThis.__laminaE2e?.pageIds() ?? [])]);
}

/** Opens `path` and switches to Organise, waiting for its first tile. */
async function organise(path: string, pages: number): Promise<void> {
  await openAndWait(path);
  await $("aria/Organise").click();
  await $(".paper-page-tile").waitForDisplayed({ timeoutMsg: "the grid didn't show" });
  expect(await order()).toHaveLength(pages);
}

/** Selects page `index`'s tile alone, as `organise.e2e.ts` does. */
async function pick(index: number): Promise<void> {
  const id = (await order())[index];
  if (id === undefined) throw new Error(`no page ${index}`);
  const selector = `.paper-page-tile[data-page-id="${id}"]`;
  await $(selector).click({ x: -20, y: -30 });
  await browser.execute((s) => globalThis.document.querySelector<globalThis.HTMLElement>(s)?.focus(), selector);
}

/** Chooses `item` in the menu behind button `menu`. */
async function choose(menu: string, item: string): Promise<void> {
  await $(`aria/${menu}`).click();
  const entry = $(`//*[@role="menuitem"][contains(normalize-space(.), ${JSON.stringify(item)})]`);
  await entry.waitForDisplayed({ timeoutMsg: `no ${item} in ${menu}` });
  await entry.click();
}

/**
 * Chooses `option` in the open dialog's native list labelled `label`. WebKit's driver finds neither
 * the list by its label nor its options, so the page chooses it as a person's choice would: the
 * value, then the change event.
 */
async function chooseIn(label: string, option: string): Promise<void> {
  const chosen = await browser.execute(
    (name, text) => {
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

/** Saves in place and waits until the title says it is done. */
async function save(file: string): Promise<void> {
  await menuClick("cmd:file.save");
  await browser.waitUntil(async () => (await browser.getTitle()) === `${file} — Lamina PDF`, {
    timeout: 15_000,
    timeoutMsg: `${file} wasn't saved`,
  });
}

/** What the window shows when a scenario is stuck: its pages, unsaved steps, and any message or dialog. */
async function seen(): Promise<string> {
  return browser.execute(() => {
    const texts = [...globalThis.document.querySelectorAll('[role="status"], [role="alert"], [role="dialog"]')].map(
      (e) => (e.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 200),
    );
    return JSON.stringify({
      trace: globalThis.__laminaE2e?.trace(),
      pages: globalThis.__laminaE2e?.pageIds().length,
      unsaved: globalThis.__laminaE2e?.unsaved(),
      texts,
    });
  });
}

/** Closes every tab, answering Close without saving for one with edits. */
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

/**
 * Opens `path` in Organise and Merge documents over it, with hello.pdf then extract_pages.pdf
 * added (the e2e build queues one file per dialog): the dialog and its button, once both are read.
 */
async function openMerge(path: string) {
  await organise(path, 3);
  await choose("More page actions", "Merge documents");
  const merge = await dialog("Merge documents");
  await queuePick(fixture("fixtures", "golden", "examples", "hello.pdf"));
  await merge.$("button=Add files…").click();
  await queuePick(fixture("fixtures", "golden", "examples", "extract_pages.pdf"));
  await merge.$("button=Add files…").click();
  const go = merge.$("button=Merge 3 files");
  await go.waitForEnabled({ timeoutMsg: "the files weren't read" });
  return { merge, go };
}

describe("pages from other files", () => {
  before(() => {
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
  });

  afterEach(closeEveryTab);

  it("[cap C.02 desktop round-trip] inserts a file's pages at the end, saved in place", async () => {
    const path = copyOf("golden", "examples", "insert_pages.pdf");
    await organise(path, 3);
    await queuePick(fixture("fixtures", "golden", "examples", "extract_pages.pdf"));
    await choose("Insert", "Insert pages from a file");
    const insert = await dialog("Insert pages from a file");
    // Its pages are read, every one chosen, and the button says how many go in.
    const go = insert.$("button=Insert 3 pages");
    await go.waitForEnabled({ timeoutMsg: "the file's pages weren't read" });
    await chooseIn("Insert", "At the end");
    await go.click();
    await browser.waitUntil(async () => (await order()).length === 6, { timeoutMsg: "the pages weren't inserted" });
    await save("insert_pages.pdf");
    expect(await textsOf(path)).toEqual(["Page 1", "Page 2", "Page 3", "Page 3", "Page 4", "Page 5"]);
  });

  it("[cap C.16 desktop round-trip] replaces the selected page with a file's page, saved in place", async () => {
    const path = copyOf("golden", "examples", "insert_pages.pdf");
    await organise(path, 3);
    await pick(1);
    await queuePick(fixture("fixtures", "golden", "examples", "hello.pdf"));
    await choose("Insert", "Replace pages");
    const replace = await dialog("Replace pages");
    const go = replace.$("button=Replace 1 page with 1");
    await go.waitForEnabled({ timeoutMsg: "the file's page wasn't read" });
    await go.click();
    await browser.waitUntil(async () => (await browser.execute(() => globalThis.__laminaE2e?.unsaved() ?? 0)) === 1, {
      timeoutMsg: "the replacement wasn't made",
    });
    await save("insert_pages.pdf");
    expect(await textsOf(path)).toEqual(["Page 1", "Hello, world!", "Page 3"]);
  });
});

describe("merging files", () => {
  afterEach(closeEveryTab);

  it("[cap C.06 desktop round-trip] merges two files into the document, in the order chosen, saved in place", async () => {
    const path = copyOf("golden", "examples", "insert_pages.pdf");
    const { go } = await openMerge(path);
    await go.click();
    try {
      await browser.waitUntil(async () => (await order()).length === 7, { timeoutMsg: "the files weren't merged" });
    } catch (error) {
      throw new Error(`${String(error)}: ${await seen()}`);
    }
    await save("insert_pages.pdf");
    expect(await textsOf(path)).toEqual(["Page 1", "Page 2", "Page 3", "Hello, world!", "Page 3", "Page 4", "Page 5"]);
  });

  it("merges two files into a new file the person chooses, leaving the document as it is", async () => {
    const path = copyOf("golden", "examples", "insert_pages.pdf");
    const merged = join(dir, "merged.pdf");
    const { merge, go } = await openMerge(path);
    await merge.$("//label[contains(normalize-space(.), 'Create a new document')]").click();
    await invoke("e2e_queue_save_as", { path: merged });
    await go.click();
    await browser.waitUntil(async () => existsSync(merged), { timeoutMsg: "the new file wasn't written" });
    expect(await textsOf(merged)).toEqual([
      "Page 1",
      "Page 2",
      "Page 3",
      "Hello, world!",
      "Page 3",
      "Page 4",
      "Page 5",
    ]);
    // The document itself has not changed.
    expect([(await order()).length, await browser.execute(() => globalThis.__laminaE2e?.unsaved() ?? 0)]).toEqual([
      3, 0,
    ]);
  });
});
