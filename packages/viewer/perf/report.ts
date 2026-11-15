// The harness's report: each browser's result checked against the web plan's §4 targets
// (docs/plans/web-app.md, "Non-functional"), the Markdown summary, and the figures the nightly
// history keeps (`cargo xtask viewer-perf --record`). The checks inform; nothing here fails a run.

import type { Counters, EditTiming, FindTiming, MemorySample, PageEditKind, Recording } from "./types.ts";

/** The §4 targets. */
export const TARGETS = {
  firstPageWarmMs: 700,
  firstPageColdMs: 3000,
  longestTaskMs: 50,
  /** Find on the 1,000-page file: the first result, and every page searched (§4's search row). */
  findFirstResultMs: 500,
  findFullMs: 10_000,
  toSharpMs: 300,
  memoryBytes: 600 * 1024 * 1024,
  /** The second pass may end at most this much above the first (the plan's "stable"). */
  memoryGrowth: 0.05,
  /** An edit on the 100-page file, to the page's first new pixels (the E1 plan's §6 budget). */
  editFirstPixelsMs: 150,
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
  /**
   * The 10-page file: warm in every browser; cold on Fast 4G in Chromium only (null elsewhere),
   * from the device (`coldMs`, the release gate's) and from a link (`coldLinkMs`, reported).
   */
  readonly firstPage: { readonly warmMs: number; readonly coldMs: number | null; readonly coldLinkMs: number | null };
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
   * Find on the 1,000-page file from page 1: a word on every page and one on page 778 only, while
   * the view scrolls.
   */
  readonly find: { readonly common: FindTiming; readonly rare: FindTiming; readonly recording: Recording };
  /** 20 citations hovered in a row on the paper (T5a): ms from dwell to crop for each, reported only. */
  readonly linkPreviews: { readonly toCropMs: readonly number[]; readonly recording: Recording };
  /**
   * The page in view turned a quarter in files of 10, 100 and 1,000 pages: each one's median turn
   * of the first five, and on a file turned more often, the last turn's first pixels.
   */
  readonly edit: readonly (EditTiming & {
    readonly pages: number;
    readonly lastOf?: number;
    readonly lastFirstPixelsMs?: number;
  })[];
  /**
   * Edits that change the page list on the 1,000-page file, the median of three each: from the
   * call to every visible page's first new pixels, against the same budget as an edit.
   */
  readonly pageEdits: readonly (EditTiming & { readonly kind: PageEditKind })[];
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
  return { ...longestTaskOf(...result.scroll.passes), name: "Longest main-thread task while scrolling" };
}

/** The longest main-thread task across `recordings`: the Long Tasks API's, else the longest frame. */
function longestTaskOf(...recordings: readonly Recording[]): Check {
  const name = "Longest main-thread task";
  const target = `≤ ${TARGETS.longestTaskMs} ms`;
  const lists = recordings.map((r) => r.longTasks);
  if (lists.every((l) => l !== null)) {
    // The API reports only tasks over 50 ms, so any entry misses the target.
    const tasks = lists.flatMap((l) => l ?? []);
    if (tasks.length === 0) return { name, target, value: "none over 50 ms", pass: true };
    return { name, target, value: `${tasks.length} over 50 ms, longest ${ms(Math.max(...tasks))}`, pass: false };
  }
  const longest = Math.max(...recordings.map((r) => r.longestFrameMs));
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
      name: "First page, 10-page file on the device, cold on Fast 4G",
      target: `≤ ${ms(TARGETS.firstPageColdMs)}`,
      value: firstPage.coldMs === null ? "not measured (Chromium only)" : ms(firstPage.coldMs),
      pass: firstPage.coldMs === null ? null : firstPage.coldMs <= TARGETS.firstPageColdMs,
    },
    {
      name: "First page, 10-page file from a link, cold on Fast 4G",
      target: `≤ ${ms(TARGETS.firstPageColdMs)}`,
      value: firstPage.coldLinkMs === null ? "not measured (Chromium only)" : ms(firstPage.coldLinkMs),
      pass: firstPage.coldLinkMs === null ? null : firstPage.coldLinkMs <= TARGETS.firstPageColdMs,
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
    ...findChecks(result),
    ...linkPreviewChecks(result),
    ...editChecks(result),
    ...pageEditChecks(result),
    mobileMemory(result),
    {
      name: "Zoom bursts: longest frame, slowest to sharp",
      target: "—",
      value: `${ms(zoom.recording.longestFrameMs)}, ${ms(Math.max(...zoom.toSharpMs))}`,
      pass: null,
    },
  ];
}

/** Find's first result and full pass against §4, and its longest task while the view scrolls. */
function findChecks({ find }: BrowserResult): Check[] {
  const first = find.common.firstResultMs;
  const full = Math.max(find.common.fullMs, find.rare.fullMs);
  return [
    {
      name: "Find, 1,000 pages: first result",
      target: `≤ ${ms(TARGETS.findFirstResultMs)}`,
      value: first === null ? "no match" : ms(first),
      pass: first === null ? false : first <= TARGETS.findFirstResultMs,
    },
    {
      name: "Find, 1,000 pages: every page searched",
      target: `≤ ${ms(TARGETS.findFullMs)}`,
      value: `${ms(find.common.fullMs)} (common word), ${ms(find.rare.fullMs)} (rare word)`,
      pass: full <= TARGETS.findFullMs,
    },
    { ...longestTaskOf(find.recording), name: "Longest main-thread task while finding and scrolling" },
  ];
}

/** Link previews: the slowest dwell-to-crop and the longest task while hovering, reported (no target). */
function linkPreviewChecks({ linkPreviews }: BrowserResult): Check[] {
  const slowest = linkPreviews.toCropMs.length === 0 ? 0 : Math.max(...linkPreviews.toCropMs);
  return [
    {
      name: "Link previews, 20 citations: dwell to crop (slowest)",
      target: "—",
      value: ms(slowest),
      pass: null,
    },
    {
      ...longestTaskOf(linkPreviews.recording),
      name: "Longest main-thread task while previewing links",
      target: "—",
      pass: null,
    },
  ];
}

/** An edit to the page's first new pixels (its quick pass) per file size, against the plan's budget at 100 pages. */
function editChecks({ edit }: BrowserResult): Check[] {
  const hundred = edit.find((e) => e.pages === 100);
  const rows: Check[] = [
    {
      name: `Edit to the page's first new pixels (quick pass), ${edit.map((e) => e.pages.toLocaleString("en")).join(" / ")} pages`,
      target: `≤ ${ms(TARGETS.editFirstPixelsMs)} at 100 pages`,
      value: edit.map((e) => ms(e.firstPixelsMs)).join(" / "),
      pass: hundred === undefined ? null : hundred.firstPixelsMs <= TARGETS.editFirstPixelsMs,
    },
  ];
  for (const e of edit) {
    if (e.lastOf === undefined || e.lastFirstPixelsMs === undefined) continue;
    rows.push({
      name: `Edit ${e.lastOf} in a row, ${e.pages.toLocaleString("en")} pages: first new pixels`,
      target: "—",
      value: ms(e.lastFirstPixelsMs),
      pass: null,
    });
  }
  return rows;
}

/** What a page-list edit does, in words. */
const PAGE_EDIT_NAMES: Record<PageEditKind, string> = {
  delete: "Delete one page",
  move: "Move 200 pages",
  insert: "Insert 10 blank pages",
};

/** Each page-list edit to the visible pages' first new pixels, against the edit budget (E2 plan §5). */
function pageEditChecks({ pageEdits }: BrowserResult): Check[] {
  return pageEdits.map((e) => ({
    name: `${PAGE_EDIT_NAMES[e.kind]}, 1,000 pages: first new pixels`,
    target: `≤ ${ms(TARGETS.editFirstPixelsMs)}`,
    value: ms(e.firstPixelsMs),
    pass: e.firstPixelsMs <= TARGETS.editFirstPixelsMs,
  }));
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
    "find-full-pass": ns(Math.max(result.find.common.fullMs, result.find.rare.fullMs)),
    "memory-peak-bytes": Math.round(Math.max(...result.scroll.passes.map((p) => p.peakMemory.total))),
  };
  if (result.firstPage.coldMs !== null) entries["first-page-cold"] = ns(result.firstPage.coldMs);
  if (result.firstPage.coldLinkMs !== null) entries["first-page-cold-link"] = ns(result.firstPage.coldLinkMs);
  if (result.find.common.firstResultMs !== null) entries["find-first-result"] = ns(result.find.common.firstResultMs);
  for (const { pages, firstPixelsMs } of result.edit) entries[`edit-first-pixels-${pages}`] = ns(firstPixelsMs);
  for (const { kind, firstPixelsMs } of result.pageEdits) entries[`page-edit-first-pixels-${kind}`] = ns(firstPixelsMs);
  if (result.mobile !== null) {
    entries["mobile-memory-peak-bytes"] = Math.round(
      result.mobile.peakMemory.canvases + result.mobile.peakMemory.bitmaps,
    );
  }
  return entries;
}
