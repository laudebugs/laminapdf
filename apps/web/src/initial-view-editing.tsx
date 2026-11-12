// Initial view… (O13), loaded with the rest of the edit session (later.ts, #380): the file's
// initial view as its edits leave it, read each time the dialog opens, and Apply's change made one
// step of the edit session. On the web the reader has no window of its own to fit, so the dialog
// says Fit the window has no effect here.

import type { InitialView, Op, TargetView } from "@laminapdf/sdk";
import { useStoreState } from "@laminapdf/ui";
import {
  type InitialViewChange,
  type InitialViewData,
  InitialViewDialog,
  type InitialZoom,
  initialViewStep,
} from "@laminapdf/ui/dialogs";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useState, useSyncExternalStore } from "react";
import type { PageInfo } from "./outline.ts";
import type { Engine } from "./session.ts";

/** A destination's view as the dialog's zoom: views it can't offer read as the reader's default. */
function zoomOf(view: TargetView): InitialZoom {
  switch (view.kind) {
    case "fit":
      return "fitPage";
    case "fit-width":
      return "fitWidth";
    case "at":
      return view.zoom === undefined || view.zoom <= 0 ? "default" : Math.round(view.zoom * 100);
    default:
      return "default";
  }
}

/** The file's initial view as the dialog reads it. */
export function dialogData(view: InitialView): InitialViewData {
  const { open, preferences } = view;
  return {
    open:
      open.kind === "page"
        ? { kind: "page", page: open.target.page, zoom: zoomOf(open.target.view) }
        : open.kind === "action"
          ? { kind: "action", action: open.action }
          : { kind: "default" },
    pageMode: view.pageMode,
    pageLayout: view.pageLayout,
    preferences,
  };
}

/** The dialog's change as the op: the opening page, an index there, named by the page's ID. */
export function initialViewOp(change: InitialViewChange, pages: readonly PageInfo[]): Op | undefined {
  const { open, ...rest } = change;
  if (open === undefined || open === null)
    return { op: "setInitialView", args: { ...rest, ...(open === null ? { open } : {}) } };
  const page = pages[open.page];
  return page === undefined
    ? undefined
    : { op: "setInitialView", args: { ...rest, open: { page: page.id, zoom: open.zoom } } };
}

/** The view's zoom now, as the dialog offers it. */
function currentZoom(viewer: Viewer): InitialZoom {
  const { zoom, percent } = viewer.getState();
  if (zoom === "fit-width") return "fitWidth";
  if (zoom === "fit-page") return "fitPage";
  return Math.round(percent);
}

/** Whether the dialog is open: Initial view… opens it (editActions), its buttons close it. */
let shown = false;
const listeners = new Set<() => void>();

/** Opens (`true`) or closes the Initial view dialog. */
export function showInitialView(open: boolean): void {
  shown = open;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The Initial view dialog over the open document, while Initial view… has it open. */
export function InitialViewEditing({ engine, viewer }: { engine: Engine; viewer: Viewer }): ReactNode {
  const isOpen = useSyncExternalStore(subscribe, () => shown);
  // Another document closes it.
  useEffect(() => () => showInitialView(false), [viewer]);
  const pageCount = useStoreState(engine, (s) => s.pages.length);
  const [view, setView] = useState<InitialViewData>();
  useEffect(() => {
    if (!isOpen) return;
    let current = true;
    void engine.initialView().then((read) => {
      if (current) setView(dialogData(read));
    }, reportError);
    // Closing forgets what was read, so the next open reads the file as its edits leave it then.
    return () => {
      current = false;
      setView(undefined);
    };
  }, [engine, isOpen]);
  return (
    <InitialViewDialog
      isOpen={isOpen}
      view={view}
      pageCount={pageCount}
      current={() => ({ page: viewer.getState().currentPage, zoom: currentZoom(viewer) })}
      fitWindowHasNoEffect
      onApply={async (change) => {
        const op = initialViewOp(change, engine.getState().pages);
        if (op === undefined) throw new RangeError("the opening page isn't in the document");
        await engine.applyEdits([op], initialViewStep());
      }}
      onClose={() => showInitialView(false)}
    />
  );
}
