// Where a signature image goes and what it is written as (E10c, Paper G01), apart from the DOM so
// it runs in Node. A signature image is a picture, never a digital signature: it is written with
// the ops any picture uses, a Stamp annotation (the default), an Ink annotation that keeps a drawn
// signature as strokes, or an image added to the page's own content, which the signature rules
// treat as a change to what was signed (ADR 0007 §3).

import type { Op } from "@laminapdf/sdk";
import { isoToPdfDate, laterMessage } from "@laminapdf/ui/editing";
import type { Box, Point } from "../comment/marks.ts";
import { boxToUser, type ToUser } from "../comment/ops.ts";

/** How a signature image is written to the file. */
export type SignaturePlacement = "stamp" | "ink" | "content";

/** A signature image ready to place: its picture in the open document and, if drawn, its strokes. */
export interface ReadySignature {
  /** The picture's ID in the document's blob store. */
  readonly blob: string;
  /** The picture's size in pixels, which fixes its proportions. */
  readonly width: number;
  readonly height: number;
  /** A drawn signature's strokes in pixels, `[x, y]` points inside `width` by `height`. */
  readonly strokes?: readonly (readonly (readonly [number, number])[])[];
  /** The ink's colour, RGB 0 to 1; black when absent. */
  readonly ink?: readonly [number, number, number];
}

/** The longer side of a signature image placed without a field, in points. */
export const SIGNATURE_SIDE = 160;
/** The pen width of a signature placed as ink, in points. */
export const INK_WIDTH = 1.6;

/** The box a signature of `width` by `height` pixels takes: fitted into `field`, else centred on `at`. */
export function signatureBox(size: { width: number; height: number }, target: { field?: Box; at?: Point }): Box {
  const ratio = Math.max(size.width, 1) / Math.max(size.height, 1);
  const field = target.field;
  if (field !== undefined) {
    const [x0, y0, x1, y1] = field;
    const w = x1 - x0;
    const h = y1 - y0;
    const fitted = Math.min(w, h * ratio);
    const fh = fitted / ratio;
    const cx = (x0 + x1) / 2;
    const cy = (y0 + y1) / 2;
    return [cx - fitted / 2, cy - fh / 2, cx + fitted / 2, cy + fh / 2];
  }
  const [cx, cy] = target.at ?? [0, 0];
  const w = ratio >= 1 ? SIGNATURE_SIDE : SIGNATURE_SIDE * ratio;
  const h = w / ratio;
  return [cx - w / 2, cy - h / 2, cx + w / 2, cy + h / 2];
}

/** Which placements are open: Ink needs a drawn signature, and the page's content needs no certification. */
export function placementChoices(drawn: boolean, certified: boolean): readonly SignaturePlacement[] {
  return ["stamp", ...(drawn ? (["ink"] as const) : []), ...(certified ? [] : (["content"] as const))];
}

/** Where an edit goes: the page by its ID, its mapping and turn, and who makes it when. */
export interface SignatureContext {
  readonly pageId: string;
  readonly toUser: ToUser;
  readonly rotation: number;
  readonly author: string | undefined;
  readonly created: Date;
  /** An ID for an image added to the page's content, unique on the page. */
  readonly contentId: string;
}

/** The op that writes `signature` in `box` as `placement`. */
export function placeSignatureOp(
  signature: ReadySignature,
  placement: SignaturePlacement,
  box: Box,
  context: SignatureContext,
): Op {
  const date = isoToPdfDate(context.created.toISOString()) ?? "";
  if (placement === "content") {
    const [x0, y0, x1, y1] = box;
    return {
      op: "addContentObject",
      args: {
        id: context.contentId,
        page: context.pageId,
        object: { image: { blob: signature.blob } },
        // The unit square to the box, in page space (top-left origin, y down, ADR 0027).
        matrix: [x1 - x0, 0, 0, y1 - y0, x0, y0],
        alt: { text: laterMessage("sign.alt") },
        modified: date,
      },
    } as Op;
  }
  const common = {
    ...(context.author === undefined || context.author === "" ? {} : { author: context.author }),
    created: date,
    modified: date,
  };
  if (placement === "ink") {
    const [r = 0, g = 0, b = 0] = signature.ink ?? [];
    return {
      op: "addAnnot",
      args: {
        page: context.pageId,
        spec: { type: "ink", strokes: inkStrokes(signature, box, context.toUser), width: INK_WIDTH },
        common: { ...common, color: [r, g, b] as [number, number, number] },
      },
    };
  }
  return {
    op: "addAnnot",
    args: {
      page: context.pageId,
      spec: {
        type: "stamp",
        rect: boxToUser(context.toUser, box),
        ...(context.rotation === 0 ? {} : { rotation: context.rotation }),
        stamp: { image: signature.blob },
      },
      common,
    },
  };
}

/** A drawn signature's strokes scaled from its pixels into `box`, then into user space. */
function inkStrokes(signature: ReadySignature, box: Box, toUser: ToUser) {
  const [x0, y0, x1, y1] = box;
  const sx = (x1 - x0) / Math.max(signature.width, 1);
  const sy = (y1 - y0) / Math.max(signature.height, 1);
  return (signature.strokes ?? []).map((stroke) => ({
    points: stroke.map(([x, y]) => toUser([x0 + x * sx, y0 + y * sy])),
  }));
}
