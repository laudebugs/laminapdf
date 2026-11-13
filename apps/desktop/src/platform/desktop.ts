// The desktop reader's Platform (ADR 0010, ADR 0008). It starts from the web platform, which works
// in any webview (the clipboard, IndexedDB storage), and reports what the desktop shell adds:
// saving in place, watching files, native printing and several windows. Opening a file goes
// through the main process's dialog, so its path never reaches the webview; so do the recent list,
// which the main process keeps, and watching, which it does for every open document. Until the shell
// implements the rest, each one rejects with NotYetSupportedError rather than falling back to the
// web behaviour silently, and shared UI never asks which platform it runs on: only these flags.
import { type Capabilities, createWebPlatform, type Platform, type PlatformFile, type SaveResult } from "@laminapdf/ui";
import type { DesktopShell } from "../engine/desktop-engine-client.ts";
import { RELOADED_EVENT, type Reloaded } from "../engine/file-changes.ts";
import { documentOf, pickDocument } from "../engine/open.ts";
import { stopper } from "../engine/stopper.ts";
import { desktopRecent } from "./recent.ts";

/** What this build of the desktop app can't do yet: opening a dropped document, and the Platform methods the desktop flags promise. */
export type DesktopFeature = "open" | "saveInPlace" | "print" | "openWindow";

/** The codes of the desktop app's errors. */
export type DesktopErrorCode = "not-yet-supported";

/** A desktop capability this build doesn't have yet: the UI can say it is coming instead of failing. */
export class NotYetSupportedError extends Error {
  override readonly name = "NotYetSupportedError";
  readonly code: DesktopErrorCode = "not-yet-supported";
  /** What was asked for. */
  readonly feature: DesktopFeature;

  constructor(feature: DesktopFeature) {
    super(`${feature} isn't in this build of Lamina PDF yet`);
    this.feature = feature;
  }
}

/**
 * Whether `error` is a {@link NotYetSupportedError}.
 * @param error - Anything caught.
 * @returns True when it is one.
 */
export function isNotYetSupportedError(error: unknown): error is NotYetSupportedError {
  return error instanceof NotYetSupportedError;
}

/**
 * The flags the desktop shell sets on top of the webview's own (desktop plan §5). `ownsKeyboard`
 * (ADR 0010): no browser keeps ⌘−, ⌘W or F1, so the desktop-only shortcuts bind.
 */
export const DESKTOP_FLAGS = {
  ownsKeyboard: true,
  canWriteInPlace: true,
  canWatchFiles: true,
  hasNativePrint: true,
  multiWindow: true,
  // The main process keeps each recent file's path, so an entry reopens with no dialog.
  canReopenRecent: true,
  // The main process opens dropped files (src-tauri/src/open.rs); the webview gets no path to open.
  shellTakesDrops: true,
} as const satisfies Partial<Capabilities>;

/**
 * The desktop Platform over `web`.
 * @param web - The webview's own platform; `createWebPlatform()` by default.
 * @param shell - The Tauri shell, whose open dialog `openFiles` runs, and which keeps the recent
 *   list and watches open files; without one (a browser), these are the web platform's.
 * @returns The web platform with the desktop flags set and their unbuilt methods rejecting with
 *   {@link NotYetSupportedError}.
 */
export function createDesktopPlatform(web: Platform = createWebPlatform(), shell?: DesktopShell): Platform {
  const capabilities: Capabilities = { ...web.capabilities, ...DESKTOP_FLAGS };
  return {
    ...web,
    capabilities,
    // One file at a time until tabs; the main process keeps its path.
    ...(shell === undefined
      ? {}
      : { openFiles: () => pickDocument(shell), recent: desktopRecent(shell), watch: watchOver(shell) }),
    // The desktop flags are always on, so there is nothing for the user to enable.
    howToEnable: (flag) => (flag in DESKTOP_FLAGS ? undefined : web.howToEnable(flag)),
    saveInPlace: (_file: PlatformFile, _bytes: Uint8Array): Promise<SaveResult> =>
      Promise.reject(new NotYetSupportedError("saveInPlace")),
    print: () => Promise.reject(new NotYetSupportedError("print")),
    openWindow: () => Promise.reject(new NotYetSupportedError("openWindow")),
  };
}

/**
 * `Platform.watch` over the main process's watcher, which watches every open document's file by
 * itself: `onChange` runs each time the document `file` stands for reloads from its changed file.
 */
function watchOver(shell: DesktopShell): NonNullable<Platform["watch"]> {
  return (file, onChange) =>
    stopper(
      shell.listen<Reloaded>(RELOADED_EVENT, ({ previous, document }) => {
        // The window rebinds the file to the new document as it shows it, before or after this
        // runs, so either id is this file's.
        const doc = documentOf(file)?.doc;
        if (doc !== undefined && (doc === previous || doc === document.doc)) onChange();
      }),
      "watching a file",
    );
}
