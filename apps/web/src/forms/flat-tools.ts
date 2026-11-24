// Flat-form presets for Fill & Sign (Paper P01, E8n): Text, Check and Date place borderless
// FreeText annotations (`typewriter` intent) where clicked, so a scanned or printed form with no
// AcroForm fields can still be filled. Text opens Comment mode's inline text editor; Check places
// a ✓ glyph (using the bundled Lamina Sans fallback, subsetted to that glyph); Date places
// today's date in the reader's locale format.

import type { Op, RichText } from "@laminapdf/sdk";
import { type FlatFormTool, isoToPdfDate, laterMessage } from "@laminapdf/ui/editing";
import type { Box, Point } from "../comment/marks.ts";
import { boxToUser, type ToUser } from "../comment/ops.ts";
import { isBlank } from "../comment/rich-text.ts";
import type { TextEdit, TextLook, TextNow } from "../comment/text-session.ts";

/** The tick placed by the Check tool. */
export const CHECK_GLYPH = "\u2713";

/** Black ink, 12 pt Helvetica, borderless: the look of flat-form text and dates. */
const FLAT_TEXT_LOOK: TextLook = {
  font: "helvetica",
  size: 12,
  align: "left",
  colour: [0, 0, 0],
  border: 0,
};

/** What placing a flat-form annotation needs of its page. */
export interface FlatPlacementContext {
  readonly pageId: string;
  readonly toUser: ToUser;
  readonly rotation: number;
  /** Optional bundled font blob ID for glyphs outside WinAnsi (such as ✓). */
  readonly fallback?: string | undefined;
  readonly created: Date;
}

/** Today's date in the reader's locale format, with spaces normalised for PDF text layout. */
export function localeDate(now: Date = new Date(), locale?: string): string {
  return new Intl.DateTimeFormat(locale).format(now).replace(/\s+/g, " ").trim();
}

/** Whether every character of `text` is printable ASCII or Latin-1 (covered by WinAnsi). */
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
export function flatTextEdit(page: number, [x, y]: Point): TextEdit {
  return {
    page,
    kind: "typewriter",
    id: undefined,
    rect: [x, Math.max(0, y - 8), x + 120, Math.max(16, y + 8)],
    text: [[]],
    look: FLAT_TEXT_LOOK,
    fallbacks: [],
  };
}

/** Builds a borderless FreeText `addAnnot` op and its undo label. */
function makeFreeTextOp(
  rect: Box,
  text: RichText,
  look: { readonly size: number; readonly align: "left" | "center" | "right" },
  context: FlatPlacementContext,
  stepKey: "form.step.addText" | "form.step.addCheck" | "form.step.addDate",
): { readonly ops: Op[]; readonly label: string } {
  const fallbacks = context.fallback !== undefined && needsGlyphFallback(text) ? [context.fallback] : undefined;
  const when = isoToPdfDate(context.created.toISOString()) ?? "";
  const op: Op = {
    op: "addAnnot",
    args: {
      page: context.pageId,
      spec: {
        type: "freeText",
        rect: boxToUser(context.toUser, rect),
        text,
        font: "helvetica",
        size: look.size,
        align: look.align,
        intent: { type: "typewriter" },
        border: { width: 0 },
        ...(context.rotation === 0 ? {} : { rotation: context.rotation }),
        ...(fallbacks === undefined ? {} : { fallbacks }),
      },
      common: {
        color: [0, 0, 0],
        created: when,
        modified: when,
      },
    },
  };
  return { ops: [op], label: laterMessage(stepKey) };
}

/** The `addAnnot` edit when typing in a flat-form Text box ends; undefined if left blank. */
export function flatTextFinished(
  now: TextNow,
  context: FlatPlacementContext,
): { readonly ops: Op[]; readonly label: string } | undefined {
  if (isBlank(now.text)) return undefined;
  return makeFreeTextOp(now.rect, now.text, { size: FLAT_TEXT_LOOK.size, align: "left" }, context, "form.step.addText");
}

/** The immediate `addAnnot` edit for clicking with the Check or Date tool at `point`. */
export function flatStampOp(
  tool: Exclude<FlatFormTool, "text">,
  [x, y]: Point,
  context: FlatPlacementContext,
  locale?: string,
): { readonly ops: Op[]; readonly label: string } {
  if (tool === "check") {
    const rect: Box = [Math.max(0, x - 8), Math.max(0, y - 9), Math.max(16, x + 8), Math.max(18, y + 9)];
    return makeFreeTextOp(
      rect,
      [[{ text: CHECK_GLYPH }]],
      { size: 14, align: "center" },
      context,
      "form.step.addCheck",
    );
  }
  const date = localeDate(context.created, locale);
  const rect: Box = [x, Math.max(0, y - 8), x + 100, Math.max(16, y + 8)];
  return makeFreeTextOp(rect, [[{ text: date }]], { size: 12, align: "left" }, context, "form.step.addDate");
}
