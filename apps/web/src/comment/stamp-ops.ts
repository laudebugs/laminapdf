// The edits of stamps and attached files (Paper C05), apart from the DOM so they run in Node: a
// stamp's box where the pointer places it, a dynamic stamp's lines resolved from the name and the
// time when it is placed, the `addAnnot` of a stamp or an attached file in user space, and the
// one edit that replaces an attached file: the annotation deleted and a new one made where it
// was, with its icon and colour, so Undo takes both back together.

import type { AnnotSpec, Op } from "@laminapdf/sdk";
import {
  type AttachmentIconName,
  type DynamicStampName,
  isoToPdfDate,
  laterMessage,
  type PickerStamp,
} from "@laminapdf/ui/editing";
import type { Box, Point } from "./marks.ts";
import { boxToUser, type ToUser } from "./ops.ts";

/** A stamp ready to place: what it shows and its size on the page, in points. */
export type ReadyStamp =
  | { readonly kind: "standard"; readonly name: PickerStamp }
  | { readonly kind: "dynamic"; readonly name: DynamicStampName; readonly lines: readonly string[] }
  | { readonly kind: "image"; readonly blob: string; readonly width: number; readonly height: number };

/** A standard stamp's size, points: Acrobat's, near enough. */
const STANDARD: readonly [number, number] = [180, 50];
/** A dynamic stamp's size: room for its title and its line. */
const DYNAMIC: readonly [number, number] = [200, 60];
/** A custom image stamp's longer side, points: its proportions are kept. */
const IMAGE_SIDE = 150;

/** The stamp's size on the page, points. */
export function stampSize(stamp: ReadyStamp): readonly [number, number] {
  if (stamp.kind === "standard") return STANDARD;
  if (stamp.kind === "dynamic") return DYNAMIC;
  const scale = IMAGE_SIDE / Math.max(stamp.width, stamp.height, 1);
  return [stamp.width * scale, stamp.height * scale];
}

/** The box a stamp of `size` takes, centred on `at`. */
export function stampBox(at: Point, [w, h]: readonly [number, number]): Box {
  return [at[0] - w / 2, at[1] - h / 2, at[0] + w / 2, at[1] + h / 2];
}

/** Each dynamic stamp's colour: green approves, red refuses, blue the rest. */
const DYNAMIC_STYLE: Readonly<Record<DynamicStampName, "green" | "red" | "blue">> = {
  approved: "green",
  reviewed: "blue",
  received: "blue",
  notApproved: "red",
};

/** "29 Oct 2026, 10:14", in the reader's language. */
export function stampTime(when: Date): string {
  return when.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

/** A dynamic stamp's lines: its title, then "by R. Osei · 29 Oct 2026, 10:14", or the time alone with no name. */
export function dynamicLines(name: DynamicStampName, author: string | undefined, when: Date): string[] {
  const time = stampTime(when);
  const line = author === undefined || author === "" ? time : laterMessage("stamps.dynamic.by", { name: author, time });
  return [laterMessage(`stamps.dynamic.${name}`), line];
}

/** The spec of `stamp` in `box` on a page `toUser` maps and `rotation` turns. */
function stampSpec(stamp: ReadyStamp, box: Box, toUser: ToUser, rotation: number): AnnotSpec {
  const kind =
    stamp.kind === "standard"
      ? { standard: stamp.name }
      : stamp.kind === "image"
        ? { image: stamp.blob }
        : { dynamic: { lines: [...stamp.lines], style: DYNAMIC_STYLE[stamp.name] } };
  return {
    type: "stamp",
    rect: boxToUser(toUser, box),
    ...(rotation === 0 ? {} : { rotation }),
    stamp: kind,
  };
}

/** What an annotation's common entries carry: its author and dates, and a colour when it has one. */
function common(author: string | undefined, created: Date, color?: readonly number[]) {
  const date = isoToPdfDate(created.toISOString()) ?? "";
  return {
    ...(color === undefined ? {} : { color: [...color] }),
    ...(author === undefined || author === "" ? {} : { author }),
    created: date,
    modified: date,
  };
}

/** Where an edit goes: the page by its ID, its mapping and turn, and who makes it when. */
export interface PlaceContext {
  readonly pageId: string;
  readonly toUser: ToUser;
  readonly rotation: number;
  readonly author: string | undefined;
  readonly created: Date;
}

/** The `addAnnot` that places `stamp` centred on `at`. */
export function placeStampOp(stamp: ReadyStamp, at: Point, context: PlaceContext): Op {
  const box = stampBox(at, stampSize(stamp));
  return {
    op: "addAnnot",
    args: {
      page: context.pageId,
      spec: stampSpec(stamp, box, context.toUser, context.rotation),
      common: common(context.author, context.created),
    },
  } as Op;
}

/** A file to attach: its bytes' blob ID, name, type and date. */
export interface FileToAttach {
  readonly blob: string;
  readonly name: string;
  readonly mime: string;
  readonly modified: Date;
  readonly description?: string;
}

/** The `addAnnot` that attaches `file` under an icon whose top-left corner is `at`. */
export function attachFileOp(
  file: FileToAttach,
  at: Point,
  context: PlaceContext,
  look: { readonly icon?: AttachmentIconName; readonly color?: readonly number[] } = {},
): Op {
  return {
    op: "addAnnot",
    args: {
      page: context.pageId,
      spec: {
        type: "fileAttachment",
        at: context.toUser(at),
        icon: look.icon ?? "paperclip",
        file: {
          file: file.blob,
          name: file.name,
          mime: file.mime,
          modified: isoToPdfDate(file.modified.toISOString()) ?? "",
          ...(file.description === undefined || file.description === "" ? {} : { description: file.description }),
        },
      },
      common: common(context.author, context.created, look.color),
    },
  } as Op;
}

/**
 * The edit that replaces attachment `id`'s file with `file`: the annotation deleted and a new one
 * at its icon's corner `at`, with its icon, colour and description, as one step.
 */
export function replaceFileOps(
  id: string,
  file: FileToAttach,
  at: Point,
  context: PlaceContext,
  look: { readonly icon?: AttachmentIconName; readonly color?: readonly number[] },
): Op[] {
  return [{ op: "deleteAnnot", args: { id } }, attachFileOp(file, at, context, look)];
}
