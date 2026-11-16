// Page appearance (todo U2c; R10) in the built reader: choosing Dark from View settings recolours
// the page's paper and text but keeps a highlight's yellow, says so in a toast with Undo, and is
// remembered for that document only; another document keeps its own colours. Under emulated
// forced colours (Chromium) the reader starts in High contrast, its paper the system's Canvas.
// No CSP violation throughout.

import { composite, contrastRatio, type Rgba } from "@laminapdf/tokens";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, opened, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const APPEARANCE = new URL("../../../fixtures/appearance/", import.meta.url);
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);
const READING_ORDER = new URL("../../../fixtures/reading-order/", import.meta.url);

/** Opens `file` from the home screen (More › Close document goes back to it) and waits for it to show. */
async function open(watched: Watched, file: URL, title: string): Promise<void> {
  const { page } = watched;
  const home = page.getByRole("heading", { name: "Open a PDF" });
  if (!(await home.isVisible())) {
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: "Close document" }).click();
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

  // Bug selection-on-dark-pages: the selection was multiplied over the page, so on a Dark page it
  // drew near black, with the words under it at 2.24:1. No screenshot: taking one in WebKit adds
  // an inline style sheet, which the reader's CSP refuses; the page's own pixels and the
  // selection's colour, as the browser resolves it from the style sheets, give what it shows.
  it("draws a mouse selection on a Dark page in its tint, with the words readable", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, { ignoreHTTPSErrors: true });
    const { page } = watched;
    await open(watched, new URL("header-footer-ordered.pdf", READING_ORDER), "header-footer-ordered.pdf");
    await choose(page, "Dark");
    await expect.poll(async () => close(await pixel(page, 5, 5), DARK_PAPER)).toBe(true);
    // A line of the first paragraph, dragged across as a reader selects it.
    const line = page.locator('.lamina-page[data-page-index="0"] .lamina-text-layer span').nth(2);
    const box = await line.boundingBox();
    if (box === null) throw new Error("no line on the first page");
    await page.mouse.move(box.x + 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width - 2, box.y + box.height / 2, { steps: 10 });
    await page.mouse.up();
    expect(await page.evaluate(() => document.getSelection()?.toString().length ?? 0)).toBeGreaterThan(10);
    // Drawn as it is over the page, not blended with it (multiply was the bug).
    expect(await line.evaluate((span) => getComputedStyle(span.parentElement as Element).mixBlendMode)).toBe("normal");
    const { selection, paper, ink } = await line.evaluate(selectionOver);
    const landed = composite(selection, paper);
    // Teal on a Dark page: #425858.
    expect(
      [landed.r - 0x42, landed.g - 0x58, landed.b - 0x58].every((d) => Math.abs(d) <= 4),
      `the selection lands on rgb(${landed.r} ${landed.g} ${landed.b})`,
    ).toBe(true);
    expect(contrastRatio(composite(selection, ink), landed)).toBeGreaterThanOrEqual(4.5);
    await clean(watched);
  });
});

/**
 * Runs in the page: `span`'s `::selection` colour as the browser resolves it, and the paper and
 * the ink the page's canvas draws under it (the commonest pixel and the one furthest from it).
 */
function selectionOver(span: Element): { selection: Rgba; paper: Rgba; ink: Rgba } {
  const rgba = (css: string): Rgba => {
    const [r = 0, g = 0, b = 0, a = 1] = (css.match(/[\d.]+/g) ?? []).map(Number);
    return { r, g, b, a };
  };
  const selection = rgba(getComputedStyle(span, "::selection").backgroundColor);
  const r = span.getBoundingClientRect();
  const [x, y] = [r.left + r.width / 2, r.top + r.height / 2];
  const canvas = [...(span.closest(".lamina-page")?.querySelectorAll("canvas") ?? [])].find((c) => {
    const b = c.getBoundingClientRect();
    return x >= b.left && x <= b.right && y >= b.top && y <= b.bottom;
  });
  if (canvas === undefined) throw new Error("no canvas under the line");
  const b = canvas.getBoundingClientRect();
  const s = canvas.width / b.width;
  const data =
    canvas
      .getContext("2d")
      ?.getImageData(
        Math.round((r.left - b.left) * s),
        Math.round((r.top - b.top) * s),
        Math.round(r.width * s),
        Math.round(r.height * s),
      ).data ?? new Uint8ClampedArray();
  const counts = new Map<string, number>();
  for (let i = 0; i < data.length; i += 4) {
    const key = `${data[i]},${data[i + 1]},${data[i + 2]}`;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  const paper = rgba([...counts].sort((p, q) => q[1] - p[1])[0]?.[0] ?? "0,0,0");
  let ink = paper;
  let far = -1;
  for (const [key, n] of counts) {
    const c = rgba(key);
    const d = Math.abs(c.r - paper.r) + Math.abs(c.g - paper.g) + Math.abs(c.b - paper.b);
    if (n > 3 && d > far) [far, ink] = [d, c];
  }
  return { selection, paper, ink };
}
