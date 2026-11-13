// The Tauri side of `DesktopShell`: commands, this window's events, the `lamina://` scheme's base
// and the webview's own decoders. The engine client takes it, so its tests can pass a fake.

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { DesktopShell } from "./desktop-engine-client.ts";

/** The Tauri shell this window runs in. */
export function tauriShell(): DesktopShell {
  return {
    invoke: (command, args) => invoke(command, args),
    // This window's own: the events sent to it, and those sent to every window. The global
    // `listen` would also hear what is sent to the other windows (a tab arriving there, their
    // engines' restarts and files).
    listen: (event, handler) =>
      getCurrentWebviewWindow().listen<unknown>(event, (heard: { payload: unknown }) =>
        handler(heard.payload as never),
      ),
    // `lamina://localhost/` on macOS and Linux, `http://lamina.localhost/` on Windows.
    tileBase: convertFileSrc("", "lamina"),
    fetch: (url, init) => fetch(url, init),
    decode: (blob) => createImageBitmap(blob),
    fromPixels: (pixels, width, height) => createImageBitmap(new ImageData(pixels, width, height)),
  };
}
