// The viewer: a scroll container whose spacer is the whole document's height, and page
// shells for only the pages in view plus a margin. Each mounted page borrows a canvas from a
// fixed pool and paints in two passes, a low-resolution whole page and then sharp tiles over the
// part in view, through the document worker at `visible` or `prefetch` priority. A page too large
// for its canvas at the full pixel ratio (past a pixel cap, or the pool's fit) keeps that canvas as
// a backdrop at a lower one, and a detail canvas over the part in view takes the sharp tiles. A page
// that leaves the margin aborts its renders, and a result that arrives for it anyway is dropped,
// never drawn. Zoom keeps the document point under the cursor (or the viewport centre) where it
// is; during a gesture the canvases are stretched by CSS and the sharp re-render waits until the
// gesture settles. Framework-free: UI layers read `getState()`, `subscribe()` and the events.

import { type PageAppearance, type PixelRect, pagePixels } from "@laminapdf/render";
import type { Op, Target } from "@laminapdf/sdk";
import { BitmapCache } from "../canvas/cache.js";
import { CanvasPool, type CanvasPoolOptions } from "../canvas/pool.js";
import type { ViewerCallOptions, ViewerClient } from "../client.js";
import { EditSession } from "../edit/session.js";
import { FindController } from "../find/controller.js";
import {
  type HistoryPlace,
  NavigationHistory,
  type NavigationHistoryOptions,
  type ViewPosition,
} from "../history/history.js";
import type { CopyMode } from "../layers/copy.js";
import { HighlightLayer } from "../layers/highlight-layer.js";
import { LinkLayer, type LinkLayerOptions, type LinkPreviewRequest } from "../layers/link-layer.js";
import { TextLayer, type TextLayerOptions, type TextSelection } from "../layers/text-layer.js";
import { isRotation, keptTop, PageLayout, type PageSize, type Rotation, rotated } from "../layout/page-layout.js";
import { range } from "../layout/range.js";
import {
  appearanceField,
  appearanceKey,
  type EditResult,
  type OpenedDocument,
  type Priority,
  renderKeyFields,
  viewKey,
} from "../protocol.js";
import { backdropRatio, bounds, contains, lowScale, sharpScale, tilesFor } from "../render/tiles.js";
import { destinationView } from "../zoom/destination.js";
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
import { type DeviceHints, deviceHints, viewerBudgets } from "./budgets.js";
import type { LinkPreviewPicture, LinkPreviews } from "./previews.js";
import { TouchGestures, type TouchHost } from "./touch.js";
import { setUserSelect } from "./user-select.js";

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
  /** How the pages' colours are drawn (`"original"`: their own). */
  readonly appearance: PageAppearance;
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
  /** Detail canvases in use now: one per page in view whose backdrop is below the full pixel ratio. */
  readonly detailCanvases: number;
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
  /** Link preview pictures kept (at most `PREVIEW_CACHE`). */
  readonly previewBitmaps: number;
  /** Each mounted page's state. */
  readonly pages: Readonly<Record<number, PageRenderState>>;
}

/** The viewer's events and what each carries. */
export interface ViewerEventMap {
  pagechange: { readonly page: number };
  zoomchange: { readonly zoom: Zoom; readonly percent: number };
  renderstats: RenderStats;
  /** The text selection changed: `undefined` when it left the viewer (UI/UX plan §5.3). */
  selectionchange: { readonly selection: TextSelection | undefined };
  /** A link asks to be previewed (hover, focus or long-press; R07). */
  linkpreview: LinkPreviewRequest;
  /** The preview of `anchor`'s link should go. */
  linkpreviewend: { readonly anchor: HTMLElement };
  /** `invalidate` redrew these pages; each has its displayed size in points now. */
  invalidate: { readonly pages: readonly InvalidatedPage[] };
}

/** A page `invalidate` redrew: its index and its displayed size in points now. */
export interface InvalidatedPage {
  readonly index: number;
  readonly width: number;
  readonly height: number;
}

/** A layer that later todos mount into each page (text, annotations, highlights, …). */
export interface PageLayer {
  /**
   * Its slot: `text`, `annot` and `highlight` exist on every page; another name gets a new one on top.
   * Every slot but `text` lets the pointer through, so the text stays selectable under it. An element a
   * layer wants clicked, such as a link, sets `pointer-events: auto` on itself.
   */
  readonly name: string;
  /** Page `page` was mounted: `element` is the layer's slot in its shell. */
  onMount?(page: number, element: HTMLElement): void;
  /** Page `page` is about to be unmounted (its renders already aborted). */
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
  /**
   * The page canvases (backdrops): default 12 canvases and 40 MP in all, less on a touch device
   * (view/budgets.ts), as are the next three.
   */
  readonly pool?: CanvasPoolOptions;
  /** The detail canvases over the part in view: default 3 canvases and 24 MP in all. */
  readonly detailPool?: CanvasPoolOptions;
  /** The bitmap cache's budget in bytes (default 256 MB). */
  readonly cacheBytes?: number;
  /** The most device pixels one page canvas holds (default 8 MP; 5 MP on a touch device). */
  readonly backdropMaxPixels?: number;
  /** The most device pixels any one canvas holds, a detail canvas too (default no cap; 5 MP on a touch device). */
  readonly canvasMaxPixels?: number;
  /** The device the budgets are chosen for; detected (a coarse pointer, `navigator.deviceMemory`) when left out. */
  readonly device?: DeviceHints;
  /** How long a zoom gesture must pause before the sharp re-render, in ms (default 150). */
  readonly settleMs?: number;
  /** The device pixel ratio (default `globalThis.devicePixelRatio`). */
  readonly dpr?: () => number;
  /** The initial zoom (default `fit-width`). */
  readonly zoom?: Zoom;
  /**
   * The text layer: selectable, copyable text over every page, and a tagged document's structure
   * for screen readers (default on; `false` leaves it out, an object sets its options).
   */
  readonly text?: boolean | TextLayerOptions;
  /**
   * The link layer: an anchor over each link, internal ones going there as history jumps (default
   * on; `false` leaves it out, an object sets its options).
   */
  readonly links?: boolean | LinkLayerOptions;
  /** The back and forward history's options. */
  readonly history?: NavigationHistoryOptions;
  /** What assistive technology hears of each page, and where focus goes after a jump. */
  readonly pages?: PageRegionOptions;
}

/**
 * Each page shell is a region that assistive technology names ("Page iv, 4 of 120") and can land
 * on: the viewer has no strings of its own, so the app gives the words.
 */
export interface PageRegionOptions {
  /** A page's name, from its index and the page count (default "Page 4 of 120"). */
  readonly name?: (index: number, count: number) => string;
  /** What a page is called as a kind of region (default "page"). */
  readonly roleDescription?: string;
  /**
   * Whether a history jump (a link, the outline, a thumbnail, go to page) moves focus to the page
   * it lands on, once that page is mounted (default false).
   */
  readonly focusOnJump?: boolean;
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
  readonly left: number;
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

/** A tile to draw: where it goes in the page's pixels, its render request and its cache key. */
interface Tile {
  readonly clip: PixelRect;
  readonly request: { page: number; scale: number; rotation: Rotation; clip: PixelRect; appearance?: PageAppearance };
  readonly key: string;
}

/** A detail canvas: the part of a page in view at the full pixel ratio, over its backdrop. */
interface Detail {
  readonly canvas: HTMLCanvasElement;
  /** Its rectangle in the page's pixels at `scale`. */
  readonly rect: PixelRect;
  /** Device px per pt: the layout's scale × the device pixel ratio. */
  readonly scale: number;
  /** The geometry (`geometryOf`) it was placed for. */
  readonly geometry: string;
  /** The sharp tiles drawn into it, by `viewKey`. */
  readonly tiles: Set<string>;
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
  /** The page appearance (`appearanceKey`) the canvas shows. */
  appearance: string;
  /**
   * The scale (device px per pt) of the canvas painted at `painted`: the sharp tiles' scale. The
   * canvas is the page at this scale exactly, as the renderer sizes it (`pagePixels`).
   */
  scale: number;
  /** The sharp tiles drawn at `painted`, by `viewKey`. */
  readonly tiles: Set<string>;
  /** The detail canvas, while the page is in view and its canvas is below the full pixel ratio. */
  detail: Detail | undefined;
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

/** The state before a document opens. */
function emptyState(zoom: Zoom): ViewState {
  return {
    pageCount: 0,
    currentPage: 0,
    zoom,
    percent: 100,
    rotation: 0,
    appearance: "original",
    scrollTop: 0,
    visibleRange: { first: 0, last: -1 },
  };
}

/** What a painted canvas depends on: a zoom or a rotation makes it stale. */
function geometryOf(layout: PageLayout): string {
  return `${layout.scale}:${layout.rotation}`;
}

/** The view rotation a `geometryOf` string was painted for. */
function rotationOf(geometry: string): number {
  return Number(geometry.slice(geometry.indexOf(":") + 1));
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
  readonly #detailPool: CanvasPool;
  readonly #backdropMaxPixels: number;
  readonly #canvasMaxPixels: number;
  readonly #touch: TouchGestures<DocumentPoint>;
  readonly #cache: BitmapCache;
  readonly #root: HTMLDivElement;
  readonly #spacer: HTMLDivElement;
  readonly #pages = new Map<number, PageView>();
  readonly #layers: PageLayer[] = [];
  /** The text layer, unless `text: false`. */
  readonly #text: TextLayer | undefined;
  readonly #highlights: HighlightLayer;
  /** The link layer, unless `links: false`. */
  readonly #links: LinkLayer | undefined;
  /** The link previews' pictures, loaded with their code on the first preview (not at startup). */
  #previews: LinkPreviews | undefined;
  #loadingPreviews: Promise<LinkPreviews> | undefined;
  /** Find in this viewer's document: query, options, matches, next and previous. */
  readonly find: FindController;
  /**
   * Back and forward through the places jumped from and to: links, `jumpToPage`,
   * `jumpToDestination` and find's matches record a jump; scrolling doesn't.
   */
  readonly history: NavigationHistory;
  /**
   * The open document's edit session: its history, and edits, undo, redo and going to a step,
   * each redrawing the pages it touched. One call runs at a time, in the order they were made.
   */
  readonly edits: EditSession;
  /** Pages kept mounted whatever the scroll (`keepMounted`): those a selection touches. */
  #pinned = new Set<number>();
  readonly #regions: PageRegionOptions;
  /** The page a jump landed on, focused once it is mounted. */
  #focusTarget: number | undefined;
  /** True while a jump that leaves focus where it is (a find match) runs. */
  #keepFocus = false;
  readonly #listeners = new Set<() => void>();
  readonly #events = new Map<keyof ViewerEventMap, Set<(data: never) => void>>();
  readonly #cleanup: (() => void)[] = [];
  /** The open document's ID (renders name it), or undefined before one opens. */
  #documentId: number | undefined;
  /** Counts `open`s: an edit answered for a document since replaced redraws nothing. */
  #opens = 0;
  #sizes: PageSize[] = [];
  /** Each edited page's revision (`invalidate` bumps it), part of its bitmaps' cache keys. */
  #revisions = new Map<number, number>();
  #layout: PageLayout | undefined;
  #zoom: Zoom;
  #rotation: Rotation = 0;
  /**
   * Where the view goes once it has a size, when it was opened or restored without one (a hidden
   * host ignores scrolling): the top of a new document, or a kept position.
   */
  #pending: ViewPosition | "top" | undefined;
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
    const budgets = viewerBudgets(options.device ?? deviceHints());
    this.#pool = new CanvasPool({ ...budgets.pool, ...options.pool });
    this.#detailPool = new CanvasPool({ ...budgets.detailPool, ...options.detailPool });
    this.#cache = new BitmapCache(options.cacheBytes ?? budgets.cacheBytes);
    this.#backdropMaxPixels = options.backdropMaxPixels ?? budgets.backdropMaxPixels;
    this.#canvasMaxPixels = options.canvasMaxPixels ?? budgets.canvasMaxPixels;
    this.#touch = new TouchGestures(this.#touchHost());
    this.#zoom = options.zoom ?? "fit-width";
    this.#root = document.createElement("div");
    this.#root.className = "lamina-viewer";
    this.#root.tabIndex = 0;
    // `touch-action: none`: the viewer pans and zooms under a finger itself, so the browser's own
    // page zoom never fights it, and zooming the chrome stays the browser's (UI/UX plan §5.4).
    Object.assign(this.#root.style, {
      position: "relative",
      overflow: "auto",
      width: "100%",
      height: "100%",
      touchAction: "none",
    });
    this.#spacer = document.createElement("div");
    this.#spacer.className = "lamina-viewer-content";
    Object.assign(this.#spacer.style, { position: "relative" });
    this.#root.append(this.#spacer);
    this.#state = emptyState(this.#zoom);
    this.#listen();
    this.#text = this.#textLayer(options.text);
    if (this.#text !== undefined) this.addLayer(this.#text);
    this.history = new NavigationHistory({ place: () => this.#place(), show: (p) => this.#show(p) }, options.history);
    this.#regions = options.pages ?? {};
    if (this.#regions.focusOnJump === true) {
      this.history.onEntry((entry, replaced) => {
        if (replaced || this.#keepFocus) return;
        this.#focusTarget = entry.point.page;
        this.#schedule();
      });
    }
    this.find = new FindController({
      find: (query, find, pages, signal) => this.#client.find(query, find, pages, { signal }),
      pageCount: () => this.#sizes.length,
      currentPage: () => this.#state.currentPage,
      // Each match find moves to is a jump, so Back returns to where the search started.
      // A match is a jump too, but focus stays in the find field.
      reveal: (page, quads) => this.#jumpKeepingFocus(() => this.#reveal(page, quads)),
      redraw: (pages) => this.#highlights.refresh(pages),
    });
    this.#highlights = new HighlightLayer({
      matchesOn: (page) => this.find.matchesOn(page),
      points: (page) => this.#sizes[page],
      rotation: () => this.#rotation,
    });
    this.addLayer(this.#highlights);
    this.#links = this.#linkLayer(options.links);
    this.edits = this.#editSession();
  }

  /**
   * The edit session over this view's calls, which follows a restarted worker: its replay tells
   * the session its history (`edit`), and lost edits leave it empty.
   */
  #editSession(): EditSession {
    const edits = new EditSession({
      applyEdit: (ops, call) => this.applyEdit(ops, call),
      undoEdit: () => this.#editCall(() => this.#client.undoEdit()),
      redoEdit: () => this.#editCall(() => this.#client.redoEdit()),
      goToStep: (position) => this.#editCall(() => this.#client.goToStep(position)),
      editHistory: () => this.#client.editHistory(),
    });
    this.#cleanup.push(
      this.#client.on("restarted", ({ reopened, edits: replay }) => {
        if (reopened) this.#reopened();
        if (replay === "lost") edits.reset(this.#client.document);
      }),
      this.#client.on("edit", ({ history }) => edits.follow(history)),
    );
    return edits;
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
    this.#previews?.clear();
    this.#documentId = document.id;
    this.#opens++;
    this.#revisions = new Map();
    this.#pinned = new Set();
    this.#focusTarget = undefined;
    this.find.reset();
    this.history.reset();
    this.edits.reset(document);
    this.#text?.open();
    this.#sizes = document.pages.map((p) => ({ width: p.width, height: p.height }));
    this.#layout = undefined;
    this.#root.scrollTop = 0;
    this.#root.scrollLeft = 0;
    // A hidden view keeps its old scroll: the new document starts at its top once it shows.
    this.#pending = this.#sized() ? undefined : "top";
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

  /**
   * Copies the text selection to the clipboard: by paragraph in reading order, or exact, as laid
   * out. Call it in a user gesture (a click, a key). Resolves false when nothing is selected or
   * the clipboard refused.
   */
  copySelection(mode: CopyMode = "paragraph"): Promise<boolean> {
    return this.#text?.copy(mode) ?? Promise.resolve(false);
  }

  /** Whether a plain drag selects a rectangle, as an Alt/Option-drag does (column selection). */
  get columnSelect(): boolean {
    return this.#text?.columnMode ?? false;
  }

  /** Turns column selection by plain drag on or off; Alt/Option-drag selects a rectangle either way. */
  set columnSelect(on: boolean) {
    if (this.#text !== undefined) this.#text.columnMode = on;
  }

  /**
   * Keeps `pages` mounted wherever the view scrolls, until the next call replaces them: the pages
   * a text selection touches, so scrolling never takes the selection away. A pinned page out of
   * view keeps its shell and its text; its canvas goes back to the pool like any other.
   */
  keepMounted(pages: Iterable<number>): void {
    const next = new Set(pages);
    if (next.size === this.#pinned.size && [...next].every((p) => this.#pinned.has(p))) return;
    this.#pinned = next;
    this.#schedule();
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
    this.#touch.stopMomentum();
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
   * Goes to `target` (an outline item's, a named destination's or a link's, from
   * `ViewerClient.outline` or `namedDestination`): its zoom if it sets one, then its point at the
   * viewport's top-left, or its page's top when it names no point (ISO 32000-2 §12.3.2.2).
   */
  goToDestination(target: Target): void {
    this.#touch.stopMomentum();
    const size = this.#sizes[target.page];
    if (this.#layout === undefined || size === undefined) return;
    const view = destinationView(target, size, this.#rotation, this.#viewport(), this.#gap);
    if (view.zoom !== undefined) {
      this.#zoom = view.zoom;
      this.#rezoom(undefined);
    }
    const layout = this.#layout;
    const at = contentPosition(layout, this.#root.clientWidth, { page: view.page, x: view.x ?? 0, y: view.y ?? 0 });
    this.#root.scrollTop = Math.max(0, view.y === undefined ? layout.offsetOf(view.page) - this.#gap / 2 : at.y);
    if (view.x !== undefined) this.#root.scrollLeft = Math.max(0, at.x);
    this.#schedule();
  }

  /** {@link goToPage} as a history jump: Back returns to where the view was. */
  jumpToPage(index: number): void {
    this.history.jump(() => this.goToPage(index));
  }

  /** {@link goToDestination} as a history jump: Back returns to where the view was. */
  jumpToDestination(target: Target): void {
    this.history.jump(() => this.goToDestination(target));
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
    // A half turn keeps every page's size, so no layer hears of it from a resize.
    this.#text?.refresh();
    this.#highlights.refresh();
    this.#links?.refresh();
  }

  /**
   * Draws the pages in `appearance` (Sepia, Dark, High contrast or their own colours), keeping the
   * scroll position: pages in view paint again, a quick pass then sharp tiles, and none of their
   * old pixels is kept; other pages give their canvas back and paint when they come into view.
   * The thumbnails follow.
   */
  setAppearance(appearance: PageAppearance): void {
    if (appearanceKey(appearance) === appearanceKey(this.#state.appearance)) return;
    const { first, last } = this.#state.visibleRange;
    for (const view of [...this.#pages.values()]) {
      this.#abort(view);
      view.tiles.clear();
      view.detail?.tiles.clear();
      if (view.index >= first && view.index <= last) {
        view.appearance = appearanceKey(appearance);
        view.painted = "";
      } else {
        // Its shell shows the new paper until it is seen (`#readyForAppearance`).
        this.#releaseCanvas(view);
      }
    }
    this.#previews?.clear();
    this.#setState({ appearance });
    this.#schedule();
  }

  /**
   * Where `target` goes, as a picture for a link preview (R07): one render of the region around its
   * point (or the top of its page) through the document worker, and the text of the lines it shows.
   * The viewer keeps the last few (`PREVIEW_CACHE`) and closes them on a zoom, rotation or
   * appearance change and when the document closes, so draw the bitmap at once and never close it.
   * Rejects with `AbortError` when `signal` aborts or the pictures are cleared while it draws.
   */
  preview(target: Target, options: { readonly signal?: AbortSignal } = {}): Promise<LinkPreviewPicture> {
    return this.#linkPreviews().then((previews) =>
      previews.picture(target, options.signal ?? new AbortController().signal),
    );
  }

  /** The pointer is over the link preview (`true`) or has left it: a hovered preview stays (WCAG 1.4.13). */
  holdLinkPreview(held: boolean): void {
    this.#links?.hold(held);
  }

  /** Escape or Close on the link preview: it goes, and stays hidden until its link is left. */
  dismissLinkPreview(): void {
    this.#links?.dismiss();
  }

  /** Whether a link is previewed or focused, so {@link followLinkPreview} has one (`nav.follow-link`). */
  canFollowLink(): boolean {
    return this.#links?.canFollow() ?? false;
  }

  /** Follows the previewed link, else the focused one (`nav.follow-link`); `false` when there is neither. */
  followLinkPreview(): boolean {
    return this.#links?.follow() ?? false;
  }

  /**
   * Draws `pages` again: an edit changed them. Their bitmaps go, here and in a thumbnail strip,
   * and their layers read their text and links again; every other page keeps what it drew.
   * `sizes` gives the pages whose displayed size in points changed (a quarter turn swaps width and
   * height): the layout takes them, and the page at the top of the viewport stays there. A page in
   * view keeps its old pixels until the new ones are drawn over them.
   */
  invalidate(pages: Iterable<number>, sizes?: ReadonlyMap<number, PageSize>): void {
    const count = this.#sizes.length;
    const changed = new Set(
      [...pages, ...(sizes?.keys() ?? [])].filter((p) => Number.isInteger(p) && p >= 0 && p < count),
    );
    if (changed.size === 0) return;
    const resized = new Set<number>();
    for (const [page, size] of sizes ?? []) {
      const old = this.#sizes[page];
      if (old === undefined || (old.width === size.width && old.height === size.height)) continue;
      this.#sizes[page] = { width: size.width, height: size.height };
      resized.add(page);
    }
    for (const page of changed) {
      this.#revisions.set(page, (this.#revisions.get(page) ?? 0) + 1);
      this.#cache.prune(page, () => false);
    }
    if (resized.size > 0) this.#resize();
    const { first, last } = this.#state.visibleRange;
    for (const page of changed) {
      const view = this.#pages.get(page);
      if (view === undefined) continue;
      this.#redraw(view, resized.has(page) || page < first || page > last);
      this.#remountLayers(view);
    }
    this.find.refresh([...changed]);
    const list = [...changed].sort((a, b) => a - b);
    this.#emit("invalidate", { pages: list.map((index) => ({ index, ...(this.#sizes[index] as PageSize) })) });
    this.#emit("renderstats", this.stats());
    this.#schedule();
  }

  /**
   * Applies `ops` to the open document through the client (`ViewerClient.applyEdit`) and redraws
   * the pages they touched ({@link invalidate}): an edit on screen in one call. A failed edit
   * changes nothing and redraws nothing.
   */
  applyEdit(ops: readonly Op[], options?: ViewerCallOptions & { readonly label?: string }): Promise<EditResult> {
    return this.#editCall(() => this.#client.applyEdit(ops, options));
  }

  /** Runs an edit session call and redraws the pages it touched, unless another document opened meanwhile. */
  async #editCall(call: () => Promise<EditResult>): Promise<EditResult> {
    const opens = this.#opens;
    const result = await call();
    // Another document opened meanwhile: these pages aren't its pages.
    if (this.#opens === opens) {
      this.invalidate(
        result.pages.map((page) => page.index),
        new Map(result.pages.map((page) => [page.index, page])),
      );
    }
    return result;
  }

  /**
   * Every page's displayed size in points, as the view lays it out: the sizes the document opened
   * with, and those edits gave since (`invalidate`). A thumbnail strip lays its pages out from it.
   * With `documentId`, empty unless the view shows that document: a page count alone can't tell two
   * documents apart.
   */
  pageSizes(documentId?: number): readonly PageSize[] {
    if (documentId !== undefined && documentId !== this.#documentId) return [];
    return this.#sizes;
  }

  /**
   * Where the reader is: the document point a gap below the viewport's top-left, the zoom and the
   * rotation; `undefined` without a document. {@link restore} puts the view back there.
   */
  viewPosition(): ViewPosition | undefined {
    const pending = this.#pending;
    if (pending === "top" && this.#sizes.length > 0) {
      return { point: { page: 0, x: 0, y: 0 }, zoom: this.#zoom, rotation: this.#rotation };
    }
    if (pending !== undefined && pending !== "top") return pending;
    const place = this.#place();
    return place === undefined ? undefined : { ...place, rotation: this.#rotation };
  }

  /**
   * Puts the view at `position` (a {@link viewPosition} kept from an earlier visit), as a history
   * jump from where the view is, so Back returns there. Its page is clamped to the document's and
   * its point to the page. Called in the task that opened the document, before the next frame, it
   * paints no other page first.
   */
  restore(position: ViewPosition): void {
    if (this.#layout === undefined || this.#sizes.length === 0) return;
    // Hidden: it takes place once the view has a size (`#listenResize`).
    if (!this.#sized()) {
      const page = Math.min(this.#sizes.length - 1, Math.max(0, Math.trunc(position.point.page) || 0));
      this.#pending = { ...position, point: { ...position.point, page } };
      return;
    }
    this.#pending = undefined;
    this.history.jump(() => {
      const turned = isRotation(position.rotation) && position.rotation !== this.#rotation;
      if (turned) this.#rotation = position.rotation;
      const clamp = (v: number) => (Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0);
      const page = Math.min(this.#sizes.length - 1, Math.max(0, Math.trunc(position.point.page) || 0));
      // The zoom is set before the scroll: #show only rezooms when the zoom changes.
      this.#zoom = position.zoom;
      this.#rezoom(undefined);
      this.#show({ point: { page, x: clamp(position.point.x), y: clamp(position.point.y) }, zoom: position.zoom });
      // As rotate() does: the layers follow once the pages are laid out for the new rotation.
      if (turned) {
        this.#text?.refresh();
        this.#highlights.refresh();
        this.#links?.refresh();
      }
    });
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
      detailCanvases: this.#detailPool.inUse,
      drawsOnUnmounted: this.#drawsOnUnmounted,
      dropped: this.#dropped,
      aborted: this.#aborted,
      cacheBytes: this.#cache.bytes,
      previewBitmaps: this.#previews?.size ?? 0,
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
    this.#previews?.clear();
    this.find.reset();
    this.#text?.destroy();
    this.#links?.destroy();
    for (const f of this.#cleanup.splice(0)) f();
    this.#root.remove();
  }

  // --- internals ---

  /**
   * The link previews' pictures, drawn at this viewer's zoom, rotation and appearance. Their code
   * loads on the first preview, so the reader's startup script doesn't carry it.
   */
  #linkPreviews(): Promise<LinkPreviews> {
    this.#loadingPreviews ??= import("./previews.js").then(({ LinkPreviews }) => {
      this.#previews = new LinkPreviews({
        client: this.#client,
        size: (page) => this.#sizes[page],
        rotation: () => this.#rotation,
        percent: () => this.#state.percent,
        appearance: () => this.#state.appearance,
        dpr: () => this.#dpr(),
      });
      return this.#previews;
    });
    return this.#loadingPreviews;
  }

  /** The link layer, added to every page, unless `links` is `false`. */
  #linkLayer(links: CreateViewerOptions["links"]): LinkLayer | undefined {
    if (links === false) return undefined;
    const layer = new LinkLayer(
      {
        root: this.#root,
        client: this.#client,
        points: (page) => this.#sizes[page],
        rotation: () => this.#rotation,
        pageCount: () => this.#sizes.length,
        goTo: (target) => this.jumpToDestination(target),
        goToPage: (page) => this.jumpToPage(page),
        preview: (request) => this.#emit("linkpreview", request),
        previewEnd: (anchor) => this.#emit("linkpreviewend", { anchor }),
      },
      links === true || links === undefined ? {} : links,
    );
    this.addLayer(layer);
    return layer;
  }

  /**
   * Where the view is, for the history: the document point a gap below the viewport's top-left (so
   * a page that `goToPage` put at the top, half a gap down, is the one named), and the zoom.
   */
  #place(): HistoryPlace | undefined {
    const layout = this.#layout;
    if (layout === undefined || layout.pageCount === 0) return undefined;
    return { point: pointAt(layout, this.#root.clientWidth, this.#scroll(), { x: 0, y: this.#gap }), zoom: this.#zoom };
  }

  /** Puts the view back at a history `place`: its zoom, then its point a gap below the viewport's top-left. */
  #show(place: HistoryPlace): void {
    this.#touch.stopMomentum();
    if (place.zoom !== this.#zoom) {
      this.#zoom = place.zoom;
      this.#rezoom(undefined);
    }
    const layout = this.#layout;
    if (layout === undefined || place.point.page >= layout.pageCount) return;
    const at = contentPosition(layout, this.#root.clientWidth, place.point);
    this.#root.scrollTop = Math.max(0, at.y - this.#gap);
    this.#root.scrollLeft = Math.max(0, at.x);
    this.#schedule();
  }

  /**
   * Scrolls so that `quads` (a find match, in page `page`'s points) are in view: nothing moves
   * when they already are, else they come to the viewport's centre.
   */
  #reveal(page: number, quads: readonly number[]): void {
    const layout = this.#layout;
    const size = this.#sizes[page];
    if (layout === undefined || size === undefined || page >= layout.pageCount || quads.length < 8) return;
    this.#touch.stopMomentum();
    // The corners as fractions of the page, turned with the view (clockwise).
    const turn = (fx: number, fy: number): [number, number] =>
      ({ 0: [fx, fy], 90: [1 - fy, fx], 180: [1 - fx, 1 - fy], 270: [fy, 1 - fx] })[this.#rotation] as [number, number];
    const xs: number[] = [];
    const ys: number[] = [];
    for (let at = 0; at + 1 < quads.length; at += 2) {
      const [x, y] = turn((quads[at] ?? 0) / size.width, (quads[at + 1] ?? 0) / size.height);
      xs.push(x);
      ys.push(y);
    }
    const viewportWidth = this.#root.clientWidth;
    const pageLeft = layout.leftOf(page, viewportWidth);
    const pageTop = layout.offsetOf(page);
    const width = layout.widths[page] ?? 0;
    const height = layout.heights[page] ?? 0;
    const left = pageLeft + Math.min(...xs) * width;
    const right = pageLeft + Math.max(...xs) * width;
    const top = pageTop + Math.min(...ys) * height;
    const bottom = pageTop + Math.max(...ys) * height;
    const view = this.#scroll();
    const viewportHeight = this.#root.clientHeight;
    if (top < view.top || bottom > view.top + viewportHeight) {
      this.#root.scrollTop = Math.max(0, (top + bottom) / 2 - viewportHeight / 2);
    }
    if (left < view.left || right > view.left + viewportWidth) {
      this.#root.scrollLeft = Math.max(0, (left + right) / 2 - viewportWidth / 2);
    }
    this.#schedule();
  }

  /**
   * The worker restarted and opened the file again as it was, without the edits: the edited pages
   * take their sizes from it and draw again (at a new revision, so no bitmap of theirs is reused).
   */
  #reopened(): void {
    const document = this.#client.document;
    if (document === undefined || document.pages.length !== this.#sizes.length) return;
    this.#documentId = document.id;
    const edited = [...this.#revisions.keys()];
    const sizes = new Map(edited.map((index) => [index, document.pages[index] as PageSize]));
    this.invalidate(edited, sizes);
  }

  /**
   * Lays the pages out again after some changed size, keeping the page at the viewport's top where
   * it is (scaled into its new height, if it is one that changed). A fixed zoom stays; a fit mode
   * fits again, since a page turned a quarter can be wider than the window at the old scale.
   */
  #resize(): void {
    const before = this.#layout;
    if (before === undefined) return;
    const viewport = this.#viewport();
    const scale =
      typeof this.#zoom === "string"
        ? scaleFor(this.#zoom, this.#reference(), this.#rotation, viewport, this.#gap)
        : before.scale;
    const after = new PageLayout(this.#sizes, { scale, rotation: before.rotation, gap: this.#gap });
    if (after.height > MAX_HEIGHT) {
      // Past what browsers can scroll: the zoom is capped again.
      this.#rezoom(undefined);
      return;
    }
    const top = keptTop(before, after, this.#root.scrollTop);
    this.#layout = after;
    this.#spacer.style.height = `${after.height}px`;
    this.#spacer.style.width = `${after.contentWidth(viewport.width)}px`;
    this.#root.scrollTop = top;
    if (scale !== before.scale) {
      const percent = percentOfScale(scale);
      this.#setState({ zoom: this.#zoom, percent });
      this.#emit("zoomchange", { zoom: this.#zoom, percent });
      this.#staleRenders();
    }
  }

  /**
   * Makes `view` draw its page again: its renders aborted (a late one is dropped), its tiles
   * forgotten. A page in view at its old size keeps its pixels, and sharp tiles replace them; a
   * page that changed size, or one outside the view, paints its quick pass first.
   */
  #redraw(view: PageView, repaint: boolean): void {
    this.#abort(view);
    view.waiting = false;
    view.tiles.clear();
    // A page whose render failed tries again from the start.
    if (repaint || view.state === "error") {
      this.#releaseDetail(view);
      view.painted = "";
      view.state = "placeholder";
    } else {
      view.detail?.tiles.clear();
      if (view.state !== "placeholder") view.state = "low";
    }
  }

  /** Mounts `view`'s layers again, so they read its text and links again, at its size now. */
  #remountLayers(view: PageView): void {
    const size = { width: this.#layout?.widths[view.index] ?? 0, height: this.#layout?.heights[view.index] ?? 0 };
    for (const layer of this.#layers) {
      const slot = this.#slot(view, layer.name);
      layer.onUnmount?.(view.index, slot);
      layer.onMount?.(view.index, slot);
      layer.onZoom?.(view.index, slot, size);
    }
  }

  #scroll() {
    return { left: this.#root.scrollLeft, top: this.#root.scrollTop };
  }

  #viewport() {
    return { width: this.#root.clientWidth || 800, height: this.#root.clientHeight || 600 };
  }

  /** Whether the view has a size (it is in a shown container), so it can scroll. */
  #sized(): boolean {
    return this.#root.clientWidth > 0 && this.#root.clientHeight > 0;
  }

  /** Puts a view that just got its size where it was opened or restored to. */
  #showPending(pending: ViewPosition | "top"): void {
    this.#pending = undefined;
    this.#relayout(this.#viewport());
    // The scroll kept while hidden is the last document's: the top first, so a restore's history
    // jump leaves from page 1.
    this.#root.scrollTop = 0;
    this.#root.scrollLeft = 0;
    if (pending === "top") this.#schedule();
    else this.restore(pending);
  }

  /** The text layer `option` asks for, or none (`false`); it reports the selection as an event too. */
  #textLayer(option: boolean | TextLayerOptions | undefined): TextLayer | undefined {
    if (option === false) return undefined;
    const host = {
      root: this.#root,
      client: this.#client,
      rotation: () => this.#rotation,
      points: (page: number) => this.#sizes[page],
      keepMounted: (pages: Iterable<number>) => this.keepMounted(pages),
    };
    const textOptions = option === true || option === undefined ? {} : option;
    return new TextLayer(host, {
      ...textOptions,
      onSelection: (selection) => {
        textOptions.onSelection?.(selection);
        this.#emit("selectionchange", { selection });
      },
    });
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
      // Pictures drawn at the old zoom or rotation would show the wrong size or turn.
      this.#previews?.clear();
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
    this.#staleRenders();
  }

  /**
   * Renders for the old geometry are stale: abort them (a page that failed gets another try); the
   * canvases stretch until the zoom settles and the new pass paints, so no page is sharp.
   */
  #staleRenders(): void {
    let unsharpened = false;
    for (const view of this.#pages.values()) {
      this.#abort(view);
      // The new geometry needs other canvases: a page that waited for one tries again.
      view.waiting = false;
      if (view.state === "error") view.state = "placeholder";
      if (view.state === "sharp") {
        view.state = "low";
        unsharpened = true;
      }
    }
    if (unsharpened) this.#emit("renderstats", this.stats());
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
      // A view waiting for its size has only an old scroll to go by: it draws once it is placed.
      if (this.#pending === undefined) this.#update();
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
      left: this.#root.scrollLeft,
      width: this.#root.clientWidth,
      height: this.#root.clientHeight,
    };
    const visible = layout.rangeIn(frame.top, frame.top + frame.height);
    const [first, last] = this.#mountRange(frame);
    // Writes.
    this.#placeShells(frame, first, last);
    this.#focusJumpTarget();
    this.#setState({
      currentPage: layout.pageAt(frame.top + frame.height / 2),
      scrollTop: frame.top,
      visibleRange: visible,
    });
    // Renders: nothing while a zoom gesture is under way.
    if (this.#settling === undefined) this.#paintRange(frame, visible, first, last);
    this.#emit("renderstats", this.stats());
  }

  /**
   * Unmounts the shells outside `first..last` (but the pinned ones), mounts the missing ones and
   * positions them all.
   */
  #placeShells(frame: Frame, first: number, last: number): void {
    const { layout, width } = frame;
    const pinned = [...this.#pinned].filter((i) => i >= 0 && i < layout.pageCount && (i < first || i > last));
    for (const [i, view] of this.#pages) if ((i < first || i > last) && !this.#pinned.has(i)) this.#unmount(view);
    for (const i of [...pinned, ...range(first, last)]) {
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

  #jumpKeepingFocus(go: () => void): void {
    this.#keepFocus = true;
    try {
      this.history.jump(go);
    } finally {
      this.#keepFocus = false;
    }
  }

  /** Focuses the page the last jump landed on, once it is mounted, keeping the scroll where it is. */
  #focusJumpTarget(): void {
    const target = this.#focusTarget;
    const view = target === undefined ? undefined : this.#pages.get(target);
    if (view === undefined) return;
    this.#focusTarget = undefined;
    view.shell.focus({ preventScroll: true });
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
    // A region for assistive technology, and a place focus can land after a jump.
    shell.setAttribute("role", "region");
    shell.setAttribute("aria-roledescription", this.#regions.roleDescription ?? "page");
    shell.setAttribute("aria-label", this.#pageName(index));
    shell.tabIndex = -1;
    // The paper shows before the page is drawn: white, or the page appearance's (--lamina-page-paper).
    Object.assign(shell.style, {
      position: "absolute",
      contain: "strict",
      background: "var(--lamina-page-paper, #fff)",
    });
    this.#spacer.append(shell);
    const view: PageView = {
      index,
      shell,
      slots: new Map(),
      canvas: undefined,
      controller: new AbortController(),
      state: "placeholder",
      painted: "",
      appearance: appearanceKey(this.#state.appearance),
      scale: 0,
      tiles: new Set(),
      detail: undefined,
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
      // The slots above the text cover the whole page: taking the pointer, they would sit between
      // every press and the spans, and no drag could select text. Nor are they part of a selection:
      // each element the viewer adds that holds nothing to select sets `none` itself, never an
      // ancestor of the spans (user-select.ts).
      if (name !== "text") {
        slot.style.pointerEvents = "none";
        setUserSelect(slot, "none");
      }
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

  #pageName(index: number): string {
    const count = this.#sizes.length;
    return this.#regions.name?.(index, count) ?? `Page ${index + 1} of ${count}`;
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
    this.#releaseDetail(view);
    if (view.canvas !== undefined) this.#pool.release(view.canvas);
    view.canvas = undefined;
    view.painted = "";
    view.tiles.clear();
    if (view.state !== "error") view.state = "placeholder";
    // A canvas is free again: pages that waited for one try now.
    for (const other of this.#pages.values()) other.waiting = false;
    this.#schedule();
  }

  /** The part of page `index` in view in `frame`, in CSS px from the page's top-left. */
  #inView(index: number, frame: Frame) {
    const { layout, top, left, width, height } = frame;
    const cssW = layout.widths[index] as number;
    const cssH = layout.heights[index] as number;
    const pageTop = layout.offsetOf(index);
    const pageLeft = layout.leftOf(index, width);
    const x0 = Math.max(0, left - pageLeft);
    const x1 = Math.min(cssW, left + width - pageLeft);
    const y0 = Math.max(0, top - pageTop);
    const y1 = Math.min(cssH, top + height - pageTop);
    return { x: x0, y: y0, width: Math.max(0, x1 - x0), height: Math.max(0, y1 - y0) };
  }

  /**
   * The sharp tiles of page `index` over `area` (CSS px from the page's top-left) at `scale` (device
   * px per pt), with their render requests and keys. They lie inside the page as the renderer sizes
   * it at that scale.
   */
  #tiles(index: number, layout: PageLayout, scale: number, area: Parameters<typeof tilesFor>[0]): Tile[] {
    const page = pagePixels(this.#sizes[index] as PageSize, scale, layout.rotation);
    return tilesFor(area, page, scale / layout.scale).map((clip) => this.#tile(index, layout, scale, clip));
  }

  /** Page `index` whole at `scale`, as one tile. */
  #wholePage(index: number, layout: PageLayout, scale: number): Tile {
    const page = pagePixels(this.#sizes[index] as PageSize, scale, layout.rotation);
    return this.#tile(index, layout, scale, { x: 0, y: 0, ...page });
  }

  /** The tile `clip` of page `index` at `scale`: its render request and cache key. */
  #tile(index: number, layout: PageLayout, scale: number, clip: PixelRect): Tile {
    const request = { page: index, scale, rotation: layout.rotation, clip, ...appearanceField(this.#state.appearance) };
    return { clip, request, key: this.#key(request) };
  }

  /** `request`'s cache key: the render key with its page's revision. */
  #key(request: Parameters<ViewerClient["render"]>[0]): string {
    return viewKey(request, this.#documentId ?? 0, this.#revisions.get(request.page) ?? 0);
  }

  /** The device px per pt of a sharp tile: the layout's scale × the device pixel ratio. */
  #fullScale(layout: PageLayout): number {
    return layout.scale * this.#dpr();
  }

  /** Whether `view`'s canvas is below the full pixel ratio, so the part in view needs a detail canvas. */
  #needsDetail(view: PageView, layout: PageLayout): boolean {
    return view.scale < this.#fullScale(layout) * (1 - 1e-9);
  }

  /** Every sharp tile now in view is drawn: into the detail canvas, or else into the page canvas. */
  #sharpInView(view: PageView, frame: Frame): boolean {
    const { layout } = frame;
    const area = this.#inView(view.index, frame);
    if (!this.#needsDetail(view, layout)) {
      return this.#tiles(view.index, layout, view.scale, area).every((t) => view.tiles.has(t.key));
    }
    const tiles = this.#tiles(view.index, layout, this.#fullScale(layout), area);
    // Nothing of the page in view (off to one side, or only the gap below it): nothing to
    // sharpen, and its detail canvas goes back to the pool for a page that has.
    if (tiles.length === 0) {
      this.#releaseDetail(view);
      return true;
    }
    const detail = view.detail;
    if (detail === undefined || detail.geometry !== geometryOf(layout)) return false;
    return tiles.every((t) => detail.tiles.has(t.key));
  }

  /**
   * Whether `view` may paint now, catching it up with the appearance: a page out of view when the
   * appearance changed waits until it is seen (`prefetch` doesn't paint it), then starts afresh.
   */
  #readyForAppearance(view: PageView, priority: PaintPriority): boolean {
    const appearance = appearanceKey(this.#state.appearance);
    if (view.appearance === appearance) return true;
    if (priority === "prefetch") return false;
    view.appearance = appearance;
    view.painted = "";
    return true;
  }

  /**
   * Whether `view`'s canvas needs nothing for `frame`: painted at its geometry and, in view, with
   * every tile in view drawn (then it is sharp). A margin page painted at its geometry waits.
   */
  #upToDate(view: PageView, priority: PaintPriority, frame: Frame): boolean {
    if (view.painted !== geometryOf(frame.layout)) return false;
    if (priority === "prefetch") return true;
    // Sharp means every tile now in view is drawn: scrolling down a tall page sharpens the rest.
    if (view.scale > 0 && this.#sharpInView(view, frame)) {
      if (view.state !== "sharp") {
        view.state = "sharp";
        this.#emit("renderstats", this.stats());
      }
      return true;
    }
    // Tiles in view are missing (the page scrolled): it is no longer sharp until they are drawn.
    if (view.state === "sharp") view.state = "low";
    return false;
  }

  /** Starts page `index`'s pass if it isn't busy and its canvas isn't done for `frame` yet. */
  #paint(index: number, priority: PaintPriority, frame: Frame): void {
    const view = this.#pages.get(index);
    // A page in the margin gives its detail canvas back for the pages in view.
    if (view !== undefined && priority === "prefetch") this.#releaseDetail(view);
    if (view === undefined || this.#documentId === undefined || view.busy || view.waiting || view.state === "error") {
      return;
    }
    if (!this.#readyForAppearance(view, priority) || this.#upToDate(view, priority, frame)) return;
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
    if (!this.#needsDetail(view, layout)) {
      this.#releaseDetail(view);
      await this.#paintTiles(pass_, this.#tiles(index, layout, view.scale, this.#inView(index, frame)));
      return;
    }
    // The part in view first, sharp; then the whole backdrop in one render (it is capped at
    // BACKDROP_MAX_PIXELS, below the renderer's limit), for what scrolls into view next.
    await this.#paintDetail(pass_, frame);
    await this.#paintTiles(pass_, [this.#wholePage(index, layout, view.scale)]);
  }

  /** The blurry pass: resizes the canvas and stretches a low-resolution page over it. */
  async #paintLow(pass: Pass): Promise<boolean> {
    const { view, layout, priority, signal, live } = pass;
    const { index } = view;
    const dpr = this.#dpr();
    const cssW = layout.widths[index] as number;
    const cssH = layout.heights[index] as number;
    const shown = rotated(this.#sizes[index] as PageSize, layout.rotation);
    const request = {
      page: index,
      scale: lowScale(layout.scale * dpr, shown.width),
      rotation: layout.rotation,
      ...appearanceField(this.#state.appearance),
    };
    const low = await this.#bitmap(request, this.#key(request), priority, signal);
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
    // The canvas's scale: the device pixel ratio, unless the backdrop's pixel cap or the pool's fit
    // lowers it (then a detail canvas takes the part in view). The canvas is the page at that scale
    // exactly as the renderer sizes it, so every tile fits inside.
    const css = { width: cssW, height: cssH };
    const ratio = backdropRatio(dpr, css, this.#backdropMaxPixels);
    // The pool is asked for the page in whole pixels, as the renderer sizes it at that ratio.
    const wanted = pagePixels(this.#sizes[index] as PageSize, layout.scale * ratio, layout.rotation);
    const fitted = this.#pool.fit(canvas, wanted.width, wanted.height);
    const scale = sharpScale(layout.scale, ratio, css, fitted, wanted);
    const { width, height } = pagePixels(this.#sizes[index] as PageSize, scale, layout.rotation);
    // What the canvas shows now, if it is this page in this orientation at another zoom: drawn back
    // at the new size, so a re-zoom never trades sharp pixels for blurry ones.
    const before = await this.#snapshot(view, layout, width, height);
    if (!live()) {
      before?.close();
      this.#drop(low);
      return false;
    }
    // Resizing clears the canvas: it happens only now, with the new pixels ready (no white flash).
    canvas.width = width;
    canvas.height = height;
    // The old pixels cover the whole page, so the low pass is drawn only when there are none.
    const drawn = this.#draw(view, canvas, before ?? low.bitmap, 0, 0, width, height);
    before?.close();
    this.#release(low);
    // Not drawn (evicted meanwhile): the canvas stays unpainted, and the next frame tries again.
    if (!drawn) return false;
    view.painted = geometryOf(layout);
    view.scale = scale;
    view.tiles.clear();
    view.state = "low";
    this.#pruneOtherGeometries(index, layout, new Set([request.scale, scale, layout.scale * dpr]));
    this.#emit("renderstats", this.stats());
    return true;
  }

  /**
   * A copy of `view`'s page canvas when it holds the page in `layout`'s rotation at another zoom (the
   * pixels the user sees stretched), no larger than the new `width` × `height`, for #paintLow to
   * draw back at the new size; else undefined.
   */
  async #snapshot(view: PageView, layout: PageLayout, width: number, height: number): Promise<ImageBitmap | undefined> {
    const canvas = view.canvas;
    if (canvas === undefined || view.painted === "" || rotationOf(view.painted) !== layout.rotation) return undefined;
    try {
      // No larger than the new canvas: zooming out shrinks the copy rather than doubling the memory.
      if (width >= canvas.width && height >= canvas.height) return await createImageBitmap(canvas);
      return await createImageBitmap(canvas, {
        resizeWidth: Math.min(width, canvas.width),
        resizeHeight: Math.min(height, canvas.height),
      });
    } catch {
      // No copy (out of memory, or a browser without resize options): the low pass stands in.
      return undefined;
    }
  }

  /** Frees page `index`'s bitmaps of other scales or rotations at once, rather than at the budget. */
  #pruneOtherGeometries(index: number, layout: PageLayout, scales: ReadonlySet<number>): void {
    this.#cache.prune(index, (key) => {
      const { scale, rotation } = renderKeyFields(key);
      return scales.has(scale) && rotation === layout.rotation;
    });
  }

  /** `tiles` (at the canvas's scale) into the page's canvas. */
  async #paintTiles(pass: Pass, tiles: readonly Tile[]): Promise<void> {
    const { view, priority, signal, live } = pass;
    const canvas = view.canvas;
    if (canvas === undefined) return;
    for (const tile of tiles) {
      if (view.tiles.has(tile.key)) continue;
      const got = await this.#bitmap(tile.request, tile.key, priority, signal);
      if (!live()) {
        this.#drop(got);
        return;
      }
      const drawn = this.#draw(view, canvas, got.bitmap, tile.clip.x, tile.clip.y);
      this.#release(got);
      if (drawn) view.tiles.add(tile.key);
    }
  }

  /**
   * Sharp tiles over the part in view at the full pixel ratio, into the detail canvas: placed (or
   * moved) to cover them, its cached tiles drawn at once so it never shows empty, then the rest.
   */
  async #paintDetail(pass: Pass, frame: Frame): Promise<void> {
    const { view, layout, priority, signal, live } = pass;
    const tiles = this.#tiles(view.index, layout, this.#fullScale(layout), this.#inView(view.index, frame));
    const rect = bounds(tiles.map((t) => t.clip));
    if (rect === undefined) {
      this.#releaseDetail(view);
      return;
    }
    const detail = this.#placeDetail(view, layout, rect);
    if (detail === undefined) {
      // No detail canvas to be had: the page shows its backdrop and waits until one is released,
      // rather than try again every frame.
      this.#releaseDetail(view);
      view.waiting = true;
      return;
    }
    const missing = tiles.filter((tile) => {
      if (detail.tiles.has(tile.key)) return false;
      const cached = this.#cache.get(tile.key);
      if (cached === undefined || !this.#drawDetail(view, detail, cached, tile.clip)) return true;
      detail.tiles.add(tile.key);
      return false;
    });
    for (const tile of missing) {
      const got = await this.#bitmap(tile.request, tile.key, priority, signal);
      if (!live() || view.detail !== detail) {
        this.#drop(got);
        return;
      }
      const drawn = this.#drawDetail(view, detail, got.bitmap, tile.clip);
      this.#release(got);
      if (drawn) detail.tiles.add(tile.key);
    }
  }

  /**
   * `view`'s detail canvas covering `rect` (device px at the full scale): the one it has, if it
   * already covers it at this geometry, else one sized and placed for `rect`. Undefined when the
   * detail pool has no canvas or not the pixels (the page then shows its backdrop).
   */
  #placeDetail(view: PageView, layout: PageLayout, rect: PixelRect): Detail | undefined {
    const scale = this.#fullScale(layout);
    const geometry = geometryOf(layout);
    const old = view.detail;
    if (old !== undefined && old.geometry === geometry && old.scale === scale && contains(old.rect, rect)) return old;
    // Past the device's cap for one canvas (a large tablet's viewport), the page keeps its backdrop.
    if (rect.width * rect.height > this.#canvasMaxPixels) return undefined;
    const canvas = old?.canvas ?? this.#detailPool.acquire();
    if (canvas === undefined) return undefined;
    const fitted = this.#detailPool.fit(canvas, rect.width, rect.height);
    if (fitted.width < rect.width || fitted.height < rect.height) {
      if (old === undefined) this.#detailPool.release(canvas);
      return undefined;
    }
    // Resizing clears it: #paintDetail draws the cached tiles back before the next frame.
    canvas.width = rect.width;
    canvas.height = rect.height;
    // In percentages of the shell, so it stretches with the page while a zoom gesture runs.
    const page = pagePixels(this.#sizes[view.index] as PageSize, scale, layout.rotation);
    const percent = (n: number, of: number) => `${(100 * n) / of}%`;
    Object.assign(canvas.style, {
      position: "absolute",
      left: percent(rect.x, page.width),
      top: percent(rect.y, page.height),
      width: percent(rect.width, page.width),
      height: percent(rect.height, page.height),
    });
    if (old === undefined) {
      canvas.className = "lamina-page-detail";
      setUserSelect(canvas, "none");
      if (view.canvas === undefined) view.shell.prepend(canvas);
      else view.canvas.after(canvas);
    }
    const detail: Detail = { canvas, rect, scale, geometry, tiles: new Set() };
    view.detail = detail;
    return detail;
  }

  #releaseDetail(view: PageView): void {
    if (view.detail === undefined) return;
    this.#detailPool.release(view.detail.canvas);
    view.detail = undefined;
    // A detail canvas is free again: pages that waited for one try now.
    for (const other of this.#pages.values()) other.waiting = false;
    this.#schedule();
  }

  /** Draws a sharp tile at `clip` (device px of the page) into `detail`, offset by its rectangle. */
  #drawDetail(view: PageView, detail: Detail, bitmap: ImageBitmap, clip: PixelRect): boolean {
    return this.#draw(view, detail.canvas, bitmap, clip.x - detail.rect.x, clip.y - detail.rect.y);
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
    const revision = this.#revisions.get(request.page) ?? 0;
    const { bitmap } = await this.#client.render(request, { priority, signal });
    // The page was edited while this render waited: its pixels may be the old file's, and its key
    // names a revision nothing asks for again. The worker runs one call at a time and `invalidate`
    // runs only once the edit has answered, so an answer that comes after it, at the same
    // revision, drew the edited file; the worker's coalescing key needs no revision for the same
    // reason (no render can run across an edit).
    if ((this.#revisions.get(request.page) ?? 0) !== revision) {
      bitmap.close();
      throw new DOMException("the page changed while it rendered", "AbortError");
    }
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

  /** The one place pixels reach a canvas of `view`: it counts a draw onto a shell out of the DOM. */
  #draw(
    view: PageView,
    canvas: HTMLCanvasElement,
    bitmap: ImageBitmap,
    x: number,
    y: number,
    width?: number,
    height?: number,
  ): boolean {
    const ctx = canvas.getContext("2d");
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
    setUserSelect(canvas, "none");
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
    this.#listenTouch();
    this.#listenGestures();
    this.#listenResize();
  }

  /** Ctrl/cmd+wheel and trackpad pinch (which browsers send as ctrl+wheel): zoom at the cursor. */
  #listenWheel(): void {
    this.#on<"wheel">(
      "wheel",
      (e) => {
        this.#touch.stopMomentum();
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
      this.#touch.stopMomentum();
      if (e.altKey) return;
      const p = this.#state.percent;
      if (e.key === "+" || e.key === "=") this.setZoom(p * 1.25);
      else if (e.key === "-") this.setZoom(p / 1.25);
      else if (e.key === "0") this.setZoom("actual-size");
      else return;
      e.preventDefault();
    });
  }

  /** Touch and pen: pan with momentum, pinch around the fingers, double-tap zoom (view/touch.ts). */
  #listenTouch(): void {
    const gestures = this.#touch;
    this.#on<"pointerdown">("pointerdown", (e) => gestures.down(e));
    this.#on<"pointermove">("pointermove", (e) => gestures.move(e));
    for (const end of ["pointerup", "pointercancel", "lostpointercapture"] as const) {
      this.#on<typeof end>(end, (e) => gestures.up(e));
    }
    this.#cleanup.push(() => gestures.stop());
  }

  /** The gestures' view of this viewer; it reads the root when called, so it may be made before the root. */
  #touchHost(): TouchHost<DocumentPoint> {
    return {
      percent: () => this.#state.percent,
      fitsWidth: () => this.#zoom === "fit-width",
      zoom: (to, anchor) => this.setZoom(to, { anchor }),
      pointAt: (at) => this.pointAt(at.x, at.y),
      positionOf: (point) => this.positionOf(point),
      scrollBy: (dx, dy) => this.#root.scrollBy({ left: dx, top: dy, behavior: "instant" }),
      local: (e) => this.#anchorAt(e.clientX, e.clientY),
      frame: (callback) => requestAnimationFrame(callback),
      cancelFrame: (id) => cancelAnimationFrame(id),
      capture: (id) => {
        try {
          this.#root.setPointerCapture(id);
        } catch {
          // The pointer is already up: nothing to keep.
        }
      },
    };
  }

  /** Safari's trackpad pinch: gesture events with a scale (and iOS's touch pinch, which the touch gestures take). */
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
      // iOS sends these for a two-finger touch pinch too: the touch gestures zoom it then.
      if (this.#touch.pinching()) return;
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
      if (this.#pending !== undefined && this.#sized()) {
        this.#showPending(this.#pending);
        return;
      }
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
