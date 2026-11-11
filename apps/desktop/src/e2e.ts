// End-to-end test builds only (Vite mode `e2e`, which tauri.e2e.conf.json sets): WebdriverIO's
// frontend plugin, and what the scenarios read instead of sleeping. A release build never
// imports this file; CI checks its bundle has none of it (D2e).
import "@wdio/tauri-plugin";
import type { PageRenderState, ViewState } from "@laminapdf/viewer";
import type { DesktopEngine } from "./app.tsx";

/** What the scenarios read through `browser.execute`. */
export interface E2eProbe {
  /** The view's page, zoom and scroll, or undefined outside the shell. */
  readonly view: () => ViewState | undefined;
  /** Each mounted page's render state. */
  readonly pages: () => Readonly<Record<number, PageRenderState>>;
}

declare global {
  // `var` is how a global property is declared in TypeScript.
  var __laminaE2e: E2eProbe | undefined;
}

/**
 * Puts the probe on `globalThis` for the scenarios.
 * @param engine - This window's engine, or undefined outside the shell.
 */
export function exposeProbe(engine: DesktopEngine | undefined): void {
  globalThis.__laminaE2e = {
    view: () => engine?.viewer.getState(),
    pages: () => engine?.viewer.stats().pages ?? {},
  };
}
