// The viewer (C3b): a scroll container whose spacer is the whole document's height, and page
// shells for only the pages in view plus a margin. Each mounted page borrows a canvas from a
// fixed pool and paints in two passes, a low-resolution whole page and then sharp tiles over the
// part in view, through the document worker (C3a) at `visible` or `prefetch` priority. A page
// that leaves the margin aborts its renders, and a result that arrives for it anyway is dropped,
// never drawn. Zoom keeps the document point under the cursor (or the viewport centre) where it
// is; during a gesture the canvases are stretched by CSS and the sharp re-render waits until the
// gesture settles. Framework-free: UI layers read `getState()`, `subscribe()` and the events.
import { BitmapCache, DEFAULT_CACHE_BYTES } from "../canvas/cache.js";
import { CanvasPool, type CanvasPoolOptions } from "../canvas/pool.js";
import type { ViewerClient } from "../client.js";
import { PageLayout, type PageSize, type Rotation } from "../layout/page-layout.js";
import type { OpenedDocument, RenderedBitmap } from "../protocol.js";
import { lowScale, tilesFor } from "../render/tiles.js";
import { type DocumentPoint, percentOfScale, pointAt, scaleFor, scrollToPoint, type Zoom } from "../zoom/zoom.js";

/** Where a mounted page's pixels are. */
export type PageRenderState = "placeholder" | "low" | "sharp" | "error";

/** What the view shows: for `useSyncExternalStore` and the toolbar. */
export interface ViewState {
  readonly pageCount: number;
  /** The page under the viewport's centre. */
  readonly currentPage: number;
  readonly zoom: Zoom;
  /** The zoom as a percentage (100 is actual size), whatever the mode. */
  readonly percent: number;
  readonly rotation: Rotation;
  readonly scrollTop: number;
  /** The pages any part of which is in the viewport. */
  readonly visibleRange: { readonly first: number; readonly last: number };
}

/** Counters for tests and the perf harness (C3c). */
export interface RenderStats {
  readonly mounted: number;
  readonly peakMounted: number;
  readonly canvases: number;
  readonly peakCanvases: number;
  readonly maxCanvases: number;
  /** Draws onto a page that wasn't mounted when its result arrived: always 0 (the guarantee). */
  readonly drawsOnUnmounted: number;
  /** Results that arrived for a page no longer mounted, or a zoom since replaced, and were dropped. */
  readonly dropped: number;
  readonly cacheBytes: number;
  /** Each mounted page's state. */
  readonly pages: Readonly<Record<number, PageRenderState>>;
}

export interface ViewerEventMap {
  pagechange: { readonly page: number };
  zoomchange: { readonly zoom: Zoom; readonly percent: number };
  renderstats: RenderStats;
}

/** A layer that later todos mount into each page (text, annotations, highlights, …). */
export interface PageLayer {
  /** Its slot: `text`, `annot` and `highlight` exist on every page; another name gets a new one on top. */
  readonly name: string;
  onMount?(page: number, element: HTMLElement): void;
  onUnmount?(page: number, element: HTMLElement): void;
  /** The page's CSS size changed (zoom or rotation). */
  onZoom?(page: number, element: HTMLElement, size: { readonly width: number; readonly height: number }): void;
}

export interface CreateViewerOptions {
  readonly client: ViewerClient;
  /** Pages mounted past each edge of the viewport (default 2; at least one viewport's worth). */
  readonly margin?: number;
  /** CSS px between pages (default 16). */
  readonly gap?: number;
  readonly pool?: CanvasPoolOptions;
  /** The bitmap cache's budget in bytes (default 256 MB). */
  readonly cacheBytes?: number;
  /** How long a zoom gesture must pause before the sharp re-render, in ms (default 150). */
  readonly settleMs?: number;
  /** The device pixel ratio (default `globalThis.devicePixelRatio`). */
  readonly dpr?: () => number;
  /** The initial zoom (default `fit-width`). */
  readonly zoom?: Zoom;
}

/** The built-in layer slots, bottom to top, above the canvas. */
const SLOTS = ["text", "annot", "highlight"] as const;

/** The tallest scroll space browsers handle (Firefox stops near 17.9 M CSS px). */
const MAX_HEIGHT = 16_000_000;

interface PageView {
  readonly index: number;
  readonly shell: HTMLDivElement;
  readonly slots: Map<string, HTMLDivElement>;
  canvas: HTMLCanvasElement | undefined;
  controller: AbortController;
  state: PageRenderState;
  /** The layout scale the canvas was last painted for. */
  paintedScale: number;
  /** Bumped on every new render pass: an older pass's results are dropped. */
  pass: number;
}

function isAbort(e: unknown): boolean {
  return e instanceof DOMException && e.name === "AbortError";
}

export class Viewer {
  readonly #client: ViewerClient;
  readonly #margin: number;
  readonly #gap: number;
  readonly #settleMs: number;
  readonly #dpr: () => number;
  readonly #pool: CanvasPool;
  readonly #cache: BitmapCache;
  readonly #root: HTMLDivElement;
  readonly #spacer: HTMLDivElement;
  readonly #pages = new Map<number, PageView>();
  readonly #layers: PageLayer[] = [];
  readonly #listeners = new Set<() => void>();
  readonly #events = new Map<keyof ViewerEventMap, Set<(data: never) => void>>();
  readonly #cleanup: (() => void)[] = [];
  #document: OpenedDocument | undefined;
  #sizes: PageSize[] = [];
  #layout: PageLayout | undefined;
  #zoom: Zoom;
  #rotation: Rotation = 0;
  #state: ViewState;
  #frame: number | undefined;
  #settling: ReturnType<typeof setTimeout> | undefined;
  #peakMounted = 0;
  #drawsOnUnmounted = 0;
  #dropped = 0;
  #destroyed = false;

  constructor(options: CreateViewerOptions) {
    this.#client = options.client;
    this.#margin = options.margin ?? 2;
    this.#gap = options.gap ?? 16;
    this.#settleMs = options.settleMs ?? 150;
    this.#dpr = options.dpr ?? (() => globalThis.devicePixelRatio || 1);
    this.#pool = new CanvasPool(options.pool);
    this.#cache = new BitmapCache(options.cacheBytes ?? DEFAULT_CACHE_BYTES);
    this.#zoom = options.zoom ?? "fit-width";
    this.#root = document.createElement("div");
    this.#root.className = "lamina-viewer";
    this.#root.tabIndex = 0;
    Object.assign(this.#root.style, { position: "relative", overflow: "auto", width: "100%", height: "100%" });
    this.#spacer = document.createElement("div");
    this.#spacer.className = "lamina-viewer-content";
    Object.assign(this.#spacer.style, { position: "relative" });
    this.#root.append(this.#spacer);
    this.#state = {
      pageCount: 0,
      currentPage: 0,
      zoom: this.#zoom,
      percent: 100,
      rotation: 0,
      scrollTop: 0,
      visibleRange: { first: 0, last: -1 },
    };
    this.#listen();
  }

  /** The scroll container (append it, or use {@link mount}). */
  get element(): HTMLDivElement {
    return this.#root;
  }

  /** Puts the viewer into `container` (which gives it its size). */
  mount(container: HTMLElement): void {
    container.append(this.#root);
    this.#schedule();
  }

  /** Shows `document` (an `open()` result of the client), at the initial zoom, from page 0. */
  open(document: OpenedDocument): void {
    this.#unmountAll();
    this.#cache.clear();
    this.#document = document;
    this.#sizes = document.pages.map((p) => ({ width: p.width, height: p.height }));
    this.#layout = undefined;
    this.#root.scrollTop = 0;
    this.#root.scrollLeft = 0;
    this.#relayout();
    this.#schedule();
  }

  getState(): ViewState {
    return this.#state;
  }

  /** Calls `listener` after every state change (for `useSyncExternalStore`). */
  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  on<E extends keyof ViewerEventMap>(event: E, listener: (data: ViewerEventMap[E]) => void): () => void {
    let set = this.#events.get(event);
    if (set === undefined) {
      set = new Set();
      this.#events.set(event, set);
    }
    const l = listener as (data: never) => void;
    set.add(l);
    return () => set.delete(l);
  }

  /** Adds a layer to every page, mounted now and later. */
  addLayer(layer: PageLayer): () => void {
    this.#layers.push(layer);
    for (const view of this.#pages.values()) this.#mountLayer(view, layer);
    return () => {
      const i = this.#layers.indexOf(layer);
      if (i >= 0) this.#layers.splice(i, 1);
      for (const view of this.#pages.values()) layer.onUnmount?.(view.index, this.#slot(view, layer.name));
    };
  }

  /** Scrolls page `index` to the top of the viewport (or its centre). */
  goToPage(index: number, options: { readonly anchor?: "top" | "center" } = {}): void {
    const layout = this.#layout;
    if (layout === undefined || layout.pageCount === 0) return;
    const page = Math.min(layout.pageCount - 1, Math.max(0, Math.trunc(index)));
    const top = layout.offsetOf(page) - this.#gap / 2;
    this.#root.scrollTop =
      options.anchor === "center"
        ? top + (layout.heights[page] as number) / 2 - this.#root.clientHeight / 2
        : Math.max(0, top);
    this.#schedule();
  }

  /**
   * Sets the zoom (a mode, or a percentage clamped to 10–800 %), keeping the document point at
   * `anchor` (viewport CSS px; default its centre) where it is.
   */
  setZoom(zoom: Zoom, options: { readonly anchor?: { readonly x: number; readonly y: number } } = {}): void {
    this.#zoom = typeof zoom === "number" ? zoom : zoom;
    this.#rezoom(options.anchor);
  }

  /** Rotates the view by `delta` degrees (a multiple of 90), keeping the viewport's centre. */
  rotate(delta: number): void {
    this.#rotation = ((((this.#rotation + delta) % 360) + 360) % 360) as Rotation;
    this.#rezoom(undefined);
  }

  /** The document point at viewport position (`x`, `y`) in CSS px. */
  pointAt(x: number, y: number): DocumentPoint | undefined {
    const layout = this.#layout;
    if (layout === undefined || layout.pageCount === 0) return undefined;
    return pointAt(layout, this.#root.clientWidth, this.#scroll(), { x, y });
  }

  /** Where `point` is now, in viewport CSS px. */
  positionOf(point: DocumentPoint): { x: number; y: number } | undefined {
    const layout = this.#layout;
    if (layout === undefined) return undefined;
    const scroll = this.#scroll();
    return {
      x:
        layout.leftOf(point.page, this.#root.clientWidth) +
        point.x * (layout.widths[point.page] as number) -
        scroll.left,
      y: layout.offsetOf(point.page) + point.y * (layout.heights[point.page] as number) - scroll.top,
    };
  }

  /** The counters (see RenderStats). */
  stats(): RenderStats {
    const pages: Record<number, PageRenderState> = {};
    for (const [i, v] of this.#pages) pages[i] = v.state;
    return {
      mounted: this.#pages.size,
      peakMounted: this.#peakMounted,
      canvases: this.#pool.inUse,
      peakCanvases: this.#pool.peak,
      maxCanvases: this.#pool.max,
      drawsOnUnmounted: this.#drawsOnUnmounted,
      dropped: this.#dropped,
      cacheBytes: this.#cache.bytes,
      pages,
    };
  }

  /** How many pages are mounted at most for the current viewport: visible + 2 × margin. */
  get mountBudget(): number {
    const layout = this.#layout;
    if (layout === undefined) return 0;
    const top = this.#root.scrollTop;
    const h = this.#root.clientHeight;
    const visible = layout.rangeIn(top, top + h);
    const [first, last] = this.#mountRange(layout, top, h);
    return Math.max(visible.last - visible.first + 1 + 2 * this.#margin, last - first + 1);
  }

  destroy(): void {
    this.#destroyed = true;
    if (this.#frame !== undefined) cancelAnimationFrame(this.#frame);
    clearTimeout(this.#settling);
    this.#unmountAll();
    this.#cache.clear();
    for (const f of this.#cleanup.splice(0)) f();
    this.#root.remove();
  }

  // --- internals ---

  #scroll() {
    return { left: this.#root.scrollLeft, top: this.#root.scrollTop };
  }

  #emit<E extends keyof ViewerEventMap>(event: E, data: ViewerEventMap[E]): void {
    for (const l of this.#events.get(event) ?? []) (l as (d: ViewerEventMap[E]) => void)(data);
  }

  #setState(next: Partial<ViewState>): void {
    const prev = this.#state;
    this.#state = { ...prev, ...next };
    if (next.currentPage !== undefined && next.currentPage !== prev.currentPage) {
      this.#emit("pagechange", { page: next.currentPage });
    }
    for (const l of this.#listeners) l();
  }

  /** The reference page for fit modes: the current one. */
  #reference(): PageSize {
    return this.#sizes[this.#state.currentPage] ?? this.#sizes[0] ?? { width: 612, height: 792 };
  }

  /** Builds the layout for the current zoom, rotation and viewport (clamped below the height cap). */
  #relayout(): PageLayout {
    const viewport = { width: this.#root.clientWidth || 800, height: this.#root.clientHeight || 600 };
    let scale = scaleFor(this.#zoom, this.#reference(), this.#rotation, viewport, this.#gap);
    let layout = new PageLayout(this.#sizes, { scale, rotation: this.#rotation, gap: this.#gap });
    if (layout.height > MAX_HEIGHT) {
      // Past what browsers can scroll: cap the zoom for this document.
      scale *= MAX_HEIGHT / layout.height;
      layout = new PageLayout(this.#sizes, { scale, rotation: this.#rotation, gap: this.#gap });
    }
    const zoomed =
      this.#layout === undefined || this.#layout.scale !== layout.scale || this.#layout.rotation !== layout.rotation;
    this.#layout = layout;
    this.#spacer.style.height = `${layout.height}px`;
    this.#spacer.style.width = `${layout.contentWidth(viewport.width)}px`;
    if (zoomed) {
      const percent = percentOfScale(layout.scale);
      this.#setState({ pageCount: layout.pageCount, zoom: this.#zoom, percent, rotation: this.#rotation });
      this.#emit("zoomchange", { zoom: this.#zoom, percent });
    }
    return layout;
  }

  #rezoom(anchor: { readonly x: number; readonly y: number } | undefined): void {
    const before = this.#layout;
    if (before === undefined || before.pageCount === 0) {
      this.#relayout();
      return;
    }
    const at = anchor ?? { x: this.#root.clientWidth / 2, y: this.#root.clientHeight / 2 };
    const point = pointAt(before, this.#root.clientWidth, this.#scroll(), at);
    const after = this.#relayout();
    const scroll = scrollToPoint(after, this.#root.clientWidth, point, at);
    this.#root.scrollTop = scroll.top;
    this.#root.scrollLeft = scroll.left;
    // Renders for the old geometry are stale: abort them; the canvases stretch until the gesture
    // settles and the new pass paints.
    for (const view of this.#pages.values()) {
      view.controller.abort();
      view.controller = new AbortController();
      view.pass++;
    }
    clearTimeout(this.#settling);
    this.#settling = setTimeout(() => {
      this.#settling = undefined;
      this.#schedule();
    }, this.#settleMs);
    this.#schedule();
  }

  #schedule(): void {
    if (this.#frame !== undefined || this.#destroyed) return;
    this.#frame = requestAnimationFrame(() => {
      this.#frame = undefined;
      this.#update();
    });
  }

  /** The mounted range: visible ± margin pages, and at least a viewport above and below. */
  #mountRange(layout: PageLayout, top: number, height: number): [number, number] {
    const visible = layout.rangeIn(top, top + height);
    const first = Math.max(0, Math.min(visible.first - this.#margin, layout.pageAt(top - height)));
    const last = Math.min(layout.pageCount - 1, Math.max(visible.last + this.#margin, layout.pageAt(top + 2 * height)));
    return [first, last];
  }

  /** One frame: every read first, then every write. */
  #update(): void {
    const layout = this.#layout;
    if (layout === undefined || layout.pageCount === 0 || this.#destroyed) return;
    // Reads.
    const top = this.#root.scrollTop;
    const height = this.#root.clientHeight;
    const width = this.#root.clientWidth;
    const visible = layout.rangeIn(top, top + height);
    const [first, last] = this.#mountRange(layout, top, height);
    const current = layout.pageAt(top + height / 2);
    // Writes: unmount what left, mount what came, position all.
    for (const [i, view] of this.#pages) if (i < first || i > last) this.#unmount(view);
    for (let i = first; i <= last; i++) {
      const view = this.#pages.get(i) ?? this.#mount(i);
      const w = layout.widths[i] as number;
      const h = layout.heights[i] as number;
      const s = view.shell.style;
      const before = s.width;
      s.top = `${layout.offsetOf(i)}px`;
      s.left = `${layout.leftOf(i, width)}px`;
      s.width = `${w}px`;
      s.height = `${h}px`;
      if (before !== s.width)
        for (const layer of this.#layers) layer.onZoom?.(i, this.#slot(view, layer.name), { width: w, height: h });
    }
    if (this.#pages.size > this.#peakMounted) this.#peakMounted = this.#pages.size;
    this.#setState({ currentPage: current, scrollTop: top, visibleRange: visible });
    // Renders: visible pages first, then the margin; nothing while a zoom gesture is under way.
    if (this.#settling === undefined) {
      for (let i = visible.first; i <= visible.last; i++) this.#paint(i, "visible", layout, top, height);
      for (let i = first; i <= last; i++)
        if (i < visible.first || i > visible.last) this.#paint(i, "prefetch", layout, top, height);
    }
    this.#emit("renderstats", this.stats());
  }

  #mount(index: number): PageView {
    const shell = document.createElement("div");
    shell.className = "lamina-page";
    shell.dataset.pageIndex = String(index);
    Object.assign(shell.style, { position: "absolute", contain: "strict", background: "#fff" });
    const slots = new Map<string, HTMLDivElement>();
    for (const name of SLOTS) {
      const slot = document.createElement("div");
      slot.className = `lamina-layer-${name}`;
      Object.assign(slot.style, { position: "absolute", inset: "0" });
      slots.set(name, slot);
      shell.append(slot);
    }
    this.#spacer.append(shell);
    const view: PageView = {
      index,
      shell,
      slots,
      canvas: undefined,
      controller: new AbortController(),
      state: "placeholder",
      paintedScale: 0,
      pass: 0,
    };
    this.#pages.set(index, view);
    for (const layer of this.#layers) this.#mountLayer(view, layer);
    return view;
  }

  #slot(view: PageView, name: string): HTMLDivElement {
    let slot = view.slots.get(name);
    if (slot === undefined) {
      slot = document.createElement("div");
      slot.className = `lamina-layer-${name}`;
      Object.assign(slot.style, { position: "absolute", inset: "0" });
      view.slots.set(name, slot);
      view.shell.append(slot);
    }
    return slot;
  }

  #mountLayer(view: PageView, layer: PageLayer): void {
    layer.onMount?.(view.index, this.#slot(view, layer.name));
  }

  #unmount(view: PageView): void {
    view.controller.abort();
    view.pass++;
    for (const layer of this.#layers) layer.onUnmount?.(view.index, this.#slot(view, layer.name));
    if (view.canvas !== undefined) this.#pool.release(view.canvas);
    view.canvas = undefined;
    view.shell.remove();
    this.#pages.delete(view.index);
  }

  #unmountAll(): void {
    for (const view of [...this.#pages.values()]) this.#unmount(view);
  }

  /** Starts page `index`'s render pass if its canvas isn't sharp at this scale yet. */
  #paint(index: number, priority: "visible" | "prefetch", layout: PageLayout, top: number, height: number): void {
    const view = this.#pages.get(index);
    const document = this.#document;
    if (view === undefined || document === undefined) return;
    const done =
      view.paintedScale === layout.scale &&
      (view.state === "sharp" || (priority === "prefetch" && view.state === "low"));
    if (done || view.state === "error") return;
    // One pass at a time per page: a newer pass bumps `pass`, and the older one stops drawing.
    if (view.controller.signal.aborted) view.controller = new AbortController();
    const pass = ++view.pass;
    void this.#pass(view, pass, priority, layout, top, height).catch((e) => {
      if (isAbort(e) || pass !== view.pass) return;
      view.state = "error";
      this.#emit("renderstats", this.stats());
    });
  }

  async #pass(
    view: PageView,
    pass: number,
    priority: "visible" | "prefetch",
    layout: PageLayout,
    top: number,
    height: number,
  ): Promise<void> {
    const { index } = view;
    const signal = view.controller.signal;
    const live = () =>
      pass === view.pass && this.#pages.get(index) === view && !signal.aborted && this.#layout === layout;
    const cssW = layout.widths[index] as number;
    const cssH = layout.heights[index] as number;
    const dpr = this.#dpr();
    const size = this.#sizes[index] as PageSize;
    const turned = layout.rotation === 90 || layout.rotation === 270;
    const widthPt = turned ? size.height : size.width;
    // Low-resolution pass (skipped when the canvas already shows this scale).
    if (view.paintedScale !== layout.scale) {
      const scale = lowScale(layout.scale * dpr, widthPt);
      const low = await this.#bitmap(index, { page: index, scale, rotation: layout.rotation }, priority, signal);
      if (!this.#commit(view, live, () => this.#drawLow(view, low, cssW, cssH, dpr))) {
        // No canvas free: try again on a later frame (one frees as pages leave).
        if (live()) setTimeout(() => this.#schedule(), 50);
        return;
      }
      view.paintedScale = layout.scale;
      view.state = "low";
      this.#emit("renderstats", this.stats());
    }
    if (priority === "prefetch") return;
    // Sharp tiles over the part in view, at the canvas's own pixel ratio (the pool may cap it).
    const canvas = view.canvas;
    if (canvas === undefined) return;
    const ratio = canvas.width / cssW;
    const pageTop = layout.offsetOf(index);
    const inView = {
      x: 0,
      y: Math.max(0, top - pageTop),
      width: cssW,
      height: Math.max(0, Math.min(cssH, top + height - pageTop) - Math.max(0, top - pageTop)),
    };
    for (const clip of tilesFor(inView, { width: cssW, height: cssH }, ratio)) {
      const tile = await this.#bitmap(
        index,
        { page: index, scale: layout.scale * ratio, rotation: layout.rotation, clip },
        priority,
        signal,
      );
      if (!this.#commit(view, live, (ctx) => ctx.drawImage(tile, clip.x, clip.y))) return;
    }
    view.state = "sharp";
    this.#emit("renderstats", this.stats());
  }

  /** A bitmap for `request`, from the cache or the worker. */
  async #bitmap(
    page: number,
    request: Parameters<ViewerClient["render"]>[0],
    priority: "visible" | "prefetch",
    signal: AbortSignal,
  ): Promise<ImageBitmap> {
    const { scale, rotation = 0, clip } = request;
    const key = `${page}|${rotation}|${scale}|${clip === undefined ? "page" : `${clip.x},${clip.y},${clip.width},${clip.height}`}`;
    const cached = this.#cache.get(key);
    if (cached !== undefined) return cached;
    const rendered: RenderedBitmap = await this.#client.render(request, { priority, signal });
    this.#cache.set(key, page, rendered.bitmap, this.#state.currentPage);
    return rendered.bitmap;
  }

  /** Draws on `view`'s canvas if the pass is still live; counts a dropped result otherwise. */
  #commit(view: PageView, live: () => boolean, draw: (ctx: CanvasRenderingContext2D) => void): boolean {
    if (!live()) {
      this.#dropped++;
      return false;
    }
    if (this.#pages.get(view.index) !== view) {
      // Unreachable while `live()` holds: the guarantee the tests read.
      this.#drawsOnUnmounted++;
      return false;
    }
    const canvas = view.canvas ?? this.#attachCanvas(view);
    // No canvas free in the pool: the page keeps its placeholder until one is released.
    const ctx = canvas?.getContext("2d");
    if (ctx === undefined || ctx === null) return false;
    draw(ctx);
    return true;
  }

  #attachCanvas(view: PageView): HTMLCanvasElement | undefined {
    const canvas = this.#pool.acquire();
    if (canvas === undefined) return undefined;
    canvas.className = "lamina-page-canvas";
    Object.assign(canvas.style, { position: "absolute", inset: "0", width: "100%", height: "100%" });
    view.shell.prepend(canvas);
    view.canvas = canvas;
    return canvas;
  }

  /** (Re)sizes the page's canvas for its CSS size × DPR (within the pool's cap) and stretches `low` over it. */
  #drawLow(view: PageView, low: ImageBitmap, cssW: number, cssH: number, dpr: number): void {
    const canvas = view.canvas;
    if (canvas === undefined) return;
    const { width, height } = this.#pool.fit(canvas, cssW * dpr, cssH * dpr);
    // Resizing clears the canvas: it happens only now, with the new pixels ready (no white flash).
    canvas.width = width;
    canvas.height = height;
    canvas.getContext("2d")?.drawImage(low, 0, 0, width, height);
  }

  #listen(): void {
    const root = this.#root;
    const on = <K extends keyof HTMLElementEventMap>(
      type: K,
      f: (e: HTMLElementEventMap[K]) => void,
      options?: AddEventListenerOptions,
    ) => {
      root.addEventListener(type, f as EventListener, options);
      this.#cleanup.push(() => root.removeEventListener(type, f as EventListener, options));
    };
    on("scroll", () => this.#schedule(), { passive: true });
    // Ctrl/cmd+wheel and trackpad pinch (which browsers send as ctrl+wheel): zoom at the cursor.
    on(
      "wheel",
      (e) => {
        if (!e.ctrlKey && !e.metaKey) return;
        e.preventDefault();
        const rect = root.getBoundingClientRect();
        this.setZoom(this.#state.percent * Math.exp(-e.deltaY * 0.01), {
          anchor: { x: e.clientX - rect.left, y: e.clientY - rect.top },
        });
      },
      { passive: false },
    );
    on("keydown", (e) => {
      if (e.altKey) return;
      const p = this.#state.percent;
      if (e.key === "+" || e.key === "=") this.setZoom(p * 1.25);
      else if (e.key === "-") this.setZoom(p / 1.25);
      else if (e.key === "0") this.setZoom("actual-size");
      else return;
      e.preventDefault();
    });
    // Touch pinch: two pointers; the zoom follows their distance, anchored at their midpoint.
    const pointers = new Map<number, { x: number; y: number }>();
    let pinch: { distance: number; percent: number } | undefined;
    const spread = () => {
      const [a, b] = [...pointers.values()];
      return a === undefined || b === undefined
        ? undefined
        : { d: Math.hypot(a.x - b.x, a.y - b.y), x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    };
    on("pointerdown", (e) => {
      if (e.pointerType !== "touch") return;
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const s = spread();
      if (s !== undefined) pinch = { distance: s.d, percent: this.#state.percent };
    });
    on("pointermove", (e) => {
      if (!pointers.has(e.pointerId)) return;
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const s = spread();
      if (s === undefined || pinch === undefined || pinch.distance === 0) return;
      const rect = root.getBoundingClientRect();
      this.setZoom(pinch.percent * (s.d / pinch.distance), { anchor: { x: s.x - rect.left, y: s.y - rect.top } });
    });
    const up = (e: PointerEvent) => {
      pointers.delete(e.pointerId);
      if (pointers.size < 2) pinch = undefined;
    };
    on("pointerup", up);
    on("pointercancel", up);
    // Safari's trackpad pinch: gesture events with a scale.
    let gesture: number | undefined;
    const g = root as unknown as {
      addEventListener(t: string, f: (e: Event) => void): void;
      removeEventListener(t: string, f: (e: Event) => void): void;
    };
    const start = (e: Event) => {
      e.preventDefault();
      gesture = this.#state.percent;
    };
    const change = (e: Event) => {
      if (gesture === undefined) return;
      e.preventDefault();
      const ge = e as Event & { scale: number; clientX: number; clientY: number };
      const rect = root.getBoundingClientRect();
      this.setZoom(gesture * ge.scale, { anchor: { x: ge.clientX - rect.left, y: ge.clientY - rect.top } });
    };
    const end = () => {
      gesture = undefined;
    };
    g.addEventListener("gesturestart", start);
    g.addEventListener("gesturechange", change);
    g.addEventListener("gestureend", end);
    this.#cleanup.push(() => {
      g.removeEventListener("gesturestart", start);
      g.removeEventListener("gesturechange", change);
      g.removeEventListener("gestureend", end);
    });
    // A resized viewport re-fits a fit mode (keeping the centre) and re-lays out.
    const resize = new ResizeObserver(() => {
      if (this.#layout === undefined) return;
      if (typeof this.#zoom === "string") this.#rezoom(undefined);
      else this.#schedule();
    });
    resize.observe(root);
    this.#cleanup.push(() => resize.disconnect());
  }
}

/** Creates a viewer over `options.client`'s document worker. */
export function createViewer(options: CreateViewerOptions): Viewer {
  return new Viewer(options);
}
