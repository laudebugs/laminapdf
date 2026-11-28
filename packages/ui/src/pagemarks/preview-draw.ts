// How the Page marks dialog's preview draws a mark over a page's picture: the text in the font the
// browser has for the choice (so close to, not the same as, what the core embeds), a picture, the
// colour, opacity and angle. Nothing here writes to the document. The numbers are the core's
// (model.ts), so the preview and the overlap notice agree about where a line of text lands.

import { CONTENT_COLOURS } from "../content/commands.js";
import {
  type BatesValues,
  batesText,
  fillTokens,
  type HeaderFooterValues,
  lineHeight,
  MARK_SLOTS,
  type MarkSlot,
  type MarkTab,
  type PageSize,
  pictureSize,
  type Rect,
  slotBox,
  type TokenValues,
  type WatermarkValues,
  watermarkWidth,
} from "./model.js";

/** The picture of a watermark the preview draws. */
export interface MarkImage {
  readonly name: string;
  /** Where the browser reads it from (an object URL). */
  readonly url: string;
  readonly width: number;
  readonly height: number;
}

/** What the preview needs to know of the page it shows. */
export interface PreviewPage {
  readonly size: PageSize;
  readonly tokens: TokenValues;
  /** How many pages before this one the mark is on: a Bates number counts one more for each. */
  readonly ordinal: number;
  /** The CSS font stack of the chosen font. */
  readonly css: string;
}

/** One line of a header, footer or Bates number: its place, its text and the box it fills. */
export interface SlotLine {
  readonly slot: MarkSlot;
  readonly text: string;
  readonly box: Rect;
}

let context: CanvasRenderingContext2D | null | undefined;

/** How wide `text` is in `css` at `size` points, as the browser sets it. */
function measureWidth(text: string, css: string, size: number): number {
  context ??= document.createElement("canvas").getContext("2d");
  if (context === null || context === undefined) return text.length * size * 0.5;
  context.font = `${size}px ${css}`;
  return context.measureText(text).width;
}

/** The lines a header and footer, or a Bates number, puts on `page`. */
export function slotLines(
  tab: MarkTab,
  values: HeaderFooterValues | BatesValues,
  page: PreviewPage,
  measure = measureWidth,
): SlotLine[] {
  const make = (slot: MarkSlot, text: string, size: number, margins: { top: number; bottom: number }) => {
    const first = text.split("\n")[0] ?? "";
    return { slot, text: first, box: slotBox(slot, measure(first, page.css, size), size, margins, page.size) };
  };
  if (tab === "bates") {
    const bates = values as BatesValues;
    const text = batesText(bates, bates.start + page.ordinal);
    return text.trim() === "" ? [] : [make(bates.slot, text, bates.size, { top: 36, bottom: 36 })];
  }
  const headerFooter = values as HeaderFooterValues;
  return MARK_SLOTS.flatMap((slot, i) => {
    const text = fillTokens(headerFooter.slots[i] ?? "", page.tokens);
    return text.trim() === "" ? [] : [make(slot, text, headerFooter.size, headerFooter)];
  });
}

/** The colour of a watermark as CSS. */
function inkOf(colour: WatermarkValues["colour"]): string {
  const [r, g, b] = (CONTENT_COLOURS.find((c) => c.id === colour) ?? CONTENT_COLOURS[0]).rgb;
  return `rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;
}

/** What `drawWatermark` needs besides the values. */
export interface WatermarkDrawing {
  readonly values: WatermarkValues;
  readonly css: string;
  /** The picture once it has loaded. */
  readonly image: { readonly element: CanvasImageSource; readonly width: number; readonly height: number } | undefined;
}

/** Draws a watermark or a background on `ctx`, whose unit is one point of the page `page`. */
export function drawWatermark(ctx: CanvasRenderingContext2D, page: PageSize, draw: WatermarkDrawing): void {
  const { values } = draw;
  ctx.save();
  ctx.globalAlpha = Math.min(Math.max(values.opacity / 100, 0), 1);
  // A background lies behind the page's own ink: multiplying keeps the ink in front of it.
  if (values.layer === "behind") ctx.globalCompositeOperation = "multiply";
  ctx.translate(page.width / 2, page.height / 2);
  ctx.rotate((-values.angle * Math.PI) / 180);
  if (values.source === "image") {
    if (draw.image !== undefined) {
      const size = pictureSize(draw.image, page, values.layer, values.angle);
      ctx.drawImage(draw.image.element, -size.width / 2, -size.height / 2, size.width, size.height);
    }
  } else {
    const lines = values.text.split("\n");
    ctx.font = `${values.size}px ${draw.css}`;
    ctx.fillStyle = inkOf(values.colour);
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    const limit = watermarkWidth(page);
    const step = lineHeight(values.size);
    lines.forEach((line, i) => {
      ctx.fillText(line, 0, (i - (lines.length - 1) / 2) * step, limit);
    });
  }
  ctx.restore();
}

/** Draws the lines of a header and footer or a Bates number, each in the colour it prints in and inside a dashed box. */
export function drawSlots(
  ctx: CanvasRenderingContext2D,
  lines: readonly SlotLine[],
  look: {
    readonly css: string;
    readonly size: number;
    readonly ink: string;
    readonly edge: string;
    /** One screen pixel in points, so the dashed box is 1 px wide whatever the scale. */
    readonly pixel: number;
  },
): void {
  ctx.save();
  ctx.font = `${look.size}px ${look.css}`;
  ctx.textBaseline = "middle";
  ctx.fillStyle = look.ink;
  ctx.strokeStyle = look.edge;
  ctx.lineWidth = look.pixel;
  ctx.setLineDash([3 * look.pixel, 2 * look.pixel]);
  for (const line of lines) {
    const [x0, y0, x1, y1] = line.box;
    ctx.strokeRect(x0 - 2 * look.pixel, y0, x1 - x0 + 4 * look.pixel, y1 - y0);
    ctx.fillText(line.text, x0, (y0 + y1) / 2, Math.max(x1 - x0, 1));
  }
  ctx.restore();
}
