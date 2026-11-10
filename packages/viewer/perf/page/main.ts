// The harness page: the viewer over one fixture, and `window.__laminaPerf` (perf/types.ts), which
// perf/run.ts drives with Playwright. Only this page has the hook; the package ships none of it.
// With `?open=<name>` it opens that fixture at once, fetching it while the backend loads, as an
// app does when a file is chosen during startup (the cold scenario).

import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { createViewer, type LinkPreviewRequest, ViewerClient, type Zoom } from "@laminapdf/viewer";
import type { FindTiming, MemorySample, PerfHook, Recording } from "../types.ts";

const host = document.querySelector<HTMLDivElement>("#viewer");
if (host === null) throw new Error("the page has no #viewer");

async function fixture(name: string): Promise<Blob> {
  const response = await fetch(`./${name}.pdf`);
  if (!response.ok) throw new Error(`fixture ${name}: HTTP ${response.status}`);
  return response.blob();
}

const params = new URLSearchParams(location.search);
const auto = params.get("open");
// `&remote` opens it by its address, with Range requests from the document worker, as the reader
// opens a linked file; otherwise the page fetches it whole, as a file chosen on the device arrives.
const autoFile =
  auto === null
    ? undefined
    : params.has("remote")
      ? Promise.resolve({ url: new URL(`./${auto}.pdf`, location.href).href })
      : fixture(auto);

const client = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
const viewer = createViewer({ client });
viewer.mount(host);

const frame = () => new Promise<number>((resolve) => requestAnimationFrame(resolve));

/** Resolves once `done()` holds, checking each frame; throws `what` if `limitMs` passes first. */
async function until(done: () => boolean, limitMs: number, what: string): Promise<void> {
  const start = performance.now();
  while (!done()) {
    if (performance.now() - start > limitMs) throw new Error(`${what} within ${limitMs} ms`);
    await frame();
  }
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

// When the view last scrolled or zoomed, and since when every visible page has been sharp (null
// while one isn't). Checked on each event and each frame, so the moment a page turns sharp is
// caught when `renderstats` reports it, not when the harness next asks.
let lastChange = performance.now();
let sharpSince: number | null = null;
function checkSharp(): void {
  if (!allSharp()) sharpSince = null;
  else sharpSince ??= performance.now();
}
viewer.element.addEventListener("scroll", () => {
  lastChange = performance.now();
  checkSharp();
});
viewer.on("zoomchange", () => {
  lastChange = performance.now();
  checkSharp();
});
viewer.on("renderstats", checkSharp);
function watch(): void {
  checkSharp();
  requestAnimationFrame(watch);
}
requestAnimationFrame(watch);

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
    await until(firstPainted, 60_000, `${name}: the first page didn't paint`);
    return performance.now() - start;
  },
  setZoom: (zoom) => viewer.setZoom(zoom as Zoom),
  async hoverLinks(count) {
    const links = [...viewer.element.querySelectorAll<HTMLElement>("a.lamina-link")]
      .filter((a) => a.getAttribute("aria-label")?.startsWith("Link to page") === true)
      .slice(0, count);
    const times: number[] = [];
    for (const link of links) {
      const asked = new Promise<LinkPreviewRequest>((resolve) => {
        const off = viewer.on("linkpreview", (request) => {
          off();
          resolve(request);
        });
      });
      const at = { bubbles: true, pointerType: "mouse", pointerId: 1 };
      link.dispatchEvent(new PointerEvent("pointerover", at));
      const request = await asked;
      const start = performance.now();
      if (request.shows.kind === "target") await viewer.preview(request.shows.target);
      times.push(performance.now() - start);
      link.dispatchEvent(new PointerEvent("pointerout", { ...at, relatedTarget: viewer.element }));
      viewer.dismissLinkPreview();
    }
    return times;
  },
  find(query) {
    return new Promise<FindTiming>((resolve) => {
      const start = performance.now();
      let firstResultMs: number | null = null;
      const off = viewer.find.subscribe(() => {
        const state = viewer.find.getState();
        if (state.query !== query) return;
        if (firstResultMs === null && state.total > 0) firstResultMs = performance.now() - start;
        if (!state.done) return;
        off();
        resolve({ firstResultMs, fullMs: performance.now() - start, total: state.total });
      });
      viewer.find.find(query);
    });
  },
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
    // The view has stopped once nothing moved or zoomed it for 50 ms; then wait for the pages.
    await until(() => performance.now() - lastChange >= 50, limitMs, "the view didn't stop moving");
    await until(() => sharpSince !== null, limitMs, "the visible pages weren't sharp");
    // Pages sharp before the last change (a scroll within sharp pages) took no time.
    return Math.max(0, (sharpSince ?? lastChange) - lastChange);
  },
  memory,
  counters() {
    const { drawsOnUnmounted, aborted, dropped, peakMounted, peakCanvases } = viewer.stats();
    return { drawsOnUnmounted, aborted, dropped, peakMounted, peakCanvases };
  },
};

if (autoFile !== undefined) {
  viewer.open(await client.open(await autoFile));
  await until(firstPainted, 60_000, `${auto}: the first page didn't paint`);
  hook.firstPaintMs = performance.now();
}
globalThis.__laminaPerf = hook;
