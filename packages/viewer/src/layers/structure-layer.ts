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
  L: "list",
  LI: "listitem",
  Table: "table",
  TR: "row",
  TH: "columnheader",
  TD: "cell",
  Figure: "img",
  Formula: "math",
  Caption: "caption",
  BlockQuote: "blockquote",
  Link: "link",
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

  constructor(root: HTMLElement, untaggedLabel: string) {
    this.#root = root;
    this.#untagged = untaggedLabel;
  }

  /** Builds the tree from the worker's answer, or the one-time note for an untagged document. */
  show(structure: StructureData): void {
    this.reset();
    const tree = document.createElement("div");
    tree.className = "lamina-structure";
    Object.assign(tree.style, HIDDEN);
    if (structure.tree === undefined || structure.tree.length === 0) {
      tree.setAttribute("role", "status");
      tree.textContent = this.#untagged;
    } else {
      for (const node of structure.tree) tree.append(this.#element(node));
    }
    this.#root.prepend(tree);
    this.#tree = tree;
  }

  /** Span `id` drew marked content `mcid` of `page`: its element owns it. */
  link(page: number, mcid: number, id: string): void {
    for (const owner of this.#owners.get(`${page}:${mcid}`) ?? []) {
      const owned = (owner.getAttribute("aria-owns") ?? "").split(" ").filter(Boolean);
      if (!owned.includes(id)) owner.setAttribute("aria-owns", [...owned, id].join(" "));
    }
  }

  /** Page `page` unmounted: its spans leave the elements that owned them. */
  unlink(page: number): void {
    const prefix = `lamina-text-${page}-`;
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

  remove(): void {
    this.reset();
  }

  #element(node: StructureNode): HTMLElement {
    const el = document.createElement("div");
    const heading = /^H([1-6])$/.exec(node.kind);
    const role = heading === null ? ROLES[node.kind] : "heading";
    if (role !== undefined) el.setAttribute("role", role);
    if (heading !== null) el.setAttribute("aria-level", heading[1] as string);
    if (node.alt !== undefined && node.alt !== "") el.setAttribute("aria-label", node.alt);
    if (node.lang !== undefined && node.lang !== "") el.lang = node.lang;
    for (const kid of node.kids) this.#kid(el, kid);
    return el;
  }

  #kid(parent: HTMLElement, kid: StructureKid): void {
    if (kid.type === "node") {
      parent.append(this.#element(kid));
      return;
    }
    const key = `${kid.page}:${kid.mcid}`;
    const owners = this.#owners.get(key) ?? [];
    owners.push(parent);
    this.#owners.set(key, owners);
  }
}
