// Saving in the desktop window (E1 plan §7, Paper O02): File ▸ Save writes the file in place and
// File ▸ Save as… writes a new one, each planned first. A save that would invalidate signatures
// asks, naming each one, and one that breaks a standard the file claims asks too, in the same
// dialogs as the web reader's. The main process writes the file (src-tauri/src/save/).
//
// When another app changed the file while the document holds unsaved edits, or a save finds the
// file changed, the window asks what to do (S04): reload it and lose the edits, save the edits as a
// new file, or overwrite the other app's change. A save that fails says why, and the edits stay.

import { Button, Dialog, message, type ReaderController, type ToastQueue } from "@laminapdf/ui";
import { ConformanceDialog, RemoveSignaturesDialog, type SaveSignature } from "@laminapdf/ui/editing";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DesktopEngine } from "./app.tsx";
import { engineError } from "./engine/desktop-engine-client.ts";
import type { DesktopSaved, PlannedSave, SaveOptions, SaveTarget } from "./engine/desktop-saves.ts";
import { stopper } from "./engine/stopper.ts";

/** The event the main process sends when another app changed a document with unsaved edits (src-tauri/src/watch/app.rs). */
const CONFLICT_EVENT = "document://conflict";

/** What a save waits on the user for. */
type Question =
  | { readonly kind: "signatures"; readonly signatures: readonly SaveSignature[] }
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
}

/** Why a save failed, in words: the file is read-only, the disk is full, … */
function failureReason(error: unknown): string {
  const { code, message: text } = engineError(error);
  if (code === "write-refused") return message("save.reason.permission");
  if (code === "disk-full") return message("save.reason.full");
  if (code === "save-refused" || /would-invalidate-signatures|requires-rewrite|encrypted-base/.test(text))
    return message("save.reason.refused");
  return message("save.reason.other");
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
        onAnswer={reply}
      />
      <ConformanceDialog warnings={question?.kind === "conformance" ? question.warnings : undefined} onAnswer={reply} />
      {question?.kind === "conflict" ? <ConflictDialog name={question.name} onAnswer={reply} /> : null}
    </>
  );
  return { actions, save, dialog };
}

/** Saving the shown document, and what it asks. */
interface SaveFlow {
  /** Saves to `target`: true once saved; false when the user said no, or it failed (and said so). */
  save(target: SaveTarget): Promise<boolean>;
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
        if (engineError(error).code === "file-changed") {
          await flow.resolveConflict(current.doc);
          return false;
        }
        toasts.add({ message: message("save.failed", { name: current.name, reason: failureReason(error) }) });
        return false;
      }
    },
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
      if (plan.refusal !== undefined) throw new Error(plan.refusal);
      const signatures = plan.wouldInvalidate.map(signature);
      if (!(await ask<boolean>({ kind: "signatures", signatures }))) return undefined;
      options = overridden;
    } else if (plan.refusal !== undefined) {
      throw new Error(plan.refusal);
    }
    const warnings = plan.conformanceWarnings.map((w) => w.message);
    if (warnings.length > 0 && !(await ask<boolean>({ kind: "conformance", warnings }))) return undefined;
    return options;
  }

  return flow;
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
