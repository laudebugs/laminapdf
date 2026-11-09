// The text layer (web-app §6 C5): on every mounted page, one transparent span per text run of
// Lamina PDF's extraction, laid over the rendered glyphs, so the browser's own selection, copy
// and screen readers work on the page's text. Spans are placed in page points inside a box that
// one CSS transform scales and rotates to the page's current size, so a zoom or a rotation moves
// no span. The viewer is the one selection root: only spans are selectable, never canvases, gaps
// or the app around it, so a selection runs from one page into the next; the pages it touches
// stay mounted while it lasts. Copy writes the extracted text, joined in reading order.

import type { ViewerClient } from "../client.js";
import type { Rotation } from "../layout/page-layout.js";
import { range as range_ } from "../layout/range.js";
import type { OpenedDocument } from "../protocol.js";
import type { PageLayer } from "../view/viewer.js";
import { placeBox } from "./box.js";
import { joinPieces, layoutSpans, type Piece, type SpanSpec } from "./spans.js";
import { StructureLayer } from "./structure-layer.js";

/** What the text layer needs of its viewer. */
export interface TextLayerHost {
  /** The viewer's root: the selection root, and where the structure tree lives. */
  readonly root: HTMLElement;
  readonly client: Pick<ViewerClient, "pageText" | "structure">;
  /** The view's rotation now. */
  rotation(): Rotation;
  /** Keeps these pages mounted (the ones the selection touches). */
  keepMounted(pages: Iterable<number>): void;
}

/** A selection in the text layer, for the app's selection surfaces (UI/UX plan §5.3). */
export interface TextSelection {
  /** The selected text, as copy writes it. */
  readonly text: string;
  /** Its client rects, to place a toolbar or a menu near. */
  readonly rects: readonly DOMRect[];
  /** The pages it touches, in order. */
  readonly pages: readonly number[];
  /** The pointer that made it: `mouse`, `pen`, `touch`, or `""` for the keyboard. */
  readonly pointerType: string;
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
  #points: readonly { width: number; height: number }[] = [];
  /** Each mounted page's size in CSS px, as the last resize gave it. */
  readonly #sizes = new Map<number, { readonly width: number; readonly height: number }>();
  #pointerType = "";
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
    // Nothing in the viewer is selectable but the spans (set on each): canvases, gaps and other
    // layers stay out of every selection.
    selectable(host.root, "none");
    this.#listen(document, "selectionchange", () => this.#selectionChanged());
    this.#listen(document, "copy", (e) => this.#copy(e as ClipboardEvent));
    this.#listen(host.root, "keydown", (e) => this.#selectAll(e as KeyboardEvent));
    this.#listen(host.root, "pointerdown", (e) => {
      this.#pointerType = (e as PointerEvent).pointerType;
    });
  }

  /** A new document: its page sizes, and its structure tree, read once. */
  open(document: OpenedDocument): void {
    this.#points = document.pages.map((p) => ({ width: p.width, height: p.height }));
    this.#structure.reset();
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
    // The spans lie over the canvas, so an app's ::selection fill (opaque, as a selection tint is)
    // would paint over the glyphs it selects. Multiplied, the fill shows where the page is paper and
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
    const points = this.#points[page];
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
      selectable(span, "text");
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

  /** The selection's range, when it is in this viewer's text. */
  #range(): Range | undefined {
    const selection = document.getSelection();
    if (selection === null || selection.rangeCount === 0 || selection.isCollapsed) return undefined;
    const range = selection.getRangeAt(0);
    return this.#host.root.contains(range.commonAncestorContainer) ? range : undefined;
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

  #copy(event: ClipboardEvent): void {
    const range = this.#range();
    if (range === undefined || event.clipboardData === null) return;
    event.clipboardData.setData("text/plain", joinPieces(this.#pieces(range)));
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
    if (this.#options.onSelection === undefined) return;
    const pieces = this.#pieces(range);
    this.#options.onSelection({
      text: joinPieces(pieces),
      rects: [...range.getClientRects()],
      pages: [...new Set(pieces.map((p) => p.page))],
      pointerType: this.#pointerType,
    });
  }
}

/** Sets `element`'s `user-select`, with WebKit's prefix (Safari before 26 reads only that). */
function selectable(element: HTMLElement, value: "none" | "text"): void {
  element.style.setProperty("user-select", value);
  element.style.setProperty("-webkit-user-select", value);
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
