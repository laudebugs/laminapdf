// The harness page: the viewer over one fixture, and `window.__laminaPerf` (perf/types.ts), which
// perf/run.ts drives with Playwright. Only this page has the hook; the package ships none of it.
// With `?open=<name>` it opens that fixture at once, fetching it while the backend loads, as an
// app does when a file is chosen during startup (the cold scenario).

import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { createViewer, ViewerClient, type Zoom } from "@laminapdf/viewer";
import type { MemorySample, PerfHook, Recording } from "../types.ts";

const host = document.querySelector<HTMLDivElement>("#viewer");
if (host === null) throw new Error("the page has no #viewer");

async function fixture(name: string): Promise<Blob> {
  const response = await fetch(`./${name}.pdf`);
  if (!response.ok) throw new Error(`fixture ${name}: HTTP ${response.status}`);
  return response.blob();
}

const auto = new URLSearchParams(location.search).get("open");
const autoFile = auto === null ? undefined : fixture(auto);

const client = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
const viewer = createViewer({ client });
viewer.mount(host);

const frame = () => new Promise<number>((resolve) => requestAnimationFrame(resolve));

/** Resolves once `done()` holds, checking each frame; false if `limitMs` passes first. */
async function until(done: () => boolean, limitMs: number): Promise<boolean> {
  const start = performance.now();
  while (!done()) {
    if (performance.now() - start > limitMs) return false;
    await frame();
  }
  return true;
}

const firstPainted = () => {
  const state = viewer.stats().pages[0];
  return state === "low" || state === "sharp";
};

/** Every page in view is sharp (or failed, which also ends its rendering). */
function allSharp(): boolean {
  const { first, last } = viewer.getState().visibleRange;
  const pages = viewer.stats().pages;
  for (let i = first; i <= last; i++) {
    if (pages[i] !== "sharp" && pages[i] !== "error") return false;
  }
  return last >= first;
}

async function memory(): Promise<MemorySample> {
  const reported = (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory;
  const heap = reported?.usedJSHeapSize ?? null;
  const worker = await client.memory();
  const bitmaps = viewer.stats().cacheBytes;
  let canvases = 0;
  for (const canvas of viewer.element.querySelectorAll("canvas")) canvases += canvas.width * canvas.height * 4;
  const wasm = worker.backend + worker.sdk;
  return { heap, wasm, bitmaps, canvases, total: (heap ?? 0) + wasm + bitmaps + canvases };
}

interface Live {
  readonly frames: number[];
  readonly longTasks: number[];
  last: number | undefined;
  peak: MemorySample | undefined;
  readonly sampler: ReturnType<typeof setInterval>;
}
let live: Live | undefined;

const longTaskApi = PerformanceObserver.supportedEntryTypes.includes("longtask");
if (longTaskApi) {
  new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) live?.longTasks.push(entry.duration);
  }).observe({ type: "longtask" });
}

function tick(now: number): void {
  if (live === undefined) return;
  if (live.last !== undefined) live.frames.push(now - live.last);
  live.last = now;
  requestAnimationFrame(tick);
}

async function sample(): Promise<void> {
  const reading = await memory();
  if (live !== undefined && (live.peak === undefined || reading.total > live.peak.total)) live.peak = reading;
}

const hook: PerfHook = {
  firstPaintMs: null,
  async open(name) {
    const file = await fixture(name);
    const start = performance.now();
    viewer.open(await client.open(file));
    await until(firstPainted, 60_000);
    return performance.now() - start;
  },
  setZoom: (zoom) => viewer.setZoom(zoom as Zoom),
  goToPage: (index) => viewer.goToPage(index),
  scroll: () => ({ top: viewer.element.scrollTop, max: viewer.element.scrollHeight - viewer.element.clientHeight }),
  record() {
    live = { frames: [], longTasks: [], last: undefined, peak: undefined, sampler: setInterval(sample, 500) };
    requestAnimationFrame(tick);
  },
  async stop(): Promise<Recording> {
    const done = live;
    if (done === undefined) throw new Error("stop() without record()");
    await sample();
    clearInterval(done.sampler);
    live = undefined;
    return {
      longestFrameMs: Math.max(0, ...done.frames),
      framesOver50: done.frames.filter((ms) => ms > 50).length,
      longTasks: longTaskApi ? done.longTasks : null,
      peakMemory: done.peak ?? (await memory()),
    };
  },
  async untilSharp(limitMs = 20_000) {
    // The view has stopped once its scroll position and zoom hold for three frames; the clock
    // runs from the last frame that changed either, so a smooth scroll's tail counts.
    let previous = "";
    let changed = performance.now();
    let still = 0;
    await until(() => {
      const { scrollTop, percent } = viewer.getState();
      const now = `${scrollTop}:${percent}`;
      if (now === previous) still++;
      else [previous, changed, still] = [now, performance.now(), 0];
      return still >= 3;
    }, limitMs);
    await until(allSharp, limitMs);
    return Math.min(performance.now() - changed, limitMs);
  },
  memory,
  counters() {
    const { drawsOnUnmounted, aborted, dropped, peakMounted, peakCanvases } = viewer.stats();
    return { drawsOnUnmounted, aborted, dropped, peakMounted, peakCanvases };
  },
};

if (autoFile !== undefined) {
  viewer.open(await client.open(await autoFile));
  await until(firstPainted, 60_000);
  hook.firstPaintMs = performance.now();
}
globalThis.__laminaPerf = hook;
