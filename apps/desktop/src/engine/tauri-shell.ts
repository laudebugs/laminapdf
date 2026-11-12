// The Tauri side of `DesktopShell`: commands, this window's events, the `lamina://` scheme's base
// and the webview's own decoders. The engine client takes it, so its tests can pass a fake.

import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DesktopShell } from "./desktop-engine-client.ts";

/** The Tauri shell this window runs in. */
export function tauriShell(): DesktopShell {
  return {
    invoke: (command, args) => invoke(command, args),
    listen: (event, handler) => listen(event, ({ payload }) => handler(payload as never)),
    // `lamina://localhost/` on macOS and Linux, `http://lamina.localhost/` on Windows.
    tileBase: convertFileSrc("", "lamina"),
    fetch: (url, init) => fetch(url, init),
    decode: (blob) => createImageBitmap(blob),
    fromPixels: (pixels, width, height) => createImageBitmap(new ImageData(pixels, width, height)),
  };
}
