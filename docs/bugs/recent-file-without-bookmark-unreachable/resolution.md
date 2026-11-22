# Resolution: a recent file without a bookmark is unreachable in the sandboxed build

[Diagnosis](detail.md).

## The fix

- **A third state.** `recent.rs` works out how each entry is reached: `Ready` (a path that opens), `NeedsPicker` (the file is there, but in the sandbox the entry has no bookmark that resolves) or `Missing`. The state is computed on every listing, not stored, so a `recent.json` from before bookmarks and a bookmark that stops resolving need no migration and no new field. The direct download never reports `NeedsPicker`.
- **Listing.** A `NeedsPicker` entry stays in the list with `hasHandle: false` and `missing: false`. `RecentList` already marks such a row "Choose the file again to reopen".
- **Opening.** `recent_open` opens the file picker for such an entry, starting in the file's folder, and returns nothing when the user cancels. The webview shows a toast first: "Lamina PDF can't reopen {name} on its own. Choose it to open it again." The file the user picks is recorded with a fresh bookmark, so the entry is ready the next time. File ▸ Open Recent lists only entries that open with no dialog; the others are on the home screen.
- **The reason, once.** The start window shows one line under the list the first time an entry needs the picker (the Mac App Store version can reopen a file only after you choose it). The webview's storage remembers it was shown, so it doesn't return on the next launch.

## How it was checked

- `recent` unit tests (21, and 22 with `app-store`): the reach rule for each case, the listing and `way_to_open` in and out of the sandbox, a deleted file, and a `recent.json` from before bookmarks.
- Vitest for the desktop: which entries need the picker, when the note shows, and that a cancelled picker resolves nothing. A browser test of `RecentList` for the row.

## Not checked

The real sandboxed app: this Mac can't run the store build, so the sandbox's refusal of a bare path was not seen, only the rule that follows from it. Where macOS answers `is_file` for a file the sandbox can't read, an unreachable file is listed as needing the picker, which is the intended reading.

## Left alone

A stale bookmark that resolves but leads to a deleted file is still listed as missing, as before.
