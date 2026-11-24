// Recent files the picker must find again. In the Mac App Store build, App Sandbox lets the app
// reopen a file only through a bookmark; an entry without a usable one stays in the list, marked,
// and pressing it asks for the file in the picker (src-tauri/src/recent.rs). The list says why once.

import type { PlatformStorage, RecentFile } from "@laminapdf/ui";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";

/** Where the webview remembers it has explained why some entries need the picker. */
export const PICKER_NOTE_KEY = "desktop.recent.pickerNoteSeen";

/** The file is there, but only the picker can reach it again. */
export function needsPicker(entry: RecentFile): boolean {
  return entry.hasHandle !== true && entry.missing !== true;
}

/** The entry to open from the palette with none named: the newest that opens with no dialog. */
export function newestReady(entries: readonly RecentFile[]): RecentFile | undefined {
  return entries.find((entry) => entry.hasHandle === true);
}

/** Whether the list should explain itself now: an entry needs the picker, and it hasn't before. */
export function wantsPickerNote(entries: readonly RecentFile[], seen: boolean | undefined): boolean {
  return seen === false && entries.some(needsPicker);
}

/**
 * Clears the recent list in the main process, and with it the memory that the note was shown: the
 * one path every clear takes (the start window's, the palette's, Settings').
 */
export async function clearRecentList(shell: DesktopShell, storage: PlatformStorage): Promise<void> {
  await shell.invoke("recent_clear");
  await storage.delete(PICKER_NOTE_KEY);
}

/**
 * Sets the preferences in the main process. Turning "Remember recent files" off clears the list
 * there, so the note's memory goes too.
 */
export async function setPreferences(
  shell: DesktopShell,
  storage: PlatformStorage,
  was: { readonly rememberRecent: boolean },
  value: { readonly rememberRecent: boolean },
): Promise<void> {
  await shell.invoke("set_preferences", { value });
  if (was.rememberRecent && !value.rememberRecent) await storage.delete(PICKER_NOTE_KEY);
}
