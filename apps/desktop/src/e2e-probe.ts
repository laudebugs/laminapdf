// What an end-to-end test build puts on `globalThis` for the WebdriverIO scenarios (src/e2e.ts
// fills it in; e2e/ reads it). Types only, so the scenarios typecheck without the app.
import type { PageRenderState, ViewState } from "@laminapdf/viewer";

/** A page's render state changing, at `performance.now()` in this window. */
export interface Paint {
  readonly page: number;
  readonly state: PageRenderState;
  readonly at: number;
}

/** What the scenarios read through `browser.execute`. */
export interface E2eProbe {
  /** `performance.now()` in this window, to compare with paints' times. */
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
  /** The tile requests that failed, other than ones the view cancelled, as `name: message`. */
  readonly tileFailures: () => readonly string[];
}

declare global {
  // `var` is how a global property is declared in TypeScript.
  var __laminaE2e: E2eProbe | undefined;
}
