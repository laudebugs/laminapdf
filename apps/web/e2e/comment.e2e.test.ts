// Comment mode (Paper C01, C02), in this run's browser, with no file pickers, so every save is a
// download. Each tool makes its annotation with the mouse and with the keyboard; the file saved
// and opened again in pdf.js holds each one with its appearance, colour and author. The inspector
// changes a rectangle's colour, fill, width, border and opacity, and Alt+arrows resize it. On a
// file certified at P2 the banner shows as the mode opens; on a P3 file there is none, and a
// highlight saves as an incremental update. An opaque rectangle over text shows the covered-text
// notice; one with no fill, or over empty space, shows none. Each new surface passes axe, in three
// themes.

import { readFileSync } from "node:fs";
import { env } from "node:process";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Locator, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const TEXT = new URL("../../../fixtures/reading-order/two-columns-ordered.pdf", import.meta.url);
const CERTIFIED_P2 = new URL("../../../fixtures/signed/certified-p2.pdf", import.meta.url);
const CERTIFIED_P3 = new URL("../../../fixtures/signed/certified-p3.pdf", import.meta.url);
/** Where screenshots of the mode go, when asked for (LAMINA_PDF_E2E_SHOTS=<dir>). */
const SHOTS = env.LAMINA_PDF_E2E_SHOTS;

/** No pickers, so saves download; a name for comments already chosen, so the first tool asks nothing; a theme. */
function setup(theme: string, author: string | null) {
  return {
    script: ([name, appearance]: [string | null, string]) => {
      Reflect.deleteProperty(window, "showOpenFilePicker");
      Reflect.deleteProperty(window, "showSaveFilePicker");
      if (name !== null) localStorage.setItem("lamina.comment.author", name);
      localStorage.setItem("lamina.appearance", appearance);
    },
    arg: [author, JSON.stringify({ theme })] as [string | null, string],
  };
}

/** The reader, wide, with `file` open in Comment mode. */
async function comment(file: URL, options: { theme?: string; author?: string | null } = {}): Promise<Watched> {
  const watched = await load(
    browser,
    origin,
    "/",
    setup(options.theme ?? "light", options.author === undefined ? "Ada" : options.author) as never,
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 900 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Comment", exact: true }).click();
  await toolbar(page).waitFor();
  if (file === TEXT) await page.locator('.lamina-page[data-page-index="0"] .lamina-text-layer span').first().waitFor();
  return watched;
}

const toolbar = (page: Page) => page.getByRole("toolbar", { name: "Comment tools" });
const PAGE = '.lamina-page[data-page-index="0"]';

/** Turns on the toolbar's tool `name`, or the shape `shape` from Shapes ▾. */
async function tool(page: Page, name: string, shape?: string): Promise<void> {
  if (shape === undefined) {
    await toolbar(page).getByRole("button", { name, exact: true }).click();
    return;
  }
  await toolbar(page)
    .getByRole("button", { name: /^Shapes/ })
    .click();
  await page.getByRole("menuitemradio", { name: shape, exact: true }).click();
}

/** The viewport position of (`x`, `y`) points from page 1's top-left. */
async function at(page: Page, x: number, y: number): Promise<{ x: number; y: number }> {
  const box = await page.locator(PAGE).boundingBox();
  if (box === null) throw new Error("page 1 isn't shown");
  // The fixture's page is A4: 595 pt wide upright, 842 pt turned a quarter.
  const scale = box.width / (box.height >= box.width ? 595.275591 : 841.889764);
  return { x: box.x + x * scale, y: box.y + y * scale };
}

/** A drag on page 1 through `points` (page points). */
async function drag(page: Page, points: readonly (readonly [number, number])[], shift = false): Promise<void> {
  const [first, ...rest] = points;
  if (first === undefined) return;
  const start = await at(page, ...first);
  await page.mouse.move(start.x, start.y);
  if (shift) await page.keyboard.down("Shift");
  await page.mouse.down();
  for (const point of rest) {
    const next = await at(page, ...point);
    await page.mouse.move(next.x, next.y, { steps: 4 });
  }
  await page.mouse.up();
  if (shift) await page.keyboard.up("Shift");
}

/** A click on page 1 at (`x`, `y`) points. */
async function click(page: Page, x: number, y: number): Promise<void> {
  const point = await at(page, x, y);
  await page.mouse.click(point.x, point.y);
}

/** The box of page 1's text span `index`, in page points. */
async function span(page: Page, index: number): Promise<[number, number, number, number]> {
  const pageBox = await page.locator(PAGE).boundingBox();
  const box = await page.locator(`${PAGE} .lamina-text-layer span`).nth(index).boundingBox();
  if (pageBox === null || box === null) throw new Error("no text");
  const s = pageBox.width / 595.275591;
  return [
    (box.x - pageBox.x) / s,
    (box.y - pageBox.y) / s,
    (box.x + box.width - pageBox.x) / s,
    (box.y + box.height - pageBox.y) / s,
  ];
}

/** The count of unsaved changes the top bar says, once it says `count`. */
async function changes(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 20_000 }).toBe(text);
}

/** Saves (a download, with no pickers) and returns the bytes. */
async function save(page: Page): Promise<Uint8Array> {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** Page 1's annotations as pdf.js reads them, pop-ups left out. */
async function pdfjs(bytes: Uint8Array) {
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const annotations = await (await doc.getPage(1)).getAnnotations();
    return annotations.filter((a: { subtype: string }) => a.subtype !== "Popup") as {
      subtype: string;
      hasAppearance: boolean;
      color: Uint8ClampedArray | null;
      interiorColor?: Uint8ClampedArray | null;
      borderStyle: { width: number; rawWidth?: number; style: number };
      titleObj: { str: string };
      contentsObj: { str: string };
      rect: number[];
    }[];
  } finally {
    await task.destroy();
  }
}

/** A screenshot of `what`, when screenshots were asked for. */
async function shot(page: Page | Locator, name: string): Promise<void> {
  if (SHOTS !== undefined) await page.screenshot({ path: `${SHOTS}/${BROWSER}-${name}.png` });
}

/**
 * Each annotation as `subtype colour width`: pdf.js's colour as bytes, and its outline's width as
 * the file gives it (`rawWidth`: pdf.js shows a width wider than half the rectangle as 1, as for a
 * level line's thin rectangle).
 */
function styles(saved: Awaited<ReturnType<typeof pdfjs>>): string[] {
  const drawn = new Set(["Ink", "Square", "Circle", "Line", "Polygon", "PolyLine"]);
  return saved
    .map(
      (a) =>
        `${a.subtype} ${[...(a.color ?? [])].join(",")}${drawn.has(a.subtype) ? ` ${a.borderStyle.rawWidth ?? a.borderStyle.width}` : ""}`,
    )
    .sort();
}

/** What each tool draws with by default: its colour as bytes (the design language's tokens), and a line's width. */
const DEFAULTS = {
  highlight: "Highlight 242,214,117",
  underline: "Underline 47,106,69",
  strike: "StrikeOut 179,38,30",
  caret: "Caret 45,93,168",
  note: "Text 242,214,117",
  ink: "Ink 45,93,168 2",
  rectangle: "Square 45,93,168 1.5",
  ellipse: "Circle 45,93,168 1.5",
  line: "Line 38,36,33 1.5",
  arrow: "Line 179,38,30 1.5",
  polygon: "Polygon 45,93,168 1.5",
  polyline: "PolyLine 38,36,33 1.5",
} as const;

/** How many covered-text checks have finished: each mark made, and each fill set, runs one. */
function checks(page: Page): Promise<number> {
  return page.locator(".web-comment-bar").evaluate((bar) => Number((bar as HTMLElement).dataset.coverChecks ?? 0));
}

/** Waits for the covered-text check after `before` checks to finish. */
async function checked(page: Page, before: number): Promise<void> {
  await expect.poll(() => checks(page), { timeout: 20_000 }).toBeGreaterThan(before);
}

/** Selects page 1's text spans `from` to `to` with a DOM range, as caret browsing would. */
async function selectSpans(page: Page, from: number, to: number): Promise<void> {
  await page.evaluate(
    ([selector, a, b]) => {
      const spans = document.querySelectorAll(selector);
      const range = document.createRange();
      range.setStart(spans[a] as Node, 0);
      range.setEndAfter(spans[b] as Node);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    },
    [`${PAGE} .lamina-text-layer span`, from, to] as const,
  );
}

/** Focuses the view, where Enter places a mark. */
async function focusView(page: Page): Promise<void> {
  await page.locator(".lamina-viewer").first().focus();
}

describe(`Comment mode in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap B.06 web round-trip] [cap D.01 web round-trip] [cap D.02 web round-trip] [cap D.03 web round-trip] [cap D.04 web round-trip] [cap D.05 web round-trip] makes each mark with the mouse, and pdf.js reads each back with its appearance and author", async () => {
    const watched = await comment(TEXT);
    const { page } = watched;
    await shot(page, "mode");
    const [x0, y0, x1, y1] = await span(page, 0);
    const middle = (y0 + y1) / 2;
    await tool(page, "Highlight");
    await shot(toolbar(page), "toolbar-highlight");
    await drag(page, [
      [x0 + 1, middle],
      [x1 - 1, middle],
    ]);
    await changes(page, 1);
    const [, u0, , u1] = await span(page, 1);
    await tool(page, "Underline");
    await drag(page, [
      [x0 + 1, (u0 + u1) / 2],
      [x1 - 1, (u0 + u1) / 2],
    ]);
    await changes(page, 2);
    const [, s0, , s1] = await span(page, 2);
    await tool(page, "Strike");
    await drag(page, [
      [x0 + 1, (s0 + s1) / 2],
      [x0 + 40, (s0 + s1) / 2],
    ]);
    await changes(page, 3);
    await tool(page, "Caret");
    await click(page, x0 + 20, middle);
    await changes(page, 4);
    await tool(page, "Ink");
    await drag(page, [
      [60, 230],
      [100, 210],
      [140, 250],
    ]);
    await changes(page, 5);
    await tool(page, "Shapes", "Rectangle");
    await drag(page, [
      [100, 260],
      [180, 300],
    ]);
    await changes(page, 6);
    await tool(page, "Shapes", "Ellipse");
    await drag(page, [
      [200, 260],
      [280, 300],
    ]);
    await changes(page, 7);
    await tool(page, "Shapes", "Arrow");
    await drag(page, [
      [300, 260],
      [380, 295],
    ]);
    await changes(page, 8);
    await tool(page, "Shapes", "Polygon");
    for (const [x, y] of [
      [400, 260],
      [450, 260],
      [425, 300],
      [425, 300],
    ] as const)
      await click(page, x, y);
    await changes(page, 9);
    await tool(page, "Shapes", "Polyline");
    for (const [x, y] of [
      [200, 325],
      [250, 310],
      [300, 325],
      [300, 325],
    ] as const)
      await click(page, x, y);
    await changes(page, 10);
    await tool(page, "Note");
    await click(page, 520, 230);
    await changes(page, 11);
    // The note's field has the focus: what is typed is its comment.
    const inspector = page.getByRole("complementary", { name: "Comment" });
    await inspector.getByLabel("Note").waitFor();
    await expect.poll(() => page.evaluate(() => document.activeElement?.tagName)).toBe("TEXTAREA");
    await page.keyboard.type("Check this figure");
    await page.keyboard.press("Tab");
    await changes(page, 12);
    await shot(page, "inspector-note");
    expect(await serious(page)).toEqual([]);

    const saved = await pdfjs(await save(page));
    const subtypes = saved.map((a) => a.subtype).sort();
    expect(subtypes).toEqual(
      [
        "Caret",
        "Circle",
        "Highlight",
        "Ink",
        "Line",
        "PolyLine",
        "Polygon",
        "Square",
        "StrikeOut",
        "Text",
        "Underline",
      ].sort(),
    );
    expect(saved.every((a) => a.hasAppearance)).toBe(true);
    expect(saved.every((a) => a.titleObj.str === "Ada")).toBe(true);
    const { line: _, ...drawn } = DEFAULTS;
    expect(styles(saved)).toEqual(Object.values(drawn).sort());
    expect(saved.find((a) => a.subtype === "Text")?.contentsObj.str).toBe("Check this figure");
    await clean(watched);
  });

  it("makes each mark from the keyboard: Enter marks selected text or places a mark in the middle of the page", async () => {
    const watched = await comment(TEXT);
    const { page } = watched;
    let made = 0;
    for (const name of ["Highlight", "Underline", "Strike"]) {
      await selectSpans(page, made, made);
      await tool(page, name);
      made++;
      await changes(page, made);
    }
    const places: [string, string | undefined][] = [
      ["Caret", undefined],
      ["Note", undefined],
      ["Ink", undefined],
      ["Shapes", "Rectangle"],
      ["Shapes", "Ellipse"],
      ["Shapes", "Line"],
      ["Shapes", "Arrow"],
      ["Shapes", "Polygon"],
      ["Shapes", "Polyline"],
    ];
    for (const [name, shape] of places) {
      await tool(page, name, shape);
      await focusView(page);
      await page.keyboard.press("Enter");
      made++;
      await changes(page, made);
    }
    const saved = await pdfjs(await save(page));
    expect(saved.every((a) => a.hasAppearance)).toBe(true);
    expect(styles(saved)).toEqual(Object.values(DEFAULTS).sort());
    await clean(watched);
  });

  it("changes a rectangle's colour, fill, width, border and opacity in the inspector, and Alt+arrows resize it", async () => {
    const watched = await comment(TEXT);
    const { page } = watched;
    await tool(page, "Shapes", "Rectangle");
    await drag(page, [
      [100, 260],
      [180, 300],
    ]);
    await changes(page, 1);
    await tool(page, "Select");
    await click(page, 100, 280);
    const inspector = page.getByRole("complementary", { name: "Comment" });
    await inspector.getByRole("heading", { name: "Rectangle" }).waitFor();
    await shot(page, "inspector-rectangle");
    expect(await serious(page)).toEqual([]);
    await inspector.getByRole("group", { name: "Line colour" }).getByRole("button", { name: "Red" }).click();
    await changes(page, 2);
    await inspector.getByRole("group", { name: "Fill" }).getByRole("button", { name: "Blue" }).click();
    await changes(page, 3);
    await inspector.getByLabel("Width").selectOption("4");
    await changes(page, 4);
    await inspector.getByRole("group", { name: "Border" }).getByRole("button", { name: "Dashed" }).click();
    await changes(page, 5);
    await inspector.getByLabel("Opacity").fill("60");
    await inspector.getByLabel("Opacity").press("Enter");
    await changes(page, 6);
    const button = page.locator(`${PAGE} .lamina-annotation[aria-label="Rectangle"]`);
    await button.focus();
    await page.keyboard.press("Alt+Shift+ArrowRight");
    await changes(page, 7);
    expect(await page.locator(`${PAGE} .lamina-annotation-grip`).count()).toBe(8);
    await shot(page, "inspector-rectangle-changed");
    // Escape from the inspector deselects: the inspector goes, and the handles and outline with it.
    await inspector.getByRole("group", { name: "Border" }).getByRole("button", { name: "Dashed" }).focus();
    await page.keyboard.press("Escape");
    await expect.poll(() => inspector.count()).toBe(0);
    await expect
      .poll(() => page.locator(`${PAGE} .lamina-annotation-handle, ${PAGE} .lamina-annotation-grip`).count())
      .toBe(0);
    const bytes = await save(page);
    const saved = await pdfjs(bytes);
    const square = saved.find((a) => a.subtype === "Square");
    expect({
      colour: [...(square?.color ?? [])],
      width: square?.borderStyle.width,
      // pdf.js numbers the styles: 1 solid, 2 dashed.
      style: square?.borderStyle.style,
    }).toEqual({ colour: [179, 38, 30], width: 4, style: 2 });
    // pdf.js keeps no record of a fill: the saved rectangle's /IC is the inspector's blue.
    const fill = /\/IC\s*\[\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)\s*\]/.exec(Buffer.from(bytes).toString("latin1"));
    expect(fill?.slice(1).map((v) => Math.round(Number(v) * 255))).toEqual([169, 200, 232]);
    // 80 pt wide, then 10 pt wider by Shift+Alt+→; its rectangle holds the 4 pt outline too.
    const [left = 0, , right = 0] = square?.rect ?? [];
    expect(right - left).toBeGreaterThan(89);
    expect(right - left).toBeLessThan(96);
    await clean(watched);
  });

  it("maps a mark through the page's new turn after the page is rotated in Comment mode", async () => {
    const watched = await comment(TEXT);
    const { page } = watched;
    await tool(page, "Shapes", "Rectangle");
    await drag(page, [
      [100, 100],
      [180, 150],
    ]);
    await changes(page, 1);
    await page.keyboard.press("ControlOrMeta+K");
    await page.keyboard.type("Rotate page right");
    await page.keyboard.press("Enter");
    await changes(page, 2);
    await expect
      .poll(async () => {
        const box = await page.locator(PAGE).boundingBox();
        return box !== null && box.width > box.height;
      })
      .toBe(true);
    await tool(page, "Shapes", "Rectangle");
    await drag(page, [
      [300, 100],
      [380, 150],
    ]);
    await changes(page, 3);
    const squares = (await pdfjs(await save(page))).filter((a) => a.subtype === "Square");
    // Turned a quarter clockwise, page space's (x, y) is user space's (y, x): the second
    // rectangle runs 100–150 pt across and 300–380 pt up, inside its 0.75 pt stroke margin.
    const turned = squares.map(({ rect: [x0 = 0, y0 = 0, x1 = 0, y1 = 0] }) =>
      [x0 + 0.75, y0 + 0.75, x1 - 0.75, y1 - 0.75].map((v) => Math.round(v)),
    );
    expect(turned).toContainEqual([100, 300, 150, 380]);
    await clean(watched);
  });

  it("shows the banner as the mode opens on a file certified at P2", async () => {
    const watched = await comment(CERTIFIED_P2);
    const { page } = watched;
    const banner = page.getByRole("region", { name: "Signature warning" });
    await banner.getByText("Saving will invalidate the signature.").waitFor();
    await banner.getByText("certified for form filling only").waitFor();
    await shot(page, "certified-p2");
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });

  // The P3 fixture's page has no text to highlight, so a note stands in for the highlight: every
  // mark is one addAnnot of the same class, which the planner gates alike.
  it("shows no banner on a file certified at P3, where a comment saves as an incremental update", async () => {
    const watched = await comment(CERTIFIED_P3);
    const { page } = watched;
    await tool(page, "Note");
    await click(page, 100, 100);
    await changes(page, 1);
    expect(await page.getByRole("region", { name: "Signature warning" }).count()).toBe(0);
    const saved = await save(page);
    const original = new Uint8Array(readFileSync(CERTIFIED_P3));
    // An incremental update keeps every byte of the signed file and appends after it.
    expect(saved.length).toBeGreaterThan(original.length);
    expect(Buffer.from(saved.subarray(0, original.length)).equals(Buffer.from(original))).toBe(true);
    await clean(watched);
  });

  it("shows the covered-text notice for an opaque filled rectangle over text, and none for one with no fill, a see-through one or one over empty space", async () => {
    const watched = await comment(TEXT);
    const { page } = watched;
    const [x0, y0, x1, y1] = await span(page, 0);
    const notice = page.getByText("This shape covers text", { exact: false });
    // No fill: the check runs and finds nothing.
    let before = await checks(page);
    await tool(page, "Shapes", "Rectangle");
    await drag(page, [
      [x0 - 2, y0 - 2],
      [x1 + 2, y1 + 2],
    ]);
    await changes(page, 1);
    await checked(page, before);
    expect(await notice.count()).toBe(0);
    // Half opaque, then filled: the text shows through, so no notice.
    await tool(page, "Select");
    await click(page, x0 - 2, (y0 + y1) / 2);
    const inspector = page.getByRole("complementary", { name: "Comment" });
    await inspector.getByLabel("Opacity").fill("50");
    await inspector.getByLabel("Opacity").press("Enter");
    await changes(page, 2);
    await expect.poll(() => inspector.getByLabel("Opacity").inputValue(), { timeout: 20_000 }).toBe("50");
    before = await checks(page);
    await inspector.getByRole("group", { name: "Fill" }).getByRole("button", { name: "Yellow" }).click();
    await changes(page, 3);
    await checked(page, before);
    expect(await notice.count()).toBe(0);
    // The fill is the rectangle tool's from then on: a new rectangle over text, opaque, hides it.
    const [, s0, , s1] = await span(page, 1);
    await tool(page, "Shapes", "Rectangle");
    await drag(page, [
      [x0, s0],
      [x0 + 60, s1],
    ]);
    await changes(page, 4);
    await notice.waitFor({ timeout: 20_000 });
    await shot(page, "covered-text");
    expect(await serious(page)).toEqual([]);
    await page.getByRole("button", { name: "Dismiss" }).click();
    await expect.poll(() => notice.count()).toBe(0);
    // The next filled rectangle, over empty space, shows none.
    before = await checks(page);
    await drag(page, [
      [60, 305],
      [120, 330],
    ]);
    await changes(page, 5);
    await checked(page, before);
    expect(await notice.count()).toBe(0);
    await clean(watched);
  });

  it("asks for the name comments carry at the first mark, then makes that mark with it", async () => {
    const watched = await comment(TEXT, { author: null });
    const { page } = watched;
    const dialog = page.getByRole("dialog", { name: "Your name on comments" });
    await tool(page, "Shapes", "Rectangle");
    expect(await dialog.count()).toBe(0);
    await drag(page, [
      [100, 260],
      [180, 300],
    ]);
    await dialog.waitFor();
    await shot(page, "author-dialog");
    expect(await serious(page)).toEqual([]);
    await dialog.getByLabel("Name").fill("R. Osei");
    await dialog.getByRole("button", { name: "Use this name" }).click();
    await expect.poll(() => dialog.count()).toBe(0);
    // The rectangle that asked is made once answered, with the name.
    await changes(page, 1);
    await drag(page, [
      [200, 260],
      [280, 300],
    ]);
    await changes(page, 2);
    expect(await dialog.count()).toBe(0);
    const saved = await pdfjs(await save(page));
    expect(saved.map((a) => a.titleObj.str)).toEqual(["R. Osei", "R. Osei"]);
    await clean(watched);
  });

  it("makes the first mark without a name when the question is left off", async () => {
    const watched = await comment(TEXT, { author: null });
    const { page } = watched;
    await tool(page, "Note");
    await click(page, 520, 230);
    const dialog = page.getByRole("dialog", { name: "Your name on comments" });
    await dialog.getByRole("button", { name: "Leave it off" }).click();
    await changes(page, 1);
    const saved = await pdfjs(await save(page));
    expect(saved.map((a) => a.titleObj.str)).toEqual([""]);
    await clean(watched);
  });

  it.each(["dark", "high-contrast"])("passes axe in Comment mode with the inspector open, %s theme", async (theme) => {
    const watched = await comment(TEXT, { theme });
    const { page } = watched;
    await tool(page, "Shapes", "Ellipse");
    await drag(page, [
      [100, 260],
      [180, 300],
    ]);
    await changes(page, 1);
    expect(await serious(page)).toEqual([]);
    await tool(page, "Select");
    await click(page, 100, 280);
    await page.getByRole("complementary", { name: "Comment" }).getByRole("heading", { name: "Ellipse" }).waitFor();
    expect(await serious(page)).toEqual([]);
    await shot(page, `inspector-${theme}`);
    await clean(watched);
  });
});
