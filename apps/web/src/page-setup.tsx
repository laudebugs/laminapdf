// Organise's Page setup ▾ in the reader (Paper O09, O10; E2g): Crop pages…, Resize pages…, Page
// boxes…, Make landscape, Make portrait and Page labels…, on the selected pages, or on every page
// when none is selected. Each dialog's Apply is one op and one step of the edit session, asked
// about first when its save would break a signature (OrganiseController.edit). Loaded with the
// Organise mode (later.ts).

import type { Op, PageLabelRange } from "@laminapdf/sdk";
import { useStoreState } from "@laminapdf/ui";
import {
  type ApplyTo,
  type BoxChanges,
  CropDialog,
  type Margins,
  PageBoxesDialog,
  type PageBoxValues,
  PageLabelsDialog,
  pageRanges,
  type ResizeAnchor,
  type ResizeContent,
  ResizeDialog,
  type SetupPageSize,
  laterCountMessage as setupCount,
  laterMessage as setupWords,
  unitFor,
} from "@laminapdf/ui/editing";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import type { OrganiseController } from "./organise.tsx";
import type { PageInfo } from "./outline.ts";
import type { Engine } from "./session.ts";

/** The pages Page setup changes: the selection, or every page when none is selected. */
function targets(controller: OrganiseController): number[] {
  const selected = controller.selectedIndices();
  return selected.length > 0 ? selected : controller.pages().map((_, i) => i);
}

/** "3", "1–2, 5": the pages at `indices`, by label. */
function names(pages: readonly PageInfo[], indices: readonly number[]): string {
  return pageRanges(indices, (i) => pages[i]?.label ?? String(i + 1));
}

/** "Crop page 3", "Crop pages 1–4": a step's name or its toast. */
function counted(
  ids: { one: Parameters<typeof setupWords>[0]; many: Parameters<typeof setupWords>[0] },
  pages: readonly PageInfo[],
  indices: readonly number[],
): string {
  return setupCount(indices.length, ids, { pages: names(pages, indices) });
}

/** Makes the pages Page setup changes landscape or portrait, turning only those that aren't already. */
function orient(controller: OrganiseController, orientation: "landscape" | "portrait"): void {
  const pages = controller.pages();
  const indices = targets(controller).filter((i) => {
    const page = pages[i];
    return page !== undefined && (orientation === "landscape" ? page.height > page.width : page.width > page.height);
  });
  if (indices.length === 0) {
    controller.say(setupWords(orientation === "landscape" ? "setup.already.landscape" : "setup.already.portrait"));
    return;
  }
  const ids = indices.map((i) => pages[i]?.id ?? "");
  const op = { op: "setOrientation", args: { pages: ids, orientation, method: "rotate" } };
  controller.edit(
    [op],
    indices,
    counted({ one: `setup.step.${orientation}.one`, many: `setup.step.${orientation}` }, pages, indices),
    () => counted({ one: "setup.done.orientation.one", many: "setup.done.orientation" }, pages, indices),
  );
}

/** The handlers of Page setup's commands while Organise is the mode. */
export function pageSetupActions(controller: OrganiseController) {
  return {
    "pages.crop": () => controller.open("crop"),
    "pages.resize": () => controller.open("resize"),
    "pages.edit-boxes": () => controller.open("boxes"),
    "pages.edit-labels": () => controller.open("labels"),
    "pages.make-landscape": () => orient(controller, "landscape"),
    "pages.make-portrait": () => orient(controller, "portrait"),
  } as const;
}

/** Crops the pages `to` names by `margins`. */
function crop(controller: OrganiseController, margins: Margins, to: ApplyTo): void {
  const pages = controller.pages();
  const indices = to === "all" ? pages.map((_, i) => i) : targets(controller);
  const op = { op: "cropPages", args: { pages: indices.map((i) => pages[i]?.id ?? ""), margins } };
  controller.edit([op], indices, counted({ one: "setup.step.crop.one", many: "setup.step.crop" }, pages, indices), () =>
    counted({ one: "setup.done.crop.one", many: "setup.done.crop" }, pages, indices),
  );
}

/**
 * Resizes the pages Page setup changes to `size` as they show. The op takes the paper before
 * `/Rotate`, so a page turned a quarter gets the size with its sides swapped: one op per way up,
 * applied together as one step.
 */
function resize(
  controller: OrganiseController,
  size: SetupPageSize,
  content: ResizeContent,
  anchor: ResizeAnchor,
): void {
  const pages = controller.pages();
  const indices = targets(controller);
  const mode = content === "extend" ? { extend: { anchor } } : "scale";
  const ops: object[] = [];
  for (const turned of [false, true]) {
    const ids = indices.flatMap((i) => {
      const page = pages[i];
      return page !== undefined && (page.rotation % 180 !== 0) === turned ? [page.id] : [];
    });
    if (ids.length === 0) continue;
    const paper = turned ? { width: size.height, height: size.width } : { width: size.width, height: size.height };
    ops.push({ op: "resizePages", args: { pages: ids, size: paper, mode } });
  }
  controller.edit(
    ops,
    indices,
    counted({ one: "setup.step.resize.one", many: "setup.step.resize" }, pages, indices),
    () => counted({ one: "setup.done.resize.one", many: "setup.done.resize" }, pages, indices),
  );
}

/** Sets the boxes on the pages Page setup changes. */
function setBoxes(controller: OrganiseController, boxes: BoxChanges): void {
  const pages = controller.pages();
  const indices = targets(controller);
  const op = { op: "setPageBoxes", args: { pages: indices.map((i) => pages[i]?.id ?? ""), boxes } };
  controller.edit(
    [op],
    indices,
    counted({ one: "setup.step.boxes.one", many: "setup.step.boxes" }, pages, indices),
    () => counted({ one: "setup.done.boxes.one", many: "setup.done.boxes" }, pages, indices),
  );
}

/** Sets the page labels to `ranges`. */
function setLabels(controller: OrganiseController, ranges: readonly PageLabelRange[]): void {
  const op = { op: "setPageLabels", args: { ranges } } as unknown as Op;
  controller.edit([op], [], setupWords("setup.step.labels"), () => setupWords("setup.done.labels"));
}

/**
 * What the boxes and labels dialogs read when they open, from the document as its edits leave it:
 * read once each time `open` turns on (or `read` changes while open: give it a stable function,
 * as `useCallback` makes), so typing in the dialog never reads it again.
 */
function useOpened<T>(open: boolean, read: () => Promise<T>): T | undefined {
  const [value, setValue] = useState<T | undefined>(undefined);
  useEffect(() => {
    setValue(undefined);
    if (!open) return;
    let live = true;
    read().then(
      (v) => {
        if (live) setValue(v);
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [open, read]);
  return value;
}

/** Page setup's dialogs. */
export function PageSetupDialogs({
  controller,
  engine,
}: {
  controller: OrganiseController;
  engine: Engine;
}): ReactNode {
  // Read through the stores, not the controller's methods, which the React Compiler would keep from the first render.
  const dialog = useStoreState(controller, (s) => s.dialog);
  const selectedIds = useStoreState(controller, (s) => s.selected);
  const pages = useStoreState(engine, (s) => s.pages);
  const selected = pages.flatMap((p, i) => (selectedIds.has(p.id) ? [i] : []));
  const chosen = selected.length > 0 ? selected : pages.map((_, i) => i);
  const first = chosen[0] ?? 0;
  const unit = unitFor(globalThis.navigator?.language ?? "en");
  const close = () => controller.open(undefined);
  const readBoxes = useCallback(() => engine.pageBoxes(first), [engine, first]);
  const readRanges = useCallback(() => engine.labelRanges(), [engine]);
  const boxes = useOpened<PageBoxValues>(dialog === "boxes", readBoxes);
  const ranges = useOpened<PageLabelRange[]>(dialog === "labels", readRanges);
  const sizes = pages.map((p) => ({ width: p.width, height: p.height }));
  return (
    <>
      <CropDialog
        isOpen={dialog === "crop"}
        onClose={close}
        unit={unit}
        pages={sizes}
        selected={selected}
        onCrop={(margins, to) => crop(controller, margins, to)}
      />
      <ResizeDialog
        isOpen={dialog === "resize"}
        onClose={close}
        unit={unit}
        pages={chosen.flatMap((i) => (sizes[i] === undefined ? [] : [sizes[i]]))}
        onResize={(size, content, anchor) => resize(controller, size, content, anchor)}
      />
      <PageBoxesDialog
        isOpen={dialog === "boxes"}
        onClose={close}
        page={pages[first]?.label ?? String(first + 1)}
        count={chosen.length}
        boxes={boxes}
        onApply={(changes) => setBoxes(controller, changes)}
      />
      <PageLabelsDialog
        isOpen={dialog === "labels" && ranges !== undefined}
        onClose={close}
        pages={pages.map((p) => p.id)}
        ranges={ranges ?? []}
        onApply={(next) => setLabels(controller, next as PageLabelRange[])}
      />
    </>
  );
}
