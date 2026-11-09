// Recent files (R01, plan §5): each local document the reader opens is listed on the home screen,
// in this browser only, unless the user turns that off. A linked document is never listed: ADR
// 0011 keeps its address out of storage and the recent list. Pressing an entry reopens it from the
// handle kept where the browser allows (Chromium), else from the picker. The "Storage and
// privacy" section of the Settings dialog holds the choice and the button that clears the list
// and every cached copy.

import {
  Button,
  type CommandId,
  message,
  type Platform,
  type ReaderController,
  type ReaderState,
  type RecentFile,
  RecentList,
  type SettingsRegistry,
  Switch,
  type ToastQueue,
  useSettingsSection,
} from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import { type Engine, remoteUrlOf } from "./session.ts";
import { readChoices, type StorageChoices, saveChoices } from "./storage-choices.ts";

/** What the hook gives the app. */
export interface Recent {
  /** The home screen's list. */
  readonly home: ReactNode;
  /** `file.open-recent` and `file.clear-recent`. */
  readonly actions: Readonly<Partial<Record<CommandId, (args: unknown) => void>>>;
}

/** The entries in the platform's store, kept fresh after each change here. */
function useEntries(platform: Platform): [readonly RecentFile[], () => void] {
  const [entries, setEntries] = useState<readonly RecentFile[]>([]);
  const refresh = useCallback(() => {
    platform.recent.list().then(setEntries, reportError);
  }, [platform]);
  useEffect(refresh, [refresh]);
  return [entries, refresh];
}

/** Lists each document as it opens, when the choices allow. */
function useRecording(
  platform: Platform,
  engine: Engine,
  state: ReaderState,
  choices: StorageChoices,
  refresh: () => void,
): void {
  const file = state.phase === "open" ? state.file : undefined;
  const pages = state.phase === "open" ? engine.getState().pages.length : 0;
  useEffect(() => {
    if (file === undefined) return;
    // A linked document's address is never kept (ADR 0011), and a local one only when allowed.
    if (remoteUrlOf(file) !== undefined || !choices.rememberLocal) return;
    platform.recent.add(file, pages > 0 ? { pages } : {}).then(refresh, reportError);
  }, [file, pages, choices, platform, refresh]);
}

/** Reopens `entry`: from its kept handle, or the picker. */
async function reopen(entry: RecentFile, platform: Platform, reader: ReaderController) {
  const file = entry.hasHandle === true ? await platform.recent.reopen?.(entry.id) : undefined;
  return file === undefined ? reader.openFromPicker() : reader.open(file);
}

/** The Storage and privacy section of the Settings dialog. */
function StorageSettings({
  choices,
  onChange,
  onClear,
}: {
  choices: StorageChoices;
  onChange: (choices: StorageChoices) => void;
  onClear: () => void;
}): ReactNode {
  return (
    <div className="web-storage-settings">
      <Switch isSelected={choices.rememberLocal} onChange={(on) => onChange({ ...choices, rememberLocal: on })}>
        {message("storage.rememberLocal")}
      </Switch>
      <p className="web-settings-help">{message("storage.rememberLocal.help")}</p>
      <Button onPress={onClear}>{message("storage.clear")}</Button>
      <p className="web-settings-help">{message("storage.clear.help")}</p>
    </div>
  );
}

/**
 * Recent files: recorded as documents open, listed on the home screen, reopened on a press, and
 * the Storage and privacy settings; `clearCache` empties the cached copies of documents too.
 */
export function useRecent(
  platform: Platform,
  engine: Engine,
  state: ReaderState,
  registry: SettingsRegistry,
  toasts: ToastQueue,
  clearCache: () => Promise<void>,
): Recent {
  const [choices, setChoices] = useState(readChoices);
  const [entries, refresh] = useEntries(platform);
  useRecording(platform, engine, state, choices, refresh);
  const clear = useCallback(
    async (all: boolean) => {
      await platform.recent.clear();
      if (all) await clearCache();
      refresh();
    },
    [platform, clearCache, refresh],
  );
  const change = (next: StorageChoices) => {
    setChoices(next);
    saveChoices(next);
    // Turning it off forgets what it had listed, so nothing it allowed outlives it.
    if (!next.rememberLocal) void platform.recent.clear().then(refresh, reportError);
  };
  useSettingsSection(registry, "storage", message("storage.title"), () => (
    <StorageSettings
      choices={choices}
      onChange={change}
      onClear={() => void clear(true).then(() => toasts.add({ message: message("storage.cleared") }), reportError)}
    />
  ));
  const open = (entry: RecentFile) => void reopen(entry, platform, engine.reader).catch(reportError);
  return {
    home: <RecentList entries={entries} onOpen={open} onClear={() => void clear(false).catch(reportError)} />,
    actions: {
      "file.clear-recent": () => void clear(false).catch(reportError),
      "file.open-recent": (id) => {
        // From the palette, with no entry named: the newest. An id the list lacks opens nothing.
        const entry = id === undefined ? entries[0] : entries.find((e) => e.id === id);
        if (entry !== undefined) open(entry);
      },
    },
  };
}
