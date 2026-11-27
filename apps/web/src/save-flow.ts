// Saving in the reader (E1 plan §7, Paper O02): Save, Save as…, Save as optimised and Download a
// copy, each planned first, and a copy with a change a signature refused (O06's Save a copy). A
// save that would invalidate signatures asks, naming each one, and one that breaks a standard the
// file claims asks too; the engine then writes the file through the platform, and the document
// takes it as saved only once that worked. The plan for a save now is kept for the strip under the
// top bar, read again as the history changes. Loaded with the rest of the edit session once a
// document is open (later.ts).

import type { Op, PlannedSave, SaveOptions } from "@laminapdf/sdk";
import { type ExternalStore, message, type Platform, type ToastQueue } from "@laminapdf/ui";
import { type CopyKind, type SavePlanSlice, type SaveSignature, signatureKind } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { downloadName } from "./download-name.ts";
import type { Engine } from "./session.ts";

/** Which save: in place (or as the platform can), to a new file, or rewritten whole. */
export type SaveHow = "save" | "save-as" | "optimised";

/** A question a save waits on. */
export type SaveQuestion =
  | {
      readonly kind: "signatures";
      readonly signatures: readonly SaveSignature[];
      /** Set for a copy, which leaves the open file signed: saved as a file, or downloaded. */
      readonly copy?: CopyKind;
    }
  | { readonly kind: "conformance"; readonly warnings: readonly string[] };

/** What the strip and the dialogs read. */
export interface SaveFlowState {
  /** How a save now would go, while there are unsaved changes; undefined otherwise. */
  readonly plan: SavePlanSlice | undefined;
  /** What a save waits on the user for; undefined when nothing is asked. */
  readonly asking: SaveQuestion | undefined;
  /** The last save's failure, until a save works or it is dismissed. */
  readonly failure: { readonly name: string; readonly reason: string } | undefined;
}

const IDLE: SaveFlowState = { plan: undefined, asking: undefined, failure: undefined };

/** Why a write failed, in words: the disk was full, permission was refused, … */
function failureReason(error: unknown): string {
  const name = error instanceof DOMException ? error.name : undefined;
  const text = String(error instanceof Error ? error.message : error);
  if (name === "QuotaExceededError" || /quota|disk is full|no space/i.test(text)) return message("save.reason.full");
  if (name === "NotAllowedError" || name === "SecurityError" || /permission/i.test(text))
    return message("save.reason.permission");
  if (/changed after this copy/.test(text)) return message("save.reason.changed");
  if (/would-invalidate-signatures|requires-rewrite|encrypted-base/.test(text)) return message("save.reason.refused");
  return message("save.reason.other");
}

/** The slice of a plan the strip shows; `invalidated` lists what a save with the override would break. */
function slice(plan: PlannedSave, invalidated: readonly SaveSignature[]): SavePlanSlice {
  return {
    // The WASM type says string; the planner names only these four.
    constraint: plan.constraint as SavePlanSlice["constraint"],
    signatures: plan.signatures.map(signature),
    wouldInvalidate: invalidated,
  };
}

/** A signature the planner found, as the confirmations name it. */
function signature(s: PlannedSave["signatures"][number]): SaveSignature {
  return s.field === undefined ? { kind: s.kind } : { kind: s.kind, field: s.field };
}

/** The reader's saving, for the open document `viewer` shows. */
export class SaveFlow implements ExternalStore<SaveFlowState> {
  readonly #engine: Engine;
  readonly #viewer: Viewer;
  readonly #platform: Platform;
  readonly #toasts: ToastQueue;
  /** The open file's name, as the chrome shows it. */
  readonly #fileName: () => string;
  readonly #now: () => Date;
  readonly #id: () => string;
  readonly #listeners = new Set<() => void>();
  #state = IDLE;
  #answer: ((yes: boolean) => void) | undefined;
  /** Counts plans asked for the strip: only the newest one's answer is kept. */
  #planned = 0;
  #retry: (() => void) | undefined;
  /** Flatten's "also remove the comments from the file's history": the next save writes the whole file anew. */
  #rewrite = false;
  /** How many steps were done when that was asked for: undoing the flatten that asked ends it. */
  #rewriteAt = 0;

  constructor(
    reader: {
      readonly engine: Engine;
      readonly viewer: Viewer;
      readonly platform: Platform;
      readonly toasts: ToastQueue;
      readonly fileName: () => string;
    },
    clock: { readonly now: () => Date; readonly id: () => string } = {
      now: () => new Date(),
      id: () => crypto.randomUUID(),
    },
  ) {
    this.#engine = reader.engine;
    this.#viewer = reader.viewer;
    this.#platform = reader.platform;
    this.#toasts = reader.toasts;
    this.#fileName = reader.fileName;
    this.#now = clock.now;
    this.#id = clock.id;
  }

  getState = (): SaveFlowState => this.#state;

  /**
   * Makes the next save a full rewrite, which drops the earlier versions that still hold flattened
   * comments (C07); a save that works, or `false`, ends it. A signed file asks first, as any save
   * that would invalidate a signature does.
   */
  rewriteNextSave(on: boolean): void {
    this.#rewrite = on;
    this.#rewriteAt = this.#viewer.edits.getState().history.position;
    void this.#replan(this.#viewer.edits.getState().history.unsaved > 0);
  }

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Plans again whenever the history changes, for the strip; returns the function that stops. */
  start(): () => void {
    const edits = this.#viewer.edits;
    let unsaved = -1;
    let position = -1;
    const follow = () => {
      const { history, saving } = edits.getState();
      if (saving || (history.unsaved === unsaved && history.position === position)) return;
      unsaved = history.unsaved;
      position = history.position;
      if (this.#rewrite && position < this.#rewriteAt) this.#rewrite = false;
      void this.#replan(history.unsaved > 0);
    };
    follow();
    return edits.subscribe(follow);
  }

  async #replan(changed: boolean): Promise<void> {
    const turn = ++this.#planned;
    if (!changed) {
      this.#set({ plan: undefined });
      return;
    }
    try {
      const edits = this.#viewer.edits;
      const rewrite: SaveOptions = this.#rewrite ? { mode: "fullRewrite" } : {};
      const plan = await edits.plan(rewrite);
      const invalidated =
        plan.refusal === "would-invalidate-signatures"
          ? (await edits.plan({ ...rewrite, invalidateSignatures: true })).wouldInvalidate.map(signature)
          : [];
      if (turn === this.#planned) this.#set({ plan: slice(plan, invalidated) });
    } catch {
      // No plan (the document closed, or can't be edited): the strip says nothing.
      if (turn === this.#planned) this.#set({ plan: undefined });
    }
  }

  /** Answers the question asked. */
  answer(yes: boolean): void {
    const answer = this.#answer;
    this.#answer = undefined;
    this.#set({ asking: undefined });
    answer?.(yes);
  }

  /** Forgets the last failure. */
  dismissFailure(): void {
    this.#retry = undefined;
    this.#set({ failure: undefined });
  }

  /** Runs the failed save again. */
  retry(): void {
    const retry = this.#retry;
    this.dismissFailure();
    retry?.();
  }

  #ask(question: SaveQuestion): Promise<boolean> {
    // A second question cancels the first.
    this.#answer?.(false);
    return new Promise((resolve) => {
      this.#answer = resolve;
      this.#set({ asking: question });
    });
  }

  /**
   * Saves as `how` says; resolves true once the file is written and the document saved, false when
   * the user cancelled or it failed (the failure shows until dismissed or retried).
   */
  async save(how: SaveHow): Promise<boolean> {
    const name = this.#fileName();
    const rewrite = this.#rewrite;
    let options: SaveOptions = {
      mode: how === "optimised" || rewrite ? "fullRewrite" : "auto",
      metadataDate: this.#now().toISOString(),
      instanceId: this.#id(),
    };
    try {
      // A signed file keeps its signatures by saving after them: a rewrite isn't offered (O02).
      if (how === "optimised" && !rewrite && (await this.#blocksRewrite())) return false;
      const prepared = await this.#prepare(options);
      if (prepared === undefined) return false;
      options = prepared;
      const saved = await this.#engine.saveEdits(options, how === "save-as" ? "pick" : "auto");
      if (saved === undefined) return false;
      this.#rewrite = false;
      this.dismissFailure();
      this.#toasts.add({ message: message(saved.downloaded ? "save.downloaded" : "save.done", { name: saved.name }) });
      if (saved.report.metadataNotUpdated) this.#toasts.add({ message: message("save.metadataNotUpdated") });
      return true;
    } catch (error) {
      reportError(error);
      this.#retry = () => void this.save(how);
      this.#set({ failure: { name, reason: failureReason(error) } });
      return false;
    }
  }

  /** Whether the file is signed and saves only after its signatures: says so, and Save as optimised does nothing. */
  async #blocksRewrite(): Promise<boolean> {
    const plan = await this.#viewer.edits.plan({});
    if (plan.constraint !== "appendOnly") return false;
    const kind = signatureKind(plan.signatures[0]?.kind ?? "approval");
    this.#toasts.add({ message: message("save.optimisedBlocked", { kind }) });
    return true;
  }

  /**
   * Download a copy: the document with its edits, saved under a new name, which stays unsaved.
   * Resolves true once the copy is written.
   */
  async copy(): Promise<boolean> {
    try {
      const options = { metadataDate: this.#now().toISOString(), instanceId: this.#id() };
      const prepared = await this.#prepare(options, this.#copyKind());
      if (prepared === undefined) return false;
      const { name, bytes } = await this.#engine.copyEdits(prepared);
      return await this.#writeCopy(name, bytes);
    } catch (error) {
      this.#failed(error, () => void this.copy());
      return false;
    }
  }

  /**
   * Save a copy of a change a signature refused (O06): the document with `ops` applied, written
   * without the signatures they break once the person agrees, which the confirmation names
   * (`signatures`). The open document stays as it was, still signed. Resolves true once written.
   */
  async copyWith(ops: readonly Op[], signatures: readonly SaveSignature[]): Promise<boolean> {
    try {
      if (!(await this.#ask({ kind: "signatures", signatures, copy: this.#copyKind() }))) return false;
      const options = {
        metadataDate: this.#now().toISOString(),
        instanceId: this.#id(),
        invalidateSignatures: true,
      };
      const { name, bytes } = await this.#engine.copyWithEdit(ops, options);
      return await this.#writeCopy(name, bytes);
    } catch (error) {
      this.#failed(error, () => void this.copyWith(ops, signatures));
      return false;
    }
  }

  /** How a copy is written here: saved as a file where the browser can, else downloaded. */
  #copyKind(): CopyKind {
    return this.#platform.capabilities.canSaveAsDialog ? "save" : "download";
  }

  /** Save a copy's name here, as its command's: "Save a copy…", or "Download a copy" where the browser downloads it. */
  copyTitle(): string {
    return message(this.#copyKind() === "save" ? "command.file.save-copy.save" : "command.file.save-copy");
  }

  /** Writes a copy as a new file named after `name`; true once written. */
  async #writeCopy(name: string, bytes: Uint8Array): Promise<boolean> {
    const result = await this.#platform.saveAs(bytes, downloadName(name));
    if (result.saved) this.#toasts.add({ message: message("download.done", { name: result.name }) });
    return result.saved;
  }

  /** Shows a write's failure under the top bar, with Try again running `retry`. */
  #failed(error: unknown, retry: () => void): void {
    reportError(error);
    this.#retry = retry;
    this.#set({ failure: { name: this.#fileName(), reason: failureReason(error) } });
  }

  /**
   * Plans a save with `options` and asks what it needs: before invalidating signatures (the
   * options then override them; worded for a copy when `copy` says how it is written) and before
   * breaking a standard the file claims. Undefined when the user said no.
   *
   * @throws a refusal no answer can lift (an encrypted file, a rewrite a signature forbids being chosen).
   */
  async #prepare(options: SaveOptions, copy?: CopyKind): Promise<SaveOptions | undefined> {
    const edits = this.#viewer.edits;
    let plan = await edits.plan(options);
    if (plan.refusal === "would-invalidate-signatures") {
      const overridden = { ...options, invalidateSignatures: true };
      plan = await edits.plan(overridden);
      if (plan.refusal !== undefined) throw new Error(plan.refusal);
      const signatures = plan.wouldInvalidate.map(signature);
      const question = copy === undefined ? { signatures } : { signatures, copy };
      if (!(await this.#ask({ kind: "signatures", ...question }))) return undefined;
      options = overridden;
    } else if (plan.refusal !== undefined) {
      throw new Error(plan.refusal);
    }
    const warnings = plan.conformanceWarnings.map((w) => w.message);
    if (warnings.length > 0 && !(await this.#ask({ kind: "conformance", warnings }))) return undefined;
    return options;
  }

  #set(next: Partial<SaveFlowState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of [...this.#listeners]) listener();
  }
}
