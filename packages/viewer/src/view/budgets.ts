// The viewer's memory on touch devices (UI/UX plan §5.4): iOS Safari caps one canvas at about
// 16.7 MP and all of a tab's canvases at 224–384 MB, and kills the tab past that; pdf.js caps a
// mobile canvas at 5 MP. On a coarse pointer every canvas holds at most 5 MP, and the pools and the
// bitmap cache together stay under the lowest of those totals unless the browser reports 4 GB of
// memory or more (`navigator.deviceMemory`). Desktop keeps the defaults.

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
  /** The most device pixels any one canvas holds, a detail canvas too. */
  readonly canvasMaxPixels: number;
  readonly pool: CanvasPoolOptions;
  readonly detailPool: CanvasPoolOptions;
  readonly cacheBytes: number;
}

const MP = 1024 * 1024;
const MB = 1024 * 1024;
/** One canvas on a touch device: pdf.js's mobile cap. */
const TOUCH_CANVAS = 5 * MP;

/** The desktop budgets: the viewer's defaults before touch devices had their own. */
export const DESKTOP_BUDGETS: ViewerBudgets = {
  backdropMaxPixels: BACKDROP_MAX_PIXELS,
  canvasMaxPixels: Number.POSITIVE_INFINITY,
  pool: { max: 12, maxPixels: 40 * MP },
  detailPool: { max: 3, maxPixels: 24 * MP },
  cacheBytes: DEFAULT_CACHE_BYTES,
};

const TOUCH = { backdropMaxPixels: TOUCH_CANVAS, canvasMaxPixels: TOUCH_CANVAS } as const;

/** The decoded bytes `budgets` allow at most: 4 a pixel across both pools, and the bitmap cache. */
export function budgetBytes(budgets: ViewerBudgets): number {
  return 4 * ((budgets.pool.maxPixels ?? 0) + (budgets.detailPool.maxPixels ?? 0)) + budgets.cacheBytes;
}

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
      ...TOUCH,
      pool: { max: 6, maxPixels: 12 * MP },
      detailPool: { max: 1, maxPixels: 5 * MP },
      cacheBytes: 48 * MB,
    };
  }
  if (memory !== undefined && memory >= 4) {
    return {
      ...TOUCH,
      pool: { max: 10, maxPixels: 32 * MP },
      detailPool: { max: 2, maxPixels: 12 * MP },
      cacheBytes: 128 * MB,
    };
  }
  return {
    ...TOUCH,
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
