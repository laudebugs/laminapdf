// What the harness page (page/main.ts) gives the runner (run.ts) through `globalThis.__laminaPerf`.

declare global {
  /** The harness page's hook, set once the page is ready (it exists on the harness page only). */
  var __laminaPerf: PerfHook;
}

/** One memory reading, in bytes. */
export interface MemorySample {
  /** The page's JS heap, where the browser reports it (Chromium's `performance.memory`), else null. */
  readonly heap: number | null;
  /** The document worker's WebAssembly memories: the backend's and Lamina PDF's. */
  readonly wasm: number;
  /** The viewer's bitmap cache (`renderstats`' `cacheBytes`). */
  readonly bitmaps: number;
  /** The mounted canvases' pixels, at 4 bytes each. */
  readonly canvases: number;
  /** The sum of the four. */
  readonly total: number;
}

/** What happened between `record()` and `stop()`. */
export interface Recording {
  /** The longest gap between two animation frames, in ms: every browser has it. */
  readonly longestFrameMs: number;
  /** Frames that took more than 50 ms. */
  readonly framesOver50: number;
  /** Long tasks' durations in ms (Chromium's Long Tasks API), or null where the browser has none. */
  readonly longTasks: readonly number[] | null;
  /** The reading with the largest total, taken every 500 ms. */
  readonly peakMemory: MemorySample;
}

/** The viewer's counters the harness reads (a part of `RenderStats`). */
export interface Counters {
  /** Draws onto an unmounted page, that is cancelled renders that painted: must stay 0. */
  readonly drawsOnUnmounted: number;
  readonly aborted: number;
  readonly dropped: number;
  readonly peakMounted: number;
  readonly peakCanvases: number;
}

/** `globalThis.__laminaPerf`. */
export interface PerfHook {
  /** With `?open=<name>`: ms from navigation start until the first page painted (null until then). */
  firstPaintMs: number | null;
  /** Opens fixture `<name>.pdf`, read before the clock starts; ms until the first page painted. */
  open(name: string): Promise<number>;
  setZoom(zoom: string): void;
  goToPage(index: number): void;
  /** The scroll position and its largest value, in CSS px. */
  scroll(): { readonly top: number; readonly max: number };
  record(): void;
  stop(): Promise<Recording>;
  /**
   * Waits for the view to stop scrolling and zooming, then returns ms from its last change until
   * every visible page was sharp; throws if either takes over `limitMs`.
   */
  untilSharp(limitMs?: number): Promise<number>;
  memory(): Promise<MemorySample>;
  counters(): Counters;
}
