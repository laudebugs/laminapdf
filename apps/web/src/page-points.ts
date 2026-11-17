// Where on the page the pointer and the text selection are, in page space (points from the
// displayed page's top-left, ADR 0027) with the page they're on, whatever the view's turn: what
// the link tool and Comment mode's tools read.

import type { Viewer } from "@laminapdf/viewer";
import type { Engine } from "./session.ts";

/** A box in page space: left, top, right, bottom, in points from the page's top-left. */
type Box = readonly [number, number, number, number];

/** A point of the page (fractions 0 to 1) as shown in a view turned clockwise by `rotation`. */
export function turn(x: number, y: number, rotation: number): [number, number] {
  switch (rotation) {
    case 90:
      return [1 - y, x];
    case 180:
      return [1 - x, 1 - y];
    case 270:
      return [y, 1 - x];
    default:
      return [x, y];
  }
}

/** A point of the page as shown, turned back to the page's own orientation: the turn the other way. */
const unturn = (x: number, y: number, rotation: number) => turn(x, y, (360 - rotation) % 360);

/** The page-space point under the pointer, with its page. */
export function pointOn(viewer: Viewer, engine: Engine, event: { readonly clientX: number; readonly clientY: number }) {
  const r = viewer.element.getBoundingClientRect();
  const at = viewer.pointAt(event.clientX - r.left, event.clientY - r.top);
  const size = at === undefined ? undefined : engine.getState().pages[at.page];
  if (at === undefined || size === undefined) return undefined;
  const [x, y] = unturn(at.x, at.y, viewer.getState().rotation);
  return { page: at.page, x: x * size.width, y: y * size.height };
}

/** The selected text's lines on its first page, in page space. */
export function selectedLines(viewer: Viewer, engine: Engine, rects: readonly DOMRect[]) {
  const r = viewer.element.getBoundingClientRect();
  const rotation = viewer.getState().rotation;
  const lines: Box[] = [];
  let page: number | undefined;
  for (const rect of rects) {
    const a = viewer.pointAt(rect.left - r.left, rect.top - r.top);
    const b = viewer.pointAt(rect.right - r.left, rect.bottom - r.top);
    if (a === undefined || b === undefined || a.page !== b.page) continue;
    page ??= a.page;
    const size = engine.getState().pages[a.page];
    if (a.page !== page || size === undefined) continue;
    const [ax, ay] = unturn(a.x, a.y, rotation);
    const [bx, by] = unturn(b.x, b.y, rotation);
    const box: Box = [
      Math.min(ax, bx) * size.width,
      Math.min(ay, by) * size.height,
      Math.max(ax, bx) * size.width,
      Math.max(ay, by) * size.height,
    ];
    // A line the browser split into pieces is one line: join a piece that shares its band.
    const same = lines.findIndex((l) => Math.abs((l[1] + l[3]) / 2 - (box[1] + box[3]) / 2) < (box[3] - box[1]) / 2);
    const line = lines[same];
    if (line === undefined) lines.push(box);
    else
      lines[same] = [
        Math.min(line[0], box[0]),
        Math.min(line[1], box[1]),
        Math.max(line[2], box[2]),
        Math.max(line[3], box[3]),
      ];
  }
  if (page === undefined || lines.length === 0) return undefined;
  const bounds: Box = [
    Math.min(...lines.map((l) => l[0])),
    Math.min(...lines.map((l) => l[1])),
    Math.max(...lines.map((l) => l[2])),
    Math.max(...lines.map((l) => l[3])),
  ];
  return { page, box: bounds, lines };
}
