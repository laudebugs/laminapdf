// Document: open, read, edit with ops (transactions, undo), save. A thin layer over
// WasmDocument (ADR 0028 F12): every call is synchronous and throws LaminaError.
import type { WasmDocument, WasmPage, WasmSaveJob } from "@lamina/wasm/base";
import { guard, isLaminaError, type LaminaWarning, packageError } from "./errors.js";
import { baseModule } from "./init.js";
import { type RangeOptions, type RangeSource, type RangeState, settle } from "./source/range.js";
import { Transaction } from "./transaction.js";
import type {
  DocumentInfo,
  Edited,
  Op,
  OpenOptions,
  OpLog,
  PageInfo,
  Progress,
  SaveOptions,
  SaveReport,
  UndoOutcome,
} from "./types.js";

function isThenable(value: unknown): boolean {
  return (
    (typeof value === "object" || typeof value === "function") &&
    value !== null &&
    typeof (value as { then?: unknown }).then === "function"
  );
}

/** How many warnings one cursor call fetches. */
const WARNING_BATCH = 256;

function bytesOf(input: Uint8Array | ArrayBuffer): Uint8Array {
  return input instanceof Uint8Array ? input : new Uint8Array(input);
}

/** A page of a {@link Document}, held by its ID: moving pages doesn't retarget it (ADR 0028 F2). */
export class PageRef {
  readonly #page: WasmPage;

  /** @internal */
  constructor(page: WasmPage) {
    this.#page = page;
  }

  /** The page's stable ID (`p12.0`). */
  get id(): string {
    return this.#page.id;
  }

  /** The page's current 0-based index. @throws {@link LaminaError} `unknown-page` when it is gone. */
  get index(): number {
    return guard(() => this.#page.index());
  }

  /** The page's facts now. @throws {@link LaminaError} `unknown-page` when it is gone. */
  get info(): PageInfo {
    return guard(() => this.#page.info());
  }
}

/** A save in progress, stepped by the caller with a budget of work units (ADR 0028 §3). */
export class SaveJob {
  readonly #job: WasmSaveJob;

  /** @internal */
  constructor(job: WasmSaveJob) {
    this.#job = job;
  }

  /**
   * Runs at most `budget` work units.
   *
   * @returns `{ state: "progress", … }`, or `{ state: "done", bytes, report }` at the end.
   * @throws {@link LaminaError} `cancelled`, `document-changed`, `job-finished`, or the save's errors.
   */
  step(
    budget: number,
  ):
    | ({ readonly state: "progress" } & Progress)
    | { readonly state: "done"; readonly bytes: Uint8Array; readonly report: SaveReport } {
    return guard(() => this.#job.step(budget));
  }

  /** Stops the job: its next step fails with `cancelled`. */
  cancel(): void {
    this.#job.cancel();
  }

  /** Frees the job's memory now (otherwise when it is garbage-collected). */
  free(): void {
    this.#job.free();
  }
}

/**
 * An open PDF: read it, edit it with ops (atomic transactions with undo), save it.
 *
 * @example
 * ```ts
 * await init();
 * const doc = Document.open(bytes);
 * doc.transact((tx) => tx.setInfo({ title: "Final" }));
 * const updated = doc.save({ mode: "incremental" });
 * doc.free();
 * ```
 */
export class Document {
  readonly #doc: WasmDocument;
  /** Set for a document opened with {@link Document.openRange}: where more bytes come from. */
  readonly #range: RangeState | undefined;
  #freed = false;
  /** Whether a `transact` callback of this document is running. */
  #transacting = false;

  /** @internal Wraps a raw document (from `open`, `openRange` or `Builder.finish`). */
  constructor(doc: WasmDocument, range?: RangeState) {
    this.#doc = doc;
    this.#range = range;
  }

  /**
   * Opens a document whose bytes are fetched range by range ({@link httpRangeSource},
   * {@link callbackSource}): only what the open needs is fetched (the end of the file, the xref,
   * the catalog). Calls on the document stay synchronous; one that needs bytes not fetched yet
   * throws `data-pending` with `ranges`, so wrap later reads in {@link Document.ensure}.
   *
   * @param source - the file (or a promise of it, as `httpRangeSource` returns).
   * @param options - open options (password, strict, limits) and fetch options.
   * @returns the document, once open has everything it needs.
   * @throws {@link LaminaError} as {@link Document.open}; `data-pending` when fetching made no
   *   progress or `maxRounds` ran out; `source-budget-exceeded`; the source's own errors
   *   (`fetch-failed`, `source-changed`); the signal's reason when aborted.
   */
  static async openRange(
    source: RangeSource | Promise<RangeSource>,
    options: OpenOptions & RangeOptions = {},
  ): Promise<Document> {
    // The source first: a rejected source promise must not go unhandled behind not-initialized.
    const src = await source;
    const wasm = baseModule();
    const sparse = guard(() => new wasm.WasmSparseSource(src.size, options.budget));
    const { signal: _s, blockSize: _b, readahead: _r, maxRounds: _m, budget: _g, ...open } = options;
    const state: RangeState = { sparse, source: src, options };
    const openWith = (opts: OpenOptions) =>
      settle(
        state,
        () =>
          new Document(
            guard(() => wasm.WasmDocument.openSource(sparse, opts)),
            state,
          ),
      );
    const password = open.password;
    const nfkc = password?.normalize("NFKC");
    try {
      if (password === undefined || nfkc === password) return await openWith(open);
      try {
        return await openWith(open);
      } catch (e) {
        if (!isLaminaError(e, "wrong-password")) throw e;
        return await openWith({ ...open, password: nfkc ?? password });
      }
    } catch (e) {
      // Nothing holds the fetched bytes now: free them at once rather than at collection.
      sparse.free();
      throw e;
    }
  }

  /**
   * Runs a synchronous call on this document, fetching what it needs first: for a document
   * from {@link Document.openRange}, a `data-pending` makes it fetch the ranges asked for and
   * call `f` again. For other documents it just returns `f()`.
   *
   * @example
   * ```ts
   * const doc = await Document.openRange(httpRangeSource(url));
   * const pages = await doc.ensure(() => doc.pages);
   * ```
   * @param f - the call (it may run several times; keep it free of side effects).
   * @returns what `f` returns.
   * @throws what `f` throws, and as {@link Document.openRange}.
   */
  async ensure<T>(f: () => T): Promise<T> {
    return this.#range === undefined ? f() : settle(this.#range, f);
  }

  /**
   * Opens a document from its bytes (the bytes are copied into the WASM memory).
   *
   * The password is tried as given. When that fails with `wrong-password` and its NFKC form
   * differs, the NFKC form is tried too: R5/R6 passwords are prepared with SASLprep, whose
   * normalisation the WASM modules leave out (ADR 0016 §7), while R2–R4 passwords must reach
   * the core unchanged.
   *
   * @param bytes - the PDF file.
   * @param options - password, strict mode, limits.
   * @returns the document.
   * @throws {@link LaminaError} `not-pdf`, `password-required`, `wrong-password`, `limit-exceeded`,
   *   `strict`, …; `unknown-option` / `invalid-argument` for bad options; `not-initialized`.
   */
  static open(bytes: Uint8Array | ArrayBuffer, options?: OpenOptions): Document {
    const wasm = baseModule();
    const data = bytesOf(bytes);
    const open = (opts: OpenOptions | undefined) => new Document(guard(() => wasm.WasmDocument.openBytes(data, opts)));
    const password = options?.password;
    const nfkc = password?.normalize("NFKC");
    if (password === undefined || nfkc === password) return open(options);
    try {
      return open(options);
    } catch (e) {
      if (!isLaminaError(e, "wrong-password")) throw e;
      return open({ ...options, password: nfkc ?? password });
    }
  }

  /** The number of pages. @throws {@link LaminaError} reading errors. */
  get pageCount(): number {
    return guard(() => this.#doc.pageCount());
  }

  /** Every page's facts, in order (one page-tree walk). @throws {@link LaminaError} reading errors. */
  get pages(): readonly PageInfo[] {
    return guard(() => this.#doc.pageInfos());
  }

  /**
   * Page `index`'s facts.
   *
   * @throws {@link LaminaError} `page-index-out-of-range`.
   */
  pageInfo(index: number): PageInfo {
    return guard(() => this.#doc.pageInfo(index));
  }

  /**
   * A handle on page `index` that follows the page by its ID.
   *
   * @throws {@link LaminaError} `page-index-out-of-range`.
   */
  page(index: number): PageRef {
    return new PageRef(guard(() => this.#doc.page(index)));
  }

  /**
   * A handle on the page with `id` (`p12.0`).
   *
   * @throws {@link LaminaError} `invalid-argument`, `unknown-page`.
   */
  pageById(id: string): PageRef {
    return new PageRef(guard(() => this.#doc.pageById(id)));
  }

  /** The Info dictionary's text entries (document text). @throws {@link LaminaError} reading errors. */
  get info(): DocumentInfo {
    return guard(() => this.#doc.info());
  }

  /** Every warning so far (build warnings first), each the ADR 0026 §1 object. */
  get warnings(): readonly LaminaWarning[] {
    const cursor = this.#doc.warnings();
    try {
      const out: LaminaWarning[] = [];
      for (;;) {
        const batch = cursor.nextBatch(WARNING_BATCH) as LaminaWarning[];
        if (batch.length === 0) return out;
        out.push(...batch);
      }
    } finally {
      cursor.free();
    }
  }

  /** Whether there are changes no save has written yet. */
  get hasUnsavedChanges(): boolean {
    return this.#doc.hasUnsavedChanges();
  }

  /**
   * Applies `ops` in order as one transaction: all of them, or none.
   *
   * @returns the pages the edit touched.
   * @throws {@link LaminaError} `unknown-option` / `invalid-argument` for a value that isn't an op;
   *   the first failing op's error (`unknown-page`, …), with nothing applied.
   */
  apply(ops: readonly Op[]): Edited {
    this.#notInTransaction("apply");
    return guard(() => this.#doc.apply(ops as Op[]));
  }

  /**
   * Applies an op log (`{ v: 1, ops }`, as JSON text or an object) as one transaction.
   *
   * @throws {@link LaminaError} `invalid-op-log`, `unsupported-version`; as {@link Document.apply}.
   */
  applyLog(log: string | OpLog): Edited {
    this.#notInTransaction("applyLog");
    const json = typeof log === "string" ? log : JSON.stringify(log);
    return guard(() => this.#doc.applyLog(json));
  }

  /**
   * Runs `edit` with a transaction handle and applies what it collected as one transaction
   * (ADR 0006 §2). When `edit` throws, nothing is applied and the error propagates. The ops are
   * applied when `edit` returns: reads inside `edit` see the document before the transaction.
   *
   * @param edit - collects ops on `tx`; synchronous only (a callback returning a promise is
   *   refused with `invalid-argument` and nothing is applied).
   * @returns what `edit` returned.
   * @throws {@link LaminaError} `nested-transaction` when called (or when `apply`, `undo`, `redo`,
   *   `save`… are called) inside a `transact` of the same document; `invalid-argument` for an
   *   async callback; as {@link Document.apply}.
   * @example
   * ```ts
   * doc.transact((tx) => {
   *   tx.setInfo({ title: "Report" });
   *   tx.rotatePage(doc.page(0).id, 90);
   * });
   * ```
   */
  transact<T>(edit: (tx: Transaction) => T): T {
    this.#notInTransaction("transact");
    this.#transacting = true;
    const tx = new Transaction();
    let result: T;
    try {
      result = edit(tx);
    } finally {
      this.#transacting = false;
      tx.close();
    }
    if (isThenable(result)) {
      // The ops of an async callback would be applied before its awaits finish: refuse it.
      throw packageError("invalid-argument", "transact takes a synchronous callback; it returned a promise", {
        data: { argument: "edit", detail: "the callback returned a promise; nothing was applied" },
      });
    }
    if (tx.ops.length > 0) this.apply(tx.ops);
    return result;
  }

  /** Throws `nested-transaction` while a `transact` callback of this document runs. */
  #notInTransaction(call: string): void {
    if (this.#transacting) {
      throw packageError("nested-transaction", `${call} was called inside a transaction of the same document`);
    }
  }

  /**
   * Undoes the newest transaction.
   *
   * @returns how it went, or `undefined` when there is nothing to undo.
   * @throws {@link LaminaError} `saved-not-rebased`; reading errors.
   */
  undo(): UndoOutcome | undefined {
    this.#notInTransaction("undo");
    return guard(() => this.#doc.undo());
  }

  /**
   * Redoes the newest undone transaction.
   *
   * @returns how it went, or `undefined` when there is nothing to redo.
   * @throws {@link LaminaError} as {@link Document.undo}.
   */
  redo(): UndoOutcome | undefined {
    this.#notInTransaction("redo");
    return guard(() => this.#doc.redo());
  }

  /**
   * Saves and rebases onto the output (later edits and saves build on it).
   *
   * @param options - `mode` (`auto`, `incremental`, `fullRewrite`), overrides, output form.
   * @returns the file's bytes.
   * @throws {@link LaminaError} `would-invalidate-signatures`, `requires-rewrite`, `encrypted-base`, …
   */
  save(options?: SaveOptions): Uint8Array {
    return this.saveWithReport(options).bytes;
  }

  /**
   * As {@link Document.save}, with the save's report (mode used, bytes written, renumbering, …).
   *
   * @throws {@link LaminaError} as {@link Document.save}.
   */
  saveWithReport(options?: SaveOptions): { readonly bytes: Uint8Array; readonly report: SaveReport } {
    this.#notInTransaction("save");
    return guard(() => this.#doc.save(options));
  }

  /**
   * A save as a job stepped by the caller (to keep a UI responsive).
   *
   * @throws {@link LaminaError} `unknown-option` / `invalid-argument` for bad options.
   */
  saveJob(options?: SaveOptions): SaveJob {
    this.#notInTransaction("saveJob");
    return new SaveJob(guard(() => this.#doc.saveJob(options)));
  }

  /**
   * Rebases onto `bytes`, the output of a save whose rebase failed (`not-rebased`).
   *
   * @throws {@link LaminaError} `no-pending-save`; reading errors in `bytes`.
   */
  rebase(bytes: Uint8Array): void {
    this.#notInTransaction("rebase");
    guard(() => this.#doc.rebase(bytes));
  }

  /**
   * Frees the document's WASM memory now; without it, it is freed when garbage-collected.
   * Calling it twice is harmless; using the document afterwards is not (L2b adds `disposed`).
   */
  free(): void {
    if (this.#freed) return;
    this.#freed = true;
    this.#doc.free();
    // A range document's fetched bytes live in its sparse source.
    this.#range?.sparse.free();
  }

  /** Frees the document at the end of a `using` block. */
  [Symbol.dispose](): void {
    this.free();
  }
}
