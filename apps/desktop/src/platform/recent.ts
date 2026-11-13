// The desktop's recent files (src-tauri/src/recent.rs): the main process keeps the list, records
// each document as it opens and adds it to the OS's own list, so the webview only reads it, names
// an entry by its id to reopen or remove it, and clears it. No path reaches the webview.

import type { PlatformFile, RecentFile, RecentFiles } from "@laminapdf/ui";
import { type DesktopDocument, type DesktopShell, engineError } from "../engine/desktop-engine-client.ts";
import { desktopFile, failedFile } from "../engine/open.ts";

/**
 * The recent list over `shell`. `add` records nothing: the main process already did when the
 * document opened. `reopen` opens the entry in this window; a file it can't find rejects (the
 * reader says so in a toast), and one it found but couldn't open stands for its failure.
 */
export function desktopRecent(shell: DesktopShell): RecentFiles {
  return {
    list: () => shell.invoke<RecentFile[]>("recent_list"),
    add: () => Promise.resolve(""),
    remove: (id) => shell.invoke("recent_remove", { id }),
    clear: () => shell.invoke("recent_clear"),
    reopen: async (id): Promise<PlatformFile> => {
      try {
        return desktopFile(await shell.invoke<DesktopDocument>("recent_open", { id }));
      } catch (caught) {
        const error = engineError(caught);
        // Missing, or open already (the main process showed its tab): nothing opens here.
        if (error.code === "io" || error.code === "already-open") throw error;
        return failedFile("", error);
      }
    },
  };
}
