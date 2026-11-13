// Page appearance (todo U2c; R10) in the built reader: choosing Dark from View settings recolours
// the page's paper and text but keeps a highlight's yellow, says so in a toast with Undo, and is
// remembered for that document only; another document keeps its own colours. Under emulated
// forced colours (Chromium) the reader starts in High contrast, its paper the system's Canvas.
// No CSP violation throughout.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, opened, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const APPEARANCE = new URL("../../../fixtures/appearance/", import.meta.url);
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);

/** Opens `file` from the home screen (More › Choose a file goes back to it) and waits for it to show. */
async function open(watched: Watched, file: URL, title: string): Promise<void> {
  const { page } = watched;
  const home = page.getByRole("heading", { name: "Open a PDF" });
  if (!(await home.isVisible())) {
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: "Choose a file" }).click();
    await home.waitFor();
  }
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).first().click();
  await (await chooser).setFiles(file.pathname);
  // The viewer's first canvas, then the title: a cold first open (compiling the WebAssembly)
  // can take longer than a title poll waits.
  await opened(watched, title);
}

/** The first page's canvas pixel at (`x`, `y`) in points of a 300-point-wide page. */
function pixel(page: Page, x: number, y: number): Promise<number[]> {
  return page.evaluate(
    ([px = 0, py = 0]) => {
      const canvas = document.querySelector<HTMLCanvasElement>('.lamina-page[data-page-index="0"] canvas');
      if (canvas === null) return [];
      const s = canvas.width / 300;
      return Array.from(canvas.getContext("2d")?.getImageData(Math.round(px * s), Math.round(py * s), 1, 1).data ?? []);
    },
    [x, y],
  );
}

const close = (a: number[], b: number[]) =>
  a.length >= 3 && a.slice(0, 3).every((v, i) => Math.abs(v - (b[i] ?? 0)) <= 3);
const DARK_PAPER = [0x22, 0x21, 0x1f];
const WHITE = [255, 255, 255];
const HIGHLIGHT = [255, 230, 51];

/** Chooses `name` among the page appearance chips in View settings. */
async function choose(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: "View settings" }).click();
  await page.getByRole("button", { name, exact: true }).click();
  await page.keyboard.press("Escape");
}

describe(`page appearance in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("recolours the page but not its highlight, offers Undo, and remembers it for that document", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, { ignoreHTTPSErrors: true });
    const { page } = watched;
    await open(watched, new URL("annotated.pdf", APPEARANCE), "annotated.pdf");
    await expect.poll(() => pixel(page, 5, 5)).toEqual([...WHITE, 255]);
    await choose(page, "Dark");
    await page.getByText("Page appearance: Dark, for this document only").waitFor();
    await expect.poll(async () => close(await pixel(page, 5, 5), DARK_PAPER)).toBe(true);
    // Above the highlighted words, inside the highlight: still yellow.
    expect(close(await pixel(page, 25, 37.5), HIGHLIGHT)).toBe(true);
    // The toast's Undo, not the top bar's, which undoes the document's edits.
    await page.locator(".paper-toast-action", { hasText: /^Undo$/ }).click();
    await expect.poll(async () => close(await pixel(page, 5, 5), WHITE)).toBe(true);

    // Chosen again, it comes back with the document, and another document keeps its colours.
    await choose(page, "Dark");
    await expect.poll(async () => close(await pixel(page, 5, 5), DARK_PAPER)).toBe(true);
    await page.reload();
    await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    await open(watched, new URL("hello.pdf", CORPUS), "hello.pdf");
    await expect.poll(async () => close(await pixel(page, 5, 5), WHITE)).toBe(true);
    await open(watched, new URL("annotated.pdf", APPEARANCE), "annotated.pdf");
    await expect.poll(async () => close(await pixel(page, 5, 5), DARK_PAPER)).toBe(true);
    await clean(watched);
  });

  it.runIf(BROWSER === "chromium")("starts in High contrast under forced colours", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      forcedColors: "active",
    });
    const { page } = watched;
    await open(watched, new URL("annotated.pdf", APPEARANCE), "annotated.pdf");
    await page.getByRole("button", { name: "View settings" }).click();
    await expect
      .poll(() => page.getByRole("button", { name: "High contrast", exact: true }).getAttribute("aria-pressed"))
      .toBe("true");
    await page.keyboard.press("Escape");
    // The paper is the system's Canvas colour, as the browser resolves it.
    const canvas = await page.evaluate(() => {
      const probe = document.createElement("span");
      probe.style.color = "Canvas";
      document.body.append(probe);
      const colour = getComputedStyle(probe).color;
      probe.remove();
      return (colour.match(/\d+/g) ?? []).slice(0, 3).map(Number);
    });
    await expect.poll(async () => close(await pixel(page, 5, 5), canvas)).toBe(true);
    await clean(watched);
  });
});
