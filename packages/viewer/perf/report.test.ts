// The perf report's arithmetic: each check against its target at the boundary, the memory
// stability rule, the long-task fallback where a browser has no Long Tasks API, the summary's
// shape and the history's units.

import { describe, expect, it } from "vitest";
import { type BrowserResult, checks, historyEntries, summary, TARGETS } from "./report.ts";
import type { MemorySample, Recording } from "./types.ts";

const MB = 1024 * 1024;
const memory = (total: number): MemorySample => ({ heap: null, wasm: total, bitmaps: 0, canvases: 0, total });
const recording = (over: Partial<Recording> = {}): Recording => ({
  longestFrameMs: 20,
  framesOver50: 0,
  longTasks: [],
  peakMemory: memory(300 * MB),
  ...over,
});
const counters = { drawsOnUnmounted: 0, aborted: 12, dropped: 3, peakMounted: 9, peakCanvases: 9 };

function result(over: Partial<BrowserResult> = {}): BrowserResult {
  return {
    browser: "chromium",
    version: "141.0",
    quick: false,
    machine: { cpu: "Test CPU", cores: 8, memoryBytes: 16 * 1024 * MB, os: "Darwin 25.0" },
    firstPage: { warmMs: 400, coldMs: 2500 },
    scroll: {
      passes: [recording(), recording()],
      settled: [memory(200 * MB), memory(205 * MB)],
      toSharpMs: [120, 180, 250, 90],
      counters,
    },
    fling: { toSharpMs: 280, counters },
    zoom: { recording: recording(), toSharpMs: [150, 200] },
    mobile: {
      device: "Pixel 8",
      peakMemory: { ...memory(150 * MB), canvases: 80 * MB, bitmaps: 60 * MB },
      budgetBytes: 184 * MB,
    },
    ...over,
  };
}

const byName = (r: BrowserResult) => Object.fromEntries(checks(r).map((c) => [c.name, c]));

describe("checks", () => {
  it("passes a run that meets every target, at the boundaries too", () => {
    const r = result({ firstPage: { warmMs: TARGETS.firstPageWarmMs, coldMs: TARGETS.firstPageColdMs } });
    expect(checks(r).every((c) => c.pass !== false)).toBe(true);
  });

  it("marks each target a run misses", () => {
    const r = result({
      firstPage: { warmMs: 701, coldMs: 3001 },
      fling: { toSharpMs: 301, counters: { ...counters, drawsOnUnmounted: 1 } },
    });
    const c = byName(r);
    expect(c["First page, 10-page file, warm"]?.pass).toBe(false);
    expect(c["First page, 10-page file, cold on Fast 4G"]?.pass).toBe(false);
    expect(c["Pages sharp after scrolling stops"]?.value).toBe("301 ms");
    expect(c["Pages sharp after scrolling stops"]?.pass).toBe(false);
    expect(c["Cancelled renders that painted"]?.pass).toBe(false);
  });

  it("allows the second pass to end 5 % above the first, no more", () => {
    const stable = byName(result({ scroll: { ...result().scroll, settled: [memory(200 * MB), memory(210 * MB)] } }));
    expect(stable["Memory after the second pass"]?.pass).toBe(true);
    const grew = byName(result({ scroll: { ...result().scroll, settled: [memory(200 * MB), memory(211 * MB)] } }));
    expect(grew["Memory after the second pass"]?.pass).toBe(false);
    expect(grew["Memory after the second pass"]?.value).toBe("+5.5 %");
  });

  it("checks peak memory against 600 MB", () => {
    const passes: [Recording, Recording] = [recording(), recording({ peakMemory: memory(601 * MB) })];
    expect(byName(result({ scroll: { ...result().scroll, passes } }))["Memory, 1,000 pages end to end"]?.pass).toBe(
      false,
    );
  });

  it("fails on any long task where the browser reports them, and uses the longest frame elsewhere", () => {
    const longTask = recording({ longTasks: [64] });
    const chromium = byName(result({ scroll: { ...result().scroll, passes: [recording(), longTask] } }));
    expect(chromium["Longest main-thread task while scrolling"]?.value).toBe("1 over 50 ms, longest 64 ms");
    expect(chromium["Longest main-thread task while scrolling"]?.pass).toBe(false);
    const frames: [Recording, Recording] = [
      recording({ longTasks: null, longestFrameMs: 48 }),
      recording({ longTasks: null, longestFrameMs: 50 }),
    ];
    const webkit = byName(result({ browser: "webkit", scroll: { ...result().scroll, passes: frames } }));
    expect(webkit["Longest main-thread task while scrolling"]?.value).toBe("50 ms (longest frame)");
    expect(webkit["Longest main-thread task while scrolling"]?.pass).toBe(true);
  });

  it("leaves the cold first page unjudged where it wasn't measured", () => {
    const c = byName(result({ browser: "firefox", firstPage: { warmMs: 400, coldMs: null } }));
    expect(c["First page, 10-page file, cold on Fast 4G"]?.pass).toBeNull();
  });
});

describe("summary", () => {
  it("has a column per browser and marks each judged cell", () => {
    const text = summary([
      result(),
      result({ browser: "webkit", version: "26.0", firstPage: { warmMs: 800, coldMs: null } }),
    ]);
    expect(text).toContain("| Check | Target | chromium 141.0 | webkit 26.0 |");
    expect(text).toContain("| First page, 10-page file, warm | ≤ 700 ms | 400 ms (pass) | 800 ms (miss) |");
    expect(text).toContain("Taken on Test CPU (8 cores, 16384 MB), Darwin 25.0.");
    expect(text).not.toContain("Quick run");
  });

  it("says when a run was quick", () => {
    expect(summary([result({ quick: true })])).toContain("Quick run");
  });
});

describe("historyEntries", () => {
  it("holds a phone's canvases and bitmaps to its touch budgets, and says where there was no phone", () => {
    const over = result({
      mobile: {
        device: "iPhone 15",
        peakMemory: { ...memory(0), canvases: 150 * MB, bitmaps: 40 * MB },
        budgetBytes: 184 * MB,
      },
    });
    const name = "Canvases and bitmaps on a phone, 1,000 pages";
    expect([
      byName(result())[name]?.pass,
      byName(over)[name]?.pass,
      byName(result({ mobile: null }))[name]?.pass,
    ]).toEqual([true, false, null]);
  });

  it("records times in nanoseconds and memory in bytes, and the cold figure only where measured", () => {
    expect(historyEntries(result())).toEqual({
      "first-page-warm": 400_000_000,
      "first-page-cold": 2_500_000_000,
      "scroll-longest-frame": 20_000_000,
      "to-sharp": 280_000_000,
      "zoom-to-sharp": 200_000_000,
      "memory-peak-bytes": 300 * MB,
      "mobile-memory-peak-bytes": 140 * MB,
    });
    expect(historyEntries(result({ firstPage: { warmMs: 1, coldMs: null } }))).not.toHaveProperty("first-page-cold");
  });
});
