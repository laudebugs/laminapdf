// File ▸ Document properties… on the desktop (R08, O04): the web reader's dialog, over what the
// engine reads in one call when it opens (its facts, the metadata the Description form edits and
// the XMP packet, with the edits so far), since only the engine parses (ADR 0008 §2). Apply makes
// the form's and the XMP tab's changes one step of the shown document's edit session, which a save
// then writes in place. A document the engine can't edit (encrypted, or its edits diverged) shows
// the facts alone.

import { message, type Platform, type PropertiesContext, type ToastQueue } from "@laminapdf/ui";
import { PropertiesDialog, type PropertiesEditing, type PropertiesState, propertiesOps } from "@laminapdf/ui/dialogs";
import { type ReactNode, useMemo, useRef, useState } from "react";
import type { DesktopEngine } from "./app.tsx";
import type { DesktopDocument } from "./engine/desktop-engine-client.ts";
import { usePrint } from "./print.tsx";

/**
 * What the dialog shows: the document it opened over (the tab may change while it is open), its
 * facts once read, and the form's data when that document can be edited.
 */
interface Shown {
  readonly doc: number;
  readonly fileName: string;
  readonly context: PropertiesContext;
  readonly state: PropertiesState;
  readonly editing: Omit<PropertiesEditing, "onApply"> | undefined;
}

/** The dialog over `document` while its data is read. */
function loading(document: DesktopDocument): Shown {
  return {
    doc: document.doc,
    fileName: document.name,
    context: { pages: document.pages },
    state: { status: "loading" },
    editing: undefined,
  };
}

/** The edit session's ops. */
type Ops = Parameters<DesktopEngine["viewer"]["edits"]["apply"]>[0];

/**
 * Print… and Document properties, as the web reader's output.tsx groups them: both handlers, the
 * print's status-bar item, and the properties dialog.
 */
export function useOutput(
  engine: DesktopEngine | undefined,
  platform: Platform,
  toasts: ToastQueue,
): { readonly actions: Readonly<Record<string, () => void>>; readonly status: ReactNode; readonly dialog: ReactNode } {
  const print = usePrint(engine, toasts);
  const properties = useDocumentProperties(engine, platform, toasts);
  const actions = useMemo(() => ({ ...print.actions, ...properties.actions }), [print.actions, properties.actions]);
  return { actions, status: print.status, dialog: properties.dialog };
}

/**
 * The `file.properties` handler and its dialog.
 * @param engine - The window's engine, or `undefined` in a browser (no handler then).
 * @param platform - Copies a value.
 * @param toasts - Where a copy says it worked.
 */
function useDocumentProperties(
  engine: DesktopEngine | undefined,
  platform: Platform,
  toasts: ToastQueue,
): { readonly actions: Readonly<Record<string, () => void>>; readonly dialog: ReactNode } {
  const [open, setOpen] = useState(false);
  const [shown, setShown] = useState<Shown>();
  // The read the dialog shows: a later open's answer replaces an earlier one's.
  const reads = useRef(0);
  const actions = useMemo(() => {
    if (engine === undefined) return {};
    return {
      "file.properties": () => {
        const document = engine.client.current;
        if (document === undefined) return;
        const read = ++reads.current;
        const start = loading(document);
        setShown(start);
        setOpen(true);
        void readShown(engine, start).then(
          (next) => {
            if (read === reads.current) setShown(next);
          },
          (error: unknown) => {
            if (read === reads.current) setShown({ ...start, state: { status: "failed" } });
            console.error("reading the document's properties failed", error);
          },
        );
      },
    };
  }, [engine]);
  const dialog =
    engine === undefined || shown === undefined ? null : (
      <PropertiesDialog
        isOpen={open}
        fileName={shown.fileName}
        state={shown.state}
        context={shown.context}
        onCopy={(text) => {
          platform.copyText(text).then(
            () => toasts.add({ message: message("shell.copied") }),
            (error: unknown) => console.error("copying failed", error),
          );
        }}
        onClose={() => {
          reads.current += 1;
          setOpen(false);
        }}
        editing={shown.editing === undefined ? undefined : { ...shown.editing, onApply: applier(engine, shown.doc) }}
      />
    );
  return { actions, dialog };
}

/** What the engine reads for the dialog `start` opened, and whether that document can take its edits. */
async function readShown(engine: DesktopEngine, start: Shown): Promise<Shown> {
  const { client, viewer } = engine;
  const editable = client.current?.encrypted === false && viewer.edits.getState().blocked === undefined;
  const [read, plan] = await Promise.all([client.properties(), editable ? viewer.edits.plan({}) : undefined]);
  const context = read.labels === undefined ? start.context : { ...start.context, labels: read.labels };
  const state: PropertiesState = { status: "ready", properties: read.properties };
  if (plan === undefined) return { ...start, context, state, editing: undefined };
  return {
    ...start,
    context,
    state,
    editing: {
      metadata: { status: "ready", metadata: read.metadata },
      xmp: read.xmp,
      // A removed value leaves no earlier version behind only when the save rewrites the file.
      rewritesWhole: plan.mode === "fullRewrite" || plan.constraint === "requiresRewrite",
    },
  };
}

/**
 * The dialog's Apply: its changes as one step of document `doc`'s edit session. The session is the
 * shown document's, so Apply refuses once another tab shows, and the dialog says it couldn't.
 */
function applier(engine: DesktopEngine, doc: number): PropertiesEditing["onApply"] {
  return async (change, xmp, options) => {
    if (engine.client.current?.doc !== doc) throw new Error("another document shows now");
    // The dialog's ops are shapes of the SDK's (@laminapdf/ui doesn't depend on it).
    const ops = propertiesOps(change, xmp, options) as unknown as Ops;
    if (ops.length > 0) await engine.viewer.edits.apply(ops, message("properties.edit.step"));
  };
}
