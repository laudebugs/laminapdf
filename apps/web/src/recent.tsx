// Recent files (R01, plan §5): each local document the reader opens is listed on the home screen,
// in this browser only, unless the user turns that off. A linked document is never listed: ADR
// 0011 keeps its address out of storage and the recent list. Pressing an entry reopens it from the
// handle kept where the browser allows (Chromium), else from the picker. The document open when
// the app last closed comes first, as Continue reading, with the page its reader stopped at
// (reading/positions.ts); every row says that page where one is kept. The "Storage and privacy"
// section of the Settings dialog holds the choices, the button that clears the list and every
// cached copy, and Forget reading positions.

import {
  Button,
  type CommandId,
  type ContinueReading,
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
import { type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { SESSION_KEY, stoppedAt } from "./reading/positions.ts";
import { type Engine, remoteUrlOf } from "./session.ts";
import { readChoices, type StorageChoices, saveChoices } from "./storage-choices.ts";

/** What the hook gives the app. */
export interface Recent {
  /** The home screen's list. */
  readonly home: ReactNode;
  /** `file.open-recent`, `file.clear-recent` and `file.clear-reading-positions`. */
  readonly actions: Readonly<Partial<Record<CommandId, (args: unknown) => void>>>;
}

/** What the home screen lists: the entries, the page each stopped at, and the one to continue. */
interface Listing {
  readonly entries: readonly RecentFile[];
  readonly stoppedAt: ReadonlyMap<string, number>;
  readonly continueReading: ContinueReading | undefined;
}

const NOTHING_LISTED: Listing = { entries: [], stoppedAt: new Map(), continueReading: undefined };

/** The listing from the platform's stores: each entry's kept position, and the session record's entry. */
async function readListing(platform: Platform): Promise<Listing> {
  const entries = await platform.recent.list();
  const kept = await Promise.all(
    entries.map(async (entry) => {
      if (entry.fingerprint === undefined) return undefined;
      return stoppedAt(await platform.positions.get(entry.fingerprint).catch(() => undefined));
    }),
  );
  const pages = new Map<string, number>();
  entries.forEach((entry, i) => {
    const at = kept[i];
    if (at !== undefined) pages.set(entry.id, at.page);
  });
  const session = (await platform.storage.get(SESSION_KEY).catch(() => undefined)) as
    | { recentId?: unknown }
    | undefined;
  const index = entries.findIndex((entry) => entry.id === session?.recentId);
  const entry = entries[index];
  const at = kept[index];
  return {
    entries,
    stoppedAt: pages,
    continueReading: entry === undefined || at === undefined ? undefined : { entry, ...at },
  };
}

/** The listing, kept fresh after each change here. */
function useListing(platform: Platform): [Listing, () => void] {
  const [listing, setListing] = useState<Listing>(NOTHING_LISTED);
  const refresh = useCallback(() => {
    readListing(platform).then(setListing, reportError);
  }, [platform]);
  useEffect(refresh, [refresh]);
  return [listing, refresh];
}

/**
 * Lists each document as it opens, when the choices allow, and keeps the session record: the
 * listed document open now, which comes back first at the next start. A document that isn't
 * listed, or closing one, removes the record.
 */
function useRecording(
  platform: Platform,
  engine: Engine,
  state: ReaderState,
  choices: StorageChoices,
  refresh: () => void,
): void {
  const file = state.phase === "open" ? state.file : undefined;
  const pages = state.phase === "open" ? engine.getState().pages.length : 0;
  const fingerprint = state.phase === "open" ? engine.getState().fingerprint : undefined;
  // Only a document closing removes the record: the home screen at start shows it as Continue reading.
  const wasOpen = useRef(false);
  const closed = state.phase === "empty";
  useEffect(() => {
    // The home screen shows again: its pages are read afresh, after the closing document's last save.
    if (closed && wasOpen.current) platform.storage.delete(SESSION_KEY).then(refresh, reportError);
    wasOpen.current = state.phase === "open";
  }, [closed, state.phase, platform, refresh]);
  useEffect(() => {
    if (file === undefined) return;
    // A linked document's address is never kept (ADR 0011), and a local one only when allowed.
    if (remoteUrlOf(file) !== undefined || !choices.rememberLocal) {
      void platform.storage.delete(SESSION_KEY).catch(reportError);
      return;
    }
    // The document closed (or another opened) before its entry was written: no record for it.
    let current = true;
    const details = { ...(pages > 0 ? { pages } : {}), ...(fingerprint === undefined ? {} : { fingerprint }) };
    const record = async () => {
      const recentId = await platform.recent.add(file, details);
      if (!current) return;
      // Coming back to the last document is part of remembering where the reader stopped.
      if (choices.rememberPositions) await platform.storage.set(SESSION_KEY, { recentId });
      else await platform.storage.delete(SESSION_KEY);
    };
    record().then(refresh, reportError);
    return () => {
      current = false;
    };
  }, [file, pages, fingerprint, choices, platform, refresh]);
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
  onForgetPositions,
}: {
  choices: StorageChoices;
  onChange: (choices: StorageChoices) => void;
  onClear: () => void;
  onForgetPositions: () => void;
}): ReactNode {
  return (
    <div className="web-storage-settings">
      <Switch isSelected={choices.rememberLocal} onChange={(on) => onChange({ ...choices, rememberLocal: on })}>
        {message("storage.rememberLocal")}
      </Switch>
      <p className="web-settings-help">{message("storage.rememberLocal.help")}</p>
      <Switch isSelected={choices.rememberPositions} onChange={(on) => onChange({ ...choices, rememberPositions: on })}>
        {message("storage.rememberPositions")}
      </Switch>
      <p className="web-settings-help">{message("storage.rememberPositions.help")}</p>
      <Button onPress={onForgetPositions}>{message("storage.forgetPositions")}</Button>
      <p className="web-settings-help">{message("storage.forgetPositions.help")}</p>
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
  const [listing, refresh] = useListing(platform);
  const { entries } = listing;
  useRecording(platform, engine, state, choices, refresh);
  const clear = useCallback(
    async (all: boolean) => {
      await platform.recent.clear();
      await platform.storage.delete(SESSION_KEY);
      if (all) {
        await clearCache();
        await platform.positions.clear();
      }
      refresh();
    },
    [platform, clearCache, refresh],
  );
  const forgetPositions = useCallback(async () => {
    await platform.positions.clear();
    await platform.storage.delete(SESSION_KEY);
    refresh();
  }, [platform, refresh]);
  const forgetWithToast = () =>
    void forgetPositions().then(() => toasts.add({ message: message("storage.positionsForgotten") }), reportError);
  const change = (next: StorageChoices) => {
    setChoices(next);
    saveChoices(next);
    // Turning it off forgets the list and the kept copies, so nothing it allowed outlives it (a
    // copy open now goes when its document closes, or at the next Clear).
    if (!next.rememberLocal) void clear(true).catch(reportError);
    // Turning positions off forgets every one, and none is written while it stays off.
    if (!next.rememberPositions) void forgetPositions().catch(reportError);
  };
  useSettingsSection(
    registry,
    "storage",
    message("storage.title"),
    () => (
      <StorageSettings
        choices={choices}
        onChange={change}
        onClear={() => void clear(true).then(() => toasts.add({ message: message("storage.cleared") }), reportError)}
        onForgetPositions={forgetWithToast}
      />
    ),
    [choices],
  );
  const open = (entry: RecentFile) => void reopen(entry, platform, engine.reader).catch(reportError);
  return {
    home: (
      <RecentList
        entries={entries}
        stoppedAt={listing.stoppedAt}
        continueReading={choices.rememberPositions ? listing.continueReading : undefined}
        onOpen={open}
        onClear={() => void clear(false).catch(reportError)}
      />
    ),
    actions: {
      "file.clear-recent": () => void clear(false).catch(reportError),
      "file.clear-reading-positions": forgetWithToast,
      "file.open-recent": (id) => {
        // From the palette, with no entry named: the newest. An id the list lacks opens nothing.
        const entry = id === undefined ? entries[0] : entries.find((e) => e.id === id);
        if (entry !== undefined) open(entry);
      },
    },
  };
}
