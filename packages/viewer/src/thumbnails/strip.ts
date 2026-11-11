// The thumbnails strip: one small picture per page in a scrolling list, the
// current page marked, a click or Enter going there. Like the viewer it mounts only the
// thumbnails in view and a few either side (the same `PageLayout` at thumbnail scale), each with
// a canvas from its own small pool, rendered at `background` priority so the main view's pages
// always come first; a thumbnail scrolled away aborts its render. Bitmaps stay in a cache of their
// own (32 MB by default), so scrolling back costs nothing. Framework-free; styles are set through
// the CSSOM, which a `style-src 'self'` policy allows.

import type { PageAppearance } from "@laminapdf/render";
import type { PageInfo } from "@laminapdf/sdk";
import { BitmapCache } from "../canvas/cache.js";
import { CanvasPool } from "../canvas/pool.js";
import type { ViewerClient } from "../client.js";
import { PageLayout, type PageSize, type Rotation, rotated } from "../layout/page-layout.js";
import { appearanceField, appearanceKey, type OpenedDocument } from "../protocol.js";
import type { InvalidatedPage, Viewer } from "../view/viewer.js";

/** How a strip looks and how much it keeps. */
export interface ThumbnailStripOptions {
  /** The widest page's thumbnail width in CSS px (default 120). */
  readonly width?: number;
  /** Thumbnails mounted past each end of the view (default 4). */
  readonly margin?: number;
  /** The bitmap cache's budget in bytes (default 32 MB). */
  readonly cacheBytes?: number;
  /**
   * A thumbnail's accessible name and visible caption: by default its page label, and the
   * name adds "Page … of …".
   */
  readonly label?: (page: PageInfo, count: number) => { readonly caption: string; readonly name: string };
  /** The list's own accessible name (default "Pages"). */
  readonly name?: string;
}

/** Space for the caption under each thumbnail, and between thumbnails, in CSS px. */
const CAPTION = 20;
const GAP = 12;

/** The default caption (the page label, else the number) and name. */
function defaultLabel(page: PageInfo, count: number): { caption: string; name: string } {
  const number = String(page.index + 1);
  const caption = page.label ?? number;
  const name = caption === number ? `Page ${number} of ${count}` : `Page ${caption} (${number} of ${count})`;
  return { caption, name };
}

/** A mounted thumbnail. */
interface Item {
  readonly index: number;
  readonly root: HTMLElement;
  readonly frame: HTMLElement;
  canvas: HTMLCanvasElement | undefined;
  abort: AbortController | undefined;
  /** Painted, or failed to render: either way not asked for again while mounted. */
  done: boolean;
}

/** The pages as thumbnails. Make one with `new ThumbnailStrip(client, viewer)`, then `mount` and `open`. */
export class ThumbnailStrip {
  /** The scrolling list (`role=listbox`); size it with CSS. */
  readonly element: HTMLElement;
  readonly #client: ViewerClient;
  readonly #viewer: Viewer;
  readonly #width: number;
  readonly #margin: number;
  readonly #label: NonNullable<ThumbnailStripOptions["label"]>;
  readonly #spacer: HTMLElement;
  readonly #pool = new CanvasPool({ max: 32, maxPixels: 8 * 1024 * 1024 });
  readonly #cache: BitmapCache;
  readonly #items = new Map<number, Item>();
  readonly #cleanup: (() => void)[] = [];
  #document: OpenedDocument | undefined;
  #layout: PageLayout | undefined;
  #rotation: Rotation = 0;
  #appearance: PageAppearance = "original";
  /** The page the viewer shows, marked selected. */
  #current = 0;
  /** The page that takes Tab (roving focus). */
  #focus = 0;
  #frame: number | undefined;

  constructor(client: ViewerClient, viewer: Viewer, options: ThumbnailStripOptions = {}) {
    this.#client = client;
    this.#viewer = viewer;
    this.#width = options.width ?? 120;
    this.#margin = options.margin ?? 4;
    this.#label = options.label ?? defaultLabel;
    this.#cache = new BitmapCache(options.cacheBytes ?? 32 * 1024 * 1024);
    const root = document.createElement("div");
    root.className = "lamina-thumbnails";
    root.setAttribute("role", "listbox");
    root.setAttribute("aria-label", options.name ?? "Pages");
    Object.assign(root.style, { position: "relative", overflowY: "auto", overflowX: "hidden" });
    this.#spacer = document.createElement("div");
    this.#spacer.style.position = "relative";
    root.append(this.#spacer);
    this.element = root;
    const on = <K extends keyof HTMLElementEventMap>(type: K, f: (e: HTMLElementEventMap[K]) => void) => {
      root.addEventListener(type, f);
      this.#cleanup.push(() => root.removeEventListener(type, f));
    };
    on("scroll", () => this.#schedule());
    on("click", (e) => this.#clicked(e));
    on("keydown", (e) => this.#key(e));
    this.#cleanup.push(viewer.subscribe(() => this.#viewChanged()));
    this.#cleanup.push(viewer.on("invalidate", ({ pages }) => this.#invalidate(pages)));
    // A strip shown later (its tab opened, its sheet raised) mounts what it now shows.
    const resized = new ResizeObserver(() => this.#schedule());
    resized.observe(root);
    this.#cleanup.push(() => resized.disconnect());
  }

  /** Puts the strip in `container`. */
  mount(container: HTMLElement): void {
    container.append(this.element);
    this.#schedule();
  }

  /** Shows `document`'s pages (the one the viewer opened). */
  open(document: OpenedDocument): void {
    this.#unmountAll();
    this.#cache.clear();
    this.#document = document;
    this.#current = 0;
    this.#focus = 0;
    this.#rotation = this.#viewer.getState().rotation;
    this.element.scrollTop = 0;
    this.#relayout();
  }

  /** Takes the strip down: renders aborted, canvases and bitmaps freed, listeners removed. */
  destroy(): void {
    if (this.#frame !== undefined) cancelAnimationFrame(this.#frame);
    this.#unmountAll();
    this.#cache.clear();
    for (const f of this.#cleanup.splice(0)) f();
    this.element.remove();
  }

  // --- internals ---

  #relayout(): void {
    const pages = this.#sizes();
    const widest = Math.max(1, ...pages.map((p) => rotated(p, this.#rotation).width));
    this.#layout = new PageLayout(pages, { scale: this.#width / widest, rotation: this.#rotation, gap: GAP + CAPTION });
    this.#spacer.style.height = `${this.#layout.height}px`;
    this.#unmountAll();
    this.#schedule();
  }

  /**
   * The pages' sizes: the viewer's, which follow its edits (one home for them). Before the viewer
   * shows this document (the strip opened first), the document's own.
   */
  #sizes(): readonly PageSize[] {
    const pages = this.#document?.pages ?? [];
    const viewed = this.#document === undefined ? [] : this.#viewer.pageSizes(this.#document.id);
    return viewed.length === pages.length ? viewed : pages;
  }

  /** Whether page `index` keeps its box in `layout` at `size` (the same arithmetic as the layout's). */
  #fits(layout: PageLayout, index: number, size: PageSize): boolean {
    const shown = rotated(size, layout.rotation);
    return layout.widths[index] === shown.width * layout.scale && layout.heights[index] === shown.height * layout.scale;
  }

  /** The viewer redrew `pages` after an edit: their thumbnails are drawn again, at their new sizes. */
  #invalidate(pages: readonly InvalidatedPage[]): void {
    const layout = this.#layout;
    let resized = false;
    for (const { index, width, height } of pages) {
      // Draw it afresh: an unmounted thumbnail's render is aborted, and its late bitmap never kept.
      this.#cache.prune(index, () => false);
      const item = this.#items.get(index);
      if (item !== undefined) this.#unmount(item);
      if (layout !== undefined && !this.#fits(layout, index, { width, height })) resized = true;
    }
    if (resized) this.#relayout();
    else this.#schedule();
  }

  #viewChanged(): void {
    const state = this.#viewer.getState();
    if (state.rotation !== this.#rotation) {
      this.#rotation = state.rotation;
      this.#relayout();
    }
    // The thumbnails follow the page appearance: drawn again in it.
    if (appearanceKey(state.appearance) !== appearanceKey(this.#appearance)) {
      this.#appearance = state.appearance;
      this.#unmountAll();
      this.#schedule();
    }
    if (state.currentPage === this.#current || this.#document === undefined) return;
    this.#current = state.currentPage;
    // Tab lands on the current page, unless focus is already in the list.
    if (!this.element.contains(document.activeElement)) this.#focus = this.#current;
    this.#reveal(this.#current);
    this.#schedule();
  }

  /** Scrolls the strip just enough to show page `index`'s thumbnail. */
  #reveal(index: number): void {
    const layout = this.#layout;
    if (layout === undefined || index >= layout.pageCount) return;
    const top = layout.offsetOf(index) - GAP;
    const bottom = top + (layout.heights[index] as number) + CAPTION + 2 * GAP;
    const view = this.element;
    if (top < view.scrollTop) view.scrollTop = top;
    else if (bottom > view.scrollTop + view.clientHeight) view.scrollTop = bottom - view.clientHeight;
  }

  #schedule(): void {
    if (this.#frame !== undefined) return;
    this.#frame = requestAnimationFrame(() => {
      this.#frame = undefined;
      this.#update();
    });
  }

  #update(): void {
    const layout = this.#layout;
    if (layout === undefined || layout.pageCount === 0) return;
    // Nothing renders while the strip is out of the page or hidden (its tab not shown).
    if (!this.element.isConnected || this.element.clientHeight === 0) return;
    const top = this.element.scrollTop;
    const view = layout.rangeIn(top, top + Math.max(1, this.element.clientHeight));
    const first = Math.max(0, view.first - this.#margin);
    const last = Math.min(layout.pageCount - 1, view.last + this.#margin);
    for (const [index, item] of this.#items) if (index < first || index > last) this.#unmount(item);
    for (let index = first; index <= last; index++) {
      const item = this.#items.get(index) ?? this.#mount(index, layout);
      this.#mark(item);
      if (!item.done && item.abort === undefined) void this.#draw(item, layout);
    }
  }

  #mount(index: number, layout: PageLayout): Item {
    const page = this.#document?.pages[index] as PageInfo;
    const { caption, name } = this.#label(page, layout.pageCount);
    const root = document.createElement("div");
    root.className = "lamina-thumbnail";
    root.setAttribute("role", "option");
    root.setAttribute("aria-label", name);
    root.setAttribute("aria-posinset", String(index + 1));
    root.setAttribute("aria-setsize", String(layout.pageCount));
    root.dataset.pageIndex = String(index);
    const width = layout.widths[index] as number;
    const height = layout.heights[index] as number;
    Object.assign(root.style, {
      position: "absolute",
      top: `${layout.offsetOf(index) - GAP / 2}px`,
      left: "0",
      right: "0",
      height: `${height + CAPTION + GAP}px`,
      display: "flex",
      flexDirection: "column",
      alignItems: "center",
      paddingTop: `${GAP / 2}px`,
    });
    const frame = document.createElement("div");
    frame.className = "lamina-thumbnail-page";
    Object.assign(frame.style, { width: `${width}px`, height: `${height}px`, flex: "none" });
    const label = document.createElement("div");
    label.className = "lamina-thumbnail-label";
    label.textContent = caption;
    label.setAttribute("aria-hidden", "true");
    root.append(frame, label);
    this.#spacer.append(root);
    const item: Item = { index, root, frame, canvas: undefined, abort: undefined, done: false };
    this.#items.set(index, item);
    return item;
  }

  /** Sets `item`'s selected state and whether Tab reaches it. */
  #mark(item: Item): void {
    item.root.setAttribute("aria-selected", String(item.index === this.#current));
    item.root.tabIndex = item.index === this.#focus ? 0 : -1;
  }

  async #draw(item: Item, layout: PageLayout): Promise<void> {
    const ratio = globalThis.devicePixelRatio || 1;
    const scale = layout.scale * ratio;
    const appearance = this.#appearance;
    const id = this.#client.document?.id ?? 0;
    const key = `${id}:${item.index}:${layout.rotation}:${scale.toFixed(4)}:${appearanceKey(appearance)}`;
    let bitmap = this.#cache.get(key);
    let owned = false;
    if (bitmap === undefined) {
      const abort = new AbortController();
      item.abort = abort;
      try {
        const rendered = await this.#client.render(
          { page: item.index, scale, rotation: layout.rotation, ...appearanceField(appearance) },
          { priority: "background", signal: abort.signal },
        );
        bitmap = rendered.bitmap;
      } catch (error) {
        // Scrolled away or a new document: drawn again if it comes back into view.
        if (abort.signal.aborted) return;
        // A failed render keeps its placeholder, and isn't asked for again while mounted.
        item.done = true;
        reportError(error);
        return;
      } finally {
        if (item.abort === abort) item.abort = undefined;
      }
      if (abort.signal.aborted || this.#items.get(item.index) !== item || this.#layout !== layout) {
        bitmap.close();
        return;
      }
      owned = !this.#cache.set(key, item.index, bitmap, this.#current);
    }
    this.#paint(item, bitmap, layout);
    if (owned) bitmap.close();
  }

  #paint(item: Item, bitmap: ImageBitmap, layout: PageLayout): void {
    const canvas = item.canvas ?? this.#pool.acquire();
    if (canvas === undefined) return;
    item.canvas = canvas;
    const size = this.#pool.fit(canvas, bitmap.width, bitmap.height);
    canvas.width = size.width;
    canvas.height = size.height;
    Object.assign(canvas.style, {
      display: "block",
      width: `${layout.widths[item.index]}px`,
      height: `${layout.heights[item.index]}px`,
    });
    canvas.getContext("2d")?.drawImage(bitmap, 0, 0, size.width, size.height);
    if (canvas.parentElement !== item.frame) item.frame.append(canvas);
    item.done = true;
  }

  #unmount(item: Item): void {
    item.abort?.abort();
    if (item.canvas !== undefined) this.#pool.release(item.canvas);
    item.root.remove();
    this.#items.delete(item.index);
  }

  #unmountAll(): void {
    for (const item of [...this.#items.values()]) this.#unmount(item);
  }

  #clicked(e: MouseEvent): void {
    const target = (e.target as Element | null)?.closest<HTMLElement>(".lamina-thumbnail");
    const index = Number(target?.dataset.pageIndex);
    if (target === null || target === undefined || !Number.isInteger(index)) return;
    this.#go(index);
  }

  /** Arrow keys, Home and End move focus; Enter and Space go to the focused page. */
  #key(e: KeyboardEvent): void {
    const count = this.#layout?.pageCount ?? 0;
    if (count === 0) return;
    const moves: Record<string, number> = {
      ArrowDown: this.#focus + 1,
      ArrowUp: this.#focus - 1,
      Home: 0,
      End: count - 1,
    };
    const next = moves[e.key];
    if (next !== undefined) {
      e.preventDefault();
      this.#moveFocus(Math.min(count - 1, Math.max(0, next)));
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      this.#go(this.#focus);
    }
  }

  #moveFocus(index: number): void {
    this.#focus = index;
    this.#reveal(index);
    // The item may not be mounted yet: mount the range now, then focus it.
    this.#update();
    const item = this.#items.get(index);
    for (const other of this.#items.values()) this.#mark(other);
    item?.root.focus();
  }

  #go(index: number): void {
    this.#focus = index;
    // A jump: Back returns to where the view was (R06).
    this.#viewer.jumpToPage(index);
  }
}
