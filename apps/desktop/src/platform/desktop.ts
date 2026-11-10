// The desktop reader's Platform (ADR 0010, ADR 0008). It starts from the web platform, which works
// in any webview (pickers, the clipboard, IndexedDB storage), and reports what the desktop shell
// adds: saving in place, watching files, native printing and several windows. Until the todos that
// build those land, each one rejects with NotYetSupportedError, never a silent web fallback, and
// shared UI never asks which platform it runs on: only these flags.
import { type Capabilities, createWebPlatform, type Platform, type PlatformFile, type SaveResult } from "@laminapdf/ui";

/** A desktop capability whose todo hasn't landed yet: the UI can show what is coming instead of failing. */
export class NotYetSupportedError extends Error {
  override readonly name = "NotYetSupportedError";
  /** The Platform method that was called. */
  readonly feature: string;
  /** The todo that builds it (docs/todos). */
  readonly todo: string;

  constructor(feature: string, todo: string) {
    super(`${feature} isn't in this build of Lamina PDF yet (${todo})`);
    this.feature = feature;
    this.todo = todo;
  }
}

/** The flags the desktop shell sets on top of the webview's own (desktop plan §5). */
export const DESKTOP_FLAGS = {
  canWriteInPlace: true,
  canWatchFiles: true,
  hasNativePrint: true,
  multiWindow: true,
} as const satisfies Partial<Capabilities>;

/** The desktop Platform over `web` (the web platform of this webview by default). */
export function createDesktopPlatform(web: Platform = createWebPlatform()): Platform {
  const capabilities: Capabilities = { ...web.capabilities, ...DESKTOP_FLAGS };
  return {
    ...web,
    capabilities,
    // The desktop flags are always on, so there is nothing for the user to enable.
    howToEnable: (flag) => (flag in DESKTOP_FLAGS ? undefined : web.howToEnable(flag)),
    saveInPlace: (_file: PlatformFile, _bytes: Uint8Array): Promise<SaveResult> =>
      Promise.reject(new NotYetSupportedError("saveInPlace", "D3b")),
    print: () => Promise.reject(new NotYetSupportedError("print", "D4c")),
    watch: () => {
      throw new NotYetSupportedError("watch", "D3b");
    },
    openWindow: () => Promise.reject(new NotYetSupportedError("openWindow", "D4a")),
  };
}
