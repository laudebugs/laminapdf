// Saving on the desktop (E1 plan §7): what the window sends the main process and reads back. The
// main process writes the file and the engine adopts it in one command (src-tauri/src/save/), so
// no file's bytes ever reach the webview. The viewer's edit session still runs each save after the
// edits before it and marks the history saved, so the save goes through its `save`, whose copy
// step here is the whole save and whose write step has nothing left to do.

import type { ViewerEngine } from "@laminapdf/viewer";

/** `@laminapdf/sdk`'s save options, as the viewer's session passes them. */
export type SaveOptions = Parameters<ViewerEngine["planSave"]>[0];

/** `@laminapdf/sdk`'s save plan, as the viewer's session reads it. */
export type PlannedSave = Awaited<ReturnType<ViewerEngine["planSave"]>>;

/** Where a save goes: over the file, over a file another app changed, or to a new file the user picks. */
export type SaveTarget = "in-place" | "overwrite" | "new-file";

/** What the main process is asked: `SaveAsked` in Rust. */
interface SaveAsked {
  readonly rewrite: boolean;
  readonly invalidateSignatures: boolean;
  readonly metadataDate?: string;
  readonly instanceId?: readonly number[];
}

/** What a save did: `SavedView` in Rust. */
export interface DesktopSaved {
  /** The saved file's name, without its folder: a new one after Save as. */
  readonly name: string;
  readonly mode: "incremental" | "fullRewrite";
  readonly historyCleared: boolean;
  readonly metadataNotUpdated: boolean;
}

/** A plan as the main process sends it: `PlanView` in Rust, a part of `PlannedSave`. */
type PlanView = Pick<
  PlannedSave,
  "mode" | "refusal" | "constraint" | "signatures" | "wouldInvalidate" | "conformanceWarnings" | "unchanged"
>;

/** The command for each target, and whether it takes `overwrite`. */
const COMMANDS: Readonly<Record<SaveTarget, { readonly command: string; readonly overwrite?: true }>> = {
  "in-place": { command: "save_document" },
  overwrite: { command: "save_document", overwrite: true },
  "new-file": { command: "save_document_as" },
};

/** `options` as the main process reads them. */
export function asked(options: SaveOptions): SaveAsked {
  const id = options.instanceId === undefined ? undefined : uuidBytes(options.instanceId);
  return {
    rewrite: options.mode === "fullRewrite",
    invalidateSignatures: options.invalidateSignatures === true,
    ...(options.metadataDate === undefined ? {} : { metadataDate: options.metadataDate }),
    ...(id === undefined ? {} : { instanceId: id }),
  };
}

/** A UUID's 16 bytes, from its hexadecimal form with or without dashes and `uuid:`; undefined for anything else. */
function uuidBytes(id: string): number[] | undefined {
  const hex = id
    .trim()
    .replace(/^uuid:/, "")
    .replaceAll("-", "");
  if (!/^[0-9a-f]{32}$/i.test(hex)) return undefined;
  return Array.from({ length: 16 }, (_, i) => Number.parseInt(hex.slice(2 * i, 2 * i + 2), 16));
}

/** The plan the main process sent, with the parts of `PlannedSave` the desktop doesn't read at their empty values. */
export function plannedSave(view: PlanView): PlannedSave {
  return {
    rewriteReasons: [],
    violations: [],
    removesEncryption: false,
    metadata: "none",
    packetRewritten: false,
    ...view,
  } as PlannedSave;
}

/** The invoke that saves `doc` to `target`: its command and arguments. */
export function saveCall(
  doc: number,
  target: SaveTarget,
  options: SaveOptions,
): { readonly command: string; readonly args: Record<string, unknown> } {
  const { command, overwrite } = COMMANDS[target];
  return { command, args: { doc, options: asked(options), ...(overwrite ? { overwrite } : {}) } };
}

/** Thrown through the edit session when the user cancels Save as's dialog: nothing was saved. */
export const SAVE_CANCELLED = new Error("the save was cancelled");
