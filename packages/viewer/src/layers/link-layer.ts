// The link layer (web-app §6 C6, R07): on every mounted page, an anchor over each of Lamina PDF's
// links, in the page's `lamina-layer-annot` slot, above the text and below the find highlights.
// Like the other layers, the anchors sit in page points inside one box that a CSS transform scales
// and turns to the page. Only the anchors take pointer events, so text around a link still selects.
// Anchors follow page order, so Tab moves through a page's links in order, then the next page's.
// While a drag that started outside every link is under way, no anchor takes pointer events, so a
// selection begun in the text runs on over the text under a link.
//
// What a link does is decided here, never by the browser: a primary click or Enter is always
// handled (`preventDefault`). A link inside the document goes there through the viewer (a history
// jump); its `href` is the app's address for that page, so a middle click or "Open in new tab"
// opens the same document there. A web or email link asks the app to confirm first, on a click,
// a middle click or Enter. It has no `href`, so no browser menu ("Open in new tab") can open it
// without asking; its title shows the address. Anything else (another file, an action a reader
// never runs, a refused scheme, a broken destination) has no `href` either, does nothing, and
// says why on hover.
//
// Previews (R07): a link inside the document, a standard page action or a link to another
// file asks its viewer to preview it. With a mouse or pen, 300 ms after the pointer comes onto it,
// and the preview ends 300 ms after the pointer leaves both the link and the preview (which holds it
// while hovered, WCAG 1.4.13). With the keyboard, 300 ms after it takes visible focus, ending on blur.
// With touch, on a long-press of 500 ms that moves less than 10 CSS px: the click that would follow
// is cancelled, the press selects no text, and the OS's link menu stays shut. Web links never preview:
// they ask before opening, and their page is never fetched (ADR 0011).

import type { PageLink, Target } from "@laminapdf/sdk";
import type { ViewerClient } from "../client.js";
import type { PageSize, Rotation } from "../layout/page-layout.js";
import type { PageLayer } from "../view/viewer.js";
import { placeBox } from "./box.js";
import { externalLink, type OpenableLink } from "./link-url.js";

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
  /** A link that asks to run an action, which the reader never does (`Launch`, `JavaScript`…). */
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
  unsupported: "This link asks to run an action. The reader never runs these.",
  broken: "This link's destination isn't in the document",
};

/** The link layer's options (`createViewer`'s `links`). */
export interface LinkLayerOptions {
  /**
   * The address of page `page` (0-based) of this document, for an internal link's `href`, so a
   * new tab opens the same document there; `undefined` leaves the link without one.
   */
  readonly href?: (page: number) => string | undefined;
  /** A web or email link was followed: ask the reader before opening it (normalised, punycode host). */
  readonly onExternal?: (link: OpenableLink) => void;
  /** The words for links without a description, and the hover notes (default English). */
  readonly words?: LinkWords;
  /**
   * A link that does nothing was followed (another file, an action never run, a refused
   * address, a broken destination), with the note it shows: for the app's debug log.
   */
  readonly onInert?: (link: PageLink, note: string) => void;
}

/** How the pointer asked for a preview. */
export type LinkPreviewPointer = "fine" | "keyboard" | "touch";

/** A link asking to be previewed (the viewer's `linkpreview` event). */
export interface LinkPreviewRequest {
  readonly link: PageLink;
  /** What the preview shows: where the link goes, or a sentence saying what it does. */
  readonly shows:
    | { readonly kind: "target"; readonly target: Target }
    | { readonly kind: "page"; readonly page: number; readonly note: string }
    | { readonly kind: "note"; readonly note: string };
  /** The link's anchor, which the preview describes (`aria-describedby`). */
  readonly anchor: HTMLElement;
  readonly pointer: LinkPreviewPointer;
}

/** How long a link is hovered or focused before its preview shows, and after it is left before it goes (ms). */
export const PREVIEW_DWELL_MS = 300;
/** How long a finger rests on a link before its preview opens (ms). */
export const PREVIEW_LONG_PRESS_MS = 500;
/** How far a finger may move (CSS px) and still long-press. */
export const PREVIEW_SLOP = 10;

/** What the link layer needs of its viewer. */
export interface LinkHost {
  /** The viewer's root, where a selection drag starts. */
  readonly root: HTMLElement;
  readonly client: Pick<ViewerClient, "pageLinks">;
  /** Page `page`'s displayed size in points. */
  points(page: number): PageSize | undefined;
  rotation(): Rotation;
  /** The number of pages. */
  pageCount(): number;
  /** Goes to `target` as a history jump. */
  goTo(target: Target): void;
  /** Goes to page `page` (0-based) as a history jump. */
  goToPage(page: number): void;
  /** Shows a preview of a link. */
  preview?(request: LinkPreviewRequest): void;
  /** Hides the preview of `anchor`'s link. */
  previewEnd?(anchor: HTMLElement): void;
}

/** What following one anchor does. */
type Follow =
  | { readonly kind: "target"; readonly target: Target }
  | { readonly kind: "page"; readonly page: number; readonly label: string }
  | { readonly kind: "external"; readonly link: OpenableLink }
  | { readonly kind: "none"; readonly link: PageLink; readonly note: string };

/** One mounted page's links. */
interface PageLinks {
  readonly box: HTMLDivElement;
  readonly abort: AbortController;
  size: PageSize | undefined;
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

/** The selector every link anchor matches. */
const ANCHOR = "a.lamina-link";

/** The link anchor `target` is in, if any. */
function anchorOf(target: EventTarget | null): HTMLElement | undefined {
  const anchor = target instanceof Element ? target.closest(ANCHOR) : null;
  return anchor instanceof HTMLElement ? anchor : undefined;
}

/** What a link that does nothing means: named and noted by `note`. */
function inert(link: PageLink, note: string): { follow: Follow; label: string; note: string } {
  return { follow: { kind: "none", link, note }, label: note, note };
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
  readonly #links = new WeakMap<HTMLElement, PageLink>();
  readonly #listening = new AbortController();
  /** The dwell, grace or long-press timer. */
  #timer: ReturnType<typeof setTimeout> | undefined;
  /** The anchor whose preview shows, and how it was asked for. */
  #shown: { readonly anchor: HTMLElement; readonly pointer: LinkPreviewPointer } | undefined;
  /** The anchor the dwell or long-press timer is for. */
  #pending: HTMLElement | undefined;
  /** The pointer is over the preview: it stays. */
  #held = false;
  /** The anchor under a fine pointer, if any. */
  #hovered: HTMLElement | undefined;
  /** An anchor whose preview Escape closed: it shows again only after the pointer or focus leaves it. */
  #dismissed: HTMLElement | undefined;
  /** A touch on an anchor: where it started. */
  #press: { readonly x: number; readonly y: number; readonly id: number } | undefined;
  /** A long-press opened a preview: the click its lift sends is cancelled. */
  #swallowClick = false;
  /** The last pointer that went down was a finger (for `contextmenu`, which says nothing of it). */
  #touching = false;

  constructor(host: LinkHost, options: LinkLayerOptions = {}) {
    this.#host = host;
    this.#options = options;
    this.#words = options.words ?? ENGLISH;
    const { signal } = this.#listening;
    host.root.addEventListener(
      "pointerdown",
      (e) => {
        if (e.button === 0 && !this.#followOf(e)) this.#passThrough(true);
      },
      { signal },
    );
    for (const end of ["pointerup", "pointercancel"]) {
      document.addEventListener(end, () => this.#passThrough(false), { signal });
    }
    // A button released outside the window may send no pointerup: leaving the window ends the drag too.
    window.addEventListener("blur", () => this.#passThrough(false), { signal });
  }

  /** Stops listening: the viewer is going. */
  destroy(): void {
    clearTimeout(this.#timer);
    this.#listening.abort();
  }

  /** The pointer is over the preview (`true`) or has left it (`false`): a hovered preview stays. */
  hold(held: boolean): void {
    this.#held = held;
    if (held) {
      if (this.#shown !== undefined) clearTimeout(this.#timer);
    } else if (this.#shown?.pointer === "fine" && this.#hovered !== this.#shown.anchor) {
      this.#later(PREVIEW_DWELL_MS, () => this.#end());
    }
  }

  /** Escape or Close: hides the preview, which stays hidden until its link is left. */
  dismiss(): void {
    this.#dismissed = this.#shown?.anchor ?? this.#pending;
    this.#cancel();
    this.#end();
  }

  /** The previewed link, else the focused one: what `follow` would follow. */
  #followable(): HTMLElement | undefined {
    const anchor = this.#shown?.anchor ?? anchorOf(document.activeElement);
    return anchor?.isConnected === true ? anchor : undefined;
  }

  /** Whether a link is previewed or focused, so `follow` has one to follow. */
  canFollow(): boolean {
    return this.#followable() !== undefined;
  }

  /** Follows the previewed link, else the focused one (`nav.follow-link`); `false` when there is neither. */
  follow(): boolean {
    const anchor = this.#followable();
    if (anchor === undefined) return false;
    this.#cancel();
    this.#end();
    // A long-press's lift may have sent no click: this one is not that click.
    this.#swallowClick = false;
    anchor.click();
    return true;
  }

  #later(ms: number, run: () => void): void {
    clearTimeout(this.#timer);
    this.#timer = setTimeout(run, ms);
  }

  #cancel(): void {
    clearTimeout(this.#timer);
    this.#timer = undefined;
    this.#pending = undefined;
  }

  /** Asks for `anchor`'s preview, if its link has one. */
  #show(anchor: HTMLElement, pointer: LinkPreviewPointer): void {
    this.#pending = undefined;
    const link = this.#links.get(anchor);
    const follow = this.#follows.get(anchor);
    if (!anchor.isConnected || link === undefined || follow === undefined) return;
    const shows = this.#shows(link, follow);
    if (shows === undefined) return;
    if (this.#shown !== undefined && this.#shown.anchor !== anchor) this.#end();
    this.#shown = { anchor, pointer };
    this.#host.preview?.({ link, shows, anchor, pointer });
  }

  /** Hides the preview. */
  #end(): void {
    const shown = this.#shown;
    if (shown === undefined) return;
    this.#shown = undefined;
    this.#held = false;
    this.#host.previewEnd?.(shown.anchor);
  }

  /** What a preview of `link` shows; `undefined` for a link that has none (web links, inert ones). */
  #shows(link: PageLink, follow: Follow): LinkPreviewRequest["shows"] | undefined {
    switch (follow.kind) {
      case "target":
        return { kind: "target", target: follow.target };
      case "page":
        return { kind: "page", page: follow.page, note: follow.label };
      case "none":
        return link.kind === "remote" ? { kind: "note", note: follow.note } : undefined;
      default:
        return undefined;
    }
  }

  /** Listens on one page's links for what asks for a preview: the mouse, focus and touch. */
  #listenForPreviews(box: HTMLElement): void {
    this.#listenForHover(box);
    this.#listenForFocus(box);
    this.#listenForTouch(box);
  }

  /** A mouse or pen: 300 ms on a link shows its preview, 300 ms off both ends it. */
  #listenForHover(box: HTMLElement): void {
    box.addEventListener("pointerover", (e) => {
      if (e.pointerType === "touch") return;
      const anchor = anchorOf(e.target);
      if (anchor === undefined || anchor === this.#hovered) return;
      this.#hovered = anchor;
      if (anchor === this.#dismissed) return;
      if (this.#shown?.anchor === anchor) {
        clearTimeout(this.#timer);
        return;
      }
      this.#pending = anchor;
      this.#later(PREVIEW_DWELL_MS, () => this.#show(anchor, "fine"));
    });
    box.addEventListener("pointerout", (e) => {
      if (e.pointerType === "touch") return;
      const anchor = anchorOf(e.target);
      if (anchor === undefined || (e.relatedTarget instanceof Node && anchor.contains(e.relatedTarget))) return;
      this.#hovered = undefined;
      if (this.#dismissed === anchor) this.#dismissed = undefined;
      if (this.#pending === anchor) this.#cancel();
      if (this.#shown?.anchor === anchor && this.#shown.pointer === "fine" && !this.#held) {
        this.#later(PREVIEW_DWELL_MS, () => this.#end());
      }
    });
  }

  /** The keyboard: 300 ms of visible focus shows the preview; leaving the link ends it. */
  #listenForFocus(box: HTMLElement): void {
    box.addEventListener("focusin", (e) => {
      const anchor = anchorOf(e.target);
      if (anchor === undefined || anchor === this.#dismissed || !anchor.matches(":focus-visible")) return;
      this.#pending = anchor;
      this.#later(PREVIEW_DWELL_MS, () => this.#show(anchor, "keyboard"));
    });
    box.addEventListener("focusout", (e) => {
      const anchor = anchorOf(e.target);
      if (anchor === undefined) return;
      if (this.#dismissed === anchor) this.#dismissed = undefined;
      if (this.#pending === anchor) this.#cancel();
      if (this.#shown?.anchor === anchor && this.#shown.pointer === "keyboard") this.#end();
    });
  }

  /** A finger: a still long-press opens the preview, and the click its lift sends is cancelled. */
  #listenForTouch(box: HTMLElement): void {
    box.addEventListener("pointerdown", (e) => {
      // A new press: a long-press before it that sent no click leaves nothing to cancel.
      this.#swallowClick = false;
      this.#touching = e.pointerType === "touch";
      const anchor = this.#touching ? anchorOf(e.target) : undefined;
      if (anchor === undefined) return;
      this.#press = { x: e.clientX, y: e.clientY, id: e.pointerId };
      this.#pending = anchor;
      this.#later(PREVIEW_LONG_PRESS_MS, () => {
        this.#press = undefined;
        this.#swallowClick = true;
        // The press selects no text, whatever the browser began.
        document.getSelection()?.removeAllRanges();
        this.#show(anchor, "touch");
      });
    });
    box.addEventListener("pointermove", (e) => {
      const press = this.#press;
      if (press === undefined || e.pointerId !== press.id) return;
      if (Math.hypot(e.clientX - press.x, e.clientY - press.y) > PREVIEW_SLOP) {
        this.#press = undefined;
        this.#cancel();
      }
    });
    for (const end of ["pointerup", "pointercancel"] as const) {
      box.addEventListener(end, (e) => {
        if (this.#press?.id !== e.pointerId) return;
        this.#press = undefined;
        this.#cancel();
      });
    }
    // Before the click that follows: a long-press's lift follows nothing.
    box.addEventListener(
      "click",
      (e) => {
        if (!this.#swallowClick) return;
        this.#swallowClick = false;
        e.preventDefault();
        e.stopImmediatePropagation();
      },
      { capture: true },
    );
    box.addEventListener("contextmenu", (e) => {
      if (this.#touching && anchorOf(e.target) !== undefined) e.preventDefault();
    });
  }

  /** Lets the pointer through every anchor (a selection drag is under way), or stops letting it. */
  #passThrough(through: boolean): void {
    for (const { box } of this.#pages.values()) {
      for (const a of box.children) (a as HTMLElement).style.pointerEvents = through ? "none" : "auto";
    }
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
    this.#listenForPreviews(box);
    box.addEventListener("click", (e) => this.#click(e));
    // A link without an `href` (a web link) gets no click from Enter: it follows it here.
    box.addEventListener("keydown", (e) => {
      const anchor = e.target instanceof HTMLElement ? e.target : undefined;
      // One follow per press, as a native link's click: a held key's repeats ask nothing more.
      if (e.key === "Enter" && !e.repeat && anchor !== undefined && !anchor.hasAttribute("href")) this.#click(e);
    });
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
    if (view !== undefined && this.#shown !== undefined && view.box.contains(this.#shown.anchor)) this.#end();
    if (view !== undefined && this.#pending !== undefined && view.box.contains(this.#pending)) this.#cancel();
    view?.abort.abort();
    view?.box.remove();
    this.#pages.delete(page);
  }

  onZoom(page: number, _slot: HTMLElement, size: PageSize): void {
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
    const href = this.#internalHref(follow);
    if (href === undefined) {
      // No address for the browser to follow: still a link in the tab order, and named.
      a.setAttribute("role", "link");
      a.tabIndex = 0;
      if (follow.kind === "none") a.setAttribute("aria-disabled", "true");
    } else {
      a.href = href;
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
      cursor: follow.kind === "none" ? "default" : "pointer",
      // A long-press opens the preview: no OS link menu, no selection of the link.
      webkitTouchCallout: "none",
      webkitUserSelect: "none",
      userSelect: "none",
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
    this.#links.set(a, link);
    return a;
  }

  /** What `link` on page `page` does, its name, and the note it shows on hover. */
  #meaning(page: number, link: PageLink): { follow: Follow; label: string; note?: string } {
    const words = this.#words;
    switch (link.kind) {
      case "dest":
      case "named":
        return link.target === undefined
          ? inert(link, words.broken)
          : { follow: { kind: "target", target: link.target }, label: words.page(link.target.page) };
      case "uri": {
        const external = externalLink(link.uri);
        return external.kind === "open"
          ? { follow: { kind: "external", link: external }, label: external.href, note: external.href }
          : inert(link, words.refused(link.uri));
      }
      case "remote": {
        return inert(link, words.otherFile(link.file));
      }
      case "named-action": {
        const to = namedAction(link.name, page, this.#host.pageCount(), words);
        return to === undefined
          ? inert(link, words.unsupported)
          : { follow: { kind: "page", page: to.page, label: to.label }, label: to.label, note: to.label };
      }
      default:
        return inert(link, words.unsupported);
    }
  }

  #internalHref(follow: Follow): string | undefined {
    if (follow.kind === "target") return this.#options.href?.(follow.target.page);
    if (follow.kind === "page") return this.#options.href?.(follow.page);
    return undefined;
  }

  /** What following the anchor `e` happened on does. */
  #followOf(e: Event): Follow | undefined {
    const anchor = anchorOf(e.target);
    return anchor === undefined ? undefined : this.#follows.get(anchor);
  }

  #click(e: MouseEvent | KeyboardEvent): void {
    const follow = this.#followOf(e);
    if (follow === undefined) return;
    // Following a link ends its preview.
    this.#cancel();
    this.#end();
    const external = follow.kind === "external";
    // A modified click on a link inside the document opens its `href` where the browser puts it.
    const modified = e.ctrlKey || e.metaKey || e.shiftKey || e.altKey || (e instanceof MouseEvent && e.button !== 0);
    if (!external && modified) return;
    e.preventDefault();
    switch (follow.kind) {
      case "target":
        this.#host.goTo(follow.target);
        return;
      case "page":
        this.#host.goToPage(follow.page);
        return;
      case "external":
        this.#options.onExternal?.(follow.link);
        return;
      case "none":
        this.#options.onInert?.(follow.link, follow.note);
        return;
    }
  }
}
