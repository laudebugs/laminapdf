// What the Page marks dialog (Paper A03) asks and shows, apart from the DOM so Node can test it:
// the values of each tab, the tokens a header's slots take, the text a page would show, where the
// text lands (for the overlap notice and the preview) and what the footer says. The numbers
// follow the core's layout (crates/laminapdf-ops, added/mark_layout.rs): a line is 1.5 times the
// size, a header or footer is a box as wide as the page between its margins, aligned by its
// column, and a watermark's text lies on a line 90 % of the page's diagonal long.

import type { ContentColour } from "../content/commands.js";
import { type LaterMessageId, laterCountMessage, laterMessage } from "../later-messages.js";
import { parsePrintPages } from "../reader/print-pages.js";

/** The three tabs. */
export const MARK_TABS = ["watermark", "headerFooter", "bates"] as const;

/** One of them. */
export type MarkTab = (typeof MARK_TABS)[number];

/** The six places a header, a footer or a Bates number sits, in the order the core lists them. */
export const MARK_SLOTS = ["topLeft", "topCenter", "topRight", "bottomLeft", "bottomCenter", "bottomRight"] as const;

/** One of them. */
export type MarkSlot = (typeof MARK_SLOTS)[number];

/** The tokens a slot takes, in the order the buttons list them: what the dialog shows and what the core reads. */
export const MARK_TOKENS = [
  { id: "page", shown: "[Page]", written: "{page}" },
  { id: "pages", shown: "[Pages]", written: "{pages}" },
  { id: "label", shown: "[Page label]", written: "{label}" },
  { id: "date", shown: "[Date]", written: "{date}" },
  { id: "file", shown: "[File name]", written: "{file}" },
] as const;

/** One token's id. */
type MarkToken = (typeof MARK_TOKENS)[number]["id"];

/** How the date token is written. */
export const DATE_FORMATS = ["dayMonth", "iso", "monthDay"] as const;

/** One of them. */
export type DateFormat = (typeof DATE_FORMATS)[number];

/** The colours a watermark takes, in the order the design lists them. */
export const WATERMARK_COLOURS: readonly ContentColour[] = ["grey", "red", "teal", "ink"];

/** A line of text is this many times its size high. */
const LINE = 1.5;
/** A watermark's text lies on a line this share of the page's diagonal long. */
const DIAGONAL = 0.9;
/** A watermark picture shows whole inside this share of the page. */
const FILL = 0.8;
/** The share of a page's height at its top and at its bottom that running headers and footers use. */
const BAND = 0.1;
/** A margin the core keeps on a side the dialog doesn't ask about, in points. */
const SIDE_MARGIN = 36;

/** What the Watermark tab holds. */
export interface WatermarkValues {
  readonly source: "text" | "image";
  readonly text: string;
  readonly fontId: string;
  readonly size: number;
  readonly colour: ContentColour;
  /** From 0 to 100. */
  readonly opacity: number;
  /** Degrees counterclockwise, from -360 to 360. */
  readonly angle: number;
  readonly layer: "front" | "behind";
  readonly hideable: boolean;
}

/** What the Header and footer tab holds: each slot's text with its tokens in square brackets. */
export interface HeaderFooterValues {
  readonly slots: readonly [string, string, string, string, string, string];
  readonly fontId: string;
  readonly size: number;
  readonly top: number;
  readonly bottom: number;
  readonly dateFormat: DateFormat;
}

/** What the Bates numbers tab holds. */
export interface BatesValues {
  readonly prefix: string;
  readonly start: number;
  readonly digits: number;
  readonly suffix: string;
  readonly slot: MarkSlot;
  readonly fontId: string;
  readonly size: number;
}

/** The defaults each tab starts from. */
export const WATERMARK_DEFAULT: WatermarkValues = {
  source: "text",
  text: "",
  fontId: "helvetica",
  size: 96,
  colour: "grey",
  opacity: 30,
  angle: 45,
  layer: "front",
  hideable: false,
};

/** The header and footer's defaults: a page number on the bottom right. */
export const HEADER_FOOTER_DEFAULT: HeaderFooterValues = {
  slots: ["", "", "", "", "", "[Page] of [Pages]"],
  fontId: "helvetica",
  size: 9,
  top: SIDE_MARGIN,
  bottom: SIDE_MARGIN,
  dateFormat: "dayMonth",
};

/** The Bates tab's defaults. */
export const BATES_DEFAULT: BatesValues = {
  prefix: "",
  start: 1,
  digits: 6,
  suffix: "",
  slot: "bottomRight",
  fontId: "helvetica",
  size: 9,
};

/** The pages a form names: all of them, or a range of numbers and labels. */
export interface PagesChoice {
  readonly all: boolean;
  readonly range: string;
}

/** The pages a choice names (0-based, in order), or why it names none. */
export function readPages(choice: PagesChoice, labels: readonly string[]): { pages: number[]; error?: string } {
  if (choice.all) return { pages: labels.map((_, i) => i) };
  if (choice.range.trim() === "") return { pages: [] };
  const read = parsePrintPages(choice.range, labels.length, labels);
  if (!read.ok) return { pages: [], error: read.error };
  return { pages: [...new Set(read.pages)].sort((a, b) => a - b) };
}

/** The ranges of a sorted run of pages as a person says them: "2–24", "1, 3–5", by label. */
export function pagesText(pages: readonly number[], labels: readonly string[]): string {
  const parts: string[] = [];
  let start = 0;
  for (let k = 0; k < pages.length; k++) {
    const here = pages[k] as number;
    if (pages[k + 1] === here + 1) continue;
    const first = pages[start] as number;
    parts.push(first === here ? (labels[here] ?? "") : `${labels[first] ?? ""}–${labels[here] ?? ""}`);
    start = k + 1;
  }
  return parts.join(", ");
}

/** `text` with its square-bracket tokens written as the core reads them, and its own braces doubled. */
export function writeTokens(text: string): string {
  let out = text.replaceAll("{", "{{").replaceAll("}", "}}");
  for (const token of MARK_TOKENS) out = out.replaceAll(token.shown, token.written);
  return out;
}

/** `text` as the core stores it, shown with square-bracket tokens and single braces: the reverse of `writeTokens`. */
export function showTokens(text: string): string {
  const marker = (i: number) => `\u0000${i}\u0000`;
  let out = text;
  MARK_TOKENS.forEach((token, i) => {
    out = out.replaceAll(token.written, marker(i));
  });
  out = out.replaceAll("{{", "{").replaceAll("}}", "}");
  MARK_TOKENS.forEach((token, i) => {
    out = out.replaceAll(marker(i), token.shown);
  });
  return out;
}

/** What the tokens stand for on one page. */
export interface TokenValues {
  readonly page: number;
  readonly pages: number;
  readonly label: string;
  readonly date: string;
  readonly file: string;
}

/** `text` (tokens in square brackets) as page `values.page` shows it. */
export function fillTokens(text: string, values: TokenValues): string {
  const fills: Record<MarkToken, string> = {
    page: String(values.page),
    pages: String(values.pages),
    label: values.label,
    date: values.date,
    file: values.file,
  };
  let out = text;
  for (const token of MARK_TOKENS) out = out.replaceAll(token.shown, fills[token.id]);
  return out;
}

/** The date as `format` writes it, in the catalogue's language. */
export function formatDate(date: Date, format: DateFormat): string {
  if (format === "iso") {
    const two = (n: number) => String(n).padStart(2, "0");
    return `${date.getFullYear()}-${two(date.getMonth() + 1)}-${two(date.getDate())}`;
  }
  const options: Intl.DateTimeFormatOptions =
    format === "dayMonth" ? { day: "numeric", month: "short", year: "numeric" } : { dateStyle: "long" };
  return new Intl.DateTimeFormat(format === "dayMonth" ? "en-GB" : "en-US", options).format(date);
}

/** A Bates number as the core writes it: the number padded with zeros to `digits`. */
export function batesText(values: Pick<BatesValues, "prefix" | "suffix" | "digits">, number: number): string {
  return `${values.prefix}${String(number).padStart(values.digits, "0")}${values.suffix}`;
}

/** Whether a header and footer has some text to put. */
export function hasSlotText(values: Pick<HeaderFooterValues, "slots">): boolean {
  return values.slots.some((s) => s.trim() !== "");
}

/** The characters of every text a tab would draw, which its font has to cover (one list, so a check reads one string). */
export function textOfTab(
  tab: MarkTab,
  values: WatermarkValues | HeaderFooterValues | BatesValues,
  date: string,
  file: string,
): string {
  if (tab === "watermark") return (values as WatermarkValues).text;
  if (tab === "bates") {
    const b = values as BatesValues;
    return `${b.prefix}0123456789${b.suffix}`;
  }
  return `${(values as HeaderFooterValues).slots.join("\n")}\n${date}${file}0123456789`;
}

/** A rectangle in page space, points from the top-left corner: `[x0, y0, x1, y1]`. */
export type Rect = readonly [number, number, number, number];

/** A page's size in points. */
export interface PageSize {
  readonly width: number;
  readonly height: number;
}

/** The box of a slot's line of text `width` wide, as the core sets it: hung from the top margin or standing on the bottom one. */
export function slotBox(
  slot: MarkSlot,
  width: number,
  size: number,
  margins: { readonly top: number; readonly bottom: number },
  page: PageSize,
): Rect {
  const height = size * LINE;
  const room = Math.max(page.width - 2 * SIDE_MARGIN, 1);
  const shown = Math.min(width, room);
  const column = MARK_SLOTS.indexOf(slot) % 3;
  const x0 = SIDE_MARGIN + (column === 0 ? 0 : column === 1 ? (room - shown) / 2 : room - shown);
  const y0 = slot.startsWith("top") ? margins.top : page.height - margins.bottom - height;
  return [x0, y0, x0 + shown, y0 + height];
}

/** A line of text already on a page, in page space. */
export interface PageText {
  readonly text: string;
  readonly bounds: Rect;
}

/** The runs of `runs` in the page's top and bottom tenths, where running headers and footers are. */
export function bandTexts(runs: readonly PageText[], page: PageSize): PageText[] {
  return runs.filter((run) => {
    if (run.text.trim() === "") return false;
    const middle = (run.bounds[1] + run.bounds[3]) / 2;
    return middle <= page.height * BAND || middle >= page.height * (1 - BAND);
  });
}

/** The text of a page number: "Page 3 of 24", "3 / 24", "– 3 –" or the number alone. */
const PAGE_NUMBER = /^\s*(?:page\s*)?[-–—]?\s*\d{1,5}\s*(?:(?:of|\/)\s*\d{1,5})?\s*[-–—]?\s*$/i;

/** What a new slot would run into. */
export interface Overlap {
  readonly slot: MarkSlot;
  readonly text: string;
  /** Whether the text already there is a page number. */
  readonly pageNumber: boolean;
}

/** The first of `boxes` (slot, box) that touches a run of `existing` text, if any. */
export function overlapOf(
  boxes: readonly { readonly slot: MarkSlot; readonly box: Rect }[],
  existing: readonly PageText[],
): Overlap | undefined {
  for (const { slot, box } of boxes) {
    for (const run of existing) {
      const [a0, b0, a1, b1] = box;
      const [c0, d0, c1, d1] = run.bounds;
      // The line's box is taller than its ink: leave a tenth of the box at each edge out.
      const trim = (b1 - b0) * 0.1;
      if (a0 < c1 && c0 < a1 && b0 + trim < d1 && d0 < b1 - trim) {
        return { slot, text: run.text.trim(), pageNumber: PAGE_NUMBER.test(run.text) };
      }
    }
  }
  return undefined;
}

/** The size of a watermark's picture on a page, in points, and what it covers. */
export function pictureSize(
  pixels: { readonly width: number; readonly height: number },
  page: PageSize,
  layer: "front" | "behind",
  angle: number,
): { readonly width: number; readonly height: number } {
  const w = Math.max(pixels.width, 1);
  const h = Math.max(pixels.height, 1);
  if (layer === "front") {
    const scale = Math.min(page.width / w, page.height / h) * FILL;
    return { width: w * scale, height: h * scale };
  }
  const turned = (angle * Math.PI) / 180;
  const [s, c] = [Math.abs(Math.sin(turned)), Math.abs(Math.cos(turned))];
  const scale = Math.max((page.width * c + page.height * s) / w, (page.width * s + page.height * c) / h);
  return { width: w * scale, height: h * scale };
}

/** The longest line a watermark's text lies on, in points. */
export function watermarkWidth(page: PageSize): number {
  return Math.max(Math.hypot(page.width, page.height) * DIAGONAL, 1);
}

/** The height of `lines` lines of text of `size`. */
export function lineHeight(size: number, lines = 1): number {
  return size * LINE * lines;
}

/** Why a number field's text isn't a number in its range, or the number. */
export function readNumber(text: string, min: number, max: number): number | undefined {
  const parsed = Number(text.trim().replace(",", "."));
  if (text.trim() === "" || !Number.isFinite(parsed) || parsed < min || parsed > max) return undefined;
  return parsed;
}

/** The footer's summary of what the primary button does. */
export function summaryOf(tab: MarkTab, pages: readonly number[], labels: readonly string[], editing: boolean): string {
  const count = pages.length;
  if (count === 0) return laterMessage("pagemarks.summary.none");
  const where =
    count === labels.length
      ? laterCountMessage(count, { one: "pagemarks.summary.allOne", many: "pagemarks.summary.all" })
      : laterCountMessage(
          count,
          { one: "pagemarks.summary.someOne", many: "pagemarks.summary.some" },
          { pages: pagesText(pages, labels) },
        );
  const verb = `pagemarks.summary.${editing ? "updates" : "adds"}.${tab}` as LaterMessageId;
  return laterMessage(verb, { where });
}
