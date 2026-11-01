// The viewer: a scroll container whose spacer is the whole document's height, and page
// shells for only the pages in view plus a margin. Each mounted page borrows a canvas from a
// fixed pool and paints in two passes, a low-resolution whole page and then sharp tiles over the
// part in view, through the document worker at `visible` or `prefetch` priority. A page
// that leaves the margin aborts its renders, and a result that arrives for it anyway is dropped,
// never drawn. Zoom keeps the document point under the cursor (or the viewport centre) where it
// is; during a gesture the canvases are stretched by CSS and the sharp re-render waits until the
// gesture settles. Framework-free: UI layers read `getState()`, `subscribe()` and the events.
import { BitmapCache, DEFAULT_CACHE_BYTES } from "../canvas/cache.js";
import { CanvasPool, type CanvasPoolOptions } from "../canvas/pool.js";
import type { ViewerClient } from "../client.js";
import { isRotation, PageLayout, type PageSize, type Rotation, rotated } from "../layout/page-layout.js";
import { type OpenedDocument, type Priority, renderKey, renderKeyFields } from "../protocol.js";
import { lowScale, tilesFor } from "../render/tiles.js";
import {
  contentPosition,
  type DocumentPoint,
  type Point,
  percentOfScale,
  pointAt,
  scaleFor,
  scrollToPoint,
  type Zoom,
} from "../zoom/zoom.js";

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

/** Counters for the perf harness, and for checking the viewer's guarantees in production. */
export interface RenderStats {
  readonly mounted: number;
  readonly peakMounted: number;
  readonly canvases: number;
  readonly peakCanvases: number;
  readonly maxCanvases: number;
  /**
   * Draws onto a page whose shell was out of the DOM at the moment of drawing, counted at the
   * draw itself: always 0 (the guarantee that a cancelled render never paints).
   */
  readonly drawsOnUnmounted: number;
  /** Results that arrived for a page no longer mounted, or a zoom since replaced, and were dropped. */
  readonly dropped: number;
  /** Renders cancelled because their page left the margin or the zoom changed (the client rejected them). */
  readonly aborted: number;
  readonly cacheBytes: number;
  /** Each mounted page's state. */
  readonly pages: Readonly<Record<number, PageRenderState>>;
}

/** The viewer's events and what each carries. */
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

/** `createViewer`'s options: the client, and the tuning the perf harness measures. */
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

/** How a page paints: in view (`visible`), or in the margin around it (`prefetch`). */
type PaintPriority = Exclude<Priority, "background">;

/** One frame's geometry: the layout and the viewport, read once. */
interface Frame {
  readonly layout: PageLayout;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/** One running pass: what it paints, and how to tell it has gone stale. */
interface Pass {
  readonly view: PageView;
  readonly layout: PageLayout;
  readonly priority: PaintPriority;
  readonly signal: AbortSignal;
  /** False once the page was unmounted, aborted or re-laid-out: its results are dropped. */
  readonly live: () => boolean;
}

/** A bitmap a pass holds, and whether it owns it (the cache didn't keep it, so it closes it). */
interface Held {
  readonly bitmap: ImageBitmap;
  readonly owned: boolean;
}

interface PageView {
  readonly index: number;
  readonly shell: HTMLDivElement;
  readonly slots: Map<string, HTMLDivElement>;
  canvas: HTMLCanvasElement | undefined;
  controller: AbortController;
  state: PageRenderState;
  /** The geometry (scale and rotation, `geometryOf`) the canvas was last painted for ("": never). */
  painted: string;
  /** The sharp tiles drawn at `painted`, by `renderKey`. */
  readonly tiles: Set<string>;
  /** A pass is in flight: the next frame waits for it rather than send duplicates. */
  busy: boolean;
  /** No canvas was free: the page waits until one is released rather than retry every frame. */
  waiting: boolean;
  /** Bumped by an abort: an older pass's results are dropped. */
  pass: number;
}

function isAbort(e: unknown): boolean {
  return e instanceof DOMException && e.name === "AbortError";
}

/** What a painted canvas depends on: a zoom or a rotation makes it stale. */
function geometryOf(layout: PageLayout): string {
  return `${layout.scale}:${layout.rotation}`;
}

/** A closed ImageBitmap has no size: one the cache evicted while a pass held it. */
function isClosed(bitmap: ImageBitmap): boolean {
  return bitmap.width === 0 || bitmap.height === 0;
}

/**
 * The viewer: a scroll container over one document's pages. Make one with {@link createViewer},
 * `mount` it, then `open` a document from the client.
 */
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
  #aborted = 0;
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
    this.#relayout(this.#viewport());
    this.#schedule();
  }

  /** The current view state (a new object after each change). */
  getState(): ViewState {
    return this.#state;
  }

  /** Calls `listener` after every state change (for `useSyncExternalStore`). */
  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Listens for `event`; returns a function that stops listening. */
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

  /** Adds a layer to every page, mounted now and later; returns a function that removes it. */
  addLayer(layer: PageLayer): () => void {
    this.#layers.push(layer);
    for (const view of this.#pages.values()) layer.onMount?.(view.index, this.#slot(view, layer.name));
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
  setZoom(zoom: Zoom, options: { readonly anchor?: Point } = {}): void {
    this.#zoom = zoom;
    this.#rezoom(options.anchor);
  }

  /**
   * Rotates the view by `delta` degrees, keeping the viewport's centre.
   *
   * @throws RangeError when `delta` isn't a multiple of 90.
   */
  rotate(delta: number): void {
    const next = (((this.#rotation + delta) % 360) + 360) % 360;
    if (!isRotation(next)) throw new RangeError(`rotate() takes a multiple of 90, got ${delta}`);
    this.#rotation = next;
    this.#rezoom(undefined);
  }

  /** The document point at viewport position (`x`, `y`) in CSS px. */
  pointAt(x: number, y: number): DocumentPoint | undefined {
    const layout = this.#layout;
    if (layout === undefined || layout.pageCount === 0) return undefined;
    return pointAt(layout, this.#root.clientWidth, this.#scroll(), { x, y });
  }

  /** Where `point` is now, in viewport CSS px. */
  positionOf(point: DocumentPoint): Point | undefined {
    const layout = this.#layout;
    if (layout === undefined) return undefined;
    const at = contentPosition(layout, this.#root.clientWidth, point);
    const scroll = this.#scroll();
    return { x: at.x - scroll.left, y: at.y - scroll.top };
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
      aborted: this.#aborted,
      cacheBytes: this.#cache.bytes,
      pages,
    };
  }

  /** Takes the viewer down: renders aborted, canvases and bitmaps freed, listeners removed. */
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

  #viewport() {
    return { width: this.#root.clientWidth || 800, height: this.#root.clientHeight || 600 };
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

  /**
   * Builds the layout for the current zoom and rotation in `viewport` (read by the caller, before
   * any write), capped below the height browsers can scroll, and writes the spacer's size.
   */
  #relayout(viewport: { readonly width: number; readonly height: number }): PageLayout {
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

  #rezoom(anchor: Point | undefined): void {
    // Every read before the first write.
    const viewport = this.#viewport();
    const before = this.#layout;
    if (before === undefined || before.pageCount === 0) {
      this.#relayout(viewport);
      return;
    }
    const at = anchor ?? { x: viewport.width / 2, y: viewport.height / 2 };
    const point = pointAt(before, viewport.width, this.#scroll(), at);
    const after = this.#relayout(viewport);
    const scroll = scrollToPoint(after, viewport.width, point, at);
    this.#root.scrollTop = scroll.top;
    this.#root.scrollLeft = scroll.left;
    // Renders for the old geometry are stale: abort them (a page that failed gets another try);
    // the canvases stretch until the gesture settles and the new pass paints.
    for (const view of this.#pages.values()) {
      this.#abort(view);
      if (view.state === "error") view.state = "placeholder";
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
  #mountRange(frame: Frame): [number, number] {
    const { layout, top, height } = frame;
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
    const frame: Frame = {
      layout,
      top: this.#root.scrollTop,
      width: this.#root.clientWidth,
      height: this.#root.clientHeight,
    };
    const visible = layout.rangeIn(frame.top, frame.top + frame.height);
    const [first, last] = this.#mountRange(frame);
    // Writes.
    this.#placeShells(frame, first, last);
    this.#setState({
      currentPage: layout.pageAt(frame.top + frame.height / 2),
      scrollTop: frame.top,
      visibleRange: visible,
    });
    // Renders: nothing while a zoom gesture is under way.
    if (this.#settling === undefined) this.#paintRange(frame, visible, first, last);
    this.#emit("renderstats", this.stats());
  }

  /** Unmounts the shells outside `first..last`, mounts the missing ones and positions them all. */
  #placeShells(frame: Frame, first: number, last: number): void {
    const { layout, width } = frame;
    for (const [i, view] of this.#pages) if (i < first || i > last) this.#unmount(view);
    for (let i = first; i <= last; i++) {
      const view = this.#pages.get(i) ?? this.#mount(i);
      const w = layout.widths[i] as number;
      const h = layout.heights[i] as number;
      const s = view.shell.style;
      const resized = s.width !== `${w}px`;
      s.top = `${layout.offsetOf(i)}px`;
      s.left = `${layout.leftOf(i, width)}px`;
      s.width = `${w}px`;
      s.height = `${h}px`;
      if (resized) this.#notifyResize(view, w, h);
    }
    if (this.#pages.size > this.#peakMounted) this.#peakMounted = this.#pages.size;
  }

  #notifyResize(view: PageView, width: number, height: number): void {
    for (const layer of this.#layers) layer.onZoom?.(view.index, this.#slot(view, layer.name), { width, height });
  }

  /** Starts the passes: visible pages first, then the margin. */
  #paintRange(
    frame: Frame,
    visible: { readonly first: number; readonly last: number },
    first: number,
    last: number,
  ): void {
    for (let i = visible.first; i <= visible.last; i++) this.#paint(i, "visible", frame);
    for (let i = first; i < visible.first; i++) this.#paint(i, "prefetch", frame);
    for (let i = visible.last + 1; i <= last; i++) this.#paint(i, "prefetch", frame);
  }

  #mount(index: number): PageView {
    const shell = document.createElement("div");
    shell.className = "lamina-page";
    shell.dataset.pageIndex = String(index);
    Object.assign(shell.style, { position: "absolute", contain: "strict", background: "#fff" });
    this.#spacer.append(shell);
    const view: PageView = {
      index,
      shell,
      slots: new Map(),
      canvas: undefined,
      controller: new AbortController(),
      state: "placeholder",
      painted: "",
      tiles: new Set(),
      busy: false,
      waiting: false,
      pass: 0,
    };
    for (const name of SLOTS) this.#slot(view, name);
    this.#pages.set(index, view);
    for (const layer of this.#layers) layer.onMount?.(index, this.#slot(view, layer.name));
    return view;
  }

  /** The page's slot `name`, made (above the others) the first time it is asked for. */
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

  /** Stops the page's renders: their results will be dropped. */
  #abort(view: PageView): void {
    view.controller.abort();
    view.controller = new AbortController();
    view.pass++;
    view.busy = false;
  }

  #unmount(view: PageView): void {
    this.#abort(view);
    for (const layer of this.#layers) layer.onUnmount?.(view.index, this.#slot(view, layer.name));
    this.#releaseCanvas(view);
    view.shell.remove();
    this.#pages.delete(view.index);
  }

  #unmountAll(): void {
    for (const view of [...this.#pages.values()]) this.#unmount(view);
  }

  #releaseCanvas(view: PageView): void {
    if (view.canvas !== undefined) this.#pool.release(view.canvas);
    view.canvas = undefined;
    view.painted = "";
    view.tiles.clear();
    if (view.state !== "error") view.state = "placeholder";
    // A canvas is free again: pages that waited for one try now.
    for (const other of this.#pages.values()) other.waiting = false;
    this.#schedule();
  }

  /** The sharp tiles of page `index` in view in `frame`, with their render requests and keys. */
  #tilesInView(index: number, frame: Frame, ratio: number) {
    const { layout, top, height } = frame;
    const documentId = this.#document?.id ?? 0;
    const cssW = layout.widths[index] as number;
    const cssH = layout.heights[index] as number;
    const pageTop = layout.offsetOf(index);
    const y0 = Math.max(0, top - pageTop);
    const y1 = Math.min(cssH, top + height - pageTop);
    const inView = { x: 0, y: y0, width: cssW, height: Math.max(0, y1 - y0) };
    return tilesFor(inView, { width: cssW, height: cssH }, ratio).map((clip) => {
      const request = { page: index, scale: layout.scale * ratio, rotation: layout.rotation, clip };
      return { clip, request, key: renderKey(request, documentId) };
    });
  }

  /** Starts page `index`'s pass if it isn't busy and its canvas isn't done for `frame` yet. */
  #paint(index: number, priority: PaintPriority, frame: Frame): void {
    const view = this.#pages.get(index);
    if (view === undefined || this.#document === undefined || view.busy || view.waiting || view.state === "error") {
      return;
    }
    const { layout } = frame;
    if (view.painted === geometryOf(layout)) {
      if (priority === "prefetch") return;
      const canvas = view.canvas;
      const ratio = canvas === undefined ? 0 : canvas.width / (layout.widths[index] as number);
      // Sharp means every tile now in view is drawn: scrolling down a tall page sharpens the rest.
      if (ratio > 0 && this.#tilesInView(index, frame, ratio).every((t) => view.tiles.has(t.key))) {
        if (view.state !== "sharp") {
          view.state = "sharp";
          this.#emit("renderstats", this.stats());
        }
        return;
      }
      // Tiles in view are missing (the page scrolled): it is no longer sharp until they are drawn.
      if (view.state === "sharp") view.state = "low";
    }
    view.busy = true;
    const pass = view.pass;
    void this.#pass(view, pass, priority, frame)
      .catch((e) => {
        if (isAbort(e)) {
          this.#aborted++;
          return;
        }
        if (pass !== view.pass) return;
        view.state = "error";
        this.#emit("renderstats", this.stats());
      })
      .finally(() => {
        if (pass !== view.pass) return;
        view.busy = false;
        // The view may have moved while the pass ran: check again on the next frame.
        this.#schedule();
      });
  }

  /** One pass: the blurry whole page when the canvas shows another geometry, then the sharp tiles in view. */
  async #pass(view: PageView, pass: number, priority: PaintPriority, frame: Frame): Promise<void> {
    const { index } = view;
    const { layout } = frame;
    const pass_: Pass = {
      view,
      layout,
      priority,
      signal: view.controller.signal,
      live: () => pass === view.pass && this.#pages.get(index) === view && this.#layout === layout,
    };
    if (view.painted !== geometryOf(layout) && !(await this.#paintLow(pass_))) return;
    if (priority === "prefetch") return;
    await this.#paintTiles(pass_, frame);
  }

  /** The blurry pass: resizes the canvas and stretches a low-resolution page over it. */
  async #paintLow(pass: Pass): Promise<boolean> {
    const { view, layout, priority, signal, live } = pass;
    const { index } = view;
    const dpr = this.#dpr();
    const cssW = layout.widths[index] as number;
    const cssH = layout.heights[index] as number;
    const shown = rotated(this.#sizes[index] as PageSize, layout.rotation);
    const request = { page: index, scale: lowScale(layout.scale * dpr, shown.width), rotation: layout.rotation };
    const low = await this.#bitmap(request, renderKey(request, this.#document?.id ?? 0), priority, signal);
    if (!live()) {
      this.#drop(low);
      return false;
    }
    const canvas = view.canvas ?? this.#attachCanvas(view, priority);
    if (canvas === undefined) {
      view.waiting = true;
      this.#drop(low, false);
      return false;
    }
    const { width, height } = this.#pool.fit(canvas, cssW * dpr, cssH * dpr);
    // Resizing clears the canvas: it happens only now, with the new pixels ready (no white flash).
    canvas.width = width;
    canvas.height = height;
    const drawn = this.#draw(view, low.bitmap, 0, 0, width, height);
    this.#release(low);
    // Not drawn (evicted meanwhile): the canvas stays unpainted, and the next frame tries again.
    if (!drawn) return false;
    view.painted = geometryOf(layout);
    view.tiles.clear();
    view.state = "low";
    this.#pruneOtherGeometries(index, layout, new Set([request.scale, layout.scale * (width / cssW)]));
    this.#emit("renderstats", this.stats());
    return true;
  }

  /** Frees page `index`'s bitmaps of other scales or rotations at once, rather than at the budget. */
  #pruneOtherGeometries(index: number, layout: PageLayout, scales: ReadonlySet<number>): void {
    this.#cache.prune(index, (key) => {
      const { scale, rotation } = renderKeyFields(key);
      return scales.has(scale) && rotation === layout.rotation;
    });
  }

  /** Sharp tiles over the part in view, at the canvas's own pixel ratio (the pool may cap it). */
  async #paintTiles(pass: Pass, frame: Frame): Promise<void> {
    const { view, layout, priority, signal, live } = pass;
    const canvas = view.canvas;
    if (canvas === undefined) return;
    for (const tile of this.#tilesInView(view.index, frame, canvas.width / (layout.widths[view.index] as number))) {
      if (view.tiles.has(tile.key)) continue;
      const got = await this.#bitmap(tile.request, tile.key, priority, signal);
      if (!live()) {
        this.#drop(got);
        return;
      }
      const drawn = this.#draw(view, got.bitmap, tile.clip.x, tile.clip.y);
      this.#release(got);
      if (drawn) view.tiles.add(tile.key);
    }
  }

  /**
   * A bitmap for `request`, from the cache or the worker. `owned` is true when the cache didn't
   * keep it, so the caller closes it after drawing. A cached bitmap another page's pass evicted
   * (closed) meanwhile is rendered again.
   */
  async #bitmap(
    request: Parameters<ViewerClient["render"]>[0],
    key: string,
    priority: PaintPriority,
    signal: AbortSignal,
  ): Promise<Held> {
    const cached = this.#cache.get(key);
    if (cached !== undefined && !isClosed(cached)) return { bitmap: cached, owned: false };
    const { bitmap } = await this.#client.render(request, { priority, signal });
    const kept = this.#cache.set(key, request.page, bitmap, this.#state.currentPage);
    return { bitmap, owned: !kept };
  }

  /** A result that arrived too late: counted, and closed if nobody else holds it. */
  #drop(got: Held, late = true): void {
    if (late) this.#dropped++;
    this.#release(got);
  }

  #release(got: Held): void {
    if (got.owned) got.bitmap.close();
  }

  /** The one place pixels reach a canvas: it counts a draw onto a shell out of the DOM. */
  #draw(view: PageView, bitmap: ImageBitmap, x: number, y: number, width?: number, height?: number): boolean {
    const ctx = view.canvas?.getContext("2d");
    // A bitmap another pass's cache write evicted (closed) after it was fetched: not drawn.
    if (ctx === undefined || ctx === null || isClosed(bitmap)) return false;
    if (!view.shell.isConnected) this.#drawsOnUnmounted++;
    if (width === undefined || height === undefined) ctx.drawImage(bitmap, x, y);
    else ctx.drawImage(bitmap, x, y, width, height);
    return true;
  }

  /**
   * A canvas for `view` from the pool. When the pool is empty, a visible page takes the canvas of
   * the margin page farthest from the current one, so visible pages never wait on prefetched ones.
   */
  /** The margin page with a canvas farthest from the current page, if any. */
  #farthestMarginCanvas(): PageView | undefined {
    const { first, last } = this.#state.visibleRange;
    const current = this.#state.currentPage;
    let victim: PageView | undefined;
    for (const other of this.#pages.values()) {
      const inView = other.index >= first && other.index <= last;
      if (other.canvas === undefined || inView) continue;
      if (victim === undefined || Math.abs(other.index - current) > Math.abs(victim.index - current)) victim = other;
    }
    return victim;
  }

  #attachCanvas(view: PageView, priority: PaintPriority): HTMLCanvasElement | undefined {
    let canvas = this.#pool.acquire();
    const victim = canvas === undefined && priority === "visible" ? this.#farthestMarginCanvas() : undefined;
    if (victim !== undefined) {
      this.#abort(victim);
      this.#releaseCanvas(victim);
      canvas = this.#pool.acquire();
    }
    if (canvas === undefined) return undefined;
    canvas.className = "lamina-page-canvas";
    Object.assign(canvas.style, { position: "absolute", inset: "0", width: "100%", height: "100%" });
    view.shell.prepend(canvas);
    view.canvas = canvas;
    return canvas;
  }

  /** Adds `listener` to the root for `type`, removed by destroy(). Safari's gesture events take any name. */
  #on<K extends keyof HTMLElementEventMap>(
    type: K | string,
    f: (e: HTMLElementEventMap[K]) => void,
    options?: AddEventListenerOptions,
  ): void {
    const root = this.#root;
    root.addEventListener(type, f as EventListener, options);
    this.#cleanup.push(() => root.removeEventListener(type, f as EventListener, options));
  }

  /** `clientX`/`clientY` as a viewport position. */
  #anchorAt(clientX: number, clientY: number): Point {
    const rect = this.#root.getBoundingClientRect();
    return { x: clientX - rect.left, y: clientY - rect.top };
  }

  #listen(): void {
    this.#on("scroll", () => this.#schedule(), { passive: true });
    this.#listenWheel();
    this.#listenKeys();
    this.#listenTouchPinch();
    this.#listenGestures();
    this.#listenResize();
  }

  /** Ctrl/cmd+wheel and trackpad pinch (which browsers send as ctrl+wheel): zoom at the cursor. */
  #listenWheel(): void {
    this.#on<"wheel">(
      "wheel",
      (e) => {
        if (!e.ctrlKey && !e.metaKey) return;
        e.preventDefault();
        this.setZoom(this.#state.percent * Math.exp(-e.deltaY * 0.01), {
          anchor: this.#anchorAt(e.clientX, e.clientY),
        });
      },
      { passive: false },
    );
  }

  /** `+` and `-` zoom by a quarter, `0` goes to actual size. */
  #listenKeys(): void {
    this.#on<"keydown">("keydown", (e) => {
      if (e.altKey) return;
      const p = this.#state.percent;
      if (e.key === "+" || e.key === "=") this.setZoom(p * 1.25);
      else if (e.key === "-") this.setZoom(p / 1.25);
      else if (e.key === "0") this.setZoom("actual-size");
      else return;
      e.preventDefault();
    });
  }

  /** Touch pinch: two pointers; the zoom follows their distance, anchored at their midpoint. */
  #listenTouchPinch(): void {
    const pointers = new Map<number, Point>();
    let pinch: { distance: number; percent: number } | undefined;
    const spread = () => {
      const [a, b] = [...pointers.values()];
      return a === undefined || b === undefined
        ? undefined
        : { d: Math.hypot(a.x - b.x, a.y - b.y), x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    };
    this.#on<"pointerdown">("pointerdown", (e) => {
      if (e.pointerType !== "touch") return;
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const s = spread();
      if (s !== undefined) pinch = { distance: s.d, percent: this.#state.percent };
    });
    this.#on<"pointermove">("pointermove", (e) => {
      if (!pointers.has(e.pointerId)) return;
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const s = spread();
      if (s === undefined || pinch === undefined || pinch.distance === 0) return;
      this.setZoom(pinch.percent * (s.d / pinch.distance), { anchor: this.#anchorAt(s.x, s.y) });
    });
    const up = (e: PointerEvent) => {
      pointers.delete(e.pointerId);
      if (pointers.size < 2) pinch = undefined;
    };
    this.#on<"pointerup">("pointerup", up);
    this.#on<"pointercancel">("pointercancel", up);
  }

  /** Safari's trackpad pinch: gesture events with a scale. */
  #listenGestures(): void {
    let gesture: number | undefined;
    type GestureEvent = Event & { scale: number; clientX: number; clientY: number };
    this.#on("gesturestart", (e: Event) => {
      e.preventDefault();
      gesture = this.#state.percent;
    });
    this.#on("gesturechange", (e: Event) => {
      if (gesture === undefined) return;
      e.preventDefault();
      const ge = e as GestureEvent;
      this.setZoom(gesture * ge.scale, { anchor: this.#anchorAt(ge.clientX, ge.clientY) });
    });
    this.#on("gestureend", () => {
      gesture = undefined;
    });
  }

  /** A resized viewport re-fits a fit mode (keeping the centre) and re-lays out. */
  #listenResize(): void {
    const resize = new ResizeObserver(() => {
      if (this.#layout === undefined) return;
      if (typeof this.#zoom === "string") this.#rezoom(undefined);
      else this.#schedule();
    });
    resize.observe(this.#root);
    this.#cleanup.push(() => resize.disconnect());
  }
}

/** Creates a viewer over `options.client`'s document worker. */
export function createViewer(options: CreateViewerOptions): Viewer {
  return new Viewer(options);
}
