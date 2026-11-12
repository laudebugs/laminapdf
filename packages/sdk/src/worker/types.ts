// The remote handles' types: each served member of the sync class becomes an async method
// of the same name. Methods keep their parameters (plus `CallOptions`) and return a Promise of
// their result; getters become methods with no parameters. A `PageRef` result is a RemotePage.
import type { Document, PageRef } from "../document.js";
import type { ExtractFormat, ExtractOptions, ExtractResult } from "../extract.js";
import type { Progress } from "../types.js";
import type { RemotableDocumentMember, RemotablePageMember } from "./support.js";

/**
 * Per-call options of the worker client.
 *
 * @since 0.2.0
 */
export interface CallOptions {
  /** Aborting it rejects the call (`cancelled`) and stops it in the worker (see the README). */
  readonly signal?: AbortSignal;
  /** Progress of a stepped call (`save`), sent as events, at most every 50 ms. */
  readonly onProgress?: (progress: Progress) => void;
  /**
   * Transfer the input buffer to the worker instead of copying it: faster for large files, but
   * the caller's buffer is detached (empty) afterwards. Default `false` (copy). Only a buffer
   * the input covers whole moves: a subarray or a `SharedArrayBuffer` view is copied.
   */
  readonly transfer?: boolean;
}

/**
 * Options of an extension call (`client.request`, for packages that extend the worker, such as
 * `@laminapdf/viewer`): the worker runs the highest `priority` first, and runs calls with the same
 * `key` once (each caller still gets its own answer).
 *
 * @since 0.2.0
 */
export interface RequestOptions extends CallOptions {
  readonly priority?: number;
  readonly key?: string;
}

/**
 * `R` as the page receives it.
 *
 * @since 0.2.0
 */
export type RemoteResult<R> = R extends PageRef ? RemotePage : R;

/**
 * A member of `T` as a remote method.
 *
 * @since 0.2.0
 */
export type RemoteMember<T> = T extends (...args: infer A) => infer R
  ? (...args: [...A, options?: CallOptions]) => Promise<RemoteResult<R>>
  : (options?: CallOptions) => Promise<RemoteResult<T>>;

/**
 * A document in the worker. Close it (or `await using`) to free it there.
 *
 * @since 0.2.0
 */
export type RemoteDocument = {
  readonly [K in Exclude<RemotableDocumentMember, "extract">]: RemoteMember<Document[K]>;
} & {
  /**
   * {@link Document.extract} in the worker, typed by `format` like the sync one (a mapped type
   * would lose the generic). It runs to the end: an aborted `signal` rejects the call, and the
   * worker finishes the extraction before its next call.
   *
   * @since 0.2.0
   */
  extract<F extends ExtractFormat>(options: ExtractOptions<F>, call?: CallOptions): Promise<ExtractResult[F]>;
  /**
   * Frees the document in the worker; a second call, or one after the worker stopped, does nothing.
   *
   * @since 0.2.0
   */
  close(): Promise<void>;
  /**
   * `close()`, for `await using`.
   *
   * @since 0.2.0
   */
  [Symbol.asyncDispose](): Promise<void>;
};

/**
 * A page of a document in the worker (freed with its document).
 *
 * @since 0.2.0
 */
export type RemotePage = { readonly [K in RemotablePageMember]: RemoteMember<PageRef[K]> };
