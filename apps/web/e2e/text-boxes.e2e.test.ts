// Text boxes in Comment mode (Paper C03), in this run's browser, typed with the real keyboard: a
// text box, a callout and typewriter text, each in English or Arabic, a word made bold with ⌘B or
// Ctrl B, saved as a download and read back by pdf.js with the same text, its bold kept as rich
// text, and an appearance. Undo after a typing session takes the whole session back in one step,
// and a double-click opens a box for typing again. axe finds no serious violation while a box is
// typed into.

import { readFileSync } from "node:fs";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const FILE = new URL("../../../fixtures/reading-order/two-columns-ordered.pdf", import.meta.url);
const PAGE = '.lamina-page[data-page-index="0"]';
/** ⌘ on macOS, Ctrl elsewhere. */
const MOD = "ControlOrMeta";

/** The reader with `FILE` open in Comment mode, the comments' name already chosen, no file pickers. */
async function comment(): Promise<Watched> {
  const watched = await load(
    browser,
    origin,
    "/",
    {
      script: () => {
        Reflect.deleteProperty(window, "showOpenFilePicker");
        Reflect.deleteProperty(window, "showSaveFilePicker");
        localStorage.setItem("lamina.comment.author", "Ada");
      },
    } as never,
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 1600 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(FILE.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Comment", exact: true }).click();
  await toolbar(page).waitFor();
  return watched;
}

const toolbar = (page: Page) => page.getByRole("toolbar", { name: "Comment tools" });

/** Turns on a text tool from Text box ▾'s menu. */
async function textTool(page: Page, name: "Text box" | "Callout" | "Typewriter text"): Promise<void> {
  await toolbar(page).getByRole("button", { name: "Choose text tool" }).click();
  await page.getByRole("menuitemradio", { name, exact: true }).click();
}

/** The viewport position of (`x`, `y`) points from page 1's top-left (an A4 page). */
async function at(page: Page, x: number, y: number): Promise<{ x: number; y: number }> {
  const box = await page.locator(PAGE).boundingBox();
  if (box === null) throw new Error("page 1 isn't shown");
  const scale = box.width / 595.275591;
  return { x: box.x + x * scale, y: box.y + y * scale };
}

/** A press at `from` and a release at `to`, page points. */
async function drag(page: Page, from: [number, number], to: [number, number]): Promise<void> {
  const a = await at(page, ...from);
  const b = await at(page, ...to);
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 5 });
  await page.mouse.up();
}

/** The box being typed into, by its accessible name, once it has the focus. */
async function typingInto(page: Page, name: string) {
  const box = page.getByRole("textbox", { name, exact: true });
  await expect.poll(() => box.evaluate((e) => e === document.activeElement), { timeout: 20_000 }).toBe(true);
  return box;
}

/** The count of unsaved changes the top bar says, once it says `count`. */
async function changes(page: Page, count: number): Promise<void> {
  const text = count === 0 ? "No unsaved changes" : count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 30_000 }).toBe(text);
}

/** Saves (a download) and returns page 1's annotations as pdf.js reads them. */
async function savedBoxes(page: Page) {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  const bytes = new Uint8Array(readFileSync(await (await download).path()));
  const task = getDocument({ data: bytes, verbosity: 0 });
  const doc = await task.promise;
  try {
    const annotations = await (await doc.getPage(1)).getAnnotations();
    return (
      annotations as {
        subtype: string;
        hasAppearance: boolean;
        contentsObj: { str: string };
        richText?: unknown;
        borderStyle: { width: number };
      }[]
    ).filter((a) => a.subtype === "FreeText");
  } finally {
    await task.destroy();
  }
}

/** The text box buttons of the annotation layer on page 1, by their names. */
function boxNames(page: Page): Promise<string[]> {
  return page
    .locator(`${PAGE} .lamina-annotation`)
    .evaluateAll((buttons) =>
      buttons.map((b) => b.getAttribute("aria-label") ?? "").filter((n) => n.startsWith("Text box")),
    );
}

describe(`Text boxes in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap D.06 web round-trip] [cap D.16 web round-trip] types a text box, a callout and typewriter text in English and Arabic, makes a word bold, and pdf.js reads each back", async () => {
    const watched = await comment();
    const { page } = watched;

    await toolbar(page).getByRole("button", { name: "Text box", exact: true }).click();
    await drag(page, [60, 420], [260, 450]);
    await typingInto(page, "Text box text");
    await page.keyboard.type("Check the ");
    await page.keyboard.press(`${MOD}+B`);
    await page.keyboard.type("totals");
    await page.keyboard.press(`${MOD}+B`);
    // While the box is typed into, the inspector shows its text fields.
    const inspector = page.getByRole("complementary", { name: "Comment" });
    await inspector.getByRole("button", { name: "Done editing" }).waitFor();
    expect(await serious(page)).toEqual([]);
    await page.keyboard.press("Escape");
    await changes(page, 1);

    await textTool(page, "Callout");
    await drag(page, [300, 470], [340, 400]);
    await typingInto(page, "Callout text");
    await page.keyboard.type("مرحبا بالعالم");
    await page.keyboard.press("Escape");
    await changes(page, 2);

    await textTool(page, "Typewriter text");
    await drag(page, [60, 520], [60, 520]);
    await typingInto(page, "Typewriter text");
    await page.keyboard.type("Ada Lovelace");
    await page.keyboard.press("Escape");
    await changes(page, 3);

    const saved = await savedBoxes(page);
    const texts = saved.map((a) => a.contentsObj.str).sort();
    expect(texts).toEqual(["Ada Lovelace", "Check the totals", "مرحبا بالعالم"].sort());
    expect(saved.every((a) => a.hasAppearance)).toBe(true);
    const checked = saved.find((a) => a.contentsObj.str === "Check the totals");
    expect(JSON.stringify(checked?.richText ?? "")).toMatch(/bold/);
    expect(saved.find((a) => a.contentsObj.str === "Ada Lovelace")?.borderStyle.width).toBe(0);
    await clean(watched);
  });

  it("undoes a typing session in one step, and a double-click opens a box for typing again", async () => {
    const watched = await comment();
    const { page } = watched;
    await toolbar(page).getByRole("button", { name: "Text box", exact: true }).click();
    await drag(page, [60, 420], [260, 450]);
    await typingInto(page, "Text box text");
    await page.keyboard.type("One two three");
    await page.keyboard.press("Escape");
    await changes(page, 1);
    // Escape again leaves the text tool; the box stays selected.
    await page.keyboard.press("Escape");
    await toolbar(page).getByRole("button", { name: "Select", exact: true }).click();

    await expect.poll(() => boxNames(page), { timeout: 20_000 }).toEqual(["Text box: One two three"]);
    await page.locator(`${PAGE} .lamina-annotation[aria-label="Text box: One two three"]`).click();
    await page.locator(`${PAGE} .lamina-annotation[aria-label="Text box: One two three"]`).dblclick();
    await typingInto(page, "Text box text");
    await page.keyboard.press("End");
    await page.keyboard.type(" four");
    await page.keyboard.press("Escape");
    await changes(page, 2);
    await expect.poll(() => boxNames(page), { timeout: 20_000 }).toEqual(["Text box: One two three four"]);

    await page.keyboard.press(`${MOD}+Z`);
    await changes(page, 1);
    await expect.poll(() => boxNames(page), { timeout: 20_000 }).toEqual(["Text box: One two three"]);
    await clean(watched);
  });
});
