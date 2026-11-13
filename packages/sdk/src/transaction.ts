// The handle `Document.transact` passes to its callback (ADR 0006 §2): a closure can't cross the
// WASM boundary, so the handle collects ops and the document applies them as one transaction.
import { packageError } from "./errors.js";
import type { DocumentInfo, MetadataKey, MetadataPatch, Op } from "./types.js";

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

/**
 * Where pages go: before the first page, after the last, or before or after a page named by its ID.
 *
 * @since 0.2.0
 */
export type PagePosition = "start" | "end" | { readonly before: string } | { readonly after: string };

/**
 * A page size in points (1/72 inch): `{ width: 612, height: 792 }` is US Letter.
 *
 * @since 0.2.0
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
 * @since 0.2.0
 */
export type ReferencePolicy = "retarget" | "remove";

/**
 * How `rotatePages` uses its angle: `set` shows each page at the angle; `turn` adds it to each
 * page's own rotation, as rotate left and rotate right do.
 *
 * @since 0.2.0
 */
export type RotateMode = "set" | "turn";

/**
 * Options for `insertBlankPages`.
 *
 * @since 0.2.0
 */
export interface InsertBlankPagesOptions {
  /** The new pages' rotation in degrees clockwise; 0 when left out. */
  readonly rotate?: Rotation;
}

/**
 * Options for `deletePages`.
 *
 * @since 0.2.0
 */
export interface DeletePagesOptions {
  /** What happens to what points at the deleted pages; `retarget` when left out. */
  readonly references?: ReferencePolicy;
}

/**
 * Options for `rotatePages`.
 *
 * @since 0.2.0
 */
export interface RotatePagesOptions {
  /** Whether the angle is set or added to each page's rotation; `set` when left out. */
  readonly mode?: RotateMode;
}

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

/** @internal The `setMetadata` op. */
export const setMetadataOp = (patch: MetadataPatch, options?: MetadataEditOptions): Op =>
  ({ op: "setMetadata", args: { patch, ...options } }) as Op;

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
   * Inserts `count` blank pages of `size` at `at` (`insertBlankPages`).
   *
   * @param options - as {@link Document.insertBlankPages}: `rotate`.
   *
   * @since 0.2.0
   */
  insertBlankPages(at: PagePosition, count: number, size: PageSize, options?: InsertBlankPagesOptions): this {
    return this.apply(insertBlankPagesOp(at, count, size, options));
  }

  /**
   * Deletes `pages`, by ID (`deletePages`).
   *
   * @param options - as {@link Document.deletePages}: `references`.
   *
   * @since 0.2.0
   */
  deletePages(pages: readonly string[], options?: DeletePagesOptions): this {
    return this.apply(deletePagesOp(pages, options));
  }

  /**
   * Moves `pages`, by ID and in the order given, to `to` (`movePages`).
   *
   * @since 0.2.0
   */
  movePages(pages: readonly string[], to: PagePosition): this {
    return this.apply(movePagesOp(pages, to));
  }

  /**
   * Rotates `pages`, by ID, by `angle` degrees clockwise (`rotatePages`).
   *
   * @param options - as {@link Document.rotatePages}: `mode`.
   *
   * @since 0.2.0
   */
  rotatePages(pages: readonly string[], angle: Rotation, options?: RotatePagesOptions): this {
    return this.apply(rotatePagesOp(pages, angle, options));
  }

  /**
   * Reverses the order of `pages`, by ID, among the places they hold (`reversePages`).
   *
   * @since 0.2.0
   */
  reversePages(pages: readonly string[]): this {
    return this.apply(reversePagesOp(pages));
  }
}
