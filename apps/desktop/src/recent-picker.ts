// Recent files the picker must find again. In the Mac App Store build, App Sandbox lets the app
// reopen a file only through a bookmark; an entry without a usable one stays in the list, marked,
// and pressing it asks for the file in the picker (src-tauri/src/recent.rs). The list says why once.

import type { RecentFile } from "@laminapdf/ui";

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
