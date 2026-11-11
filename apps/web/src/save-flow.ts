// Saving in the reader (E1 plan §7, Paper O02): Save, Save as…, Save as optimised and Download a
// copy, each planned first. A save that would invalidate signatures asks, naming each one, and
// one that breaks a standard the file claims asks too; the engine then writes the file through
// the platform, and the document takes it as saved only once that worked. The plan for a save now
// is kept for the strip under the top bar, read again as the history changes. Loaded with the
// rest of the edit session once a document is open (later.ts).

import type { PlannedSave, SaveOptions } from "@laminapdf/sdk";
import { type ExternalStore, message, type Platform, type ToastQueue } from "@laminapdf/ui";
import { type SavePlanSlice, type SaveSignature, signatureKind } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { downloadName } from "./download-name.ts";
import type { Engine } from "./session.ts";

/** Which save: in place (or as the platform can), to a new file, or rewritten whole. */
export type SaveHow = "save" | "save-as" | "optimised";

/** A question a save waits on. */
export type SaveQuestion =
  | { readonly kind: "signatures"; readonly signatures: readonly SaveSignature[] }
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
export function failureReason(error: unknown): string {
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
      const plan = await edits.plan({});
      const invalidated =
        plan.refusal === "would-invalidate-signatures"
          ? (await edits.plan({ invalidateSignatures: true })).wouldInvalidate.map(signature)
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
    let options: SaveOptions = {
      mode: how === "optimised" ? "fullRewrite" : "auto",
      metadataDate: this.#now().toISOString(),
      instanceId: this.#id(),
    };
    try {
      // A signed file keeps its signatures by saving after them: a rewrite isn't offered (O02).
      if (how === "optimised" && (await this.#blocksRewrite())) return false;
      const prepared = await this.#prepare(options);
      if (prepared === undefined) return false;
      options = prepared;
      const saved = await this.#engine.saveEdits(options, how === "save-as" ? "pick" : "auto");
      if (saved === undefined) return false;
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

  /** Download a copy: the document with its edits, saved under a new name, which stays unsaved. */
  async copy(): Promise<void> {
    try {
      const prepared = await this.#prepare({ metadataDate: this.#now().toISOString(), instanceId: this.#id() });
      if (prepared === undefined) return;
      const { name, bytes } = await this.#engine.copyEdits(prepared);
      const result = await this.#platform.saveAs(bytes, downloadName(name));
      if (result.saved) this.#toasts.add({ message: message("download.done", { name: result.name }) });
    } catch (error) {
      reportError(error);
      this.#retry = () => void this.copy();
      this.#set({ failure: { name: this.#fileName(), reason: failureReason(error) } });
    }
  }

  /**
   * Plans a save with `options` and asks what it needs: before invalidating signatures (the
   * options then override them) and before breaking a standard the file claims. Undefined when the
   * user said no.
   *
   * @throws a refusal no answer can lift (an encrypted file, a rewrite a signature forbids being chosen).
   */
  async #prepare(options: SaveOptions): Promise<SaveOptions | undefined> {
    const edits = this.#viewer.edits;
    let plan = await edits.plan(options);
    if (plan.refusal === "would-invalidate-signatures") {
      const overridden = { ...options, invalidateSignatures: true };
      plan = await edits.plan(overridden);
      if (plan.refusal !== undefined) throw new Error(plan.refusal);
      const signatures = plan.wouldInvalidate.map(signature);
      if (!(await this.#ask({ kind: "signatures", signatures }))) return undefined;
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
