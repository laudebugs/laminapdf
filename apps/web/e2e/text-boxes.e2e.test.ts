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

/** Saves (a download): the file's bytes and page 1's text boxes as pdf.js reads them. */
async function saved(page: Page) {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  const bytes = new Uint8Array(readFileSync(await (await download).path()));
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const annotations = await (await doc.getPage(1)).getAnnotations();
    const boxes = (
      annotations as {
        subtype: string;
        hasAppearance: boolean;
        contentsObj: { str: string };
        richText?: unknown;
        borderStyle: { width: number };
      }[]
    ).filter((a) => a.subtype === "FreeText");
    return { bytes, boxes };
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

/** Types `before`, then `bold` in bold with ⌘B or Ctrl B, then `after`. */
async function typeWithBold(page: Page, before: string, bold: string, after: string): Promise<void> {
  await page.keyboard.type(before);
  await page.keyboard.press(`${MOD}+B`);
  await page.keyboard.type(bold);
  await page.keyboard.press(`${MOD}+B`);
  await page.keyboard.type(after);
}

const inspectorOf = (page: Page) => page.getByRole("complementary", { name: "Comment" });

describe(`Text boxes in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap D.06 web round-trip] [cap D.16 web round-trip] types a text box, a callout and typewriter text in English and Arabic with a bold word each, and pdf.js reads each back", async () => {
    const watched = await comment();
    const { page } = watched;
    const inspector = inspectorOf(page);

    await toolbar(page).getByRole("button", { name: "Text box", exact: true }).click();
    await drag(page, [60, 420], [260, 450]);
    const box = await typingInto(page, "Text box text");
    // An empty box shows its placeholder.
    expect([await box.textContent(), await box.getAttribute("data-placeholder")]).toEqual(["", "Type here"]);
    await typeWithBold(page, "Check the ", "totals", " مرحبا");
    // While the box is typed into, the inspector shows its text fields, and the Arabic's fallback font.
    await inspector.getByRole("button", { name: "Done editing" }).waitFor();
    await inspector.getByText("Some characters use a fallback font").waitFor();
    expect(await serious(page)).toEqual([]);
    await page.keyboard.press("Escape");
    await changes(page, 1);

    await textTool(page, "Callout");
    await drag(page, [300, 470], [340, 400]);
    await typingInto(page, "Callout text");
    await typeWithBold(page, "", "Why", " مرحبا بالعالم");
    // The leader line's handles are named, and the box with them passes axe.
    await page.getByRole("button", { name: "Callout knee", exact: true }).waitFor();
    await page.getByRole("button", { name: "Callout arrow end", exact: true }).waitFor();
    expect(await serious(page)).toEqual([]);
    await page.keyboard.press("Escape");
    await changes(page, 2);

    await textTool(page, "Typewriter text");
    await drag(page, [60, 520], [60, 520]);
    await typingInto(page, "Typewriter text");
    await typeWithBold(page, "Ada ", "Lovelace", " آدا");
    await page.keyboard.press("Escape");
    await changes(page, 3);

    const { bytes, boxes } = await saved(page);
    expect(boxes.map((a) => a.contentsObj.str).sort()).toEqual(
      ["Ada Lovelace آدا", "Check the totals مرحبا", "Why مرحبا بالعالم"].sort(),
    );
    expect(boxes.every((a) => a.hasAppearance)).toBe(true);
    expect(boxes.every((a) => /bold/.test(JSON.stringify(a.richText ?? "")))).toBe(true);
    expect(boxes.find((a) => a.contentsObj.str.startsWith("Ada"))?.borderStyle.width).toBe(0);
    // The Arabic is drawn in the bundled fallback font, embedded in the file.
    expect(new TextDecoder("latin1").decode(bytes)).toContain("NotoSansArabic");
    await clean(watched);
  });

  it("leaves no box for one left empty, opens Text box with T, and Escape after typing deselects", async () => {
    const watched = await comment();
    const { page } = watched;
    await page.locator(".lamina-viewer").first().focus();
    await page.keyboard.press("t");
    await expect
      .poll(() => toolbar(page).getByRole("button", { name: "Text box", exact: true }).getAttribute("aria-pressed"))
      .toBe("true");
    await drag(page, [60, 420], [260, 450]);
    await typingInto(page, "Text box text");
    await page.keyboard.press("Escape");
    await drag(page, [60, 470], [260, 500]);
    await typingInto(page, "Text box text");
    await page.keyboard.type("Kept");
    await page.keyboard.press("Escape");
    await changes(page, 1);
    await expect.poll(() => boxNames(page), { timeout: 20_000 }).toEqual(["Text box: Kept"]);
    // The box stays selected; Escape again deselects it and turns Select on.
    await page.locator(`${PAGE} .lamina-annotation[aria-pressed="true"]`).waitFor();
    await page.keyboard.press("Escape");
    await expect.poll(() => inspectorOf(page).count()).toBe(0);
    await expect
      .poll(() => toolbar(page).getByRole("button", { name: "Select", exact: true }).getAttribute("aria-pressed"))
      .toBe("true");
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
    await page.keyboard.press("Escape");

    await expect.poll(() => boxNames(page), { timeout: 20_000 }).toEqual(["Text box: One two three"]);
    const made = page.locator(`${PAGE} .lamina-annotation[aria-label="Text box: One two three"]`);
    await made.click();
    await made.dblclick();
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
