// The text layer (web-app §6 C5): on every mounted page, one transparent span per text run of
// Lamina PDF's extraction, laid over the rendered glyphs, so the browser's own selection, copy
// and screen readers work on the page's text. Spans are placed in page points inside a box that
// one CSS transform scales and rotates to the page's current size, so a zoom or a rotation moves
// no span. Only the spans are selectable, never canvases or gaps, so a selection runs from one
// page into the next; the pages it touches stay mounted while it lasts. Copy writes the extracted
// text of the part of a selection inside the viewer: by paragraph in the extraction's reading
// order, or exact, as laid out (layers/copy.ts). Alt/Option-drag, or a drag in column mode,
// selects the runs inside a rectangle instead (a column of a multi-column page), which the layer
// marks itself, since a browser selection can't hold a rectangle.

import type { ViewerClient } from "../client.js";
import type { Rotation } from "../layout/page-layout.js";
import { range as range_ } from "../layout/range.js";
import type { ReadingBlock } from "../protocol.js";
import { setUserSelect } from "../view/user-select.js";
import type { PageLayer } from "../view/viewer.js";
import { placeBox } from "./box.js";
import { type CopyMode, copySelection } from "./copy.js";
import { layoutSpans, type Piece, type SpanSpec } from "./spans.js";
import { StructureLayer } from "./structure-layer.js";

/** What the text layer needs of its viewer. */
export interface TextLayerHost {
  /** The viewer's root: where selections start, and where the structure tree lives. */
  readonly root: HTMLElement;
  readonly client: Pick<ViewerClient, "pageText" | "structure" | "readingOrder">;
  /** The view's rotation now. */
  rotation(): Rotation;
  /** Page `page`'s displayed size in points now. */
  points(page: number): { readonly width: number; readonly height: number } | undefined;
  /** Keeps these pages mounted (the ones the selection touches). */
  keepMounted(pages: Iterable<number>): void;
}

/** A selection in the text layer, for the app's selection surfaces (UI/UX plan §5.3). */
export interface TextSelection {
  /** The selected text, as copy writes it (by paragraph). */
  readonly text: string;
  /** Its client rects, to place a toolbar or a menu near. */
  readonly rects: readonly DOMRect[];
  /** The pages it touches, in order. */
  readonly pages: readonly number[];
  /** The pointer that made it: `mouse`, `pen`, `touch`, or `""` for the keyboard. */
  readonly pointerType: string;
  /**
   * The selection is done: the pointer that makes it is up (false during a drag, when a toolbar
   * would get in the way). A keyboard selection is always settled.
   */
  readonly settled: boolean;
  /** A rectangle's runs (column selection), rather than the browser's selection. */
  readonly column: boolean;
}

/** The text layer's options (`createViewer`'s `text`). */
export interface TextLayerOptions {
  /** Outlines each span, to check its fit over the rendered glyphs (a debug view). */
  readonly outline?: boolean;
  /** What a screen reader hears once for a document without structure (default "Untagged document"). */
  readonly untaggedLabel?: string;
  /** Called as the selection changes; `undefined` when it leaves the text layer. */
  readonly onSelection?: (selection: TextSelection | undefined) => void;
}

/** One mounted page's text. */
interface PageText {
  readonly box: HTMLDivElement;
  readonly abort: AbortController;
  spans: HTMLElement[];
}

/** Numbers each text layer, for its span ids. */
let nextViewer = 0;

/** The text layer of one viewer. */
export class TextLayer implements PageLayer {
  readonly name = "text";
  readonly #host: TextLayerHost;
  readonly #options: TextLayerOptions;
  readonly #pages = new Map<number, PageText>();
  readonly #specs = new WeakMap<HTMLElement, { readonly page: number; readonly spec: SpanSpec }>();
  readonly #structure: StructureLayer;
  readonly #cleanup: (() => void)[] = [];
  /** Each mounted page's size in CSS px, as the last resize gave it. */
  readonly #sizes = new Map<number, { readonly width: number; readonly height: number }>();
  #pointerType = "";
  /** A pointer is down in the viewer: a selection it makes isn't settled until it comes up. */
  #pointerDown = false;
  /** The mode the next copy event writes in (a command may ask for exact). */
  #copyMode: CopyMode = "paragraph";
  /** Each page's reading-order blocks, once read; a page being read has a pending entry. */
  readonly #blocks = new Map<number, readonly ReadingBlock[] | "reading">();
  /** Whether a plain drag selects a rectangle (column mode), as an Alt-drag always does. */
  #columnMode = false;
  /** The column selection: its pieces and spans, while it lasts. */
  #column: { readonly pieces: Piece[]; readonly spans: HTMLElement[] } | undefined;
  /** The rectangle being dragged out for a column selection. */
  #drag: { readonly x: number; readonly y: number; readonly box: HTMLDivElement; readonly pointer: number } | undefined;
  /** Span ids' prefix, unique to this viewer, so two viewers on a page never share an id. */
  readonly #idPrefix = `lamina-text-${nextViewer++}-`;
  /** Counts opens: a structure tree answered for an earlier document is dropped. */
  #generation = 0;
  /** Whether the last selection change was in this viewer, so a selection elsewhere is told only once. */
  #selecting = false;

  constructor(host: TextLayerHost, options: TextLayerOptions = {}) {
    this.#host = host;
    this.#options = options;
    this.#structure = new StructureLayer(host.root, options.untaggedLabel ?? "Untagged document", this.#idPrefix);
    // Only the spans are selectable (set on each); what holds nothing to select sets `none` on
    // itself (view/user-select.ts says why no ancestor of the spans may).
    this.#listen(document, "selectionchange", () => this.#selectionChanged());
    this.#listen(document, "copy", (e) => this.#copy(e as ClipboardEvent));
    this.#listen(host.root, "keydown", (e) => this.#selectAll(e as KeyboardEvent));
    this.#listen(host.root, "pointerdown", (e) => this.#pointerDownIn(e as PointerEvent));
    this.#listen(document, "pointermove", (e) => this.#dragTo(e as PointerEvent));
    this.#listen(document, "pointerup", (e) => this.#pointerUp(e as PointerEvent));
    this.#listen(document, "pointercancel", (e) => this.#pointerUp(e as PointerEvent));
    // On the document: the drag that made a column selection kept the focus where it was.
    this.#listen(document, "keydown", (e) => {
      if ((e as KeyboardEvent).key === "Escape" && this.#column !== undefined) this.#clearColumn(true);
    });
  }

  /** Whether a plain drag selects a rectangle (column mode). */
  get columnMode(): boolean {
    return this.#columnMode;
  }

  set columnMode(on: boolean) {
    this.#columnMode = on;
  }

  /**
   * Copies the selection in `mode` to the clipboard, as the copy event would (for a command or a
   * toolbar button; it must run in a user gesture). Resolves false when nothing is selected or
   * the clipboard refused.
   */
  async copy(mode: CopyMode = "paragraph"): Promise<boolean> {
    const pieces = this.#selectedPieces();
    if (pieces === undefined || pieces.length === 0) return false;
    // The browser's own copy, in this gesture: the copy event writes both formats.
    this.#copyMode = mode;
    try {
      if (document.execCommand("copy")) return true;
    } catch {
      // No synchronous copy here: the async clipboard below.
    } finally {
      this.#copyMode = "paragraph";
    }
    const copied = copySelection(pieces, mode, (page) => this.#blocksOf(page));
    try {
      if (typeof ClipboardItem !== "undefined" && navigator.clipboard?.write !== undefined) {
        await navigator.clipboard.write([
          new ClipboardItem({
            "text/plain": new Blob([copied.text], { type: "text/plain" }),
            "text/html": new Blob([copied.html], { type: "text/html" }),
          }),
        ]);
      } else await navigator.clipboard.writeText(copied.text);
      return true;
    } catch {
      return false;
    }
  }

  /** A new document: its structure tree, read once. */
  open(): void {
    this.#structure.reset();
    this.#blocks.clear();
    const opening = ++this.#generation;
    this.#host.client
      .structure()
      .then((structure) => {
        // Another document opened while this one's tree was read: it isn't this document's.
        if (opening !== this.#generation) return;
        this.#structure.show(structure);
        // Pages whose text came first: link them now that their elements exist.
        for (const page of this.#pages.keys()) this.#linkPage(page);
      })
      .catch(() => {
        // No structure (a worker restart, a closed document): the spans still read in order.
      });
  }

  onMount(page: number, slot: HTMLElement): void {
    const box = document.createElement("div");
    box.className = "lamina-text-layer";
    // The spans lie over the canvas, so an app's ::selection fill would paint over the glyphs it
    // selects. With the box multiplied over the canvas, the fill shows where the page is paper and
    // the ink stays as dark as drawn: the tint sits under the text, as the find highlights do.
    Object.assign(box.style, {
      position: "absolute",
      left: "0",
      top: "0",
      transformOrigin: "0 0",
      mixBlendMode: "multiply",
    });
    slot.append(box);
    const text: PageText = { box, abort: new AbortController(), spans: [] };
    this.#pages.set(page, text);
    this.#host.client
      .pageText(page, { signal: text.abort.signal })
      .then((data) => {
        if (this.#pages.get(page) !== text) return;
        this.#fill(page, text, layoutSpans(data.runs, data.quads));
      })
      .catch(() => {
        // Aborted (the page left), or a page whose text can't be read: it stays without text.
      });
  }

  onUnmount(page: number): void {
    const text = this.#pages.get(page);
    if (text === undefined) return;
    text.abort.abort();
    text.box.remove();
    this.#structure.unlink(page);
    this.#pages.delete(page);
    this.#sizes.delete(page);
    this.#blocks.delete(page);
    if (this.#column?.pieces.some((p) => p.page === page)) this.#clearColumn(true);
  }

  onZoom(page: number, _slot: HTMLElement, size: { readonly width: number; readonly height: number }): void {
    this.#sizes.set(page, size);
    this.#place(page, size);
  }

  /** Places every page's box again: the view turned. */
  refresh(): void {
    for (const [page, size] of this.#sizes) this.#place(page, size);
  }

  /** Scales and turns page `page`'s box to `size` at the view's rotation. */
  #place(page: number, size: { readonly width: number; readonly height: number }): void {
    const text = this.#pages.get(page);
    const points = this.#host.points(page);
    if (text === undefined || points === undefined) return;
    placeBox(text.box, points, size, this.#host.rotation());
  }

  /** Stops listening; the viewer calls it on destroy. */
  destroy(): void {
    for (const f of this.#cleanup.splice(0)) f();
    for (const page of [...this.#pages.keys()]) this.onUnmount(page);
    this.#structure.reset();
  }

  #listen(target: EventTarget, type: string, f: (e: Event) => void): void {
    target.addEventListener(type, f);
    this.#cleanup.push(() => target.removeEventListener(type, f));
  }

  /**
   * The page's spans, built once: written, then every natural width read in one pass, then each
   * stretched to its run's width in another, so the page costs one layout, not one per span.
   */
  #fill(page: number, text: PageText, specs: readonly SpanSpec[]): void {
    const fragment = document.createDocumentFragment();
    text.spans = specs.map((spec, k) => {
      const span = document.createElement("span");
      span.id = `${this.#idPrefix}${page}-${k}`;
      span.textContent = spec.text;
      if (spec.rtl) span.dir = "rtl";
      Object.assign(span.style, {
        position: "absolute",
        left: `${spec.left}px`,
        top: `${spec.top}px`,
        fontSize: `${spec.height}px`,
        lineHeight: "1",
        fontFamily: "sans-serif",
        whiteSpace: "pre",
        color: "transparent",
        cursor: "text",
        transformOrigin: "0 0",
        ...(this.#options.outline === true ? { outline: "1px solid rgb(220 0 0 / 0.7)" } : {}),
      });
      setUserSelect(span, "text");
      this.#specs.set(span, { page, spec });
      fragment.append(span);
      return span;
    });
    text.box.append(fragment);
    // Reads, then writes: each natural width in points, fractional, with the box briefly
    // untransformed (its zoom and turn would scale and rotate the rects).
    const placed = text.box.style.transform;
    text.box.style.transform = "none";
    // The box is its points wide: what the page around the viewer scales it by (a zoomed host)
    // comes out of every width.
    const outside = text.box.getBoundingClientRect().width / Math.max(1, text.box.offsetWidth);
    const natural = text.spans.map((span) => span.getBoundingClientRect().width / (outside || 1));
    text.box.style.transform = placed;
    specs.forEach((spec, k) => {
      const span = text.spans[k] as HTMLElement;
      const fit = (natural[k] ?? 0) > 0 ? spec.width / (natural[k] as number) : 1;
      span.style.transform = `rotate(${spec.angle}rad) scaleX(${fit})`;
    });
    this.#linkPage(page);
  }

  /** Gives the structure tree's elements the page's spans that their marked content drew. */
  #linkPage(page: number): void {
    const text = this.#pages.get(page);
    if (text === undefined) return;
    for (const span of text.spans) {
      const mcid = this.#specs.get(span)?.spec.mcid;
      if (mcid !== undefined) this.#structure.link(page, mcid, span.id);
    }
  }

  /** The mounted pages' spans, in page and reading order. */
  #spansInOrder(): HTMLElement[] {
    return [...this.#pages.entries()].sort(([a], [b]) => a - b).flatMap(([, text]) => text.spans);
  }

  /**
   * The selection's range in this viewer. A selection that runs on past the viewer into the app
   * around it is cut at the first or last mounted span, so copy, the pinned pages and
   * `onSelection` keep to the page text. `undefined` when it doesn't reach the viewer, or when its
   * other end is in another viewer: neither could tell the whole of it, so both leave it to the
   * browser.
   */
  #range(): Range | undefined {
    const selection = document.getSelection();
    if (selection === null || selection.rangeCount === 0 || selection.isCollapsed) return undefined;
    const range = selection.getRangeAt(0);
    const root = this.#host.root;
    if (root.contains(range.commonAncestorContainer)) return range;
    if (!range.intersectsNode(root)) return undefined;
    const outside = (node: Node) => !root.contains(node);
    const inOtherViewer = (node: Node) =>
      outside(node) && (node instanceof Element ? node : node.parentElement)?.closest(".lamina-viewer") != null;
    if (inOtherViewer(range.startContainer) || inOtherViewer(range.endContainer)) return undefined;
    const spans = this.#spansInOrder();
    const first = spans[0];
    const last = spans.at(-1);
    if (first === undefined || last === undefined) return undefined;
    const inside = range.cloneRange();
    if (outside(range.startContainer)) inside.setStart(first, 0);
    if (outside(range.endContainer)) inside.setEnd(last, last.childNodes.length);
    return inside.collapsed ? undefined : inside;
  }

  /** The selected part of each span the selection touches, in order. */
  #pieces(range: Range): Piece[] {
    const pieces: Piece[] = [];
    for (const span of this.#spansInOrder()) {
      const info = this.#specs.get(span);
      if (info === undefined || !range.intersectsNode(span)) continue;
      const full = span.textContent ?? "";
      const start = span.contains(range.startContainer) ? offsetIn(span, range.startContainer, range.startOffset) : 0;
      const end = span.contains(range.endContainer) ? offsetIn(span, range.endContainer, range.endOffset) : full.length;
      const text = full.slice(start, end);
      if (text !== "") pieces.push({ page: info.page, text, span: info.spec });
    }
    return pieces;
  }

  /** The selection's pieces: the browser selection's in this viewer, else the column's. */
  #selectedPieces(): Piece[] | undefined {
    const range = this.#range();
    if (range !== undefined) return this.#pieces(range);
    // The column, unless something else is selected or a field has the focus: that copy is theirs.
    const selection = document.getSelection();
    const elsewhere = selection !== null && selection.rangeCount > 0 && !selection.isCollapsed;
    const field = document.activeElement?.closest("input, textarea, [contenteditable]") ?? null;
    return elsewhere || field !== null ? undefined : this.#column?.pieces;
  }

  /**
   * Page `page`'s reading-order blocks if read; else starts reading them, and tells the app the
   * selection again once they come, its text then in reading order. A page whose blocks can't be
   * read gets none, once: its text is joined in run order.
   */
  #blocksOf(page: number): readonly ReadingBlock[] | undefined {
    const known = this.#blocks.get(page);
    if (known === "reading") return undefined;
    if (known !== undefined) return known;
    this.#blocks.set(page, "reading");
    const asked = this.#generation;
    const store = (blocks: readonly ReadingBlock[]) => {
      // Another document opened since: these blocks are the old one's.
      if (asked !== this.#generation || this.#blocks.get(page) !== "reading") return;
      this.#blocks.set(page, blocks);
      this.#retell();
    };
    this.#host.client
      .readingOrder(page)
      .then(store)
      .catch(() => store([]));
    return undefined;
  }

  /** Tells the app about the selection as it is now (its text may read better with new blocks). */
  #retell(): void {
    if (!this.#selecting) return;
    if (this.#column !== undefined) {
      this.#tell(
        this.#column.pieces,
        this.#column.spans.map((s) => s.getBoundingClientRect()),
        true,
      );
      return;
    }
    const range = this.#range();
    if (range !== undefined) this.#tell(this.#pieces(range), [...range.getClientRects()], false);
  }

  #copy(event: ClipboardEvent): void {
    const pieces = this.#selectedPieces();
    if (pieces === undefined || event.clipboardData === null) return;
    const copied = copySelection(pieces, this.#copyMode, (page) => this.#blocksOf(page));
    event.clipboardData.setData("text/plain", copied.text);
    event.clipboardData.setData("text/html", copied.html);
    event.preventDefault();
  }

  /** Ctrl/Cmd+A in the viewer selects the mounted pages' text (find reaches the rest). */
  #selectAll(event: KeyboardEvent): void {
    if (event.key.toLowerCase() !== "a" || !(event.ctrlKey || event.metaKey) || event.altKey) return;
    const spans = this.#spansInOrder();
    const first = spans[0];
    const last = spans.at(-1);
    if (first === undefined || last === undefined) return;
    event.preventDefault();
    const range = document.createRange();
    range.setStart(first, 0);
    range.setEnd(last, last.childNodes.length);
    const selection = document.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
  }

  #selectionChanged(): void {
    const range = this.#range();
    // A browser selection replaces a column selection.
    if (range !== undefined && this.#column !== undefined) this.#clearColumn(false);
    if (range === undefined && this.#column !== undefined) {
      // A selection made elsewhere (a field, the outline) ends the column's; a collapsed one doesn't.
      const selection = document.getSelection();
      if (selection !== null && selection.rangeCount > 0 && !selection.isCollapsed) this.#clearColumn(true);
      return;
    }
    if (range === undefined) {
      // A selection elsewhere on the page (an input, another viewer): told once, when it leaves.
      if (!this.#selecting) return;
      this.#selecting = false;
      this.#host.keepMounted([]);
      this.#options.onSelection?.(undefined);
      return;
    }
    this.#selecting = true;
    // Every page from the one the selection starts on to the one it ends on, those between too:
    // read from its two ends, not by walking every span while a drag goes on.
    // An end in a gap between pages counts as the other end's page.
    const start = pageOf(range.startContainer);
    const end = pageOf(range.endContainer);
    const [low, high] = [start ?? end, end ?? start];
    if (low !== undefined && high !== undefined)
      this.#host.keepMounted(range_(Math.min(low, high), Math.max(low, high)));
    this.#tell(this.#pieces(range), [...range.getClientRects()], false);
  }

  /** Tells the app about a selection of `pieces` (and reads their pages' blocks for copy). */
  #tell(pieces: Piece[], rects: DOMRect[], column: boolean): void {
    const pages = [...new Set(pieces.map((p) => p.page))];
    for (const page of pages) this.#blocksOf(page);
    if (this.#options.onSelection === undefined) return;
    // The text only when asked for (Search for, Share): joining it on every change of a drag costs.
    let text: string | undefined;
    const blocksOf = (page: number) => this.#blocksOf(page);
    this.#options.onSelection({
      get text() {
        text ??= copySelection(pieces, "paragraph", blocksOf).text;
        return text;
      },
      rects,
      pages,
      pointerType: this.#pointerType,
      settled: !this.#pointerDown,
      column,
    });
  }

  #pointerDownIn(event: PointerEvent): void {
    this.#pointerType = event.pointerType;
    this.#pointerDown = true;
    // The page the press is on: its blocks are read while the drag goes on, for the copy after it.
    const page = event.target instanceof Node ? pageOf(event.target) : undefined;
    if (page !== undefined && this.#pages.has(page)) this.#blocksOf(page);
    if (this.#column !== undefined) this.#clearColumn(true);
    const rectangle = (event.altKey || this.#columnMode) && event.pointerType !== "touch" && event.button === 0;
    if (!rectangle) return;
    // A rectangle, not a run of text: the browser's selection stays out of it.
    event.preventDefault();
    document.getSelection()?.removeAllRanges();
    const box = document.createElement("div");
    box.className = "lamina-column-selection";
    Object.assign(box.style, {
      position: "fixed",
      left: `${event.clientX}px`,
      top: `${event.clientY}px`,
      width: "0",
      height: "0",
      border: "1px dashed currentColor",
      background: "rgb(45 93 168 / 0.08)",
      pointerEvents: "none",
      zIndex: "1",
    });
    this.#host.root.append(box);
    this.#drag = { x: event.clientX, y: event.clientY, box, pointer: event.pointerId };
  }

  #dragTo(event: PointerEvent): void {
    const drag = this.#drag;
    if (drag === undefined || event.pointerId !== drag.pointer) return;
    const r = rectBetween(drag, event);
    Object.assign(drag.box.style, {
      left: `${r.left}px`,
      top: `${r.top}px`,
      width: `${r.width}px`,
      height: `${r.height}px`,
    });
  }

  #pointerUp(event: PointerEvent): void {
    const wasDown = this.#pointerDown;
    this.#pointerDown = false;
    const drag = this.#drag;
    if (drag !== undefined && event.pointerId === drag.pointer) {
      this.#drag = undefined;
      drag.box.remove();
      this.#selectColumn(rectBetween(drag, event));
      return;
    }
    if (!wasDown) return;
    // The drag ended: the selection is settled, and the app may show its toolbar now.
    const range = this.#range();
    if (range !== undefined) this.#tell(this.#pieces(range), [...range.getClientRects()], false);
  }

  /** Selects the runs whose centre is inside `area` (client coordinates), in page and run order. */
  #selectColumn(area: { left: number; top: number; width: number; height: number }): void {
    if (area.width < 2 || area.height < 2) return;
    const spans: HTMLElement[] = [];
    const pieces: Piece[] = [];
    const rects: DOMRect[] = [];
    for (const span of this.#spansInOrder()) {
      const info = this.#specs.get(span);
      if (info === undefined) continue;
      const r = span.getBoundingClientRect();
      const [x, y] = [r.left + r.width / 2, r.top + r.height / 2];
      if (x < area.left || x > area.left + area.width || y < area.top || y > area.top + area.height) continue;
      spans.push(span);
      rects.push(r);
      pieces.push({ page: info.page, text: span.textContent ?? "", span: info.spec });
    }
    if (pieces.length === 0) return;
    for (const span of spans) span.classList.add("lamina-column-selected");
    // The browser draws no selection for these: a tint marks them, under the ink as the
    // selection's own would be.
    for (const span of spans) span.style.backgroundColor = "rgb(45 93 168 / 0.25)";
    this.#column = { pieces, spans };
    this.#selecting = true;
    this.#host.keepMounted(new Set(pieces.map((p) => p.page)));
    this.#tell(pieces, rects, true);
  }

  /** Ends the column selection; `tell` says so to the app (not when a browser selection replaces it). */
  #clearColumn(tell: boolean): void {
    const column = this.#column;
    if (column === undefined) return;
    this.#column = undefined;
    for (const span of column.spans) {
      span.classList.remove("lamina-column-selected");
      span.style.backgroundColor = "";
    }
    if (!tell) return;
    this.#selecting = false;
    this.#host.keepMounted([]);
    this.#options.onSelection?.(undefined);
  }
}

/** The rectangle between a drag's start and the pointer now, in client coordinates. */
function rectBetween(start: { readonly x: number; readonly y: number }, now: { clientX: number; clientY: number }) {
  return {
    left: Math.min(start.x, now.clientX),
    top: Math.min(start.y, now.clientY),
    width: Math.abs(now.clientX - start.x),
    height: Math.abs(now.clientY - start.y),
  };
}

/** The index of the page `node` is on, from its shell. */
function pageOf(node: Node): number | undefined {
  const element = node instanceof Element ? node : node.parentElement;
  const index = element?.closest<HTMLElement>(".lamina-page")?.dataset.pageIndex;
  return index === undefined ? undefined : Number(index);
}

/** The character offset in `span`'s text of the boundary point (`node`, `offset`). */
function offsetIn(span: HTMLElement, node: Node, offset: number): number {
  if (node.nodeType === Node.TEXT_NODE) return offset;
  // A boundary between the span's children: before the first or after its text.
  return offset === 0 ? 0 : (span.textContent ?? "").length;
}
