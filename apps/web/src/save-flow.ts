// Saving in the reader (E1 plan §7, Paper O02): Save, Save as…, Save as optimised and Download a
// copy, each planned first, and a copy with a change a signature refused (O06's Save a copy). A
// save that would invalidate signatures asks, naming each one, and one that breaks a standard the
// file claims asks too; the engine then writes the file through the platform, and the document
// takes it as saved only once that worked. The plan for a save now is kept for the strip under the
// top bar, read again as the history changes. Loaded with the rest of the edit session once a
// document is open (later.ts).

import type { Op, PlannedSave, SaveOptions } from "@laminapdf/sdk";
import { type ExternalStore, message, type Platform, type ToastQueue } from "@laminapdf/ui";
import {
  type ClaimId,
  type ClaimIssue,
  type ClaimsAnswer,
  type ClaimsView,
  type CopyKind,
  laterMessage,
  type SavePlanSlice,
  type SaveSignature,
  signatureKind,
} from "@laminapdf/ui/editing";
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
  /** Check before saving: `saving` when a save waits on the answer, else it was opened to look. */
  | { readonly kind: "conformance"; readonly saving: boolean };

/** An answer to a question: yes or no, or what to do about a claim. */
type Answer = boolean | ClaimsAnswer;

/** What the strip and the dialogs read. */
export interface SaveFlowState {
  /** How a save now would go, while there are unsaved changes; undefined otherwise. */
  readonly plan: SavePlanSlice | undefined;
  /** What a save waits on the user for; undefined when nothing is asked. */
  readonly asking: SaveQuestion | undefined;
  /** The standards the file claims, and what the edits so far break of them. */
  readonly claims: ClaimsView;
  /** The last save's failure, until a save works or it is dismissed. */
  readonly failure: { readonly name: string; readonly reason: string } | undefined;
}

const NO_CLAIMS: ClaimsView = { names: {}, issues: [] };

const IDLE: SaveFlowState = { plan: undefined, asking: undefined, failure: undefined, claims: NO_CLAIMS };

/** The standard a conformance warning is about: PDF/A's codes name it, the rest are tagging's, which PDF/UA wants. */
function claimOf(code: string): ClaimId {
  return code.startsWith("pdfa-") || code === "imported-pdfa-unchecked" ? "pdfa" : "pdfua";
}

/**
 * The claim a warning counts against, given the claims the file makes. A warning for a claim the
 * file doesn't make counts against the one it does (PDF/A's own tagging rules); with the claims
 * unknown, every warning counts, so a check that couldn't read them still stops a save.
 */
function claimFor(code: string, names: ClaimsView["names"] | undefined): ClaimId | undefined {
  const own = claimOf(code);
  if (names === undefined || names[own] !== undefined) return own;
  const other = own === "pdfua" ? "pdfa" : "pdfua";
  return names[other] === undefined ? undefined : other;
}

/** The issues a plan's warnings are, for the claims the file makes (every warning, when they are unknown). */
function issuesOf(plan: PlannedSave, names: ClaimsView["names"] | undefined): ClaimIssue[] {
  return plan.conformanceWarnings.flatMap((w) => {
    const claim = claimFor(w.code, names);
    return claim === undefined
      ? []
      : [{ claim, code: w.code, message: w.message, needsAlt: w.code === "figure-without-alt" }];
  });
}

/** The names the strip shows: the file's own, or a plain "PDF/A" and "PDF/UA" for the claims its issues name when they couldn't be read. */
function shownNames(names: ClaimsView["names"] | undefined, issues: readonly ClaimIssue[]): ClaimsView["names"] {
  if (names !== undefined) return names;
  return Object.fromEntries(issues.map((i) => [i.claim, i.claim === "pdfa" ? "PDF/A" : "PDF/UA"]));
}

/** A claim's name as people write it: "PDF/A-2b", "PDF/UA-1". */
function claimNames(meta: { readonly pdfa?: string; readonly pdfua?: number }): ClaimsView["names"] {
  return {
    ...(meta.pdfa === undefined ? {} : { pdfa: `PDF/A-${meta.pdfa.charAt(0)}${meta.pdfa.slice(1).toLowerCase()}` }),
    ...(meta.pdfua === undefined ? {} : { pdfua: `PDF/UA-${meta.pdfua}` }),
  };
}

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
  #answer: ((answer: Answer) => void) | undefined;
  #claimsRead: Promise<ClaimsView["names"] | undefined> | undefined;
  /** The plan the strip last showed, for when the claims arrive after it. */
  #lastPlan: PlannedSave | undefined;
  readonly #fixes = new Set<() => void>();
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
   * Whether the next save rewrites: asked for, and the flatten that asked is done. Undoing it
   * ends the rewrite for now, and redoing it brings it back.
   */
  #rewriting(): boolean {
    return this.#rewrite && this.#viewer.edits.getState().history.position >= this.#rewriteAt;
  }

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
      void this.#replan(history.unsaved > 0);
    };
    follow();
    void this.#names();
    return edits.subscribe(follow);
  }

  /**
   * Which standards the file claims, read once (again when asked): undefined when they can't be
   * read, which the checks treat as every claim at stake. The strip follows when they arrive.
   */
  #names(again = false): Promise<ClaimsView["names"] | undefined> {
    if (again || this.#claimsRead === undefined) {
      const read = this.#readNames();
      this.#claimsRead = read;
      void read.then((names) => this.#showClaims(this.#lastPlan, names));
    }
    return this.#claimsRead;
  }

  async #readNames(): Promise<ClaimsView["names"] | undefined> {
    try {
      const { client } = await this.#engine.view();
      return claimNames(await client.metadata());
    } catch {
      return undefined;
    }
  }

  /** The strip's claims for `plan`: its issues, or none without one. */
  #showClaims(plan: PlannedSave | undefined, names: ClaimsView["names"] | undefined): void {
    const issues = plan === undefined ? [] : issuesOf(plan, names);
    this.#set({ claims: { names: shownNames(names, issues), issues } });
  }

  /** Runs `listener` when a person asks to fix what broke a claim (Fix it first, Add alternative text). */
  onFix(listener: () => void): () => void {
    this.#fixes.add(listener);
    return () => this.#fixes.delete(listener);
  }

  /** Goes to the object whose edit broke a claim, to fix it. */
  fix(): void {
    for (const listener of [...this.#fixes]) listener();
  }

  /** Opens Check before saving to look at, without a save waiting on it. */
  showCheck(): void {
    this.#set({ asking: { kind: "conformance", saving: false } });
  }

  async #replan(changed: boolean): Promise<void> {
    const turn = ++this.#planned;
    if (!changed) {
      this.#lastPlan = undefined;
      this.#set({ plan: undefined });
      this.#showClaims(undefined, await this.#names());
      return;
    }
    try {
      const edits = this.#viewer.edits;
      const rewrite: SaveOptions = this.#rewriting() ? { mode: "fullRewrite" } : {};
      const plan = await edits.plan(rewrite);
      const invalidated =
        plan.refusal === "would-invalidate-signatures"
          ? (await edits.plan({ ...rewrite, invalidateSignatures: true })).wouldInvalidate.map(signature)
          : [];
      const names = await this.#names();
      if (turn !== this.#planned) return;
      this.#lastPlan = plan;
      this.#set({ plan: slice(plan, invalidated) });
      this.#showClaims(plan, names);
    } catch {
      // No plan (the document closed, or can't be edited): the strip says nothing.
      if (turn !== this.#planned) return;
      this.#lastPlan = undefined;
      this.#set({ plan: undefined });
      this.#showClaims(undefined, undefined);
    }
  }

  /** Answers the question asked. */
  answer(yes: Answer): void {
    const answer = this.#answer;
    this.#answer = undefined;
    this.#set({ asking: undefined });
    // Check before saving opened to look at has no save waiting: Fix it first still goes to the fix.
    if (answer === undefined && yes === "fix") this.fix();
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

  #ask(question: SaveQuestion): Promise<Answer> {
    // A second question cancels the first.
    this.#answer?.(false);
    return new Promise((resolve) => {
      this.#answer = resolve;
      this.#set({ asking: question });
    });
  }

  /** A yes-or-no question: whether the answer was yes. */
  async #confirm(question: SaveQuestion): Promise<boolean> {
    return (await this.#ask(question)) === true;
  }

  /**
   * Check before saving, when the save breaks a claim the file makes: what to do about each is the
   * person's. Saving goes on (true) only when they save without the claim and the claim is gone
   * from the file as planned again; Fix it first, closing and a removal that failed stop the save.
   */
  async #checkClaims(issues: readonly ClaimIssue[], options: SaveOptions): Promise<boolean> {
    if (issues.length === 0) return true;
    const answer = await this.#ask({ kind: "conformance", saving: true });
    if (answer === "fix") this.fix();
    if (answer !== "remove") return false;
    const broken = new Set(issues.map((i) => i.claim));
    const names = [...broken].flatMap((id) => this.#state.claims.names[id] ?? []);
    const args = {
      ...(broken.has("pdfa") ? { pdfa: true as const } : {}),
      ...(broken.has("pdfua") ? { pdfua: true as const } : {}),
    };
    try {
      await this.#viewer.edits.apply(
        [{ op: "removeConformanceClaims", args }],
        laterMessage("claims.step.remove", { claims: names.join(", ") }),
      );
    } catch {
      // The session's failure toast says why; nothing is saved.
      return false;
    }
    // The save was planned before the claims went: plan it again, and save only if they are gone.
    const known = await this.#names(true);
    const left = issuesOf(await this.#viewer.edits.plan(options), known).filter((i) => broken.has(i.claim));
    return left.length === 0;
  }

  /**
   * Saves as `how` says; resolves true once the file is written and the document saved, false when
   * the user cancelled or it failed (the failure shows until dismissed or retried).
   */
  async save(how: SaveHow): Promise<boolean> {
    const name = this.#fileName();
    const rewrite = this.#rewriting();
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
      if (!(await this.#confirm({ kind: "signatures", signatures, copy: this.#copyKind() }))) return false;
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
      if (!(await this.#confirm({ kind: "signatures", ...question }))) return undefined;
      options = overridden;
    } else if (plan.refusal !== undefined) {
      throw new Error(plan.refusal);
    }
    if (!(await this.#checkClaims(issuesOf(plan, await this.#names()), options))) return undefined;
    return options;
  }

  #set(next: Partial<SaveFlowState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of [...this.#listeners]) listener();
  }
}
