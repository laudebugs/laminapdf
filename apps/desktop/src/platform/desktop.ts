// The desktop reader's Platform (ADR 0010, ADR 0008). It starts from the web platform, which works
// in any webview (the clipboard, IndexedDB storage), and reports what the desktop shell adds:
// saving in place, watching files, native printing and several windows. Opening a file goes
// through the main process's dialog, so its path never reaches the webview. Until the shell
// implements the rest, each one rejects with NotYetSupportedError rather than falling back to the
// web behaviour silently, and shared UI never asks which platform it runs on: only these flags.
import { type Capabilities, createWebPlatform, type Platform, type PlatformFile, type SaveResult } from "@laminapdf/ui";
import type { DesktopShell } from "../engine/desktop-engine-client.ts";
import { pickDocument } from "../engine/open.ts";

/** What this build of the desktop app can't do yet: opening a dropped document, and the Platform methods the desktop flags promise. */
export type DesktopFeature = "open" | "saveInPlace" | "print" | "watch" | "openWindow";

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

/** The flags the desktop shell sets on top of the webview's own (desktop plan §5). */
export const DESKTOP_FLAGS = {
  canWriteInPlace: true,
  canWatchFiles: true,
  hasNativePrint: true,
  multiWindow: true,
} as const satisfies Partial<Capabilities>;

/**
 * The desktop Platform over `web`.
 * @param web - The webview's own platform; `createWebPlatform()` by default.
 * @param shell - The Tauri shell, whose open dialog `openFiles` runs; without one (a browser),
 *   `openFiles` is the web platform's.
 * @returns The web platform with the desktop flags set and their methods rejecting until built.
 * @throws {@link NotYetSupportedError} from `watch`; the other unbuilt methods reject with it.
 */
export function createDesktopPlatform(web: Platform = createWebPlatform(), shell?: DesktopShell): Platform {
  const capabilities: Capabilities = { ...web.capabilities, ...DESKTOP_FLAGS };
  return {
    ...web,
    capabilities,
    // One file at a time until tabs (D4a); the main process keeps its path.
    ...(shell === undefined ? {} : { openFiles: () => pickDocument(shell) }),
    // The desktop flags are always on, so there is nothing for the user to enable.
    howToEnable: (flag) => (flag in DESKTOP_FLAGS ? undefined : web.howToEnable(flag)),
    saveInPlace: (_file: PlatformFile, _bytes: Uint8Array): Promise<SaveResult> =>
      Promise.reject(new NotYetSupportedError("saveInPlace")),
    print: () => Promise.reject(new NotYetSupportedError("print")),
    watch: () => {
      throw new NotYetSupportedError("watch");
    },
    openWindow: () => Promise.reject(new NotYetSupportedError("openWindow")),
  };
}
