// The harness's report: each browser's result checked against the web plan's §4 targets
// (docs/plans/web-app.md, "Non-functional"), the Markdown summary, and the figures the nightly
// history keeps (`cargo xtask viewer-perf --record`). The checks inform; nothing here fails a run.

import type { Counters, MemorySample, Recording } from "./types.ts";

/** The §4 targets. */
export const TARGETS = {
  firstPageWarmMs: 700,
  firstPageColdMs: 3000,
  longestTaskMs: 50,
  toSharpMs: 300,
  memoryBytes: 600 * 1024 * 1024,
  /** The second pass may end at most this much above the first (the plan's "stable"). */
  memoryGrowth: 0.05,
} as const;

/** The machine a run took its numbers on (official numbers come from the reference machine). */
export interface Machine {
  readonly cpu: string;
  readonly cores: number;
  readonly memoryBytes: number;
  readonly os: string;
}

/** One browser's run. */
export interface BrowserResult {
  readonly browser: string;
  readonly version: string;
  readonly quick: boolean;
  readonly machine: Machine;
  /** The 10-page file: warm in every browser, cold on Fast 4G in Chromium only (null elsewhere). */
  readonly firstPage: { readonly warmMs: number; readonly coldMs: number | null };
  /** The 1,000-page file scrolled end to end twice. */
  readonly scroll: {
    readonly passes: readonly [Recording, Recording];
    /** Memory once each pass has stopped and its pages are sharp. */
    readonly settled: readonly [MemorySample, MemorySample];
    /** ms until the visible pages were sharp, at each stop. */
    readonly toSharpMs: readonly number[];
    readonly counters: Counters;
  };
  /** The 300-page image file, flung across and stopped. */
  readonly fling: { readonly toSharpMs: number; readonly counters: Counters };
  /** ctrl+wheel bursts on the 10-page file. */
  readonly zoom: { readonly recording: Recording; readonly toSharpMs: readonly number[] };
  /**
   * The 1,000-page file stepped through in a phone's profile (a coarse pointer, so the viewer's
   * touch budgets), against those budgets; null in a browser without one (Firefox).
   */
  readonly mobile: {
    readonly device: string;
    readonly peakMemory: MemorySample;
    /** The canvases and bitmap cache the viewer's budgets allow on that device, in bytes. */
    readonly budgetBytes: number;
  } | null;
}

/** One row of the report: `pass` is null where nothing was measured or there is no target. */
export interface Check {
  readonly name: string;
  readonly target: string;
  readonly value: string;
  readonly pass: boolean | null;
}

const ms = (value: number) => `${Math.round(value)} ms`;
const mb = (bytes: number) => `${Math.round(bytes / (1024 * 1024))} MB`;

/** The longest main-thread task while scrolling: the Long Tasks API where there is one, else the longest frame. */
function longestTask(result: BrowserResult): Check {
  const name = "Longest main-thread task while scrolling";
  const target = `≤ ${TARGETS.longestTaskMs} ms`;
  const [first, second] = result.scroll.passes;
  if (first.longTasks !== null && second.longTasks !== null) {
    // The API reports only tasks over 50 ms, so any entry misses the target.
    const tasks = [...first.longTasks, ...second.longTasks];
    if (tasks.length === 0) return { name, target, value: "none over 50 ms", pass: true };
    return { name, target, value: `${tasks.length} over 50 ms, longest ${ms(Math.max(...tasks))}`, pass: false };
  }
  const longest = Math.max(first.longestFrameMs, second.longestFrameMs);
  return { name, target, value: `${ms(longest)} (longest frame)`, pass: longest <= TARGETS.longestTaskMs };
}

/** The checks for one browser, in the order the summary lists them. */
export function checks(result: BrowserResult): Check[] {
  const { firstPage, scroll, fling, zoom } = result;
  const sharp = Math.max(...scroll.toSharpMs, fling.toSharpMs);
  const peak = Math.max(scroll.passes[0].peakMemory.total, scroll.passes[1].peakMemory.total);
  const [first, second] = scroll.settled;
  const growth = second.total / first.total - 1;
  const painted = scroll.counters.drawsOnUnmounted + fling.counters.drawsOnUnmounted;
  return [
    {
      name: "First page, 10-page file, warm",
      target: `≤ ${ms(TARGETS.firstPageWarmMs)}`,
      value: ms(firstPage.warmMs),
      pass: firstPage.warmMs <= TARGETS.firstPageWarmMs,
    },
    {
      name: "First page, 10-page file, cold on Fast 4G",
      target: `≤ ${ms(TARGETS.firstPageColdMs)}`,
      value: firstPage.coldMs === null ? "not measured (Chromium only)" : ms(firstPage.coldMs),
      pass: firstPage.coldMs === null ? null : firstPage.coldMs <= TARGETS.firstPageColdMs,
    },
    longestTask(result),
    {
      name: "Pages sharp after scrolling stops",
      target: `≤ ${ms(TARGETS.toSharpMs)}`,
      value: ms(sharp),
      pass: sharp <= TARGETS.toSharpMs,
    },
    {
      name: "Memory, 1,000 pages end to end",
      target: `≤ ${mb(TARGETS.memoryBytes)}`,
      value: mb(peak),
      pass: peak <= TARGETS.memoryBytes,
    },
    {
      name: "Memory after the second pass",
      target: `≤ first pass + ${TARGETS.memoryGrowth * 100} %`,
      value: `${growth >= 0 ? "+" : ""}${(growth * 100).toFixed(1)} %`,
      pass: second.total <= first.total * (1 + TARGETS.memoryGrowth),
    },
    { name: "Cancelled renders that painted", target: "0", value: String(painted), pass: painted === 0 },
    mobileMemory(result),
    {
      name: "Zoom bursts: longest frame, slowest to sharp",
      target: "—",
      value: `${ms(zoom.recording.longestFrameMs)}, ${ms(Math.max(...zoom.toSharpMs))}`,
      pass: null,
    },
  ];
}

/** The phone's canvases and bitmaps at their peak, against the budgets the viewer gives that device. */
function mobileMemory({ mobile }: BrowserResult): Check {
  const name = "Canvases and bitmaps on a phone, 1,000 pages";
  if (mobile === null)
    return { name, target: "≤ the touch budgets", value: "not measured (no phone profile)", pass: null };
  const used = mobile.peakMemory.canvases + mobile.peakMemory.bitmaps;
  return {
    name,
    target: `≤ ${mb(mobile.budgetBytes)} (${mobile.device})`,
    value: mb(used),
    pass: used <= mobile.budgetBytes,
  };
}

const mark = (pass: boolean | null) => (pass === null ? "" : pass ? " (pass)" : " (miss)");

/** The Markdown summary: one column per browser, then how the numbers were taken. */
export function summary(results: readonly BrowserResult[]): string {
  const rows = results.map(checks);
  const head = `| Check | Target | ${results.map((r) => `${r.browser} ${r.version}`).join(" | ")} |`;
  const lines = [
    "## Viewer performance",
    "",
    head,
    `| --- | --- | ${results.map(() => "---").join(" | ")} |`,
    ...(rows[0] ?? []).map(
      (check, i) =>
        `| ${check.name} | ${check.target} | ${rows.map((row) => `${row[i]?.value}${mark(row[i]?.pass ?? null)}`).join(" | ")} |`,
    ),
    "",
  ];
  const machine = results[0]?.machine;
  if (machine !== undefined) {
    lines.push(`Taken on ${machine.cpu} (${machine.cores} cores, ${mb(machine.memoryBytes)}), ${machine.os}.`);
  }
  lines.push(
    "Firefox and WebKit have no Long Tasks API: their longest task is the longest gap between frames, an upper bound. " +
      "Only Chromium reports the page's JS heap; every browser counts the worker's WebAssembly memory, the bitmap cache and the canvases.",
  );
  if (results.some((r) => r.quick))
    lines.push("Quick run: the first 100 pages only, so the numbers aren't comparable with a full run.");
  return `${lines.join("\n")}\n`;
}

/**
 * The figures the nightly history keeps for one browser, in `cargo xtask bench`'s units:
 * nanoseconds for times, bytes for memory. Each is "more is worse", as its regression check assumes.
 */
export function historyEntries(result: BrowserResult): Record<string, number> {
  const ns = (value: number) => Math.round(value * 1e6);
  const entries: Record<string, number> = {
    "first-page-warm": ns(result.firstPage.warmMs),
    "scroll-longest-frame": ns(Math.max(...result.scroll.passes.map((p) => p.longestFrameMs))),
    "to-sharp": ns(Math.max(...result.scroll.toSharpMs, result.fling.toSharpMs)),
    "zoom-to-sharp": ns(Math.max(...result.zoom.toSharpMs)),
    "memory-peak-bytes": Math.round(Math.max(...result.scroll.passes.map((p) => p.peakMemory.total))),
  };
  if (result.firstPage.coldMs !== null) entries["first-page-cold"] = ns(result.firstPage.coldMs);
  if (result.mobile !== null) {
    entries["mobile-memory-peak-bytes"] = Math.round(
      result.mobile.peakMemory.canvases + result.mobile.peakMemory.bitmaps,
    );
  }
  return entries;
}
