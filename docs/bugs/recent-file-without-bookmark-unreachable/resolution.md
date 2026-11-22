# Resolution: a recent file without a bookmark is unreachable in the sandboxed build

[Diagnosis](detail.md).

## The fix

- **A third state.** `recent.rs` works out how each entry is reached: `Ready` (a path that opens), `NeedsPicker` (in the sandbox, the entry has no bookmark that resolves) or `Missing` (a file that is gone). The state is computed on every listing, not stored. A `recent.json` from before bookmarks, or a bookmark that stops resolving, therefore needs no migration and no new field. The direct download never reports `NeedsPicker`.
- **No disk check in the sandbox.** With no resolved bookmark, the sandbox build reports `NeedsPicker` without looking at the path: the app may not be allowed to stat it, and a denied stat would otherwise show "File not found" and Remove. The picker reports a file that is really gone.
- **Listing.** A `NeedsPicker` entry stays in the list with `hasHandle: false` and `missing: false`. `RecentList` already marks such a row "Choose the file again to reopen".
- **Opening.** `recent_open` opens the file picker for such an entry, starting in the file's folder, and returns nothing when the user cancels. A second press while a window's picker is open opens no second one. The file the user picks is recorded with a fresh bookmark, so the entry is ready the next time. File ▸ Open Recent lists only entries that open with no dialog; the others are on the home screen.
- **The reason, once.** The start window shows one line under the list the first time an entry needs the picker. The webview's storage remembers it was shown, so it does not return on the next launch. Clearing the recent list forgets that, so the note can show again.

## How it was checked

- `recent` unit tests: 28 pass, and 29 with `app-store`. They cover the reach rule for each case (a moved file, a deleted file, a path alone, with and without the sandbox), the listing and `way_to_open`, an entry with no parent folder, special characters and a non-UTF-8 path, one picker at a time per window, and a `recent.json` from before bookmarks.
- Vitest for the desktop: which entries need the picker, when the note shows, and that a cancelled picker resolves nothing. A browser test of `RecentList` for the row.

## Not checked

The real sandboxed app: this Mac can't run the store build. Where macOS answers `is_file` for a file the sandbox can't read, the sandbox build never asks, so a file it can't see is listed as needing the picker.

## Left alone

- **`needsPicker` in TypeScript** re-derives, from `hasHandle` and `missing`, what Rust's `Reach` knows. A `reach` field on the listing would change the shared `RecentFile` type, which the web app also uses, so the two stay as they are.
- **The note's once-only behaviour** is tested through its decision (`wantsPickerNote`), not as a component: the desktop's tests run in Node, with no React renderer.
- A sandbox entry whose file was deleted is listed as needing the picker, not as missing: the picker is where the user learns it is gone.
