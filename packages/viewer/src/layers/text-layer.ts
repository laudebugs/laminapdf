// The text layer (web-app §6 C5): on every mounted page, one transparent span per text run of
// Lamina PDF's extraction, laid over the rendered glyphs, so the browser's own selection, copy
// and screen readers work on the page's text. Spans are placed in page points inside a box that
// one CSS transform scales and rotates to the page's current size, so a zoom or a rotation moves
// no span. The viewer is the one selection root: only spans are selectable, never canvases, gaps
// or the app around it, so a selection runs from one page into the next; the pages it touches
// stay mounted while it lasts. Copy writes the extracted text, joined in reading order.

import type { ViewerClient } from "../client.js";
import type { Rotation } from "../layout/page-layout.js";
import type { OpenedDocument } from "../protocol.js";
import type { PageLayer } from "../view/viewer.js";
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

/** The page's box: its size in points, scaled and turned to `size` (CSS px) at `rotation`. */
function boxTransform(
  points: { width: number; height: number },
  size: { width: number; height: number },
  rotation: Rotation,
): string {
  const turned = rotation === 90 || rotation === 270;
  const scale = size.width / (turned ? points.height : points.width);
  const [x, y] = { 0: [0, 0], 90: [size.width, 0], 180: [size.width, size.height], 270: [0, size.height] }[rotation];
  return `translate(${x}px, ${y}px) rotate(${rotation}deg) scale(${scale})`;
}

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

  constructor(host: TextLayerHost, options: TextLayerOptions = {}) {
    this.#host = host;
    this.#options = options;
    this.#structure = new StructureLayer(host.root, options.untaggedLabel ?? "Untagged document");
    // Nothing in the viewer is selectable but the spans (set on each): canvases, gaps and other
    // layers stay out of every selection.
    host.root.style.setProperty("user-select", "none");
    host.root.style.setProperty("-webkit-user-select", "none");
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
    const pages = [...this.#pages.keys()];
    this.#host.client
      .structure()
      .then((structure) => {
        this.#structure.show(structure);
        for (const page of pages) this.#linkPage(page);
      })
      .catch(() => {
        // No structure (a worker restart, a closed document): the spans still read in order.
      });
  }

  onMount(page: number, slot: HTMLElement): void {
    const box = document.createElement("div");
    box.className = "lamina-text-layer";
    Object.assign(box.style, { position: "absolute", left: "0", top: "0", transformOrigin: "0 0" });
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
    const text = this.#pages.get(page);
    const points = this.#points[page];
    if (text === undefined || points === undefined) return;
    Object.assign(text.box.style, {
      width: `${points.width}px`,
      height: `${points.height}px`,
      transform: boxTransform(points, size, this.#host.rotation()),
    });
  }

  /** Places every page's box again: the view turned. */
  refresh(): void {
    for (const [page, size] of this.#sizes) this.onZoom(page, document.body, size);
  }

  /** Stops listening; the viewer calls it on destroy. */
  destroy(): void {
    for (const f of this.#cleanup.splice(0)) f();
    for (const page of [...this.#pages.keys()]) this.onUnmount(page);
    this.#structure.remove();
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
      span.id = `lamina-text-${page}-${k}`;
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
      span.style.setProperty("user-select", "text");
      span.style.setProperty("-webkit-user-select", "text");
      this.#specs.set(span, { page, spec });
      fragment.append(span);
      return span;
    });
    text.box.append(fragment);
    // Reads, then writes. The natural width is in the box's own units (points), whatever the zoom.
    const natural = text.spans.map((span) => span.offsetWidth);
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
      if (!range.intersectsNode(span)) continue;
      const info = this.#specs.get(span);
      const full = span.textContent ?? "";
      if (info === undefined) continue;
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
      this.#host.keepMounted([]);
      this.#options.onSelection?.(undefined);
      return;
    }
    const pieces = this.#pieces(range);
    const touched = [...new Set(pieces.map((p) => p.page))];
    const [low, high] = [Math.min(...touched), Math.max(...touched)];
    // Every page from the first to the last the selection touches: those between it as well.
    this.#host.keepMounted(touched.length === 0 ? [] : Array.from({ length: high - low + 1 }, (_, k) => low + k));
    this.#options.onSelection?.({
      text: joinPieces(pieces),
      rects: [...range.getClientRects()],
      pages: touched,
      pointerType: this.#pointerType,
    });
  }
}

/** The character offset in `span`'s text of the boundary point (`node`, `offset`). */
function offsetIn(span: HTMLElement, node: Node, offset: number): number {
  if (node.nodeType === Node.TEXT_NODE) return offset;
  // A boundary between the span's children: before the first or after its text.
  return offset === 0 ? 0 : (span.textContent ?? "").length;
}
