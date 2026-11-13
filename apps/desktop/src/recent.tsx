// Recent files on the start window (S01): the main process records each document as it opens and
// adds it to the OS's list; this lists them, newest first, reopens one by its id with no dialog,
// removes an entry whose file is gone and clears the list. Turned off in Settings, the list gives
// way to one line saying so. The command palette's Open recent and Clear recent work here too.

import {
  type CommandId,
  message,
  type Platform,
  type ReaderController,
  type ReaderState,
  type RecentFile,
  RecentList,
  type ToastQueue,
} from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useState } from "react";

/** What the hook gives the app. */
export interface DesktopRecent {
  /** The start window's list (or the line saying it is off) and the privacy line. */
  readonly home: ReactNode;
  /** `file.open-recent` and `file.clear-recent`. */
  readonly actions: Readonly<Partial<Record<CommandId, (args: unknown) => void>>>;
  /** Reads the list again (after the preferences change). */
  readonly refresh: () => void;
}

/**
 * The recent list of `platform`, read again whenever the window opens or closes a document (the
 * main process records it as it opens). `remember` is the user's "Remember recent files".
 */
export function useDesktopRecent(
  platform: Platform,
  reader: ReaderController,
  state: ReaderState,
  toasts: ToastQueue,
  remember: boolean,
): DesktopRecent {
  const [entries, setEntries] = useState<readonly RecentFile[]>([]);
  const refresh = useCallback(() => {
    platform.recent.list().then(setEntries, reportError);
  }, [platform]);
  // A document that opened was just recorded; one that closed shows the start window again.
  useEffect(refresh, [refresh, state.phase]);
  const open = useCallback(
    (entry: RecentFile) => {
      reopen(entry, platform, reader).catch(() => {
        toasts.add({ message: message("reader.open.missing", { name: entry.name }) });
        refresh();
      });
    },
    [platform, reader, toasts, refresh],
  );
  const remove = (entry: RecentFile) => void platform.recent.remove(entry.id).then(refresh, reportError);
  const clear = () => void platform.recent.clear().then(refresh, reportError);
  const home = remember ? (
    <>
      <RecentList entries={entries} onOpen={open} onClear={clear} onRemove={remove} />
      {entries.length === 0 ? null : <p className="desktop-privacy">{message("recent.privacy")}</p>}
    </>
  ) : (
    <p className="desktop-privacy">{message("recent.off")}</p>
  );
  return {
    home,
    refresh,
    actions: {
      "file.clear-recent": clear,
      "file.open-recent": (id) => {
        // From the palette, with no entry named: the newest that is still there.
        const entry = id === undefined ? entries.find((e) => e.missing !== true) : entries.find((e) => e.id === id);
        if (entry !== undefined) open(entry);
      },
    },
  };
}

/** Opens `entry` in this window, from the path the main process kept. */
async function reopen(entry: RecentFile, platform: Platform, reader: ReaderController): Promise<void> {
  const file = await platform.recent.reopen?.(entry.id);
  if (file !== undefined) await reader.open(file);
}

function reportError(error: unknown): void {
  console.error("the recent files list failed", error);
}
