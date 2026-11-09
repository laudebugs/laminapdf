// The benchmark: a canvas reader stand-in that scrolls 40 pages at a constant speed, zooms
// 100 % → 200 % → 400 % around the viewport centre, then pulls a burst of tiles as fast as
// the transport allows. Each tile is requested, received, turned into an ImageBitmap and
// painted with drawImage, as @laminapdf/viewer does.
import { invoke } from "@tauri-apps/api/core";
import { CorruptTile, type TileReq, TRANSPORTS, type TransportName, transport } from "./transports";

interface Doc {
  name: string;
  pages: [number, number][];
}
export interface Plan {
  label: string;
  mode: "quick" | "full" | "coi";
  docs: Doc[];
}
type Source = "pdf" | "synthetic";
interface Combo {
  transport: TransportName;
  source: Source;
  tile: 256 | 512;
  dpr: 1 | 2;
  inflight: number;
}
interface PageSlot {
  doc: number;
  page: number;
  w: number;
  h: number;
}

const PAGES = 40;
const GAP_CSS = 12;
const CSS_PER_PT = 4 / 3; // 100 % zoom: 96 CSS px per inch
const BITMAP_CACHE_BYTES = 256 << 20;

const canvas = document.getElementById("view") as HTMLCanvasElement;
const ctx = canvas.getContext("2d", { alpha: false }) as CanvasRenderingContext2D;
const status = document.getElementById("status") as HTMLElement;

const metrics = (msg: Record<string, unknown>) => invoke("metrics", { msg });
const log = (text: string) => {
  status.textContent = text;
  return metrics({ kind: "log", text });
};
/** The webview stopped sending frames (hidden or occluded window, display asleep, no
 * compositor): the run is void, not slow. */
export class Stalled extends Error {}
const STALL_MS = 3000;
const nextFrame = () =>
  new Promise<number>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Stalled(`no animation frame for ${STALL_MS} ms`)), STALL_MS);
    requestAnimationFrame((t) => {
      clearTimeout(timer);
      resolve(t);
    });
  });

function quantile(sorted: number[], q: number): number {
  if (sorted.length === 0) return Number.NaN;
  return sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
}
const round = (n: number, d = 2) => Math.round(n * 10 ** d) / 10 ** d;

function slots(plan: Plan, source: Source): PageSlot[] {
  return Array.from({ length: PAGES }, (_, i) => {
    const doc = source === "synthetic" ? 0 : 1 + (i % 3);
    const pages = plan.docs[doc].pages;
    const page = source === "synthetic" ? i : Math.floor(i / 3) % pages.length;
    const [w, h] = pages[page];
    return { doc, page, w, h };
  });
}

/** Page tops in CSS px at `zoom`, plus the total height. */
function layout(pages: PageSlot[], zoom: number): { tops: number[]; height: number } {
  const tops: number[] = [];
  let y = GAP_CSS;
  for (const p of pages) {
    tops.push(y);
    y += p.h * CSS_PER_PT * zoom + GAP_CSS;
  }
  return { tops, height: y };
}

interface Cached {
  bitmap: ImageBitmap;
  requested: number;
  painted: boolean;
}

interface Timing {
  scrollMs: number;
  holdMs: number;
  burst: number;
}

/** One measured (or priming) run of `combo`; returns its row and the tiles it asked for. */
async function runOnce(plan: Plan, combo: Combo, timing: Timing, verify: boolean, seen?: TileReq[]) {
  const t = transport(combo.transport);
  const pages = slots(plan, combo.source);
  const cssW = window.innerWidth;
  const cssH = window.innerHeight;
  canvas.width = Math.round(cssW * combo.dpr);
  canvas.height = Math.round(cssH * combo.dpr);
  const cacheCap = Math.max(16, Math.floor(BITMAP_CACHE_BYTES / (combo.tile * combo.tile * 4)));
  const cache = new Map<string, Cached>();
  const inflight = new Set<string>();
  const requested: TileReq[] = [];
  const latencies: number[] = []; // request → first painted (frame-quantised)
  const ready: number[] = []; // request → ImageBitmap ready
  let nextId = 1;
  let failure: unknown = null;
  let bytes = 0;
  let delivered = 0;
  let wasted = 0;
  let finished = false; // after the run, late tiles are closed, not cached

  // Long tasks where the webview reports them (Chromium: WebView2), else rAF gaps only.
  const longTaskApi = PerformanceObserver.supportedEntryTypes?.includes("longtask") ?? false;
  let longTasks = 0;
  const observer = longTaskApi ? new PerformanceObserver((list) => (longTasks += list.getEntries().length)) : null;

  // The refresh interval: the median rAF gap over half a second of idling.
  const idle: number[] = [];
  let last = await nextFrame();
  const idleEnd = last + 500;
  while (last < idleEnd) {
    const now = await nextFrame();
    idle.push(now - last);
    last = now;
  }
  const interval = quantile(idle.sort((a, b) => a - b), 0.5);

  await metrics({ kind: "start" });
  observer?.observe({ entryTypes: ["longtask"] });

  const request = (req: TileReq, key: string) => {
    inflight.add(key);
    requested.push(req);
    const started = performance.now();
    t.fetch(nextId++, req, verify).then(
      (d) => {
        inflight.delete(key);
        if (finished) {
          d.bitmap.close();
          return;
        }
        ready.push(performance.now() - started);
        bytes += d.bytes;
        delivered++;
        cache.set(key, { bitmap: d.bitmap, requested: started, painted: false });
        while (cache.size > cacheCap) {
          const [oldKey, old] = cache.entries().next().value as [string, Cached];
          if (!old.painted) wasted++;
          old.bitmap.close();
          cache.delete(oldKey);
        }
      },
      (e) => {
        inflight.delete(key);
        failure ??= e;
      },
    );
  };

  // Scroll then zoom, frame by frame.
  let zoom = 1;
  let geo = layout(pages, zoom);
  const scrollEnd = geo.height - cssH;
  let scrollY = 0;
  let zoomAt = 0;
  const fills: Record<number, number> = {};
  const gaps: number[] = [];
  let blankFrames = 0;
  const start = await nextFrame();
  let prev = start;
  const end = start + timing.scrollMs + 2 * timing.holdMs;
  let stalled: unknown = null;
  for (;;) {
    let now: number;
    try {
      now = await nextFrame();
    } catch (e) {
      stalled = e;
      break;
    }
    if (failure) break;
    gaps.push(now - prev);
    prev = now;
    if (now >= end) break;
    const elapsed = now - start;
    const want = elapsed < timing.scrollMs ? 1 : elapsed < timing.scrollMs + timing.holdMs ? 2 : 4;
    if (want !== zoom) {
      // Keep the document point under the viewport centre where it is.
      const centre = scrollY + cssH / 2;
      let i = geo.tops.findIndex((top, k) => centre < top + pages[k].h * CSS_PER_PT * zoom + GAP_CSS);
      if (i < 0) i = pages.length - 1;
      const frac = (centre - geo.tops[i]) / (pages[i].h * CSS_PER_PT * zoom);
      zoom = want;
      geo = layout(pages, zoom);
      scrollY = geo.tops[i] + frac * pages[i].h * CSS_PER_PT * zoom - cssH / 2;
      zoomAt = now;
    } else if (zoom === 1) {
      scrollY = (elapsed / timing.scrollMs) * scrollEnd;
    }

    ctx.fillStyle = "#888";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    let missing = 0;
    const scale = Math.round(zoom * CSS_PER_PT * combo.dpr * 1000);
    for (let k = 0; k < pages.length; k++) {
      const p = pages[k];
      const pageH = p.h * CSS_PER_PT * zoom;
      const top = geo.tops[k] - scrollY;
      if (top > cssH || top + pageH < 0) continue;
      const left = (cssW - p.w * CSS_PER_PT * zoom) / 2;
      const pxW = Math.max(1, Math.floor((p.w * scale) / 1000));
      const pxH = Math.max(1, Math.floor((p.h * scale) / 1000));
      const ox = Math.round(left * combo.dpr);
      const oy = Math.round(top * combo.dpr);
      // Only the tiles that intersect the viewport, in device pixels.
      const x0 = Math.max(0, Math.floor(-ox / combo.tile)) * combo.tile;
      const y0 = Math.max(0, Math.floor(-oy / combo.tile)) * combo.tile;
      for (let y = y0; y < pxH && oy + y < canvas.height; y += combo.tile) {
        for (let x = x0; x < pxW && ox + x < canvas.width; x += combo.tile) {
          const key = `${p.doc}/${p.page}/${scale}/${x}/${y}`;
          const hit = cache.get(key);
          if (hit) {
            cache.delete(key);
            cache.set(key, hit); // most recently used last
            ctx.drawImage(hit.bitmap, ox + x, oy + y);
            if (!hit.painted) {
              hit.painted = true;
              latencies.push(now - hit.requested);
            }
            continue;
          }
          missing++;
          ctx.fillStyle = "#fff";
          ctx.fillRect(ox + x, oy + y, Math.min(combo.tile, pxW - x), Math.min(combo.tile, pxH - y));
          if (!inflight.has(key) && inflight.size < combo.inflight) {
            request({ doc: p.doc, page: p.page, scaleMilli: scale, x, y, size: combo.tile }, key);
          }
        }
      }
    }
    if (missing > 0) blankFrames++;
    else if (zoom > 1 && !(zoom in fills)) fills[zoom] = now - zoomAt;
  }
  const runMs = prev - start;
  observer?.disconnect();
  if (stalled) {
    // Leave nothing running into the retry: drop this run's bitmaps and let its requests land.
    finished = true;
    for (const c of cache.values()) c.bitmap.close();
    while (inflight.size > 0) await new Promise((r) => setTimeout(r, 16));
    throw stalled;
  }

  // Burst: tiles this run already asked for (rendered and cached in Rust), as fast as possible.
  const burstReqs = (seen ?? requested).slice(0, timing.burst);
  let burstBytes = 0;
  const burstStart = performance.now();
  let cursor = 0;
  const worker = async () => {
    while (cursor < burstReqs.length && !failure) {
      const req = burstReqs[cursor++];
      try {
        const d = await t.fetch(nextId++, req, verify);
        burstBytes += d.bytes;
        d.bitmap.close();
      } catch (e) {
        failure ??= e;
      }
    }
  };
  await Promise.all(Array.from({ length: combo.inflight }, worker));
  const burstMs = performance.now() - burstStart;
  finished = true;
  for (const c of cache.values()) c.bitmap.close();
  // Let late tiles land before the next run starts.
  while (inflight.size > 0) await new Promise((r) => setTimeout(r, 16));

  const sortedGaps = [...gaps].sort((a, b) => a - b);
  const dropped = gaps.reduce((n, g) => n + Math.max(0, Math.round(g / interval) - 1), 0);
  const lat = latencies.sort((a, b) => a - b);
  const rdy = ready.sort((a, b) => a - b);
  const row = {
    ...combo,
    run_ms: round(runMs, 0),
    tiles: delivered,
    wasted_tiles: wasted,
    mb: round(bytes / 1048576),
    scroll_tiles_per_s: round((delivered / runMs) * 1000, 1),
    lat_p50_ms: round(quantile(lat, 0.5), 1),
    lat_p95_ms: round(quantile(lat, 0.95), 1),
    lat_p99_ms: round(quantile(lat, 0.99), 1),
    ready_p50_ms: round(quantile(rdy, 0.5), 1),
    ready_p95_ms: round(quantile(rdy, 0.95), 1),
    ready_p99_ms: round(quantile(rdy, 0.99), 1),
    frames: gaps.length,
    frame_interval_ms: round(interval),
    dropped_frames: dropped,
    max_gap_ms: round(sortedGaps.at(-1) ?? 0, 1),
    blank_frame_pct: round((blankFrames / Math.max(1, gaps.length)) * 100, 1),
    long_tasks: longTaskApi ? longTasks : "",
    zoom200_fill_ms: round(fills[2] ?? Number.NaN, 0),
    zoom400_fill_ms: round(fills[4] ?? Number.NaN, 0),
    burst_tiles: burstReqs.length,
    burst_tiles_per_s: round((burstReqs.length / burstMs) * 1000, 1),
    burst_mb_per_s: round((burstBytes / 1048576 / burstMs) * 1000, 1),
  };
  return { row, requested, failure };
}

/** Runs `attempt` again after a stall (up to 3 tries), logging each stall. */
async function retryStalls<T>(label: string, attempt: () => Promise<T>): Promise<T> {
  for (let tries = 1; ; tries++) {
    try {
      return await attempt();
    } catch (e) {
      if (!(e instanceof Stalled) || tries === 3) throw e;
      await log(`${label}: stalled (${e.message}), retrying`);
      await new Promise((r) => setTimeout(r, 5000));
    }
  }
}

async function measure(plan: Plan, combo: Combo, timing: Timing, run: number, file: string, seen: TileReq[]) {
  const label = `${combo.transport} ${combo.source} ${combo.tile}px dpr${combo.dpr} n${combo.inflight} run ${run}`;
  await log(label);
  return retryStalls(label, async () => {
    const { row, failure } = await runOnce(plan, combo, timing, false, seen);
    if (failure) throw failure;
    await metrics({ kind: "result", file, run, ...row });
    return row;
  });
}

/** Fills the Rust tile cache for a combo (the scroll and zoom script once), then checks every
 * transport against it with full verification. Returns the tiles the script asks for. */
async function prime(plan: Plan, combo: Combo, timing: Timing): Promise<TileReq[]> {
  const label = `prime ${combo.source} ${combo.tile}px dpr${combo.dpr}`;
  await log(label);
  const { requested, failure, row } = await retryStalls(label, () =>
    runOnce(plan, { ...combo, transport: "scheme-raw" }, timing, true),
  );
  if (failure) throw failure;
  await metrics({ kind: "result", file: "prime", run: 0, ...row });
  for (const name of TRANSPORTS) {
    await log(`verify ${name} ${combo.source} ${combo.tile}px dpr${combo.dpr}`);
    const t = transport(name);
    let id = 1;
    for (const req of requested.slice(0, timing.burst)) (await t.fetch(id++, req, true)).bitmap.close();
  }
  return requested;
}

export async function runBench(plan: Plan): Promise<void> {
  const full = plan.mode === "full";
  const timing: Timing = full ? { scrollMs: 8000, holdMs: 1500, burst: 96 } : { scrollMs: 3000, holdMs: 800, burst: 32 };
  const runs = full ? 5 : 1;
  const best = new Map<TransportName, number>(TRANSPORTS.map((t) => [t, 8]));
  try {
    const base: Omit<Combo, "transport" | "inflight"> = { source: "pdf", tile: 512, dpr: 2 };
    if (full) {
      // In-flight sweep at 512 px, DPR 2: keep the N with the highest burst throughput.
      const seen = await prime(plan, { ...base, transport: "scheme-raw", inflight: 8 }, timing);
      for (const name of TRANSPORTS) {
        let top = -1;
        for (const n of [4, 8, 16]) {
          const row = await measure(plan, { ...base, transport: name, inflight: n }, timing, 0, "sweep", seen);
          if (row.burst_tiles_per_s > top) {
            top = row.burst_tiles_per_s;
            best.set(name, n);
          }
        }
      }
    }
    const combos: Omit<Combo, "transport" | "inflight">[] = full
      ? [
          { source: "pdf", tile: 256, dpr: 1 },
          { source: "pdf", tile: 256, dpr: 2 },
          { source: "pdf", tile: 512, dpr: 1 },
          { source: "pdf", tile: 512, dpr: 2 },
          { source: "synthetic", tile: 512, dpr: 2 },
        ]
      : [base];
    for (const c of combos) {
      const seen = await prime(plan, { ...c, transport: "scheme-raw", inflight: 8 }, timing);
      const file = `${c.source}-${c.tile}-dpr${c.dpr}`;
      // Runs interleaved across transports, so drift (heat, background work) hits them alike.
      for (let run = 1; run <= runs; run++) {
        for (const name of TRANSPORTS) {
          await measure(plan, { ...c, transport: name, inflight: best.get(name) ?? 8 }, timing, run, file, seen);
        }
      }
    }
    await log("done");
    await metrics({ kind: "done", code: 0 });
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    await log(`FAILED: ${error}`);
    await metrics({ kind: "note", file: "failure", error });
    await metrics({ kind: "done", code: e instanceof CorruptTile ? 3 : e instanceof Stalled ? 4 : 1 });
  }
}
