// Inside the app, in end-to-end test builds only (Vite mode `e2e`, which tauri.e2e.conf.json
// sets): WebdriverIO's frontend plugin, the probe the scenarios read instead of sleeping, and the
// cold start's report. main.tsx imports this file only in that mode; CI checks a release bundle
// has none of it. It lives under e2e/, not src/, because src/ holds no test hooks.
import "@wdio/tauri-plugin";
import type { PageRenderState } from "@laminapdf/viewer";
import { invoke } from "@tauri-apps/api/core";
import type { DesktopEngine } from "../../src/app.tsx";
import type { Paint } from "./types.ts";

/** The page state changes kept: enough for any scenario, bounded for a long run. */
const MAX_PAINTS = 2_000;

/** Samples per side of a canvas when counting its inked pixels. */
const GRID = 32;

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
    if (paints.length > MAX_PAINTS) paints.splice(0, paints.length - MAX_PAINTS);
    last = pages;
  });
  // Counts the engine's answers at the client, the one door every tile comes through.
  let tilesDrawn = 0;
  const client = engine?.client;
  if (client !== undefined) {
    const render = client.render.bind(client);
    client.render = async (request, options) => {
      const tile = await render(request, options);
      tilesDrawn++;
      return tile;
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
    inkedPixels: (index) => inkedPixels(engine?.viewer.element, index),
    scrollToEnd: () => {
      const view = engine?.viewer.element;
      if (view !== undefined) view.scrollTop = view.scrollHeight;
    },
  };
}

/** Pixels on a `GRID` × `GRID` sample of page `index`'s canvases that are neither white nor clear. */
function inkedPixels(view: HTMLElement | undefined, index: number): number {
  let inked = 0;
  for (const canvas of view?.querySelectorAll<HTMLCanvasElement>(`[data-page-index="${index}"] canvas`) ?? []) {
    inked += inkedOnCanvas(canvas);
  }
  return inked;
}

/** One canvas's inked samples: alpha above zero and some channel darker than near-white. */
function inkedOnCanvas(canvas: HTMLCanvasElement): number {
  const context = canvas.getContext("2d");
  if (context === null || canvas.width === 0 || canvas.height === 0) return 0;
  const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
  let inked = 0;
  for (let row = 0; row < GRID; row++) {
    for (let column = 0; column < GRID; column++) {
      const x = Math.floor(((column + 0.5) * canvas.width) / GRID);
      const y = Math.floor(((row + 0.5) * canvas.height) / GRID);
      const at = (y * canvas.width + x) * 4;
      const darkest = Math.min(data[at] ?? 255, data[at + 1] ?? 255, data[at + 2] ?? 255);
      if ((data[at + 3] ?? 0) > 0 && darkest < 240) inked++;
    }
  }
  return inked;
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
