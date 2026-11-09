// The link layer (web-app §6 C6, R07): on every mounted page, an anchor over each of Lamina PDF's
// links, in the page's `lamina-layer-annot` slot, above the text and below the find highlights.
// Like the other layers, the anchors sit in page points inside one box that a CSS transform scales
// and turns to the page. Only the anchors take pointer events, so text around a link still selects.
// Anchors follow page order, so Tab moves through a page's links in order, then the next page's.
//
// What a link does is decided here, never by the browser: a primary click or Enter is always
// handled (`preventDefault`). A link inside the document goes there through the viewer (a history
// jump); its `href` is the app's address for that page, so a middle click or "Open in new tab"
// opens the same document there. A web or email link asks the app to confirm first, whatever the
// click; its `href` is the address itself, so the browser shows it and can copy it. Anything else
// (another file, an action a reader never runs, a refused scheme, a broken destination) gets no
// `href`, does nothing, and says why on hover.

import type { PageLink, Target } from "@laminapdf/sdk";
import type { ViewerClient } from "../client.js";
import type { Rotation } from "../layout/page-layout.js";
import type { PageLayer } from "../view/viewer.js";
import { placeBox } from "./box.js";
import { externalLink } from "./link-url.js";

/** The words the layer gives links without a description of their own, and its hover notes. */
export interface LinkWords {
  /** A link to page `page` (0-based). */
  page(page: number): string;
  readonly nextPage: string;
  readonly previousPage: string;
  readonly firstPage: string;
  readonly lastPage: string;
  /** A link to another file, named when the file says which. */
  otherFile(file: string | undefined): string;
  /** A link whose address can't be opened (a `javascript:`, `file:` or `data:` one). */
  refused(uri: string): string;
  /** A link that runs an action a reader never runs (`Launch`, `JavaScript`…). */
  readonly unsupported: string;
  /** A link whose destination isn't in the document. */
  readonly broken: string;
}

/** English, for a viewer whose app gives none. */
const ENGLISH: LinkWords = {
  page: (page) => `Link to page ${page + 1}`,
  nextPage: "Goes to the next page",
  previousPage: "Goes to the previous page",
  firstPage: "Goes to the first page",
  lastPage: "Goes to the last page",
  otherFile: (file) =>
    file === undefined
      ? "Links to another file, which can't be opened from here"
      : `Links to another file, ${file}, which can't be opened from here`,
  refused: (uri) => `This link's address can't be opened: ${uri}`,
  unsupported: "This link runs an action the reader doesn't run",
  broken: "This link's destination isn't in the document",
};

/** The link layer's options (`createViewer`'s `links`). */
export interface LinkLayerOptions {
  /**
   * The address of page `page` (0-based) of this document, for an internal link's `href`, so a
   * new tab opens the same document there; `undefined` leaves the link without one.
   */
  readonly href?: (page: number) => string | undefined;
  /** A web or email link was followed: ask the reader before opening `href` (normalised, punycode host). */
  readonly onExternal?: (href: string) => void;
  /** The words for links without a description, and the hover notes (default English). */
  readonly words?: LinkWords;
}

/** What the link layer needs of its viewer. */
export interface LinkHost {
  readonly client: Pick<ViewerClient, "pageLinks">;
  /** Page `page`'s displayed size in points. */
  points(page: number): { readonly width: number; readonly height: number } | undefined;
  rotation(): Rotation;
  /** The number of pages. */
  pageCount(): number;
  /** Goes to `target` as a history jump. */
  goTo(target: Target): void;
  /** Goes to page `page` (0-based) as a history jump. */
  goToPage(page: number): void;
}

/** What following one anchor does. */
type Follow =
  | { readonly kind: "target"; readonly target: Target }
  | { readonly kind: "page"; readonly page: number }
  | { readonly kind: "external"; readonly href: string }
  | { readonly kind: "none" };

/** One mounted page's links. */
interface PageLinks {
  readonly box: HTMLDivElement;
  readonly abort: AbortController;
  size: { readonly width: number; readonly height: number } | undefined;
}

/** The page a standard named action goes to from page `page` of `count`, and its words. */
function namedAction(
  name: string,
  page: number,
  count: number,
  words: LinkWords,
): { readonly page: number; readonly label: string } | undefined {
  switch (name) {
    case "NextPage":
      return { page: Math.min(count - 1, page + 1), label: words.nextPage };
    case "PrevPage":
      return { page: Math.max(0, page - 1), label: words.previousPage };
    case "FirstPage":
      return { page: 0, label: words.firstPage };
    case "LastPage":
      return { page: count - 1, label: words.lastPage };
    default:
      return undefined;
  }
}

/** The box around eight-number quads (or the rectangle, without any), in page points. */
function areas(link: PageLink): { x: number; y: number; width: number; height: number }[] {
  const out = [];
  const q = link.quads ?? [];
  for (let at = 0; at + 8 <= q.length; at += 8) {
    const xs = [q[at], q[at + 2], q[at + 4], q[at + 6]].map((v) => v ?? 0);
    const ys = [q[at + 1], q[at + 3], q[at + 5], q[at + 7]].map((v) => v ?? 0);
    const x = Math.min(...xs);
    const y = Math.min(...ys);
    out.push({ x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y });
  }
  if (out.length > 0) return out;
  const [x0, y0, x1, y1] = link.rect;
  return [{ x: Math.min(x0, x1), y: Math.min(y0, y1), width: Math.abs(x1 - x0), height: Math.abs(y1 - y0) }];
}

/** The link layer of one viewer. */
export class LinkLayer implements PageLayer {
  readonly name = "annot";
  readonly #host: LinkHost;
  readonly #options: LinkLayerOptions;
  readonly #words: LinkWords;
  readonly #pages = new Map<number, PageLinks>();
  readonly #follows = new WeakMap<HTMLElement, Follow>();

  constructor(host: LinkHost, options: LinkLayerOptions = {}) {
    this.#host = host;
    this.#options = options;
    this.#words = options.words ?? ENGLISH;
  }

  onMount(page: number, slot: HTMLElement): void {
    const box = document.createElement("div");
    box.className = "lamina-link-layer";
    Object.assign(box.style, {
      position: "absolute",
      left: "0",
      top: "0",
      transformOrigin: "0 0",
      pointerEvents: "none",
    });
    slot.append(box);
    const view: PageLinks = { box, abort: new AbortController(), size: undefined };
    this.#pages.set(page, view);
    box.addEventListener("click", (e) => this.#click(e));
    // A middle click on a web link would open it without asking: it asks too.
    box.addEventListener("auxclick", (e) => {
      if (e.button === 1 && this.#followOf(e)?.kind === "external") this.#click(e);
    });
    this.#host.client
      .pageLinks(page, { signal: view.abort.signal })
      .then((links) => {
        if (this.#pages.get(page) === view) this.#draw(page, view, links);
      })
      .catch(() => {
        // Aborted (the page left), or links that can't be read: the page stays without them.
      });
  }

  onUnmount(page: number): void {
    const view = this.#pages.get(page);
    view?.abort.abort();
    view?.box.remove();
    this.#pages.delete(page);
  }

  onZoom(page: number, _slot: HTMLElement, size: { readonly width: number; readonly height: number }): void {
    const view = this.#pages.get(page);
    if (view === undefined) return;
    view.size = size;
    this.#place(page, view);
  }

  /** Places every page again: the view turned. */
  refresh(): void {
    for (const [page, view] of this.#pages) this.#place(page, view);
  }

  #place(page: number, view: PageLinks): void {
    const points = this.#host.points(page);
    if (points !== undefined && view.size !== undefined) placeBox(view.box, points, view.size, this.#host.rotation());
  }

  #draw(page: number, view: PageLinks, links: readonly PageLink[]): void {
    view.box.replaceChildren(...links.map((link) => this.#anchor(page, link)));
  }

  /** The anchor for `link` on page `page`: placed, named, and with what following it does. */
  #anchor(page: number, link: PageLink): HTMLAnchorElement {
    const { follow, label, note } = this.#meaning(page, link);
    const a = document.createElement("a");
    a.className = "lamina-link";
    a.draggable = false;
    const href = follow.kind === "external" ? follow.href : this.#internalHref(follow);
    if (href === undefined) {
      // No address to follow: still a link in the tab order, named, and saying why it does nothing.
      a.setAttribute("role", "link");
      a.setAttribute("aria-disabled", "true");
      a.tabIndex = 0;
    } else {
      a.href = href;
      if (follow.kind === "external") a.rel = "noopener noreferrer";
      a.referrerPolicy = "no-referrer";
    }
    a.setAttribute("aria-label", link.contents ?? label);
    if (note !== undefined) a.title = note;
    const [first, ...rest] = areas(link);
    const bounds = first ?? { x: 0, y: 0, width: 0, height: 0 };
    Object.assign(a.style, {
      position: "absolute",
      left: `${bounds.x}px`,
      top: `${bounds.y}px`,
      width: `${bounds.width}px`,
      height: `${bounds.height}px`,
      pointerEvents: "auto",
      cursor: href === undefined ? "default" : "pointer",
    });
    // A link over several areas (its quads): the first is the anchor, the others its children,
    // placed from it, so only the areas follow the link and the focus ring is on the first.
    for (const area of rest) {
      const part = document.createElement("span");
      Object.assign(part.style, {
        position: "absolute",
        left: `${area.x - bounds.x}px`,
        top: `${area.y - bounds.y}px`,
        width: `${area.width}px`,
        height: `${area.height}px`,
      });
      a.append(part);
    }
    this.#follows.set(a, follow);
    return a;
  }

  /** What `link` on page `page` does, its name, and the note it shows on hover. */
  #meaning(page: number, link: PageLink): { follow: Follow; label: string; note?: string } {
    const words = this.#words;
    switch (link.kind) {
      case "dest":
      case "named":
        return link.target === undefined
          ? { follow: { kind: "none" }, label: words.broken, note: words.broken }
          : { follow: { kind: "target", target: link.target }, label: words.page(link.target.page) };
      case "uri": {
        const external = externalLink(link.uri);
        return external.kind === "open"
          ? { follow: { kind: "external", href: external.href }, label: external.href, note: external.href }
          : { follow: { kind: "none" }, label: words.refused(link.uri), note: words.refused(link.uri) };
      }
      case "remote": {
        const note = words.otherFile(link.file);
        return { follow: { kind: "none" }, label: note, note };
      }
      case "named-action": {
        const to = namedAction(link.name, page, this.#host.pageCount(), words);
        return to === undefined
          ? { follow: { kind: "none" }, label: words.unsupported, note: words.unsupported }
          : { follow: { kind: "page", page: to.page }, label: to.label, note: to.label };
      }
      default:
        return { follow: { kind: "none" }, label: words.unsupported, note: words.unsupported };
    }
  }

  #internalHref(follow: Follow): string | undefined {
    if (follow.kind === "target") return this.#options.href?.(follow.target.page);
    if (follow.kind === "page") return this.#options.href?.(follow.page);
    return undefined;
  }

  /** What following the anchor `e` happened on does. */
  #followOf(e: Event): Follow | undefined {
    const anchor = e.target instanceof Element ? e.target.closest("a.lamina-link") : null;
    return anchor instanceof HTMLElement ? this.#follows.get(anchor) : undefined;
  }

  #click(e: MouseEvent): void {
    const follow = this.#followOf(e);
    if (follow === undefined) return;
    const external = follow.kind === "external";
    // A modified click on a link inside the document opens its `href` where the browser puts it.
    if (!external && (e.button !== 0 || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey)) return;
    e.preventDefault();
    switch (follow.kind) {
      case "target":
        this.#host.goTo(follow.target);
        return;
      case "page":
        this.#host.goToPage(follow.page);
        return;
      case "external":
        this.#options.onExternal?.(follow.href);
        return;
      case "none":
        return;
    }
  }
}
