// The Page marks dialog's ops, apart from the DOM so Node can test them (Paper A03): what each tab's
// primary button writes, and what Update and Remove write. A mark's kind is the key of its
// spec, as the core keeps it. Every op names its pages by ID and carries the date it was made.

import type { Op } from "@laminapdf/sdk";
import type { BatesValues, HeaderFooterValues, WatermarkValues } from "@laminapdf/ui/editing";
import { writeTokens } from "@laminapdf/ui/editing";
import { rgbOfColour } from "../content/inspector-props.ts";
import { colour, modifiedNow, type Rgb } from "../content/ops.ts";

/** The kinds of mark the core keeps. */
type MarkKind = "watermark" | "background" | "headerFooter" | "bates";

/** A mark as the ops write it: its kind and the fields of that kind. */
export interface MarkSpec {
  readonly kind: MarkKind;
  readonly spec: Readonly<Record<string, unknown>>;
}

/** The ink of header, footer and Bates text. */
const INK: Rgb = rgbOfColour("ink");

/** A new mark's ID: 1 to 64 of letters, digits, `-`, `_` and `.`, the same on every page it is on. */
export function newMarkId(now = Date.now(), random = Math.random): string {
  return `mark-${now.toString(36)}-${Math.floor(random() * 36 ** 4).toString(36)}`;
}

/** A watermark or a background: text in `fonts`, or the picture `blob`. */
export function watermarkSpec(values: WatermarkValues, fonts: readonly string[], blob: string | undefined): MarkSpec {
  const mark =
    values.source === "image" && blob !== undefined
      ? { image: { blob } }
      : {
          text: { text: values.text, size: values.size, fonts: [...fonts], color: colour(rgbOfColour(values.colour)) },
        };
  return {
    kind: values.layer === "behind" ? "background" : "watermark",
    spec: {
      mark,
      ...(values.opacity < 100 ? { opacity: values.opacity / 100 } : {}),
      ...(values.angle === 0 ? {} : { angle: values.angle }),
      ...(values.hideable ? { hideable: true } : {}),
    },
  };
}

/** A header and footer: each slot's text with its tokens written as braces, `null` where a slot is empty. */
export function headerFooterSpec(
  values: HeaderFooterValues,
  fonts: readonly string[],
  date: string,
  file: string,
): MarkSpec {
  return {
    kind: "headerFooter",
    spec: {
      slots: values.slots.map((text) => (text.trim() === "" ? null : writeTokens(text))),
      fonts: [...fonts],
      size: values.size,
      color: colour(INK),
      margins: { top: values.top, bottom: values.bottom },
      date,
      file,
    },
  };
}

/** Bates numbers: `values.start` is the first page's number. */
export function batesSpec(values: BatesValues, fonts: readonly string[]): MarkSpec {
  return {
    kind: "bates",
    spec: {
      prefix: values.prefix,
      start: values.start,
      digits: values.digits,
      suffix: values.suffix,
      slot: values.slot,
      fonts: [...fonts],
      size: values.size,
      color: colour(INK),
    },
  };
}

/** The op that adds `mark` as `id` to `pages` (IDs). */
export function addMarkOp(pages: readonly string[], id: string, mark: MarkSpec, when?: Date): Op {
  const op = {
    watermark: "addWatermark",
    background: "addBackground",
    headerFooter: "setHeaderFooter",
    bates: "applyBates",
  }[mark.kind];
  return { op, args: { ...mark.spec, id, pages: [...pages], modified: modifiedNow(when) } } as unknown as Op;
}

/** The op that replaces mark `id` on `pages` with `mark`, of the same kind. */
export function updateMarkOp(pages: readonly string[], id: string, mark: MarkSpec, when?: Date): Op {
  return {
    op: "updatePageMark",
    args: { id, pages: [...pages], mark: { [mark.kind]: mark.spec }, modified: modifiedNow(when) },
  } as unknown as Op;
}

/** The op that takes mark `id` off `pages`. */
export function deleteMarkOp(pages: readonly string[], id: string): Op {
  return { op: "deletePageMark", args: { id, pages: [...pages] } } as unknown as Op;
}
