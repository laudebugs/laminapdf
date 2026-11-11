// The handle `Document.transact` passes to its callback (ADR 0006 §2): a closure can't cross the
// WASM boundary, so the handle collects ops and the document applies them as one transaction.
import { packageError } from "./errors.js";
import type { DocumentInfo, MetadataKey, MetadataPatch, Op } from "./types.js";

/** The rotations `/Rotate` allows. */
export type Rotation = 0 | 90 | 180 | 270;

/** An Info patch: a string sets the entry, `null` removes it, a missing key leaves it alone. */
export type InfoPatch = { readonly [K in keyof DocumentInfo]?: string | null };

/** Options for `setMetadata`. */
export interface MetadataEditOptions {
  /** The modification date to record, a PDF date from your clock (`D:20261110093000Z`). */
  readonly at?: string;
  /** Replace XMP metadata that can't be read, losing what it held (otherwise `malformed-xmp`). */
  readonly replaceMalformed?: boolean;
}

/** Options for `removeMetadata`. */
export interface MetadataRemoveOptions {
  /** As {@link MetadataEditOptions.replaceMalformed}. */
  readonly replaceMalformed?: boolean;
}

/** @internal The `setMetadata` op. */
export const setMetadataOp = (patch: MetadataPatch, options: MetadataEditOptions): Op =>
  ({ op: "setMetadata", args: { patch, ...options } }) as Op;

/** @internal The `removeMetadata` op. */
export const removeMetadataOp = (keys: readonly MetadataKey[], options: MetadataRemoveOptions): Op =>
  ({ op: "removeMetadata", args: { keys: [...keys], ...options } }) as Op;

/** Collects the ops of one {@link Document.transact} call. */
export class Transaction {
  readonly #ops: Op[] = [];
  #open = true;

  /** The ops collected so far. */
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
   */
  setMetadata(patch: MetadataPatch, options: MetadataEditOptions = {}): this {
    return this.apply(setMetadataOp(patch, options));
  }

  /**
   * Removes metadata fields from the Info dictionary and the XMP metadata (`removeMetadata`).
   *
   * @param options - as {@link Document.removeMetadata}: `replaceMalformed`.
   */
  removeMetadata(keys: readonly MetadataKey[], options: MetadataRemoveOptions = {}): this {
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
   */
  setInfo(patch: InfoPatch, at?: string): this {
    return this.apply({ op: "setInfo", args: at === undefined ? { patch } : { patch, at } } as Op);
  }

  /**
   * Sets a page's `/Rotate` (`rotatePage`).
   *
   * @param page - the page's ID (`doc.page(0).id`).
   * @param rotate - 0, 90, 180 or 270 degrees.
   */
  rotatePage(page: string, rotate: Rotation): this {
    return this.apply({ op: "rotatePage", args: { page, rotate } } as Op);
  }
}
