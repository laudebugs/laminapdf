// Saving in the desktop window (E1 plan §7, Paper O02): File ▸ Save writes the file in place and
// File ▸ Save as… writes a new one, each planned first. A save that would invalidate signatures
// asks, naming each one, and one that breaks a standard the file claims asks too, in the same
// dialogs as the web reader's. The main process writes the file (src-tauri/src/save/).
//
// When another app changed the file while the document holds unsaved edits, or a save finds the
// file changed, the window asks what to do (S04): reload it and lose the edits, save the edits as a
// new file, or overwrite the other app's change. A save that fails says why, and the edits stay.

import { Button, Dialog, message, type ReaderController, type ToastQueue } from "@laminapdf/ui";
import {
  ConformanceDialog,
  type OrganiseCopies,
  RemoveSignaturesDialog,
  type SaveSignature,
} from "@laminapdf/ui/editing";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DesktopEngine } from "./app.tsx";
import { DesktopEngineError, engineError } from "./engine/desktop-engine-client.ts";
import type { DesktopSaved, PlannedSave, SaveOptions, SaveTarget } from "./engine/desktop-saves.ts";
import { stopper } from "./engine/stopper.ts";

/** The event the main process sends when another app changed a document with unsaved edits (src-tauri/src/watch/app.rs). */
const CONFLICT_EVENT = "document://conflict";

/** What a save waits on the user for. */
type Question =
  | { readonly kind: "signatures"; readonly signatures: readonly SaveSignature[]; readonly copy?: "save" }
  | { readonly kind: "conformance"; readonly warnings: readonly string[] }
  | { readonly kind: "conflict"; readonly doc: number; readonly name: string };

/** What the user chose about a file another app changed. */
type ConflictAnswer = "reload" | "save-as" | "overwrite" | "keep";

/** What the window gets from `useDesktopSaving`. */
export interface DesktopSaving {
  /** The handlers of File ▸ Save and Save as…. */
  readonly actions: Readonly<Record<"file.save" | "file.save-as", () => void>>;
  /** Saves the shown document in place; resolves true once it is saved (for the question before closing). */
  readonly save: () => Promise<boolean>;
  /** The question a save asks, while one is asked. */
  readonly dialog: ReactNode;
  /** Save a copy of a change a signature refused, for the Organise mode's certification banner. */
  readonly copies: OrganiseCopies | undefined;
}

/** Why a save failed, in words: the file is read-only, the disk is full, … */
function failureReason(error: unknown): string {
  const { code } = engineError(error);
  if (code === "write-refused") return message("save.reason.permission");
  if (code === "disk-full") return message("save.reason.full");
  if (code === "save-refused") return message("save.reason.refused");
  return message("save.reason.other");
}

/** What the window says when a save didn't write the file as asked, by the main process's kind; undefined for a plain failure. */
const OUTCOMES: Readonly<Record<string, (name: string) => string>> = {
  // Another app wrote the file in place: the bytes the edits build on are gone.
  "base-changed": (name) =>
    `Couldn't save over ${name}: another app changed it, and your edits build on the version it replaced. Save as… keeps your edits; Reload shows the other app's version.`,
  // The file holds every edit, but the document had to reopen from it, under the saved file's
  // name, which may be a new one after Save as.
  "saved-reopened": () => "Saved, and reopened the document from the saved file.",
};

/** A plan's refusal as the error a save fails with, as the main process would refuse it. */
function refusal(code: string): DesktopEngineError {
  return new DesktopEngineError("save-refused", code);
}

function signature(s: PlannedSave["signatures"][number]): SaveSignature {
  return s.field === undefined ? { kind: s.kind } : { kind: s.kind, field: s.field };
}

/** The save part of the window: its commands, and the questions and dialogs a save asks. */
export function useDesktopSaving(
  engine: DesktopEngine | undefined,
  reader: ReaderController,
  toasts: ToastQueue,
  renamed: (doc: number, name: string) => void,
): DesktopSaving {
  const [question, setQuestion] = useState<Question | undefined>(undefined);
  const answer = useRef<((value: string | boolean) => void) | undefined>(undefined);
  const ask = useMemo(
    () =>
      <T extends string | boolean>(next: Question): Promise<T> => {
        // A second question answers the first "no".
        answer.current?.(false);
        return new Promise<T>((resolve) => {
          answer.current = (value) => resolve(value as T);
          setQuestion(next);
        });
      },
    [],
  );
  const reply = (value: string | boolean) => {
    const resolve = answer.current;
    answer.current = undefined;
    setQuestion(undefined);
    resolve?.(value);
  };
  const flow = useMemo(
    () => (engine === undefined ? undefined : saveFlow(engine, toasts, ask, renamed)),
    [engine, toasts, ask, renamed],
  );
  useConflicts(engine, reader, flow);
  const actions = useMemo(
    () => ({
      "file.save": () => void flow?.save("in-place"),
      "file.save-as": () => void flow?.save("new-file"),
    }),
    [flow],
  );
  const save = useCallback(() => flow?.save("in-place") ?? Promise.resolve(false), [flow]);
  const dialog = (
    <>
      <RemoveSignaturesDialog
        signatures={question?.kind === "signatures" ? question.signatures : undefined}
        copy={question?.kind === "signatures" ? question.copy : undefined}
        onAnswer={reply}
      />
      <ConformanceDialog warnings={question?.kind === "conformance" ? question.warnings : undefined} onAnswer={reply} />
      {question?.kind === "conflict" ? <ConflictDialog name={question.name} onAnswer={reply} /> : null}
    </>
  );
  const copies = useMemo(
    (): OrganiseCopies | undefined =>
      flow === undefined
        ? undefined
        : { copyWith: flow.copyWith, copyTitle: () => message("command.file.save-copy.save") },
    [flow],
  );
  return { actions, save, dialog, copies };
}

/** Saving the shown document, and what it asks. */
interface SaveFlow {
  /** Saves to `target`: true once saved; false when the user said no, or it failed (and said so). */
  save(target: SaveTarget): Promise<boolean>;
  /**
   * Saves a copy of the shown document with `ops` applied, which `signatures` refused, to a new
   * file, once the person agrees to lose them; the document stays as it was. True once written.
   */
  copyWith(ops: readonly object[], signatures: readonly SaveSignature[]): Promise<boolean>;
  /** Asks what to do about document `doc`'s file, which another app changed, and does it. */
  resolveConflict(doc: number): Promise<void>;
}

function saveFlow(
  engine: DesktopEngine,
  toasts: ToastQueue,
  ask: <T extends string | boolean>(question: Question) => Promise<T>,
  renamed: (doc: number, name: string) => void,
): SaveFlow {
  const { client, viewer, shell } = engine;
  const flow: SaveFlow = {
    async save(target) {
      const current = client.current;
      if (current === undefined) return false;
      try {
        const options = await prepare({
          mode: "auto",
          metadataDate: new Date().toISOString(),
          instanceId: crypto.randomUUID(),
        });
        if (options === undefined) return false;
        const saved = await client.saveTo(viewer.edits, target, options);
        if (saved === undefined) return false;
        done(current.doc, current.name, saved);
        return true;
      } catch (error) {
        const { code } = engineError(error);
        if (code === "file-changed") {
          await flow.resolveConflict(current.doc);
          return false;
        }
        const outcome = OUTCOMES[code];
        if (outcome !== undefined) {
          toasts.add({ message: outcome(current.name) });
          return code === "saved-reopened";
        }
        toasts.add({ message: message("save.failed", { name: current.name, reason: failureReason(error) }) });
        return false;
      }
    },
    copyWith: (ops, signatures) => copyWith(engine, toasts, ask, ops, signatures),
    async resolveConflict(doc) {
      const current = client.current;
      if (current?.doc !== doc) return;
      const chosen = await ask<ConflictAnswer | false>({ kind: "conflict", doc, name: current.name });
      if (chosen === "reload") {
        await shell.invoke("reload_document", { doc }).catch((error: unknown) => {
          toasts.add({ message: engineError(error).message });
        });
      } else if (chosen === "save-as" || chosen === "overwrite") {
        await flow.save(chosen === "save-as" ? "new-file" : "overwrite");
      }
    },
  };

  /** Toasts what the save did; after Save as, the tab takes the new file's name. */
  function done(doc: number, name: string, saved: DesktopSaved): void {
    if (saved.name !== name) renamed(doc, saved.name);
    toasts.add({ message: message("save.done", { name: saved.name }) });
    if (saved.metadataNotUpdated) toasts.add({ message: message("save.metadataNotUpdated") });
  }

  /**
   * Plans a save with `options` and asks what it needs: before invalidating signatures (the
   * options then override them) and before breaking a standard the file claims. Undefined when the
   * user said no. The same steps as the web reader's save flow (apps/web/src/save-flow.ts).
   *
   * @throws a refusal no answer can lift (an encrypted file).
   */
  async function prepare(options: SaveOptions): Promise<SaveOptions | undefined> {
    let plan = await viewer.edits.plan(options);
    if (plan.refusal === "would-invalidate-signatures") {
      const overridden = { ...options, invalidateSignatures: true };
      plan = await viewer.edits.plan(overridden);
      if (plan.refusal !== undefined) throw refusal(plan.refusal);
      const signatures = plan.wouldInvalidate.map(signature);
      if (!(await ask<boolean>({ kind: "signatures", signatures }))) return undefined;
      options = overridden;
    } else if (plan.refusal !== undefined) {
      throw refusal(plan.refusal);
    }
    const warnings = plan.conformanceWarnings.map((w) => w.message);
    if (warnings.length > 0 && !(await ask<boolean>({ kind: "conformance", warnings }))) return undefined;
    return options;
  }

  return flow;
}

/**
 * Saves a copy of the shown document with `ops` applied, which `signatures` refused, to a new file,
 * once the person agrees to lose them. True once it is written.
 */
async function copyWith(
  { client, viewer }: DesktopEngine,
  toasts: ToastQueue,
  ask: <T extends string | boolean>(question: Question) => Promise<T>,
  ops: readonly object[],
  signatures: readonly SaveSignature[],
): Promise<boolean> {
  const current = client.current;
  if (current === undefined) return false;
  try {
    // Worded for a copy, which leaves the open file signed.
    if (!(await ask<boolean>({ kind: "signatures", signatures, copy: "save" }))) return false;
    const options: SaveOptions = {
      mode: "fullRewrite",
      metadataDate: new Date().toISOString(),
      instanceId: crypto.randomUUID(),
      invalidateSignatures: true,
    };
    const name = await client.copyWithTo(viewer.edits, ops, options);
    if (name === undefined) return false;
    toasts.add({ message: message("save.done", { name }) });
    return true;
  } catch (error) {
    toasts.add({ message: message("save.failed", { name: current.name, reason: failureReason(error) }) });
    return false;
  }
}

/**
 * Asks about each file another app changed while its document held unsaved edits: at once for the
 * document shown, and for one in another tab once that tab shows.
 */
function useConflicts(engine: DesktopEngine | undefined, reader: ReaderController, flow: SaveFlow | undefined): void {
  const waiting = useRef(new Set<number>());
  useEffect(() => {
    if (engine === undefined || flow === undefined) return;
    const askShown = () => {
      const doc = engine.client.current?.doc;
      if (doc === undefined || !waiting.current.delete(doc)) return;
      void flow.resolveConflict(doc);
    };
    const stopListening = stopper(
      engine.shell.listen<{ doc: number }>(CONFLICT_EVENT, ({ doc }) => {
        waiting.current.add(doc);
        askShown();
      }),
      "listening for files changed outside",
    );
    const stopFollowing = reader.subscribe(askShown);
    return () => {
      stopListening();
      stopFollowing();
    };
  }, [engine, reader, flow]);
}

/** The question about a file another app changed while it held unsaved edits. */
function ConflictDialog({ name, onAnswer }: { name: string; onAnswer: (answer: ConflictAnswer) => void }): ReactNode {
  return (
    <Dialog
      isOpen
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer("keep");
      }}
      onEscape={() => onAnswer("keep")}
      title={`${name} changed on disk`}
      description="Another app changed this file, and your edits aren't saved. Reload it to see the other app's version and lose your edits, save your edits as a new file, or overwrite the other app's version with yours."
      actions={
        <>
          <Button variant="danger" onPress={() => onAnswer("reload")}>
            Reload
          </Button>
          <Button onPress={() => onAnswer("overwrite")}>Overwrite</Button>
          <Button variant="ghost" onPress={() => onAnswer("keep")}>
            {message("edit.close.cancel")}
          </Button>
          <Button variant="primary" autoFocus onPress={() => onAnswer("save-as")}>
            Save as…
          </Button>
        </>
      }
    />
  );
}
