// End-to-end test builds only (Vite mode `e2e`, which tauri.e2e.conf.json sets): WebdriverIO's
// frontend plugin, and what the scenarios read instead of sleeping. A release build never
// imports this file; CI checks its bundle has none of it (D2e).
import "@wdio/tauri-plugin";
import type { PageRenderState } from "@laminapdf/viewer";
import { invoke } from "@tauri-apps/api/core";
import type { DesktopEngine } from "./app.tsx";
import type { Paint } from "./e2e-probe.ts";

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
  // Counts the engine's answers at the client, the one door every tile comes through.
  let tilesDrawn = 0;
  const tileFailures: string[] = [];
  const client = engine?.client;
  if (client !== undefined) {
    const render = client.render.bind(client);
    client.render = async (request, options) => {
      try {
        const tile = await render(request, options);
        tilesDrawn++;
        return tile;
      } catch (error) {
        if (!(error instanceof Error && error.name === "AbortError")) {
          tileFailures.push(error instanceof Error ? `${error.name}: ${error.message}` : String(error));
        }
        throw error;
      }
    };
  }
  let restarts = 0;
  engine?.client.on("restarted", ({ reopened }) => {
    if (reopened) restarts++;
  });
  reportInteractive();
  globalThis.__laminaE2e = {
    now: () => performance.now(),
    view: () => engine?.viewer.getState(),
    pages: () => engine?.viewer.stats().pages ?? {},
    paintsSince: (since) => paints.filter((paint) => paint.at >= since),
    tilesDrawn: () => tilesDrawn,
    restarts: () => restarts,
    tileFailures: () => tileFailures,
  };
}

/**
 * Tells the main process when the empty state first accepts input (its Choose a file button is on
 * screen), for the cold start `e2e/cold-start.ts` measures. Checked each frame.
 */
function reportInteractive(): void {
  const check = () => {
    const ready = [...document.querySelectorAll("button")].some((b) => b.textContent === "Choose a file");
    if (ready) void invoke("e2e_interactive");
    else requestAnimationFrame(check);
  };
  check();
}
