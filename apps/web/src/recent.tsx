// Recent files (C7b; R01, plan §5): each document the reader opens is listed on the home screen,
// in this browser only. Local files are remembered unless the user turns that off; linked files
// only when the user turns "Remember remote URLs" on, since an address can say more than a name.
// Pressing an entry reopens it: a linked one from its address, a local one from the handle kept
// where the browser allows (Chromium), else from the picker. The "Storage and privacy" section of
// the Settings dialog holds both choices and the button that clears the list and every cached copy.

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

/** Where the choices are kept, in this browser only. */
export const STORAGE_KEY = "lamina.storage";

/** What the user chose to remember. */
export interface StorageChoices {
  /** List local files (default on). */
  readonly rememberLocal: boolean;
  /** List linked files by their address (default off). */
  readonly rememberRemote: boolean;
}

const DEFAULT_CHOICES: StorageChoices = { rememberLocal: true, rememberRemote: false };

/** The stored choices, or the defaults where storage is blocked, empty or damaged. */
export function parseChoices(stored: string | null): StorageChoices {
  try {
    const value = JSON.parse(stored ?? "{}") as Partial<Record<keyof StorageChoices, unknown>>;
    return {
      rememberLocal: typeof value.rememberLocal === "boolean" ? value.rememberLocal : DEFAULT_CHOICES.rememberLocal,
      rememberRemote: typeof value.rememberRemote === "boolean" ? value.rememberRemote : DEFAULT_CHOICES.rememberRemote,
    };
  } catch {
    return DEFAULT_CHOICES;
  }
}

function readChoices(): StorageChoices {
  try {
    return parseChoices(localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_CHOICES;
  }
}

function saveChoices(choices: StorageChoices): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(choices));
  } catch {
    // Storage blocked: the choice lasts until the tab closes.
  }
}

/** Whether a document opened from `url` (undefined for a local file) may be listed. */
export function mayRemember(choices: StorageChoices, url: string | undefined): boolean {
  return url === undefined ? choices.rememberLocal : choices.rememberRemote;
}

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
    const url = remoteUrlOf(file);
    if (!mayRemember(choices, url)) return;
    platform.recent
      .add(file, { ...(pages > 0 ? { pages } : {}), ...(url === undefined ? {} : { url }) })
      .then(refresh, reportError);
  }, [file, pages, choices, platform, refresh]);
}

/** Reopens `entry`: from its address, its kept handle, or the picker. */
async function reopen(entry: RecentFile, platform: Platform, engine: Engine, reader: ReaderController) {
  if (entry.url !== undefined) return engine.openRemote(entry.url);
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
      <Switch isSelected={choices.rememberRemote} onChange={(on) => onChange({ ...choices, rememberRemote: on })}>
        {message("storage.rememberRemote")}
      </Switch>
      <p className="web-settings-help">{message("storage.rememberRemote.help")}</p>
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
    // Turning a choice off forgets what it had listed, so nothing it allowed outlives it.
    const forget = entries.filter((e) => !mayRemember(next, e.url));
    void Promise.all(forget.map((e) => platform.recent.remove(e.id))).then(refresh, reportError);
  };
  useSettingsSection(registry, "storage", message("storage.title"), () => (
    <StorageSettings
      choices={choices}
      onChange={change}
      onClear={() => void clear(true).then(() => toasts.add({ message: message("storage.cleared") }), reportError)}
    />
  ));
  const open = (entry: RecentFile) => void reopen(entry, platform, engine, engine.reader).catch(reportError);
  return {
    home: <RecentList entries={entries} onOpen={open} onClear={() => void clear(false).catch(reportError)} />,
    actions: {
      "file.clear-recent": () => void clear(false).catch(reportError),
      "file.open-recent": (id) => {
        const entry = entries.find((e) => e.id === id) ?? entries[0];
        if (entry !== undefined) open(entry);
      },
    },
  };
}
