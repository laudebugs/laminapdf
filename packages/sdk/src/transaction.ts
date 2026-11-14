// The handle `Document.transact` passes to its callback (ADR 0006 §2): a closure can't cross the
// WASM boundary, so the handle collects ops and the document applies them as one transaction.
import { packageError } from "./errors.js";
import type {
  DocumentInfo,
  InitialViewPatch,
  MetadataKey,
  MetadataPatch,
  Op,
  PageLabelRange,
  XmpValue,
} from "./types.js";

/**
 * The rotations `/Rotate` allows.
 *
 * @since 0.2.0
 */
export type Rotation = 0 | 90 | 180 | 270;

/**
 * An Info patch: a string sets the entry, `null` removes it, a missing key leaves it alone.
 *
 * @since 0.2.0
 */
export type InfoPatch = { readonly [K in keyof DocumentInfo]?: string | null };

/**
 * Options for `setMetadata`.
 *
 * @since 0.2.0
 */
export interface MetadataEditOptions {
  /** The modification date to record, a PDF date from your clock (`D:20261110093000Z`). */
  readonly at?: string;
  /** Replace XMP metadata that can't be read, losing what it held (otherwise `malformed-xmp`). */
  readonly replaceMalformed?: boolean;
}

/**
 * Options for `removeMetadata`.
 *
 * @since 0.2.0
 */
export interface MetadataRemoveOptions {
  /** As {@link MetadataEditOptions.replaceMalformed}. */
  readonly replaceMalformed?: boolean;
}

/** @internal The `setInitialView` op. */
export const setInitialViewOp = (patch: InitialViewPatch): Op => ({ op: "setInitialView", args: patch }) as Op;

/**
 * Options for the XMP property edits, `setXmpProperty` and `removeXmpProperty`.
 *
 * @since 0.3.0
 */
export interface XmpEditOptions {
  /** As {@link MetadataEditOptions.replaceMalformed}. */
  readonly replaceMalformed?: boolean;
}

/** @internal The `setXmpProperty` op. */
export const setXmpPropertyOp = (
  namespace: string,
  prefix: string,
  name: string,
  value: XmpValue,
  options?: XmpEditOptions,
): Op => ({ op: "setXmpProperty", args: { namespace, prefix, name, value, ...options } }) as Op;

/** @internal The `removeXmpProperty` op. */
export const removeXmpPropertyOp = (namespace: string, name: string, options?: XmpEditOptions): Op =>
  ({ op: "removeXmpProperty", args: { namespace, name, ...options } }) as Op;

/** @internal The `replaceXmpPacket` op. */
export const replaceXmpPacketOp = (xml: string): Op => ({ op: "replaceXmpPacket", args: { xml } }) as Op;

/**
 * Where pages go: before the first page, after the last, or before or after a page named by its ID.
 *
 * @since 0.3.0
 */
export type PagePosition = "start" | "end" | { readonly before: string } | { readonly after: string };

/**
 * A page size in points (1/72 inch): `{ width: 612, height: 792 }` is US Letter.
 *
 * @since 0.3.0
 */
export interface PageSize {
  readonly width: number;
  readonly height: number;
}

/**
 * What deleting pages does to the outline items, links, named destinations and open action
 * that point at them: `retarget` points each at the nearest page left (the next one, else the
 * one before), showing the whole page; `remove` removes it.
 *
 * @since 0.3.0
 */
export type ReferencePolicy = "retarget" | "remove";

/**
 * How `rotatePages` uses its angle: `set` shows each page at the angle; `turn` adds it to each
 * page's own rotation, as rotate left and rotate right do.
 *
 * @since 0.3.0
 */
export type RotateMode = "set" | "turn";

/**
 * Options for `insertBlankPages`.
 *
 * @since 0.3.0
 */
export interface InsertBlankPagesOptions {
  /** The new pages' rotation in degrees clockwise; 0 when left out. */
  readonly rotate?: Rotation;
}

/**
 * Options for `deletePages`.
 *
 * @since 0.3.0
 */
export interface DeletePagesOptions {
  /** What happens to what points at the deleted pages; `retarget` when left out. */
  readonly references?: ReferencePolicy;
}

/**
 * Options for `rotatePages`.
 *
 * @since 0.3.0
 */
export interface RotatePagesOptions {
  /** Whether the angle is set or added to each page's rotation; `set` when left out. */
  readonly mode?: RotateMode;
}

/**
 * A page box in the page's own user space: `[x0, y0, x1, y1]`, in PDF points from the
 * bottom-left of the page as drawn, before any rotation, corners in either order.
 *
 * @since 0.4.0
 */
export type PageBox = readonly [number, number, number, number];

/**
 * The boxes `setPageBoxes` writes. Each box left out stays as it is; `null` removes it, so
 * the page uses the box it inherits or the default (the media box for the crop box, the crop
 * box for the others). The media box can be set but never removed.
 *
 * @since 0.4.0
 */
export interface PageBoxes {
  /** The page's physical size. */
  readonly media?: PageBox;
  /** What a viewer shows and prints. */
  readonly crop?: PageBox | null;
  /** Where a print shop's bleed ends. */
  readonly bleed?: PageBox | null;
  /** The finished page after trimming. */
  readonly trim?: PageBox | null;
  /** The meaningful content of the page. */
  readonly art?: PageBox | null;
}

/**
 * Margins `cropPages` takes off, in points of the page as it shows (rotation and user unit
 * applied), so one set crops pages of any size or rotation by the same visible amount. Each side
 * left out is 0.
 *
 * @since 0.4.0
 */
export interface CropMargins {
  /** Points off the top edge as the page shows. */
  readonly top?: number;
  /** Points off the right edge as the page shows. */
  readonly right?: number;
  /** Points off the bottom edge as the page shows. */
  readonly bottom?: number;
  /** Points off the left edge as the page shows. */
  readonly left?: number;
}

/**
 * Where `resizePages` puts the old page on the new one when it extends it, on the page as shown.
 *
 * @since 0.4.0
 */
export type Anchor =
  | "topLeft"
  | "top"
  | "topRight"
  | "left"
  | "center"
  | "right"
  | "bottomLeft"
  | "bottom"
  | "bottomRight";

/**
 * Options for `resizePages`.
 *
 * @since 0.4.0
 */
export interface ResizePagesOptions {
  /**
   * `"scale"` (the default) scales the page as shown to fit the new paper, keeping its
   * proportions, and centres it; `"extend"` keeps the content's size and places it at `anchor`.
   */
  readonly mode?: "scale" | "extend";
  /** Where `"extend"` puts the old page; `"center"` when left out. */
  readonly anchor?: Anchor;
}

/**
 * How a page is shown: taller than wide, or wider than tall.
 *
 * @since 0.4.0
 */
export type PageOrientation = "portrait" | "landscape";

/**
 * How `setOrientation` turns a page: `"rotate"` (the default) adds a quarter turn to `/Rotate`;
 * `"swapBox"` swaps the paper's sides and turns the content into it, for tools that ignore
 * `/Rotate`.
 *
 * @since 0.4.0
 */
export type OrientationMethod = "rotate" | "swapBox";

/** @internal The `insertBlankPages` op. */
export const insertBlankPagesOp = (
  at: PagePosition,
  count: number,
  size: PageSize,
  options?: InsertBlankPagesOptions,
): Op =>
  ({
    op: "insertBlankPages",
    args: { at, count, size: { width: size.width, height: size.height }, rotate: options?.rotate ?? 0 },
  }) as Op;

/** @internal The `deletePages` op. */
export const deletePagesOp = (pages: readonly string[], options?: DeletePagesOptions): Op =>
  ({
    op: "deletePages",
    args: options?.references === undefined ? { pages } : { pages, references: options.references },
  }) as Op;

/** @internal The `movePages` op. */
export const movePagesOp = (pages: readonly string[], to: PagePosition): Op =>
  ({ op: "movePages", args: { pages, to } }) as Op;

/** @internal The `rotatePages` op. */
export const rotatePagesOp = (pages: readonly string[], angle: Rotation, options?: RotatePagesOptions): Op =>
  ({
    op: "rotatePages",
    args: options?.mode === undefined ? { pages, angle } : { pages, angle, mode: options.mode },
  }) as Op;

/** @internal The `reversePages` op. */
export const reversePagesOp = (pages: readonly string[]): Op => ({ op: "reversePages", args: { pages } }) as Op;

/** @internal The `setPageBoxes` op. */
export const setPageBoxesOp = (pages: readonly string[], boxes: PageBoxes): Op =>
  ({ op: "setPageBoxes", args: { pages, boxes } }) as Op;

/** @internal The `cropPages` op. */
export const cropPagesOp = (pages: readonly string[], margins: CropMargins): Op =>
  ({ op: "cropPages", args: { pages, margins } }) as Op;

/** @internal The `resizePages` op. */
export const resizePagesOp = (pages: readonly string[], size: PageSize, options?: ResizePagesOptions): Op =>
  ({
    op: "resizePages",
    args: {
      pages,
      size,
      mode: options?.mode === "extend" ? { extend: { anchor: options.anchor ?? "center" } } : "scale",
    },
  }) as Op;

/** @internal The `setOrientation` op. */
export const setOrientationOp = (
  pages: readonly string[],
  orientation: PageOrientation,
  method: OrientationMethod = "rotate",
): Op => ({ op: "setOrientation", args: { pages, orientation, method } }) as Op;

/** @internal The `duplicatePages` op. */
export const duplicatePagesOp = (pages: readonly string[], at: PagePosition): Op =>
  ({ op: "duplicatePages", args: { pages, at } }) as Op;

/**
 * What comes along with pages copied in from another document: the annotations other than form
 * fields (`annotations`, `true` when left out), and what happens to a form field whose name the
 * document already uses (`fields`): `"rename"` (the default) gives it the next free `<name>#<n>`,
 * `"link"` makes it share the value of the field of that name when both are of one type and value.
 *
 * @since 0.3.0
 */
export interface Carry {
  readonly annotations?: boolean;
  readonly fields?: "rename" | "link";
}

/**
 * A source document {@link Document.addSource} added: its ID and its page IDs in order.
 *
 * @since 0.3.0
 */
export interface AddedSource {
  readonly id: string;
  readonly pages: readonly string[];
}

/** @internal The `insertPagesFrom` op. */
export const insertPagesFromOp = (source: string, pages: readonly string[], at: PagePosition, carry?: Carry): Op =>
  ({ op: "insertPagesFrom", args: { source, pages, at, ...(carry ? { carry } : {}) } }) as Op;

/**
 * One document {@link Document.mergeDocuments} brings in: a source document's ID (from
 * {@link Document.addSource}), the pages to bring in (all of them when left out), and the name of
 * the bookmark that holds its bookmarks (its own title when left out).
 *
 * @since 0.3.0
 */
export interface MergeSource {
  readonly source: string;
  readonly pages?: readonly string[];
  readonly title?: string;
}

/**
 * How a merge brings in what the documents hold besides their pages. `outline` says what becomes
 * of their bookmarks: `"perFile"` (the default) puts each document's bookmarks under a new
 * bookmark named after it, `"keep"` adds them at the top level, and `"none"` leaves them out.
 * `fields` is as {@link Carry}'s.
 *
 * @since 0.3.0
 */
export interface MergeOptions {
  readonly outline?: "perFile" | "keep" | "none";
  readonly fields?: "rename" | "link";
}

/** @internal The `mergeDocuments` op. */
export const mergeDocumentsOp = (sources: readonly MergeSource[], at: PagePosition, options?: MergeOptions): Op =>
  ({ op: "mergeDocuments", args: { sources, at, ...(options ? { options } : {}) } }) as Op;

/** @internal The `replacePages` op. */
export const replacePagesOp = (
  targets: readonly string[],
  source: string,
  pages: readonly string[],
  carry?: Carry,
): Op => ({ op: "replacePages", args: { targets, source, pages, ...(carry ? { carry } : {}) } }) as Op;

/** @internal The `setMetadata` op. */
export const setMetadataOp = (patch: MetadataPatch, options?: MetadataEditOptions): Op =>
  ({ op: "setMetadata", args: { patch, ...options } }) as Op;

/** @internal The `setPageLabels` op. */
export const setPageLabelsOp = (ranges: readonly PageLabelRange[]): Op =>
  ({ op: "setPageLabels", args: { ranges } }) as Op;

/** @internal The `removeMetadata` op. */
export const removeMetadataOp = (keys: readonly MetadataKey[], options?: MetadataRemoveOptions): Op =>
  ({ op: "removeMetadata", args: { keys, ...options } }) as Op;

/**
 * Collects the ops of one {@link Document.transact} call.
 *
 * @since 0.2.0
 */
export class Transaction {
  readonly #ops: Op[] = [];
  #open = true;

  /**
   * The ops collected so far.
   *
   * @since 0.2.0
   */
  get ops(): readonly Op[] {
    return this.#ops;
  }

  /** @internal Ends the transaction: the handle takes no more ops. */
  close(): void {
    this.#open = false;
  }

  /**
   * Adds `op` to the transaction.
   *
   * @throws {@link LaminaPdfError} `no-transaction` after `transact` returned.
   *
   * @since 0.2.0
   */
  apply(op: Op): this {
    if (!this.#open) {
      throw packageError("no-transaction", "the transaction has ended");
    }
    this.#ops.push(op);
    return this;
  }

  /**
   * Edits the metadata in the Info dictionary and the XMP metadata together (`setMetadata`).
   *
   * @param patch - the fields to set (a value) or remove (`null`); authors are a list.
   * @param options - as {@link Document.setMetadata}: `at`, the modification date to write
   *   (`D:20261110093000Z`; omitted, none is written), and `replaceMalformed`.
   *
   * @since 0.2.0
   */
  setMetadata(patch: MetadataPatch, options?: MetadataEditOptions): this {
    return this.apply(setMetadataOp(patch, options));
  }

  /**
   * Removes metadata fields from the Info dictionary and the XMP metadata (`removeMetadata`).
   *
   * @param options - as {@link Document.removeMetadata}: `replaceMalformed`.
   *
   * @since 0.2.0
   */
  removeMetadata(keys: readonly MetadataKey[], options?: MetadataRemoveOptions): this {
    return this.apply(removeMetadataOp(keys, options));
  }

  /**
   * Sets any XMP property (`setXmpProperty`), as {@link Document.setXmpProperty}.
   *
   * @since 0.3.0
   */
  setXmpProperty(namespace: string, prefix: string, name: string, value: XmpValue, options?: XmpEditOptions): this {
    return this.apply(setXmpPropertyOp(namespace, prefix, name, value, options));
  }

  /**
   * Removes an XMP property (`removeXmpProperty`), as {@link Document.removeXmpProperty}.
   *
   * @since 0.3.0
   */
  removeXmpProperty(namespace: string, name: string, options?: XmpEditOptions): this {
    return this.apply(removeXmpPropertyOp(namespace, name, options));
  }

  /**
   * Replaces the whole XMP packet (`replaceXmpPacket`), as {@link Document.replaceXmpPacket}.
   *
   * @since 0.3.0
   */
  replaceXmpPacket(xml: string): this {
    return this.apply(replaceXmpPacketOp(xml));
  }

  /**
   * Patches the Info dictionary (`setInfo`), and the XMP metadata with it when the document has
   * some.
   *
   * @deprecated Use {@link Transaction.setMetadata}, which writes authors as a list and the
   *   dates, and edits documents whose metadata is in XMP alone.
   * @param patch - the entries to set (a string) or remove (`null`).
   * @param at - the modification date to write (`D:20261030120000Z`); omitted, none is written.
   *
   * @since 0.2.0
   */
  setInfo(patch: InfoPatch, at?: string): this {
    return this.apply({ op: "setInfo", args: at === undefined ? { patch } : { patch, at } } as Op);
  }

  /**
   * Sets a page's `/Rotate` (`rotatePage`).
   *
   * @param page - the page's ID (`doc.page(0).id`).
   * @param rotate - 0, 90, 180 or 270 degrees.
   *
   * @since 0.2.0
   */
  rotatePage(page: string, rotate: Rotation): this {
    return this.apply({ op: "rotatePage", args: { page, rotate } } as Op);
  }

  /**
   * Changes how the document opens (`setInitialView`).
   *
   * @param patch - as {@link Document.setInitialView}.
   *
   * @since 0.3.0
   */
  setInitialView(patch: InitialViewPatch): this {
    return this.apply(setInitialViewOp(patch));
  }

  /**
   * Inserts `count` blank pages of `size` at `at` (`insertBlankPages`).
   *
   * @param options - as {@link Document.insertBlankPages}: `rotate`.
   *
   * @since 0.3.0
   */
  insertBlankPages(at: PagePosition, count: number, size: PageSize, options?: InsertBlankPagesOptions): this {
    return this.apply(insertBlankPagesOp(at, count, size, options));
  }

  /**
   * Deletes `pages`, by ID (`deletePages`).
   *
   * @param options - as {@link Document.deletePages}: `references`.
   *
   * @since 0.3.0
   */
  deletePages(pages: readonly string[], options?: DeletePagesOptions): this {
    return this.apply(deletePagesOp(pages, options));
  }

  /**
   * Moves `pages`, by ID and in the order given, to `to` (`movePages`).
   *
   * @since 0.3.0
   */
  movePages(pages: readonly string[], to: PagePosition): this {
    return this.apply(movePagesOp(pages, to));
  }

  /**
   * Rotates `pages`, by ID, by `angle` degrees clockwise (`rotatePages`).
   *
   * @param options - as {@link Document.rotatePages}: `mode`.
   *
   * @since 0.3.0
   */
  rotatePages(pages: readonly string[], angle: Rotation, options?: RotatePagesOptions): this {
    return this.apply(rotatePagesOp(pages, angle, options));
  }

  /**
   * Reverses the order of `pages`, by ID, among the places they hold (`reversePages`).
   *
   * @since 0.3.0
   */
  reversePages(pages: readonly string[]): this {
    return this.apply(reversePagesOp(pages));
  }

  /**
   * Sets or removes boxes of `pages` (`setPageBoxes`), as {@link Document.setPageBoxes}.
   *
   * @since 0.4.0
   */
  setPageBoxes(pages: readonly string[], boxes: PageBoxes): this {
    return this.apply(setPageBoxesOp(pages, boxes));
  }

  /**
   * Crops `pages` by `margins` of each page as it shows (`cropPages`), as
   * {@link Document.cropPages}.
   *
   * @since 0.4.0
   */
  cropPages(pages: readonly string[], margins: CropMargins): this {
    return this.apply(cropPagesOp(pages, margins));
  }

  /**
   * Gives `pages` new paper of `size` (`resizePages`), as {@link Document.resizePages}.
   *
   * @since 0.4.0
   */
  resizePages(pages: readonly string[], size: PageSize, options?: ResizePagesOptions): this {
    return this.apply(resizePagesOp(pages, size, options));
  }

  /**
   * Shows `pages` in `orientation` (`setOrientation`), as {@link Document.setOrientation}.
   *
   * @since 0.4.0
   */
  setOrientation(pages: readonly string[], orientation: PageOrientation, method?: OrientationMethod): this {
    return this.apply(setOrientationOp(pages, orientation, method));
  }

  /**
   * Copies `pages`, by ID and in the order given, to `at` (`duplicatePages`), as
   * {@link Document.duplicatePages}.
   *
   * @since 0.3.0
   */
  duplicatePages(pages: readonly string[], at: PagePosition): this {
    return this.apply(duplicatePagesOp(pages, at));
  }

  /**
   * Copies `pages` of the source document `source` to `at` (`insertPagesFrom`), as
   * {@link Document.insertPagesFrom}.
   *
   * @since 0.3.0
   */
  insertPagesFrom(source: string, pages: readonly string[], at: PagePosition, carry?: Carry): this {
    return this.apply(insertPagesFromOp(source, pages, at, carry));
  }

  /**
   * Replaces `targets` with `pages` of the source document `source` (`replacePages`), as
   * {@link Document.replacePages}.
   *
   * @since 0.3.0
   */
  replacePages(targets: readonly string[], source: string, pages: readonly string[], carry?: Carry): this {
    return this.apply(replacePagesOp(targets, source, pages, carry));
  }

  /**
   * Brings in every page of each of `sources` at `at` (`mergeDocuments`), as
   * {@link Document.mergeDocuments}.
   *
   * @since 0.3.0
   */
  mergeDocuments(sources: readonly MergeSource[], at: PagePosition, options?: MergeOptions): this {
    return this.apply(mergeDocumentsOp(sources, at, options));
  }

  /**
   * Replaces the document's page labels (`setPageLabels`), as {@link Document.setPageLabels}.
   *
   * @since 0.3.0
   */
  setPageLabels(ranges: readonly PageLabelRange[]): this {
    return this.apply(setPageLabelsOp(ranges));
  }
}
