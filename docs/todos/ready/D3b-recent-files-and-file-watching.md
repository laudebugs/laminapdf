# D3b — Recent files (OS and in-app) and reload on external change

| Field | Value |
| --- | --- |
| ID | `D3b` |
| Status | `ready` |
| Plan | [Desktop app §6 D3](../../plans/desktop-app.md#d3-files) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D3a](./D3a-open-from-os.md) |
| Blocks | [D4a](./D4a-windows-tabs-and-session-restore.md), [D5a](./D5a-update-worker-and-adr-0014.md), [E1e](./E1e-save-and-constraint-banner.md), [H4](./H4-document-sync.md) |
| ADRs | Follows ADR 0008 (main process owns file I/O and watching), ADR 0010 (`canWatchFiles`), ADR 0011 (privacy rules, web C2: what may be stored locally) |
| Branch | `feat/d3b-recent-files-and-watching` |
| PR title | `feat(desktop): add recent files and reload documents changed on disk` |
| Size | M–L (500–900 lines) |
| PR | — |

## Goal

Two desktop-only conveniences that power users expect. **Recent files**: every opened document is added to the OS's recent-documents list (macOS Dock/File menu, Windows Jump List/Recent, Linux `recently-used.xbel`) and to an in-app list with "clear", persisted across launches. **File watching**: when an open file changes on disk (another app saves it, a build regenerates it), lamina reloads it keeping the reader's position; when it is deleted or moved, the user is told and the document stays readable from what is already loaded where possible. D3's acceptance criterion — "editing the file externally reloads it keeping the page position" — is met here.

## Context

- [Desktop plan §4 req. 1 and §2 scope](../../plans/desktop-app.md#2-scope-and-non-goals): recent files (OS + in-app list), reopen last session (D4a), file watching with reload.
- [Desktop notes Q3](../../research/architecture/notes/desktop-shell.md#q3-performance-native-rust-over-ipc-vs-wasm-in-webview-bitmaps-1-gb-files-mmap-file-watching): use the `notify` crate in Rust rather than the fs plugin's JS `watch`.
- D2b: `MmapSource` truncation hazard and the remap requirement; `DocumentRegistry` reload entry point.
- `notify` 8 / `notify-debouncer-full` (CC0/MIT/Apache — verify), OS APIs: macOS `NSDocumentController.noteNewRecentDocumentURL` (objc2-app-kit), Windows `SHAddToRecentDocs(SHARD_PATHW)` (windows crate; feeds the Jump List's Recent category when the app is a registered handler), Linux GTK `RecentManager::default().add_item(uri)` (gtk crate already in Tauri's Linux tree).
- Privacy: the in-app list stores paths locally only (never sent anywhere; ADR 0011 rules); "clear" also clears the OS entries lamina added where the API allows.

## Scope

**In scope**

- `src-tauri/src/recent.rs`: `RecentFiles` store (max 20 entries: path, display name, last opened, last page/zoom for D4a) persisted as JSON in `app_config_dir()/recent.json` with atomic write (temp + rename); `add`, `list`, `remove`, `clear`; entries whose file no longer exists shown greyed with "Remove".
- OS integration per platform in `src-tauri/src/recent/os_{macos,windows,linux}.rs`; `clear` calls `NSDocumentController.clearRecentDocuments` (macOS), `SHAddToRecentDocs(SHARD_PIDL, NULL)` (Windows, clears the app's list), `RecentManager::remove_item` for our URIs (Linux).
- Commands (manifest + capability): `recent_list`, `recent_open(id)` (opens by ID — the webview never sends a path), `recent_remove(id)`, `recent_clear`. UI: `@lamina/ui` recent list on the empty state + a "Recent" popover; desktop `Platform.recentFiles` (web already has an OPFS-backed recent list from C4/C7 — share the component, not the storage).
- Settings toggle "Remember recent files" (default on); off clears and stops recording (settings page arrives in D5b/D7b; store the flag now with a temporary toggle in the recent popover).
- `src-tauri/src/watch.rs`: one `notify` recommended watcher via `notify-debouncer-full` (debounce 300 ms) watching the **parent directory** of each open file non-recursively (catches atomic-save rename patterns), matching by file identity (path + inode/file ID where available).
  - On modify/replace: compare size + mtime (+ a fast hash of the first and last 64 KiB) to avoid spurious reloads; remap the file (`MmapSource` new mapping), tell the engine to re-open (`ReopenDocument`), emit `document://reloaded { doc, page_count }`; the UI keeps the current page index and in-page offset (clamped if the page no longer exists) and zoom.
  - On delete/move-away: keep the current mapping only if still valid (Unix: mapping stays valid after unlink; Windows: file can't be deleted while mapped — note the user-visible implication), emit `document://missing`; UI banner "File was moved or deleted" with "Locate…" (opens dialog) and "Close".
  - On rapid successive writes (a build writing in chunks): wait until size/mtime are stable for 2 debounce periods before reloading; if the new file fails to parse, keep the old view and show "File changed but could not be read; retrying", retry on the next change.
- Platform flag `canWatchFiles: true` honoured; setting "Reload when the file changes" (default on).

**Out of scope** (and which todo covers it)

- Session restore (reopen last session): [D4a](D4a-windows-tabs-and-session-restore.md) uses the stored view state.
- "Open Recent" native menu: [D4b](D4b-native-menus.md) (reads `RecentFiles`).
- Settings page: [D5b](D5b-updater-in-app.md) creates it; [D7b](D7b-settings-renderer-switch-and-download-page.md) completes it.
- Watching for the editor's unsaved-changes conflicts: editor plan.

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2` and `rust-best-practices`.
- [ ] Add deps (`notify`, `notify-debouncer-full`, `objc2-app-kit`/`objc2-foundation` macOS, `windows` features `Win32_UI_Shell`, `gtk` Linux) with licence comments; `cargo deny check`.
- [ ] Implement `RecentFiles` with unit tests (ordering, dedupe by canonical path, cap 20, atomic persistence, corrupted JSON → reset with a log warning).
- [ ] OS recent-documents integration per platform (called from the main thread where the API requires it: `run_on_main_thread`).
- [ ] Commands + capability entries; UI components and desktop `Platform` methods.
- [ ] Watcher service: registration on open, unregistration on close, reload flow with the engine protocol message `ReopenDocument { doc, len }` (protocol version bump in `lamina-desktop-engine`).
- [ ] UI: preserve position on `document://reloaded`; banner for missing files; retry state.
- [ ] e2e scenarios: (a) open fixture copy in a temp dir, overwrite it with a different fixture via the test process → UI shows new page count, same page index; (b) atomic replace (write temp + rename) → reload; (c) delete → banner; (d) recent list shows the opened file after restart; "clear" empties it.
- [ ] Manual check per OS: OS recent list shows the file (Dock right-click / File ▸ Open Recent in Finder apps / Jump List / GNOME Files "Recent").

## Acceptance criteria

- [ ] Editing (overwriting) an open file externally reloads it within 1 s on each OS while keeping page index and zoom (e2e).
- [ ] Atomic-save replacements (vim/VS Code style) are detected (e2e).
- [ ] Deleting the open file shows the banner and the app keeps running (e2e).
- [ ] Opened files appear in the OS recent list on each OS (manual check recorded) and in the in-app list across restarts (e2e); "clear" empties both.
- [ ] Disabling "Remember recent files" stops recording and clears the list (unit + e2e).
- [ ] No full paths logged at `info` level or above (log assertion in e2e).

## Tests

- Rust unit tests for `RecentFiles`, change-detection logic (`fn classify_change(old: &FileStamp, new: &FileStamp) -> Change`), and debounce/stability rules (driven with synthetic events, no real fs).
- Rust integration test with a temp dir and the real watcher (marked `#[ignore]` on CI platforms where fs events are unreliable, but run in `desktop-e2e`).
- WebdriverIO scenarios listed above.

## Docs to update

- `docs/plans/desktop-app.md` §9 D3: tick "Recent files…", "Watch open files and reload preserving position", and complete "Integration tests for open-from-OS on each platform".
- ADR 0008: note the watch strategy (parent-dir watching, stability rule) and the Windows "can't delete while mapped" behaviour.
- Privacy page draft (ADR 0011's page, owned by web C7): add "the desktop app stores a local recent-files list; clear it in settings".

## Risks and open questions

- **Windows mapped files can't be deleted or replaced** by other apps while lamina holds the mapping — this could break other apps' saves. Mitigation options: positioned reads instead of mmap on Windows, or `FILE_SHARE_DELETE` + copy-on-change. Decide here (and update ADR 0008); this is a user-visible behaviour.
- **Network shares and cloud-sync folders** (OneDrive/Dropbox placeholders) produce noisy or missing events; stability rule + fallback polling every 5 s for network paths.
- **Very large files rewritten in place** might be read mid-write; the stability rule mitigates; a parse failure keeps the old view.
- `notify`'s macOS FSEvents backend reports directory-level events with latency (~1 s); acceptable within the 1 s target? Measure; widen to 2 s if needed and record.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
