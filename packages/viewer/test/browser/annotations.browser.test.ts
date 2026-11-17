// The annotation layer in Chromium, Firefox and WebKit, over fixtures/annotations/every-subtype.pdf,
// driven by the real mouse and keyboard and judged by what the page shows: a button per annotation
// in reading order; dragging the highlight shows it once while it moves (the page drawn without
// it, the preview at the pointer) and once after release (at its new place, the preview gone);
// a cancelled drag shows it where it was at once; the thumbnail never loses it; the arrow keys
// move it 1 pt, and 10 pt with Shift.

import { decodePng } from "@laminapdf/test-utils/browser-harness";
import { AnnotationLayer, createViewer, ThumbnailStrip, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterAll, afterEach, beforeAll, describe, expect, it } from "vitest";
import { commands, userEvent } from "vitest/browser";
import { viewer as client, read } from "./helpers.js";

declare module "vitest/browser" {
  interface BrowserCommands {
    /** The real mouse at `selector`'s centre moved by (dx, dy): pressed, moved or released (vitest.browser.config.ts). */
    mouseAt: (selector: string, action: "down" | "move" | "up", dx?: number, dy?: number) => Promise<void>;
    /** A PNG screenshot of the element `selector` names, base64 (vitest.browser.config.ts). */
    screenshotOf: (selector: string) => Promise<string>;
  }
}

/** The highlight: the first annotation in reading order, yellow, 100 × 15 pt at (20, 25) of the page. */
const HIGHLIGHT = '.lamina-annotation[data-annotation-id="a10.0"]';
/** The underline, 100 × 15 pt at (20, 45), just below the highlight. */
const UNDERLINE = '.lamina-annotation[data-annotation-id="a11.0"]';
const PAGE = '.lamina-page[data-page-index="0"]';
/** How far the drags move the highlight down: onto an empty band of the page. */
const DOWN_PT = 40;

let rpc: ViewerClient;
let bytes: Uint8Array;
let view: Viewer | undefined;
let strip: ThumbnailStrip | undefined;
const hosts: HTMLElement[] = [];

/** Each render the client is asked for while set (a test sets it to count them). */
let renders: unknown[] | undefined;

beforeAll(async () => {
  rpc = await client();
  const render = rpc.render.bind(rpc);
  rpc.render = (request, options) => {
    renders?.push(request);
    return render(request, options);
  };
  bytes = await read("fixtures/annotations/every-subtype.pdf");
});
afterAll(() => rpc.terminate());
afterEach(() => {
  renders = undefined;
  strip?.destroy();
  view?.destroy();
  for (const host of hosts.splice(0)) host.remove();
  strip = undefined;
  view = undefined;
});

/** A fixed box on the page at (`left`, 0). */
function box(left: number, width: number, height: number): HTMLElement {
  const host = document.createElement("div");
  Object.assign(host.style, {
    width: `${width}px`,
    height: `${height}px`,
    position: "fixed",
    top: "0",
    left: `${left}px`,
  });
  document.body.append(host);
  hosts.push(host);
  return host;
}

/** The viewer over a fresh copy of the file with the annotation layer, page 1 sharp, and a thumbnail strip. */
async function opened(): Promise<{ viewer: Viewer; layer: AnnotationLayer }> {
  const viewer = createViewer({ client: rpc });
  view = viewer;
  viewer.mount(box(0, 700, 700));
  const document_ = await rpc.open(bytes);
  viewer.open(document_);
  const layer = new AnnotationLayer(viewer, rpc);
  viewer.addLayer(layer);
  strip = new ThumbnailStrip(rpc, viewer);
  strip.mount(box(720, 160, 400));
  strip.open(document_);
  await expect.poll(() => document.querySelector(HIGHLIGHT) !== null, { timeout: 20_000 }).toBe(true);
  await expect.poll(() => document.querySelector(PAGE)?.getAttribute("data-render"), { timeout: 20_000 }).toBe("sharp");
  await expect.poll(() => document.querySelector(".lamina-thumbnail canvas") !== null, { timeout: 20_000 }).toBe(true);
  return { viewer, layer };
}

/** CSS px per point on the page now: the highlight's button is 100 pt wide. */
function scale(): number {
  return (document.querySelector(HIGHLIGHT) as HTMLElement).getBoundingClientRect().width / 100;
}

/**
 * How far each channel may stray from the highlight's colour: WebKit's screenshots go through
 * colour management, so a drawn colour comes back a few levels off.
 */
const TOLERANCE = 48;

/**
 * Whether `r`, `g`, `b` is the highlight's yellow: full red and green, blue at most half, as the
 * page draws it (blue 0) or as the preview does (blue 127, half opaque over white).
 */
function isYellow(r: number, g: number, b: number): boolean {
  return r >= 255 - TOLERANCE && g >= 255 - TOLERANCE && b <= 127 + TOLERANCE;
}

/** Whether the screenshot of `selector` is the highlight's yellow at (`x`, `y`) CSS px from its top-left. */
async function yellowAt(selector: string, points: readonly (readonly [number, number])[]): Promise<boolean[]> {
  const element = document.querySelector(selector) as HTMLElement;
  const png = Uint8Array.from(atob(await commands.screenshotOf(selector)), (c) => c.charCodeAt(0));
  const shot = await decodePng(png);
  const ratio = shot.width / element.getBoundingClientRect().width;

  return points.map(([x, y]) => {
    const at = (Math.round(y * ratio) * shot.width + Math.round(x * ratio)) * 4;
    const [r = 0, g = 0, b = 255] = shot.pixels.subarray(at, at + 3);
    return isYellow(r, g, b);
  });
}

/** Where the highlight's middle is, and where it is `DOWN_PT` lower, in CSS px of the page. */
function places(): { before: [number, number]; after: [number, number] } {
  const s = scale();
  return { before: [70 * s, 32.5 * s], after: [70 * s, (32.5 + DOWN_PT) * s] };
}

/** The highlight's record as the worker reads it now. */
async function highlight(): Promise<[number, number, number, number]> {
  const record = (await rpc.pageAnnotations(0)).find((a) => a.id === "a10.0");
  return record?.rect as [number, number, number, number];
}

const preview = () => document.querySelector(".lamina-annotation-preview path");

describe("the annotation layer", () => {
  it("makes a named button for each annotation the page shows, in reading order", async () => {
    await opened();
    const names = [...document.querySelectorAll(`${PAGE} .lamina-annotation`)].map((b) => b.getAttribute("aria-label"));
    // Widgets, pop-ups and links are left to other layers.
    expect(names).toEqual([
      "Note: Check this",
      "Highlight",
      "Underline",
      "Rectangle",
      "Ink",
      "Polygon",
      "Line",
      "Text box: Callout",
    ]);
  });

  it("shows a dragged annotation once during the drag and once after release", async () => {
    await opened();
    const { before, after } = places();
    expect(await yellowAt(PAGE, [before, after])).toEqual([true, false]);
    const thumbnail = () => [thumbnailYellow()];
    await expect.poll(thumbnail, { timeout: 20_000 }).toEqual([true]);
    const down = DOWN_PT * scale();
    // The thumbnail never shows the page without the highlight: sampled at every step of the drag.
    const seen: boolean[] = [];
    await commands.mouseAt(HIGHLIGHT, "down");
    seen.push(thumbnailYellow());
    await commands.mouseAt(HIGHLIGHT, "move", 0, down / 2);
    seen.push(thumbnailYellow());
    await commands.mouseAt(HIGHLIGHT, "move", 0, down);
    // The preview shows once the page is drawn without the highlight: only the preview is yellow.
    await expect.poll(preview, { timeout: 20_000 }).not.toBeNull();
    seen.push(thumbnailYellow());
    expect(await yellowAt(PAGE, [before, after])).toEqual([false, true]);
    seen.push(thumbnailYellow());
    expect(seen).toEqual([true, true, true, true]);
    await commands.mouseAt(HIGHLIGHT, "up", 0, down);
    // The preview goes, and what shows at the new place is the page's own pixels.
    await expect.poll(() => document.querySelector(".lamina-annotation-preview"), { timeout: 20_000 }).toBeNull();
    expect(await yellowAt(PAGE, [before, after])).toEqual([false, true]);
    expect([canvasYellowAt(...before), canvasYellowAt(...after)]).toEqual([false, true]);
    const [x0, y0] = await highlight();
    expect([x0, Math.round(y0)]).toEqual([20, 25 + DOWN_PT]);
  });

  it("shows the annotation where it was at once when a drag is cancelled", async () => {
    await opened();
    const { before, after } = places();
    await commands.mouseAt(HIGHLIGHT, "down");
    await commands.mouseAt(HIGHLIGHT, "move", 0, DOWN_PT * scale());
    await expect.poll(preview, { timeout: 20_000 }).not.toBeNull();
    renders = [];
    await userEvent.keyboard("{Escape}");
    await expect.poll(preview).toBeNull();
    // The page came back from the bitmap cache in the task that removed the preview: no render.
    expect(await yellowAt(PAGE, [before, after])).toEqual([true, false]);
    expect(canvasYellowAt(...before)).toBe(true);
    expect(renders).toEqual([]);
    await commands.mouseAt(HIGHLIGHT, "up");
    expect((await highlight())[1]).toBe(25);
  });

  it("moves the focused annotation 1 pt with an arrow key and 10 pt with Shift", async () => {
    await opened();
    (document.querySelector(HIGHLIGHT) as HTMLElement).focus();
    await userEvent.keyboard("{ArrowRight}");
    await expect.poll(async () => (await highlight())[0], { timeout: 20_000 }).toBe(21);
    // The moved annotation's button has the focus again, so the keys keep moving it.
    await expect.poll(() => document.activeElement?.matches(HIGHLIGHT)).toBe(true);
    await userEvent.keyboard("{Shift>}{ArrowDown}{/Shift}");
    await expect.poll(async () => (await highlight())[1], { timeout: 20_000 }).toBe(35);
    await expect.poll(() => document.querySelector(HIGHLIGHT)?.getAttribute("aria-pressed")).toBe("true");
  });

  it("selects with a press, adds with Shift, draws the handles round the selection and moves it whole", async () => {
    const { layer } = await opened();
    await commands.mouseAt(HIGHLIGHT, "down");
    await commands.mouseAt(HIGHLIGHT, "up");
    expect(layer.selection().map((a) => a.id)).toEqual(["a10.0"]);
    expect(document.querySelectorAll(`${PAGE} .lamina-annotation-handle`)).toHaveLength(8);
    await userEvent.keyboard("{Shift>}");
    await commands.mouseAt(UNDERLINE, "down");
    await commands.mouseAt(UNDERLINE, "up");
    await userEvent.keyboard("{/Shift}");
    expect(layer.selection().map((a) => a.id)).toEqual(["a10.0", "a11.0"]);
    const pressed = () => [HIGHLIGHT, UNDERLINE].map((s) => document.querySelector(s)?.getAttribute("aria-pressed"));
    expect(pressed()).toEqual(["true", "true"]);
    // Dragging one moves both, as one edit.
    const down = DOWN_PT * scale();
    await commands.mouseAt(HIGHLIGHT, "down");
    await commands.mouseAt(HIGHLIGHT, "move", 0, down);
    await expect.poll(preview, { timeout: 20_000 }).not.toBeNull();
    await commands.mouseAt(HIGHLIGHT, "up", 0, down);
    const tops = async () => {
      const records = await rpc.pageAnnotations(0);
      return ["a10.0", "a11.0"].map((id) => Math.round(records.find((a) => a.id === id)?.rect?.[1] ?? 0));
    };
    await expect.poll(tops, { timeout: 20_000 }).toEqual([25 + DOWN_PT, 45 + DOWN_PT]);
    await expect.poll(pressed).toEqual(["true", "true"]);
    (document.querySelector(HIGHLIGHT) as HTMLElement).focus();
    await userEvent.keyboard("{Escape}");
    expect(layer.selection()).toEqual([]);
    expect(document.querySelectorAll(`${PAGE} .lamina-annotation-handle`)).toHaveLength(0);
  });
});

/**
 * Whether the first thumbnail's canvas is the highlight's yellow at the highlight's middle. Read
 * from the canvas itself: the strip lies outside the test's viewport, where no screenshot reaches.
 */
function thumbnailYellow(): boolean {
  const thumb = document.querySelector<HTMLCanvasElement>(".lamina-thumbnail canvas");
  if (thumb === null) return false;
  const copy = document.createElement("canvas");
  copy.width = thumb.width;
  copy.height = thumb.height;
  const ctx = copy.getContext("2d");
  if (ctx === null) return false;
  ctx.drawImage(thumb, 0, 0);
  const [r = 0, g = 0, b = 255] = ctx.getImageData(
    Math.floor((70 / 300) * thumb.width),
    Math.floor((32.5 / 300) * thumb.height),
    1,
    1,
  ).data;
  return isYellow(r, g, b);
}

/** Whether the page's own canvas (no preview, no layer) is yellow at (`x`, `y`) CSS px of the page. */
function canvasYellowAt(x: number, y: number): boolean {
  const page = document.querySelector(PAGE) as HTMLElement;
  const canvas = page.querySelector<HTMLCanvasElement>("canvas.lamina-page-detail") ?? page.querySelector("canvas");
  if (canvas === null) return false;
  const pageBox = page.getBoundingClientRect();
  const box = canvas.getBoundingClientRect();
  const [cx, cy] = [pageBox.left + x - box.left, pageBox.top + y - box.top];
  const copy = document.createElement("canvas");
  copy.width = canvas.width;
  copy.height = canvas.height;
  const ctx = copy.getContext("2d");
  if (ctx === null) return false;
  ctx.drawImage(canvas, 0, 0);
  const [r = 0, g = 0, b = 255] = ctx.getImageData(
    Math.floor((cx / box.width) * canvas.width),
    Math.floor((cy / box.height) * canvas.height),
    1,
    1,
  ).data;
  return isYellow(r, g, b);
}
