// End-to-end test builds only (Vite mode `e2e`, which tauri.e2e.conf.json sets): WebdriverIO's
// frontend plugin, and what the scenarios read instead of sleeping. A release build never
// imports this file; CI checks its bundle has none of it (D2e).
import "@wdio/tauri-plugin";
import type { PageRenderState, ViewState } from "@laminapdf/viewer";
import type { DesktopEngine } from "./app.tsx";

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
}

declare global {
  // `var` is how a global property is declared in TypeScript.
  var __laminaE2e: E2eProbe | undefined;
}

/** The changes kept: enough for any scenario, bounded for a long run. */
const KEPT = 2_000;

/**
 * Puts the probe on `globalThis` for the scenarios.
 * @param engine - This window's engine, or undefined outside the shell.
 */
export function exposeProbe(engine: DesktopEngine | undefined): void {
  const paints: Paint[] = [];
  let last: Readonly<Record<number, PageRenderState>> = {};
  engine?.viewer.on("renderstats", ({ pages }) => {
    const at = performance.now();
    for (const [key, state] of Object.entries(pages)) {
      const page = Number(key);
      if (last[page] !== state) paints.push({ page, state, at });
    }
    if (paints.length > KEPT) paints.splice(0, paints.length - KEPT);
    last = pages;
  });
  globalThis.__laminaE2e = {
    now: () => performance.now(),
    view: () => engine?.viewer.getState(),
    pages: () => engine?.viewer.stats().pages ?? {},
    paintsSince: (since) => paints.filter((paint) => paint.at >= since),
  };
}
