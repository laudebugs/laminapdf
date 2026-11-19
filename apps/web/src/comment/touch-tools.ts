// The touch tools of a phone's annotate bar and a tablet's tool shelf (C08, C09), beside the
// desktop's in tools.ts: the Highlighter, which marks text when it starts over text and otherwise
// draws a translucent stroke; the Eraser, which removes whole ink strokes it passes over; and the
// Lasso, which selects the comments a loop holds.

import { onText, type Point } from "./marks.ts";
import { type DrawHost, inkTool, type MarkTool, textTool } from "./tools.ts";

/** The interaction modes these tools add, by declaration merging. */
declare module "@laminapdf/ui" {
  interface InteractionModes {
    highlighter: true;
    eraser: true;
    lasso: true;
  }
}

/** What the Eraser and the Lasso need of Comment mode, beside the drawing tools' host. */
export interface TouchHost extends DrawHost {
  /** Removes the ink strokes `path` passes over on `page`, as one step. */
  eraseAlong(page: number, path: readonly Point[]): void;
  /** Selects the comments on `page` that the loop `path` holds. */
  selectInside(page: number, path: readonly Point[]): void;
}

/** Highlighter: over text, a highlight of the words dragged across; elsewhere, a translucent stroke. */
function highlighterTool(host: DrawHost): MarkTool {
  const words = textTool("highlight", host);
  // A highlighter's line is as wide as its tip, however hard it is pressed.
  const stroke = inkTool(host, { pressure: false });
  let on: MarkTool | undefined;
  return {
    name: "highlighter",
    cursor: "crosshair",
    selectsText: false,
    pointerDown(event, toolHost) {
      const at = host.at(event);
      if (at === undefined || event.button !== 0) return false;
      const text = host.text(at.page);
      on = text !== undefined && onText(text, at.point) ? words : stroke;
      return on.pointerDown?.(event, toolHost) ?? false;
    },
    pointerMove(event, toolHost) {
      on?.pointerMove?.(event, toolHost);
    },
    pointerUp(event, toolHost) {
      on?.pointerUp?.(event, toolHost);
    },
    done() {
      words.done();
      stroke.done();
    },
    cancel() {
      words.cancel();
      stroke.cancel();
    },
  };
}

/** A tool that follows a press as a path and hands it over on release. */
function pathTool(
  name: "eraser" | "lasso",
  cursor: string,
  host: DrawHost,
  done: (page: number, path: Point[]) => void,
  /** Whether the path shows as it is drawn, as the Lasso's loop does. */
  shown: boolean,
): MarkTool {
  let press: { page: number; path: Point[] } | undefined;
  const clear = (page: number) => {
    if (shown) host.finish(page, undefined);
  };
  return {
    name,
    cursor,
    selectsText: false,
    pointerDown(event) {
      const at = host.at(event);
      if (at === undefined || event.button !== 0) return false;
      press = { page: at.page, path: [at.point] };
      return true;
    },
    pointerMove(event) {
      const at = host.at(event);
      if (press === undefined || at === undefined || at.page !== press.page) return;
      press.path.push(at.point);
      if (shown) host.draw(press.page, { type: "poly", kind: "polygon", vertices: press.path });
    },
    pointerUp(event) {
      if (press === undefined) return;
      const { page, path } = press;
      press = undefined;
      clear(page);
      if (event.type !== "pointercancel") done(page, path);
    },
    done() {
      this.cancel();
    },
    cancel() {
      if (press !== undefined) clear(press.page);
      press = undefined;
    },
  };
}

/** Eraser (C08): a tap or a drag takes every ink stroke it passes over, a drawing with its last. */
function eraserTool(host: TouchHost): MarkTool {
  return pathTool("eraser", "cell", host, (page, path) => host.eraseAlong(page, path), false);
}

/** Lasso (C09): a loop drawn round comments selects them, to move, recolour or delete. */
function lassoTool(host: TouchHost): MarkTool {
  return pathTool("lasso", "crosshair", host, (page, path) => host.selectInside(page, path), true);
}

/** The touch tools. */
export function touchTools(host: TouchHost): MarkTool[] {
  return [highlighterTool(host), eraserTool(host), lassoTool(host)];
}
