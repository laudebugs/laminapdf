// The structure layer (web-app §6 C5): a tagged document's structure tree as a hidden but
// accessible tree of elements with ARIA roles (headings, paragraphs, lists, tables, figures with
// their alternate text) whose leaves own the text layer's spans through `aria-owns`, matched by
// page and MCID. Assistive technology then reads the document in structure order with its
// semantics, whichever backend renders it (the pdf.js pattern). The spans join their elements as
// their pages mount and leave as they unmount. A document without structure says so once.

import type { StructureKid, StructureNode } from "@laminapdf/sdk";
import type { StructureData } from "../protocol.js";

/** An ARIA role (and level) per standard structure type; types not here are plain groups. */
const ROLES: Readonly<Record<string, string>> = {
  P: "paragraph",
  H: "heading",
  Title: "heading",
  L: "list",
  LI: "listitem",
  Table: "table",
  THead: "rowgroup",
  TBody: "rowgroup",
  TFoot: "rowgroup",
  TR: "row",
  TH: "columnheader",
  TD: "cell",
  Formula: "math",
  BlockQuote: "blockquote",
  TOC: "list",
  TOCI: "listitem",
  Note: "note",
};

/** Hidden from sight, present for assistive technology (as sr-only CSS does). */
const HIDDEN = {
  position: "absolute",
  width: "1px",
  height: "1px",
  overflow: "hidden",
  clipPath: "inset(50%)",
  whiteSpace: "nowrap",
} as const;

/** One viewer's structure tree. */
export class StructureLayer {
  readonly #root: HTMLElement;
  readonly #untagged: string;
  #tree: HTMLElement | undefined;
  /** The elements that own each page's marked content, by `page:mcid`. */
  #owners = new Map<string, HTMLElement[]>();

  readonly #idPrefix: string;

  constructor(root: HTMLElement, untaggedLabel: string, idPrefix: string) {
    this.#root = root;
    this.#untagged = untaggedLabel;
    this.#idPrefix = idPrefix;
  }

  /** Builds the tree from the worker's answer, or the one-time note for an untagged document. */
  show(structure: StructureData): void {
    this.reset();
    const tree = document.createElement("div");
    tree.className = "lamina-structure";
    Object.assign(tree.style, HIDDEN);
    const untagged = structure.tree === undefined || structure.tree.length === 0;
    if (untagged) tree.setAttribute("role", "status");
    else for (const node of structure.tree) tree.append(this.#element(node, undefined));
    this.#root.prepend(tree);
    this.#tree = tree;
    // A live region speaks what changes in it after it is in the page, not what it was made with.
    if (untagged) {
      requestAnimationFrame(() => {
        if (this.#tree === tree) tree.textContent = this.#untagged;
      });
    }
  }

  /** Span `id` drew marked content `mcid` of `page`: its element owns it. */
  link(page: number, mcid: number, id: string): void {
    for (const owner of this.#owners.get(`${page}:${mcid}`) ?? []) {
      const owned = (owner.getAttribute("aria-owns") ?? "").split(" ").filter(Boolean);
      // In page and reading order, whatever order the pages mounted in.
      if (!owned.includes(id)) owner.setAttribute("aria-owns", [...owned, id].sort(this.#byPlace).join(" "));
    }
  }

  /** Orders span ids (`<prefix><page>-<index>`) by page, then by index. */
  readonly #byPlace = (a: string, b: string): number => {
    const place = (id: string) => id.slice(this.#idPrefix.length).split("-").map(Number);
    const [pa = 0, ia = 0] = place(a);
    const [pb = 0, ib = 0] = place(b);
    return pa - pb || ia - ib;
  };

  /** Page `page` unmounted: its spans leave the elements that owned them. */
  unlink(page: number): void {
    const prefix = `${this.#idPrefix}${page}-`;
    for (const [key, owners] of this.#owners) {
      if (!key.startsWith(`${page}:`)) continue;
      for (const owner of owners) {
        const kept = (owner.getAttribute("aria-owns") ?? "")
          .split(" ")
          .filter((id) => id !== "" && !id.startsWith(prefix));
        if (kept.length === 0) owner.removeAttribute("aria-owns");
        else owner.setAttribute("aria-owns", kept.join(" "));
      }
    }
  }

  /** Forgets the tree (a new document). */
  reset(): void {
    this.#tree?.remove();
    this.#tree = undefined;
    this.#owners = new Map();
  }

  /** `node` as an element; `parent` is its parent's structure type (a caption needs a table or figure). */
  #element(node: StructureNode, parent: string | undefined): HTMLElement {
    const el = document.createElement("div");
    const heading = /^H([1-6])$/.exec(node.kind);
    const alt = node.alt !== undefined && node.alt !== "" ? node.alt : undefined;
    const role = heading !== null ? "heading" : roleOf(node.kind, parent, alt);
    if (role !== undefined) el.setAttribute("role", role);
    if (heading !== null) el.setAttribute("aria-level", heading[1] as string);
    if (node.kind === "Title") el.setAttribute("aria-level", "1");
    if (alt !== undefined) el.setAttribute("aria-label", alt);
    if (node.lang !== undefined && node.lang !== "") el.lang = node.lang;
    for (const kid of node.kids) this.#kid(el, kid, node.kind);
    return el;
  }

  #kid(parent: HTMLElement, kid: StructureKid, kind: string): void {
    if (kid.type === "node") {
      parent.append(this.#element(kid, kind));
      return;
    }
    const key = `${kid.page}:${kid.mcid}`;
    const owners = this.#owners.get(key) ?? [];
    owners.push(parent);
    this.#owners.set(key, owners);
  }
}

/**
 * The role of a structure type that ROLES doesn't settle alone: a figure is an image only with
 * alternate text to name it (else a group), and a caption only inside a table or a figure.
 */
function roleOf(kind: string, parent: string | undefined, alt: string | undefined): string | undefined {
  if (kind === "Figure") return alt === undefined ? "group" : "img";
  if (kind === "Caption") return parent === "Table" || parent === "Figure" ? "caption" : undefined;
  return ROLES[kind];
}
