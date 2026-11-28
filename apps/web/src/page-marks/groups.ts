// The marks of a document as Manage page marks lists them, apart from the DOM so Node can test it:
// the marks each page lists, gathered by ID (a mark has one on every page it is on), what each
// says, and the dialog's values for editing one. What a page lists is what the page keeps: a
// header's slots are the text with its braces, a Bates number's `start` is that page's own number.

import type { PageMarkInfo } from "@laminapdf/sdk";
import {
  BATES_DEFAULT,
  type BatesValues,
  batesText,
  type DateFormat,
  HEADER_FOOTER_DEFAULT,
  type HeaderFooterValues,
  laterMessage,
  type ManagedKind,
  type ManagedMark,
  type MarkEdit,
  type MarkImage,
  type MarkSlot,
  pagesText,
  showTokens,
  WATERMARK_DEFAULT,
  type WatermarkValues,
} from "@laminapdf/ui/editing";
import { colourIdOf } from "../content/inspector-props.ts";
import { rgbOf } from "../content/ops.ts";

/** For reading what a mark says, where its font doesn't matter. */
const NO_FONTS: EditContext = { fontIdOf: () => undefined, imageOf: () => undefined };

/** One mark with the pages it is on. */
export interface MarkGroup {
  readonly id: string;
  /** What the first page keeps of it. */
  readonly first: PageMarkInfo["mark"];
  /** What the last page keeps of it, whose Bates number is the highest. */
  readonly last: PageMarkInfo["mark"];
  /** The pages, 0-based, in order. */
  readonly pages: readonly number[];
}

/** The marks `perPage` lists (index = page), gathered by ID in the order they first appear. */
export function groupMarks(perPage: readonly (readonly PageMarkInfo[])[]): MarkGroup[] {
  const groups = new Map<
    string,
    { id: string; first: PageMarkInfo["mark"]; last: PageMarkInfo["mark"]; pages: number[] }
  >();
  perPage.forEach((marks, page) => {
    for (const { id, mark } of marks) {
      const group = groups.get(id);
      if (group === undefined) groups.set(id, { id, first: mark, last: mark, pages: [page] });
      else {
        group.last = mark;
        group.pages.push(page);
      }
    }
  });
  return [...groups.values()];
}

/** A mark's kind and its fields: the kind is the only key of the mark's object. */
function specOf(mark: PageMarkInfo["mark"]): { readonly kind: ManagedKind; readonly spec: Record<string, unknown> } {
  const [kind, spec] = Object.entries(mark)[0] as [ManagedKind, Record<string, unknown>];
  return { kind, spec };
}

/** A mark's kind. */
export function kindOf(mark: PageMarkInfo["mark"]): ManagedKind {
  return specOf(mark).kind;
}

/** The text of a watermark that says something: its words, or "Picture". */
function watermarkWords(spec: { mark: object }): string {
  const content = spec.mark as { text?: { text: string }; image?: unknown };
  return content.text === undefined
    ? laterMessage("pagemarks.manage.picture")
    : content.text.text.replaceAll("\n", " ");
}

/** What a mark says, in a line: its words, its slots or its first and last number. */
function summaryOf(group: MarkGroup): string {
  const { kind, spec } = specOf(group.first);
  switch (kind) {
    case "headerFooter": {
      const slots = spec.slots as (string | null)[];
      return slots.flatMap((text) => (text === null || text.trim() === "" ? [] : [showTokens(text)])).join(" · ");
    }
    case "bates": {
      const one = batesOf(spec, NO_FONTS);
      const last = batesOf(specOf(group.last).spec, NO_FONTS);
      return group.pages.length === 1
        ? batesText(one, one.start)
        : `${batesText(one, one.start)} … ${batesText(last, last.start)}`;
    }
    default:
      return watermarkWords(spec as { mark: object });
  }
}

/** The mark as the list shows it. */
export function managedOf(group: MarkGroup, labels: readonly string[]): ManagedMark {
  return {
    id: group.id,
    kind: kindOf(group.first),
    summary: summaryOf(group),
    pages: pagesText(group.pages, labels),
  };
}

/** What the dialog needs to read the values of a mark back. */
export interface EditContext {
  /** The registry font a name in the mark stands for, when this session still knows it. */
  readonly fontIdOf: (name: string) => string | undefined;
  /** The picture a blob names, when this session still has it. */
  readonly imageOf: (blob: string) => MarkImage | undefined;
}

/** A stored font list as the registry's font, Helvetica when the first isn't one the session knows. */
function fontOf(fonts: readonly string[] | undefined, context: EditContext): string {
  const name = fonts?.[0];
  return (name === undefined ? undefined : context.fontIdOf(name)) ?? "helvetica";
}

/** The values of a watermark or a background. */
function watermarkValues(
  kind: "watermark" | "background",
  spec: Record<string, unknown>,
  context: EditContext,
): MarkEdit {
  const mark = spec.mark as {
    text?: { text: string; size: number; fonts?: string[]; color?: number[] };
    image?: { blob: string };
  };
  const text = mark.text;
  const rgb = rgbOf(text?.color);
  const values: WatermarkValues = {
    ...WATERMARK_DEFAULT,
    source: text === undefined ? "image" : "text",
    text: text?.text ?? "",
    fontId: fontOf(text?.fonts, context),
    size: text?.size ?? WATERMARK_DEFAULT.size,
    colour: colourIdOf(rgb) ?? "grey",
    opacity: Math.round(((spec.opacity as number | undefined) ?? 1) * 100),
    angle: (spec.angle as number | undefined) ?? 0,
    layer: kind === "background" ? "behind" : "front",
    hideable: spec.hideable === true,
  };
  return {
    tab: "watermark",
    values,
    pages: [],
    image: mark.image === undefined ? undefined : context.imageOf(mark.image.blob),
  };
}

/** The values of a header and footer. */
function headerFooterValues(spec: Record<string, unknown>, context: EditContext): MarkEdit {
  const slots = (spec.slots as (string | null)[]).map((text) => showTokens(text ?? ""));
  const margins = (spec.margins ?? {}) as { top?: number; bottom?: number };
  const values: HeaderFooterValues = {
    ...HEADER_FOOTER_DEFAULT,
    slots: slots as unknown as HeaderFooterValues["slots"],
    fontId: fontOf(spec.fonts as string[] | undefined, context),
    size: (spec.size as number | undefined) ?? 12,
    top: margins.top ?? 36,
    bottom: margins.bottom ?? 36,
    dateFormat: HEADER_FOOTER_DEFAULT.dateFormat as DateFormat,
  };
  return { tab: "headerFooter", values, pages: [] };
}

/** Bates numbers as a page keeps them: the core leaves out what has its default (six digits, no prefix or suffix). */
function batesOf(spec: Record<string, unknown>, context: EditContext): BatesValues {
  return {
    ...BATES_DEFAULT,
    prefix: (spec.prefix as string | undefined) ?? "",
    start: spec.start as number,
    digits: (spec.digits as number | undefined) ?? 6,
    suffix: (spec.suffix as string | undefined) ?? "",
    slot: spec.slot as MarkSlot,
    fontId: fontOf(spec.fonts as string[] | undefined, context),
    size: (spec.size as number | undefined) ?? 12,
  };
}

/** The values of Bates numbers. */
function batesValues(spec: Record<string, unknown>, context: EditContext): MarkEdit {
  return { tab: "bates", values: batesOf(spec, context), pages: [] };
}

/** The dialog's values for changing `group`, on the pages it is on. */
export function editOf(group: MarkGroup, context: EditContext): MarkEdit {
  const { kind, spec } = specOf(group.first);
  const read =
    kind === "headerFooter"
      ? headerFooterValues(spec, context)
      : kind === "bates"
        ? batesValues(spec, context)
        : watermarkValues(kind, spec, context);
  return { ...read, pages: group.pages } as MarkEdit;
}
