// The find highlights: on every mounted page, a box over each match, in the page's
// `lamina-layer-highlight` slot. Like the text layer, the boxes sit in page points inside one box
// that a CSS transform scales and turns to the page, so a zoom or a rotation moves none of them.
// Each match quad is drawn as its bounding box. The current match gets its own class. The viewer
// adds no style sheet: colours come from `--lamina-find-match` and `--lamina-find-current`.
// The boxes lie over the page's drawing, so the box that holds them is blended with it
// (`--lamina-find-blend`: multiply, for a light page, by default; screen for a dark one), which
// leaves the ink under a match as drawn. The blend is on that box, not on each mark: the box is
// transformed, so it is a group of its own, and a mark blended inside it would blend with nothing.
// Each match also gets an unblended frame, in a second box, so it shows over a picture too: a
// 1 px bottom edge (`--lamina-find-match-edge`), and on the current match an outline
// (`--lamina-find-current-outline`), so the current one never differs by colour alone. The boxes
// take no pointer events, so they never stand in a selection's way.

import type { Rotation } from "../layout/page-layout.js";
import type { PageLayer } from "../view/viewer.js";
import { placeBox } from "./box.js";

/** What the highlight layer reads of the find controller and the viewer. */
export interface HighlightHost {
  /** Page `page`'s matches (quads, 8 numbers per run) and which one is current, if any. */
  matchesOn(page: number): { readonly matches: readonly (readonly number[])[]; readonly current: number | undefined };
  /** Page `page`'s displayed size in points. */
  points(page: number): { readonly width: number; readonly height: number } | undefined;
  rotation(): Rotation;
}

/** One mounted page's highlights: the blended fills and the unblended frames. */
interface PageHighlights {
  readonly box: HTMLDivElement;
  readonly frames: HTMLDivElement;
  size: { readonly width: number; readonly height: number } | undefined;
}

/** A box in page points over the page, with `style` on top of the placement every box has. */
function layerBox(className: string, style: Partial<CSSStyleDeclaration> = {}): HTMLDivElement {
  const box = document.createElement("div");
  box.className = className;
  Object.assign(box.style, {
    position: "absolute",
    left: "0",
    top: "0",
    transformOrigin: "0 0",
    pointerEvents: "none",
    ...style,
  });
  return box;
}

/** The axis-aligned box around a quad's four corners. */
function bounds(q: readonly number[], at: number): { x: number; y: number; width: number; height: number } {
  const xs = [q[at], q[at + 2], q[at + 4], q[at + 6]].map((v) => v ?? 0);
  const ys = [q[at + 1], q[at + 3], q[at + 5], q[at + 7]].map((v) => v ?? 0);
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}

/** The find highlights of one viewer. */
export class HighlightLayer implements PageLayer {
  readonly name = "highlight";
  readonly #host: HighlightHost;
  readonly #pages = new Map<number, PageHighlights>();

  constructor(host: HighlightHost) {
    this.#host = host;
  }

  onMount(page: number, slot: HTMLElement): void {
    const box = layerBox("lamina-highlight-layer", { mixBlendMode: "var(--lamina-find-blend, multiply)" });
    const frames = layerBox("lamina-highlight-frames");
    slot.append(box, frames);
    this.#pages.set(page, { box, frames, size: undefined });
    this.#draw(page);
  }

  onUnmount(page: number): void {
    const view = this.#pages.get(page);
    view?.box.remove();
    view?.frames.remove();
    this.#pages.delete(page);
  }

  onZoom(page: number, _slot: HTMLElement, size: { readonly width: number; readonly height: number }): void {
    const view = this.#pages.get(page);
    if (view === undefined) return;
    view.size = size;
    this.#place(page, view);
  }

  /** Draws the pages again: matches arrived or went, the current one moved, or the view turned. */
  refresh(pages?: Iterable<number>): void {
    for (const page of pages ?? this.#pages.keys()) {
      const view = this.#pages.get(page);
      if (view === undefined) continue;
      this.#place(page, view);
      this.#draw(page);
    }
  }

  #place(page: number, view: PageHighlights): void {
    const points = this.#host.points(page);
    if (points === undefined || view.size === undefined) return;
    for (const box of [view.box, view.frames]) placeBox(box, points, view.size, this.#host.rotation());
  }

  #draw(page: number): void {
    const view = this.#pages.get(page);
    if (view === undefined) return;
    const { matches, current } = this.#host.matchesOn(page);
    const marks: HTMLElement[] = [];
    const frames: HTMLElement[] = [];
    matches.forEach((quads, index) => {
      for (let at = 0; at + 8 <= quads.length; at += 8) {
        const r = bounds(quads, at);
        const isCurrent = index === current;
        const place = {
          position: "absolute",
          left: `${r.x}px`,
          top: `${r.y}px`,
          width: `${r.width}px`,
          height: `${r.height}px`,
          boxSizing: "border-box",
        };
        const mark = document.createElement("div");
        mark.className = isCurrent ? "lamina-find-match lamina-find-current" : "lamina-find-match";
        Object.assign(mark.style, {
          ...place,
          background: isCurrent
            ? "var(--lamina-find-current, rgba(237, 191, 106, 0.6))"
            : "var(--lamina-find-match, rgba(246, 227, 188, 0.6))",
        });
        const frame = document.createElement("div");
        frame.className = "lamina-find-frame";
        Object.assign(frame.style, {
          ...place,
          ...(isCurrent
            ? { outline: "2px solid var(--lamina-find-current-outline, #8a5300)" }
            : { borderBottom: "1px solid var(--lamina-find-match-edge, #8a5300)" }),
        });
        marks.push(mark);
        frames.push(frame);
      }
    });
    view.box.replaceChildren(...marks);
    view.frames.replaceChildren(...frames);
  }
}
