// The parts of the edit session that wait until a document is open (loaded with import(), as the
// print and properties dialogs are, #380): the edit commands registered, the toasts (edit-toasts.ts);
// the confirmation before the reader closes or replaces a document with unsaved changes (O03, with
// Save, or Save a copy… when saving would invalidate a signature); saving's questions and its strip
// (save-flow.ts, O02); and the interaction mode that reads pointer input on the page (select, unless
// a later tool is chosen).

import type { ArticleThread } from "@laminapdf/sdk";
import {
  Button,
  type CommandRegistry,
  countMessage,
  Dialog,
  type EditControls,
  type KeyPlatform,
  type ModeRegistry,
  message,
  Notice,
  RAIL_DRAWER_QUERY,
  type RailTab,
  type ReaderController,
  type ShellLayout,
  stepThreadPart,
  type ToastQueue,
  useMediaQuery,
  useStoreState,
} from "@laminapdf/ui";
import {
  attachTools,
  ConformanceDialog,
  conflictLine,
  HistoryPanel,
  RemoveSignaturesDialog,
  registerEditCommands,
  type SaveSignature,
  SaveStrip,
} from "@laminapdf/ui/editing";
import { TEXT_CURSOR_PROPERTY, type Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { CommentMode } from "./comment/mode.tsx";
import { showEditToasts } from "./edit-toasts.ts";
import { toolsOf } from "./edit-tools.tsx";
import { type EditActions, HISTORY_TAB } from "./editing.tsx";
import { InitialViewEditing, showInitialView } from "./initial-view-editing.tsx";
import { LinkEditing } from "./links.tsx";
import { type OrganiseController, OrganiseMode } from "./organise.tsx";
import { type PageInfo, pageLabel } from "./outline.ts";
import type { SaveFlow } from "./save-flow.ts";
import type { Engine } from "./session.ts";
import { followedPart, goToThreadPart } from "./thread-parts.ts";

/** Toasts after undos and redos, and when a restarted worker lost the edits. */
function useEditToasts(viewer: Viewer, engine: Engine, toasts: ToastQueue): void {
  useEffect(() => showEditToasts(viewer.edits, toasts), [viewer, toasts]);
  const lost = useStoreState(engine, (s) => s.editsLost);
  const seen = useRef(lost);
  useEffect(() => {
    if (lost === seen.current) return;
    seen.current = lost;
    toasts.add({ message: message("edit.toast.lost") });
  }, [lost, toasts]);
}

/**
 * The reader's close guard while there are unsaved changes: the confirmation's state, resolved by
 * its buttons. Clean documents close without asking.
 */
function useCloseGuard(reader: ReaderController, edits: EditControls) {
  const [asking, setAsking] = useState<((leave: boolean) => void) | undefined>(undefined);
  // The question waiting for an answer: a second close while the dialog shows (the address
  // changing twice) answers the first "stay", so no caller is left waiting for good.
  const pending = useRef<((leave: boolean) => void) | undefined>(undefined);
  useEffect(() => {
    return reader.setCloseGuard(() => {
      if (edits.getState().history.unsaved === 0) return Promise.resolve(true);
      pending.current?.(false);
      return new Promise<boolean>((resolve) => {
        pending.current = resolve;
        setAsking(() => resolve);
      });
    });
  }, [reader, edits]);
  const answer = (leave: boolean) => {
    asking?.(leave);
    pending.current = undefined;
    setAsking(undefined);
  };
  return { asking: asking !== undefined, answer };
}

/**
 * The confirmation (O03): Save, then close; Don't save, which loses the changes; or Cancel. A save
 * that fails or is cancelled keeps the document open, the failure shown under the top bar. When
 * saving would invalidate a signature (`conflict`), Save is Save a copy…: the copy is written
 * without it, the file stays signed, and the document closes once the copy is written.
 */
function CloseDialog({
  isOpen,
  fileName,
  unsaved,
  conflict,
  copyTitle,
  onAnswer,
  onSave,
  onCopy,
}: {
  isOpen: boolean;
  fileName: string;
  unsaved: number;
  /** The signatures a save would invalidate, when it would; undefined otherwise. */
  conflict: readonly SaveSignature[] | undefined;
  /** Save a copy…'s name here: "Save a copy…", or "Download a copy" where the browser downloads it. */
  copyTitle: string;
  onAnswer: (leave: boolean) => void;
  onSave: () => Promise<boolean>;
  onCopy: () => Promise<boolean>;
}): ReactNode {
  const count = countMessage(unsaved, { one: "edit.close.one", many: "edit.close.many" });
  return (
    <Dialog
      isOpen={isOpen}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={message("edit.close.title", { file: fileName })}
      description={conflict === undefined ? count : `${count} ${conflictLine(conflict)}`}
      actions={
        <>
          <Button variant="danger" onPress={() => onAnswer(true)}>
            {message("edit.close.discard")}
          </Button>
          <Button onPress={() => onAnswer(false)}>{message("edit.close.cancel")}</Button>
          {conflict === undefined ? (
            <Button variant="primary" autoFocus onPress={() => void onSave().then(onAnswer)}>
              {message("edit.close.save")}
            </Button>
          ) : (
            <Button variant="primary" autoFocus onPress={() => void onCopy().then(onAnswer)}>
              {copyTitle}
            </Button>
          )}
        </>
      }
    />
  );
}

/**
 * Shows the panel the open document asks for when it opens (O13, E3 plan §5): Outline for
 * /UseOutlines, Pages for /UseThumbs, focus mode for /FullScreen; /UseAttachments and /UseOC open
 * nothing until the reader has those panels. The engine reads it once the first page painted.
 * Where the rail covers the page (a drawer, the phone's sheet) it stays shut: opening it there
 * would hide the page the person just opened.
 */
function useOpeningPanel(engine: Engine, layout: ShellLayout): void {
  // The compiler can't follow the latest-value ref kept here for effects (useEffectEvent could replace it).
  "use no memo";
  const opening = useStoreState(engine, (s) => s.opening);
  const covers = useMediaQuery(RAIL_DRAWER_QUERY);
  // Read when the document opens, not followed: resizing the window later opens nothing.
  const covering = useRef(covers);
  covering.current = covers;
  useEffect(() => {
    const show = (tab: string) => {
      if (covering.current) return;
      layout.setRailTab(tab);
      layout.setRailOpen(true);
    };
    switch (opening?.pageMode) {
      case "outlines":
        show("outline");
        break;
      case "thumbnails":
        show("pages");
        break;
      case "fullScreen":
        if (!layout.getState().focus) layout.toggleFocus();
        break;
      default:
        break;
    }
  }, [opening, layout]);
}

/** Passes pointer input on the view to the interaction mode on. */
function useTools(viewer: Viewer, reader: ReaderController): void {
  const tools = toolsOf(viewer.edits);
  useEffect(() => {
    const element = viewer.element;
    return attachTools(tools, {
      element,
      textCursorProperty: TEXT_CURSOR_PROPERTY,
      scrollBy: (dx, dy) => element.scrollBy(dx, dy),
    });
  }, [viewer, tools]);
  // One mode: the reader machine's. A `set-mode` turns the tool on; a name no tool has goes back
  // to the tool on, so the machine never names a mode nothing reads.
  useEffect(() => {
    const follow = () => {
      const state = reader.getState();
      if (state.phase !== "open" || state.mode === tools.getState().current) return;
      if (!tools.use(state.mode)) reader.setMode(tools.getState().current);
    };
    follow();
    const stops = [reader.subscribe(follow), tools.subscribe(() => reader.setMode(tools.getState().current))];
    return () => {
      for (const stop of stops) stop();
    };
  }, [reader, tools]);
}

/** Props of `EditingRuntime`. */
export interface EditingRuntimeProps {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly registry: CommandRegistry;
  readonly toasts: ToastQueue;
  readonly fileName: string;
  readonly flow: SaveFlow;
  /** The shell's layout: the panel the document asks for opens in it. */
  readonly layout: ShellLayout;
  /** The Organise mode's state for this document, the shell's modes and how shortcut chips are written. */
  readonly organise: OrganiseController;
  readonly modes: ModeRegistry;
  readonly keys: KeyPlatform;
}

/** Asks the browser to confirm closing the tab while `unsaved`; its own prompt, whose words can't be set. */
function useBeforeUnload(unsaved: boolean): void {
  useEffect(() => {
    if (!unsaved) return;
    const ask = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      // Older browsers ask only when `returnValue` is set.
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", ask);
    return () => window.removeEventListener("beforeunload", ask);
  }, [unsaved]);
}

/** The edit session's runtime for the document `viewer` shows; draws the close confirmation and saving's questions. */
export function EditingRuntime({
  engine,
  viewer,
  registry,
  toasts,
  fileName,
  flow,
  layout,
  ...organising
}: EditingRuntimeProps): ReactNode {
  useEffect(() => registerEditCommands(registry), [registry]);
  useEffect(() => flow.start(), [flow]);
  useEditToasts(viewer, engine, toasts);
  useTools(viewer, engine.reader);
  useOpeningPanel(engine, layout);
  const guard = useCloseGuard(engine.reader, viewer.edits);
  const unsaved = useStoreState(viewer.edits, (s) => s.history.unsaved);
  const asking = useStoreState(flow, (s) => s.asking);
  const plan = useStoreState(flow, (s) => s.plan);
  useBeforeUnload(unsaved > 0);
  return (
    <>
      <LinkEditing engine={engine} viewer={viewer} registry={registry} toasts={toasts} />
      <CloseDialog
        isOpen={guard.asking}
        fileName={fileName}
        unsaved={unsaved}
        conflict={plan?.constraint === "conflict" ? plan.wouldInvalidate : undefined}
        copyTitle={flow.copyTitle()}
        onAnswer={guard.answer}
        onSave={() => flow.save("save")}
        onCopy={() => flow.copy()}
      />
      <RemoveSignaturesDialog
        signatures={asking?.kind === "signatures" ? asking.signatures : undefined}
        copy={asking?.kind === "signatures" ? asking.copy : undefined}
        onAnswer={(yes) => flow.answer(yes)}
      />
      <ConformanceDialog
        warnings={asking?.kind === "conformance" ? asking.warnings : undefined}
        onAnswer={(yes) => flow.answer(yes)}
      />
      <InitialViewEditing engine={engine} viewer={viewer} />
      <CommentMode
        engine={engine}
        viewer={viewer}
        modes={organising.modes}
        registry={registry}
        layout={layout}
        flow={flow}
      />
      <OrganiseMode
        controller={organising.organise}
        fileName={fileName}
        engine={engine}
        viewer={viewer}
        modes={organising.modes}
        registry={registry}
        keys={organising.keys}
      />
    </>
  );
}

/**
 * Under the top bar: how the file saves when that isn't plain (signed, rewritten, or a save that
 * breaks a signature), and the last save that failed, with Try again (O02).
 */
export function SaveNotices({ flow }: { flow: SaveFlow }): ReactNode {
  const plan = useStoreState(flow, (s) => s.plan);
  const failure = useStoreState(flow, (s) => s.failure);
  return (
    <>
      {failure === undefined ? null : (
        <div className="web-save-failure">
          <Notice tone="danger" title={message("save.failed", failure)} />
          <Button size={26} onPress={() => flow.retry()}>
            {message("save.failed.retry")}
          </Button>
          <Button variant="ghost" size={26} onPress={() => flow.dismissFailure()}>
            {message("save.failed.dismiss")}
          </Button>
        </div>
      )}
      <SaveStrip plan={plan} copy={{ label: flow.copyTitle(), onPress: () => void flow.copy() }} />
    </>
  );
}

/** The History tab (O01), once the document has an edit session: the rail lists it last. */
export function historyTab(edits: EditControls, pages: readonly PageInfo[], openedAt: number | undefined): RailTab {
  return {
    id: HISTORY_TAB,
    label: message("edit.history"),
    panel: (
      <HistoryPanel
        edits={edits}
        headingLevel={3}
        pageName={(index) => pageLabel(pages, index)}
        {...(openedAt === undefined ? {} : { openedAt })}
      />
    ),
  };
}

/**
 * The delegated edit commands' handlers for the document `viewer` shows: Show history, Initial
 * view…, the page turns, the saves, Download a copy with the edits while there are any
 * (`unsaved`), and Next and Previous thread part while the file has threads.
 */
export function editActions(
  engine: Engine,
  layout: ShellLayout,
  viewer: Viewer,
  flow: SaveFlow,
  state: { readonly unsaved: boolean; readonly threads: readonly ArticleThread[] },
): EditActions {
  const { unsaved, threads } = state;
  // Next and Previous thread part only while the file has threads: without, they stay out of view.
  const step = (direction: 1 | -1) => () => {
    const part = stepThreadPart(threads, followedPart(threads), direction);
    if (part !== undefined) goToThreadPart(viewer, threads, part);
  };
  // A failed turn is said by the session's failure toast (edit-toasts.ts).
  const rotate = (delta: 90 | -90) => () =>
    void engine.rotatePage(viewer.getState().currentPage, delta).catch(() => undefined);
  return {
    "edit.history": () => {
      layout.setRailTab(HISTORY_TAB);
      layout.setRailOpen(true);
    },
    "organise.initial-view": () => showInitialView(true),
    "pages.rotate-right": rotate(90),
    "pages.rotate-left": rotate(-90),
    "file.save": () => void flow.save("save"),
    "file.save-as": () => void flow.save("save-as"),
    "file.save-optimised": () => void flow.save("optimised"),
    ...(unsaved ? { "file.save-copy": () => void flow.copy() } : {}),
    ...(threads.length === 0 ? {} : { "nav.next-thread-part": step(1), "nav.previous-thread-part": step(-1) }),
  };
}
