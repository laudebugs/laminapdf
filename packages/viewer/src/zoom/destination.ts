// Where a destination (ISO 32000-2 §12.3.2.2, Table 149) puts the view: Lamina PDF gives its page
// and its coordinates in page space (points from the displayed page's top-left, `/Rotate`
// applied); this turns them by the view's own rotation into fractions of the page box on screen,
// and its fit into a zoom. Fits are read as pdf.js reads them: `fit-width` fits the viewport's
// width whichever way the page is turned.

import type { Target } from "@laminapdf/sdk";
import { type PageSize, type Rotation, rotated } from "../layout/page-layout.js";
import { percentOfScale, type Viewport, type Zoom } from "./zoom.js";

/** Where to go: the zoom (`undefined` keeps the current one), and the page point for the viewport's top-left. */
export interface DestinationView {
  readonly zoom: Zoom | undefined;
  readonly page: number;
  /** Fraction of the page box on screen across (0..1); `undefined` keeps the horizontal scroll. */
  readonly x: number | undefined;
  /** Fraction down; `undefined` is the page's top. */
  readonly y: number | undefined;
}

type Pair = readonly [number | undefined, number | undefined];

/** Fractions (`x`, `y`) of the unturned page turned clockwise by `rotation`. */
function turn([x, y]: Pair, rotation: Rotation): Pair {
  const flip = (v: number | undefined) => (v === undefined ? undefined : 1 - v);
  switch (rotation) {
    case 90:
      return [flip(y), x];
    case 180:
      return [flip(x), flip(y)];
    case 270:
      return [y, flip(x)];
    default:
      return [x, y];
  }
}

/**
 * The view for `target` on a page of `size` (points, `/Rotate` applied) shown at `rotation`, in a
 * `viewport` with `gap` CSS px around pages. A coordinate off the page is clamped onto it.
 */
export function destinationView(
  target: Target,
  size: PageSize,
  rotation: Rotation,
  viewport: Viewport,
  gap: number,
): DestinationView {
  const fraction = (v: number | undefined, of: number) =>
    v === undefined || of <= 0 ? undefined : Math.min(1, Math.max(0, v / of));
  const at = (x: number | undefined, y: number | undefined) =>
    turn([fraction(x, size.width), fraction(y, size.height)], rotation);
  const shown = rotated(size, rotation);
  const room = { width: Math.max(1, viewport.width - 2 * gap), height: Math.max(1, viewport.height - 2 * gap) };
  const { page, view } = target;
  const go = (zoom: Zoom | undefined, [x, y]: Pair): DestinationView => ({ zoom, page, x, y });
  switch (view.kind) {
    case "at":
      return go(view.zoom === undefined ? undefined : view.zoom * 100, at(view.x, view.y));
    case "fit":
      return go("fit-page", [undefined, undefined]);
    case "fit-width":
      return go("fit-width", at(undefined, view.y));
    case "fit-height":
      return go(percentOfScale(room.height / shown.height), at(view.x, undefined));
    case "fit-rect": {
      const [ax, ay] = at(view.x0, view.y0);
      const [bx, by] = at(view.x1, view.y1);
      const [x0, x1] = [Math.min(ax ?? 0, bx ?? 0), Math.max(ax ?? 0, bx ?? 0)];
      const [y0, y1] = [Math.min(ay ?? 0, by ?? 0), Math.max(ay ?? 0, by ?? 0)];
      // The rectangle's size on screen in points.
      const w = Math.max((x1 - x0) * shown.width, 1);
      const h = Math.max((y1 - y0) * shown.height, 1);
      return go(percentOfScale(Math.min(room.width / w, room.height / h)), [x0, y0]);
    }
    default:
      return go(undefined, [undefined, undefined]);
  }
}
