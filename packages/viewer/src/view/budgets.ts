// The viewer's memory on touch devices (UI/UX plan §5.4): iOS Safari caps one canvas at about
// 16.7 MP and all of a tab's canvases at 224–384 MB, and kills the tab past that; pdf.js caps a
// mobile canvas at 5 MP. On a coarse pointer each page canvas holds at most 5 MP, and the pools and
// the bitmap cache together stay under the lowest of those totals, scaled by
// `navigator.deviceMemory` where the browser reports it. Desktop keeps the defaults.

import { DEFAULT_CACHE_BYTES } from "../canvas/cache.js";
import type { CanvasPoolOptions } from "../canvas/pool.js";
import { BACKDROP_MAX_PIXELS } from "../render/tiles.js";

/** What the budgets depend on: whether the main pointer is coarse, and the device's memory in GB, if known. */
export interface DeviceHints {
  readonly coarse: boolean;
  readonly memoryGb?: number | undefined;
}

/** The viewer's pixel and byte budgets. */
export interface ViewerBudgets {
  /** The most device pixels one page canvas holds. */
  readonly backdropMaxPixels: number;
  readonly pool: CanvasPoolOptions;
  readonly detailPool: CanvasPoolOptions;
  readonly cacheBytes: number;
}

const MP = 1024 * 1024;
const MB = 1024 * 1024;

/** The desktop budgets: the viewer's defaults before touch devices had their own. */
export const DESKTOP_BUDGETS: ViewerBudgets = {
  backdropMaxPixels: BACKDROP_MAX_PIXELS,
  pool: { max: 12, maxPixels: 40 * MP },
  detailPool: { max: 3, maxPixels: 24 * MP },
  cacheBytes: DEFAULT_CACHE_BYTES,
};

/**
 * The budgets for a device. On a coarse pointer the canvases and cache total at most 4 bytes per
 * pixel × (pool + detail pool) + cache: 184 MB when the memory is unknown (iOS Safari reports none),
 * 116 MB at 2 GB or less, 304 MB at 4 GB or more.
 */
export function viewerBudgets(hints: DeviceHints): ViewerBudgets {
  if (!hints.coarse) return DESKTOP_BUDGETS;
  const memory = hints.memoryGb;
  if (memory !== undefined && memory <= 2) {
    return {
      backdropMaxPixels: 5 * MP,
      pool: { max: 6, maxPixels: 12 * MP },
      detailPool: { max: 1, maxPixels: 5 * MP },
      cacheBytes: 48 * MB,
    };
  }
  if (memory !== undefined && memory >= 4) {
    return {
      backdropMaxPixels: 5 * MP,
      pool: { max: 10, maxPixels: 32 * MP },
      detailPool: { max: 2, maxPixels: 12 * MP },
      cacheBytes: 128 * MB,
    };
  }
  return {
    backdropMaxPixels: 5 * MP,
    pool: { max: 8, maxPixels: 20 * MP },
    detailPool: { max: 2, maxPixels: 10 * MP },
    cacheBytes: 64 * MB,
  };
}

/** This device's hints: a coarse main pointer, and `navigator.deviceMemory` (Chromium only). */
export function deviceHints(): DeviceHints {
  const coarse = typeof matchMedia === "function" && matchMedia("(pointer: coarse)").matches;
  const memory = (globalThis.navigator as { deviceMemory?: unknown } | undefined)?.deviceMemory;
  return { coarse, memoryGb: typeof memory === "number" ? memory : undefined };
}
