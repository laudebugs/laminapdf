// Annotating on a phone and a tablet (Paper C08, C09), in this run's browser: pen and finger input
// as pointer events with their pointer type, pressure and contact size, dispatched where a
// person's would land. On a tablet a pen draws while a finger scrolls, and the tool shelf takes
// the toolbar's place; a pressure ramp saves widths that follow it. On a phone a finger draws
// strokes that join one drawing, the Eraser takes one of three, and the top bar's Undo brings it
// back. axe finds nothing serious on the bars, and every target is at least 44 px.

import { readFileSync } from "node:fs";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const TEXT = new URL("../../../fixtures/reading-order/two-columns-ordered.pdf", import.meta.url);
/** Three pages: a tablet scrolls through it. */
const REPORT = new URL("../../../fixtures/golden/examples/mark_up.pdf", import.meta.url);
const PAGE = '.lamina-page[data-page-index="0"]';
const TABLET = { width: 1024, height: 1366 };
const PHONE = { width: 390, height: 844 };

/** The reader on a touch screen of `viewport`, with `file` open in Comment mode. */
async function comment(viewport: { width: number; height: number }, file = TEXT): Promise<Watched> {
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
    { ignoreHTTPSErrors: true, viewport, hasTouch: true, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  // A phone goes into Comment through Annotate in the bottom bar (C08); a tablet through the mode switcher.
  const into =
    viewport.width < 600
      ? page.getByRole("toolbar", { name: "Document" }).getByRole("button", { name: "Annotate" })
      : page.getByRole("button", { name: "Comment", exact: true });
  await into.click({ timeout: 30_000 });
  return watched;
}

/** The viewport position of (`x`, `y`) points from page 1's top-left. */
async function at(page: Page, x: number, y: number): Promise<{ x: number; y: number }> {
  const box = await page.locator(PAGE).boundingBox();
  if (box === null) throw new Error("page 1 isn't shown");
  const scale = box.width / 595.275591;
  return { x: box.x + x * scale, y: box.y + y * scale };
}

/** One point of a stroke: page points, and the pen's pressure there. */
type StrokePoint = readonly [number, number, number?];

/** What a stroke's pointer event is made with: plain values, so it crosses into the page. */
interface Init {
  readonly pointerId: number;
  readonly pointerType: string;
  readonly clientX: number;
  readonly clientY: number;
  readonly pressure: number;
  readonly width: number;
  readonly height: number;
  readonly button: number;
  readonly buttons: number;
  readonly isPrimary: boolean;
  readonly bubbles: boolean;
  readonly cancelable: boolean;
}

/** One pointer event of a stroke, ready to dispatch. */
interface Fired {
  readonly kind: "pointerdown" | "pointermove" | "pointerup";
  readonly init: Init;
}

/** The events of a stroke of `type` through viewport points `at`, each with its pressure. */
function strokeEvents(type: "pen" | "touch", at: readonly { x: number; y: number; p: number }[], id: number): Fired[] {
  const contact = type === "touch" ? 10 : 1;
  const event = (kind: Fired["kind"], { x, y, p }: { x: number; y: number; p: number }): Fired => ({
    kind,
    init: {
      // A finger's id never matches the mouse's (1 in WebKit), as e2e/devices does.
      pointerId: 10 + id,
      pointerType: type,
      clientX: x,
      clientY: y,
      pressure: kind === "pointerup" ? 0 : p,
      width: contact,
      height: contact,
      button: kind === "pointermove" ? -1 : 0,
      buttons: kind === "pointerup" ? 0 : 1,
      isPrimary: true,
      bubbles: true,
      cancelable: true,
    },
  });
  const [first, ...rest] = at;
  if (first === undefined) return [];
  return [
    event("pointerdown", first),
    ...rest.map((p) => event("pointermove", p)),
    event("pointerup", rest.at(-1) ?? first),
  ];
}

/** A stroke of `type` ("pen" or "touch") through page points, one frame per event, landing on what is under it in the viewer. */
async function stroke(page: Page, type: "pen" | "touch", points: readonly StrokePoint[], id = 3): Promise<void> {
  const at_ = await Promise.all(points.map(async ([x, y, p]) => ({ ...(await at(page, x, y)), p: p ?? 0.5 })));
  await page.evaluate(
    async (events) => {
      const root = document.querySelector(".lamina-viewer") ?? document.body;
      for (const { kind, init } of events) {
        const under = document.elementFromPoint(init.clientX, init.clientY);
        (under !== null && root.contains(under) ? under : root).dispatchEvent(new PointerEvent(kind, init));
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
    },
    strokeEvents(type, at_, id),
  );
}

/** The count of unsaved changes the top bar says, once it says `count`. */
async function changes(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 30_000 }).toBe(text);
}

/** The targets inside `area` (a CSS selector) narrower or shorter than 44 px, by name. */
async function small(page: Page, area: string): Promise<string[]> {
  return page.locator(area).evaluate((bar) =>
    [...bar.querySelectorAll("button, label")]
      .filter((b) => {
        const r = b.getBoundingClientRect();
        return r.width > 0 && (r.width < 44 || r.height < 44);
      })
      .map(
        (b) =>
          `${b.tagName}.${b.className} ${Math.round(b.getBoundingClientRect().width)}x${Math.round(b.getBoundingClientRect().height)} ${b.getAttribute("aria-label") ?? b.textContent ?? ""}`,
      ),
  );
}

/** Saves (a download): page 1's ink as pdf.js reads it, and the file's bytes. */
async function saved(page: Page) {
  const download: Promise<Download> = page.waitForEvent("download");
  const button = page.getByRole("button", { name: "Download", exact: true });
  if (await button.isVisible()) await button.click();
  else {
    // A phone keeps Download in the top bar's More while annotating: the annotate bar has the bottom.
    await page.getByRole("banner").getByRole("button", { name: "More" }).click();
    await page.getByRole("menuitem", { name: "Download a copy" }).click();
  }
  const bytes = new Uint8Array(readFileSync(await (await download).path()));
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  try {
    const doc = await task.promise;
    const annotations = (await (await doc.getPage(1)).getAnnotations()) as {
      subtype: string;
      inkLists?: unknown[];
    }[];
    return { ink: annotations.filter((a) => a.subtype === "Ink"), bytes };
  } finally {
    await task.destroy();
  }
}

/** The per-point widths a pressure-varying stroke saved, as Lamina PDF writes them (`/LMNA_Widths`). */
function widths(bytes: Uint8Array): number[] {
  const text = new TextDecoder("latin1").decode(bytes);
  const found = text.match(/\/LMNA_Widths\s*\[\s*\[([^\]]*)\]/);
  return (found?.[1] ?? "").trim().split(/\s+/).filter(Boolean).map(Number);
}

describe(`Annotating on phones and tablets in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap D.04 web round-trip] on a tablet, a pen draws and a finger scrolls; the shelf takes the toolbar's place, and a pressure ramp saves widths that follow it", async () => {
    const watched = await comment(TABLET, REPORT);
    const { page } = watched;
    await page.getByRole("toolbar", { name: "Comment tools" }).getByRole("button", { name: "Ink" }).click();
    const ramp: StrokePoint[] = [0, 1, 2, 3, 4, 5].map((i) => [100 + i * 30, 300, 0.1 + i * 0.18]);
    await stroke(page, "pen", ramp);
    await changes(page, 1);
    const shelf = page.getByRole("toolbar", { name: "Pen tools" });
    await shelf.waitFor();
    expect([await small(page, ".paper-tool-shelf"), await serious(page)]).toEqual([[], []]);
    // Draw with finger went off when the pen drew: a finger now scrolls, and draws nothing.
    const before = await page.locator(".lamina-viewer").evaluate((v) => v.scrollTop);
    await stroke(page, "touch", [
      [300, 700],
      [300, 600],
      [300, 450],
      [300, 300],
    ]);
    await expect.poll(() => page.locator(".lamina-viewer").evaluate((v) => v.scrollTop)).toBeGreaterThan(before);
    await changes(page, 1);
    // With Draw with finger back on, a finger draws: a drawing of its own, more than a second after the pen's.
    await shelf.getByText("Draw with finger").click();
    await page.waitForTimeout(1100);
    // Low on page 1, which the scroll keeps on screen.
    await stroke(page, "touch", [
      [100, 700],
      [200, 720],
    ]);
    await changes(page, 2);
    const { ink, bytes } = await saved(page);
    const w = widths(bytes);
    expect([
      ink.length,
      w.length,
      w.every((x, i) => i === 0 || x >= (w[i - 1] ?? 0)),
      (w.at(-1) ?? 0) > (w[0] ?? 0),
    ]).toEqual([2, ramp.length, true, true]);
    await clean(watched);
  });

  it("on a phone, finger strokes join one drawing and the eraser takes one of three", async () => {
    const watched = await comment(PHONE);
    const { page } = watched;
    const bar = page.getByRole("toolbar", { name: "Annotate tools" });
    await bar.getByRole("button", { name: "Pen" }).click();
    for (const y of [200, 260, 320]) {
      await stroke(page, "touch", [
        [60, y],
        [140, y],
        [220, y],
      ]);
    }
    expect([await small(page, ".paper-annotate"), await serious(page)]).toEqual([[], []]);
    await page.waitForTimeout(1500);
    expect((await saved(page)).ink.map((a) => a.inkLists?.length)).toEqual([3]);
    await bar.getByRole("button", { name: "Eraser" }).click();
    await stroke(page, "touch", [
      [140, 245],
      [140, 275],
    ]);
    await page.waitForTimeout(1500);
    expect((await saved(page)).ink.map((a) => a.inkLists?.length)).toEqual([2]);
    // Undo sits in the top bar, named after the step it takes back (C08), and brings the stroke back.
    await page
      .getByRole("banner")
      .getByRole("button", { name: /^Undo erase/ })
      .click();
    await page.waitForTimeout(1500);
    expect((await saved(page)).ink.map((a) => a.inkLists?.length)).toEqual([3]);
    await clean(watched);
  });
});
