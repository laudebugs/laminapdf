// What the e2e build puts on `globalThis` for the scenarios: probe.ts fills it in inside the app,
// and the scenarios read it through `browser.execute`. Types only, so the scenarios typecheck
// without the app.
import type { PageRenderState, ViewState } from "@laminapdf/viewer";

/** A page's render state changing, at `performance.now()` in the app's window. */
export interface Paint {
  readonly page: number;
  readonly state: PageRenderState;
  readonly at: number;
}

/** What the scenarios read through `browser.execute`. */
export interface E2eProbe {
  /** `performance.now()` in the app's window, to compare with paints' times. */
  readonly now: () => number;
  /** The view's page, zoom and scroll, or undefined outside the shell. */
  readonly view: () => ViewState | undefined;
  /** Each mounted page's render state. */
  readonly pages: () => Readonly<Record<number, PageRenderState>>;
  /** Every change of a page's state since `since` (a `now()`), oldest first. */
  readonly paintsSince: (since: number) => readonly Paint[];
  /** Tiles the engine has answered since the window opened: rises only when it draws. */
  readonly tilesDrawn: () => number;
  /** Restarts of this window's engine that reopened its document. */
  readonly restarts: () => number;
  /** Pixels of page `index`'s canvases that aren't blank paper, on a sample grid. */
  readonly inkedPixels: (index: number) => number;
  /** Scrolls the view to its end, as a scroll bar dragged to the bottom would. */
  readonly scrollToEnd: () => void;
}

declare global {
  // `var` is how a global property is declared in TypeScript.
  var __laminaE2e: E2eProbe | undefined;
}
