// Flat-form presets for Fill & Sign (Paper P01): Text, Check and Date place borderless
// FreeText annotations (`typewriter` intent) where clicked, so a scanned or printed form with no
// AcroForm fields can still be filled. Text opens Comment mode's inline text editor; Check places
// a ✓ glyph (using the bundled Lamina Sans fallback, subsetted to that glyph); Date places
// today's date in the reader's locale format.

import type { Op, RichText } from "@laminapdf/sdk";
import { type FlatFormTool, laterMessage } from "@laminapdf/ui/editing";
import type { Box, Point } from "../comment/marks.ts";
import { isBlank } from "../comment/rich-text.ts";
import { addTextOp, type TextContext, type TextEdit, type TextLook, type TextNow } from "../comment/text-session.ts";

/** The tick placed by the Check tool. */
export const CHECK_GLYPH = "\u2713";

/** Default box dimensions (points) for each flat-form placement tool. */
const TEXT_SIZE = { width: 120, height: 16 } as const;
const CHECK_SIZE = { width: 16, height: 18 } as const;
const DATE_SIZE = { width: 100, height: 16 } as const;

/** Black ink, 12 pt Helvetica, borderless: the look of flat-form text and dates. */
const FLAT_TEXT_LOOK: TextLook = {
  font: "helvetica",
  size: 12,
  align: "left",
  colour: [0, 0, 0],
  border: 0,
};

/** Clamps a box of `size` to non-negative page coordinates around or starting at `[x, y]`. */
function clampedBox([x, y]: Point, size: { readonly width: number; readonly height: number }, centerX = false): Box {
  const x0 = centerX ? Math.max(0, x - size.width / 2) : Math.max(0, x);
  const y0 = Math.max(0, y - size.height / 2);
  return [x0, y0, x0 + size.width, y0 + size.height];
}

/** Today's date in the reader's locale format, with spaces normalised for PDF text layout. */
export function localeDate(now: Date = new Date(), locale?: string): string {
  return new Intl.DateTimeFormat(locale).format(now).replace(/\s+/g, " ").trim();
}

/** Whether any character of `text` lies outside Latin-1 (and needs a fallback font beyond WinAnsi). */
export function needsGlyphFallback(text: RichText): boolean {
  return text.some((p) =>
    p.some((s) =>
      [...s.text].some((ch) => {
        const code = ch.codePointAt(0) ?? 0;
        return code > 0xff;
      }),
    ),
  );
}

/** Opens an inline borderless text box at `point` on `page` for the flat-form Text tool. */
export function flatTextEdit(page: number, point: Point): TextEdit {
  return {
    page,
    kind: "typewriter",
    id: undefined,
    rect: clampedBox(point, TEXT_SIZE),
    text: [[]],
    look: FLAT_TEXT_LOOK,
    fallbacks: [],
  };
}

/** Builds a borderless FreeText `addAnnot` op via `addTextOp` and pairs it with its undo label. */
function makeFlatOp(
  page: number,
  rect: Box,
  text: RichText,
  look: TextLook,
  context: TextContext,
  stepKey: "form.step.addText" | "form.step.addCheck" | "form.step.addDate",
): { readonly ops: Op[]; readonly label: string } {
  const fallbacks = context.fallback !== undefined && needsGlyphFallback(text) ? [context.fallback] : [];
  const edit: TextEdit = { page, kind: "typewriter", id: undefined, rect, text, look, fallbacks };
  const op = addTextOp(edit, { text, rect, look }, context);
  return { ops: [op], label: laterMessage(stepKey) };
}

/** The `addAnnot` edit when typing in a flat-form Text box ends; undefined if left blank. */
export function flatTextFinished(
  edit: TextEdit,
  now: TextNow,
  context: TextContext,
): { readonly ops: Op[]; readonly label: string } | undefined {
  if (isBlank(now.text)) return undefined;
  return makeFlatOp(edit.page, now.rect, now.text, FLAT_TEXT_LOOK, context, "form.step.addText");
}

/** The immediate `addAnnot` edit for clicking with the Check or Date tool at `point`. */
export function flatStampOp(
  tool: Exclude<FlatFormTool, "text">,
  page: number,
  point: Point,
  context: TextContext,
  locale?: string,
): { readonly ops: Op[]; readonly label: string } {
  if (tool === "check") {
    const look: TextLook = { ...FLAT_TEXT_LOOK, size: 14, align: "center" };
    return makeFlatOp(
      page,
      clampedBox(point, CHECK_SIZE, true),
      [[{ text: CHECK_GLYPH }]],
      look,
      context,
      "form.step.addCheck",
    );
  }
  const date = localeDate(context.created, locale);
  return makeFlatOp(
    page,
    clampedBox(point, DATE_SIZE),
    [[{ text: date }]],
    FLAT_TEXT_LOOK,
    context,
    "form.step.addDate",
  );
}
