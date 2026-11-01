// The canvas pool: a fixed number of canvases, reused as pages mount and unmount, with a
// cap on the device pixels they hold together (browsers, Safari above all, fail or crash past an
// undocumented total canvas area). A canvas comes back reset: 1×1, so it holds no pixels.

/** The pool's limits, and how it makes canvases. */
export interface CanvasPoolOptions {
  /** Most canvases at once (default 12). */
  readonly max?: number;
  /** Most device pixels across every acquired canvas (default 64 MP). */
  readonly maxPixels?: number;
  /** Makes a canvas (default `document.createElement("canvas")`). */
  readonly create?: () => HTMLCanvasElement;
}

/** The default limits: 12 canvases, 64 million device pixels in all. */
const DEFAULT_POOL = Object.freeze({ max: 12, maxPixels: 64 * 1024 * 1024 });

/** A fixed set of canvases lent to mounted pages, within a total pixel cap. */
export class CanvasPool {
  readonly max: number;
  readonly maxPixels: number;
  readonly #create: () => HTMLCanvasElement;
  readonly #free: HTMLCanvasElement[] = [];
  readonly #used = new Set<HTMLCanvasElement>();
  /** The most canvases ever acquired at once (reported in the viewer's render stats). */
  peak = 0;

  constructor(options: CanvasPoolOptions = {}) {
    this.max = options.max ?? DEFAULT_POOL.max;
    this.maxPixels = options.maxPixels ?? DEFAULT_POOL.maxPixels;
    this.#create = options.create ?? (() => document.createElement("canvas"));
  }

  /** Canvases lent out now. */
  get inUse(): number {
    return this.#used.size;
  }

  /** Device pixels the acquired canvases hold now. */
  get pixels(): number {
    let total = 0;
    for (const c of this.#used) total += c.width * c.height;
    return total;
  }

  /** A canvas, or undefined when `max` are in use (the page keeps its placeholder until one frees). */
  acquire(): HTMLCanvasElement | undefined {
    if (this.#used.size >= this.max) return undefined;
    const canvas = this.#free.pop() ?? this.#create();
    this.#used.add(canvas);
    if (this.#used.size > this.peak) this.peak = this.#used.size;
    return canvas;
  }

  /**
   * The size `canvas` may take, `width × height` device pixels scaled down (keeping the aspect)
   * so that every acquired canvas together stays within `maxPixels`.
   */
  fit(canvas: HTMLCanvasElement, width: number, height: number): { width: number; height: number } {
    const others = this.pixels - canvas.width * canvas.height;
    const room = Math.max(0, this.maxPixels - others);
    const want = width * height;
    if (want <= room) return { width: Math.max(1, Math.round(width)), height: Math.max(1, Math.round(height)) };
    const k = Math.sqrt(room / want);
    return { width: Math.max(1, Math.floor(width * k)), height: Math.max(1, Math.floor(height * k)) };
  }

  /** Returns `canvas`, reset (1×1, no pixels, default context state). */
  release(canvas: HTMLCanvasElement): void {
    if (!this.#used.delete(canvas)) return;
    // Setting the size clears the pixels and the context's state.
    canvas.width = 1;
    canvas.height = 1;
    canvas.remove();
    this.#free.push(canvas);
  }
}
