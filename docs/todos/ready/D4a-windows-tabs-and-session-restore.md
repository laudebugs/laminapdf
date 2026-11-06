# D4a — Multiple windows, tabs, tab drag between windows, session restore

| Field | Value |
| --- | --- |
| ID | `D4a` |
| Status | `ready` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D3b](./D3b-recent-files-and-file-watching.md) |
| ADRs | Follows ADR 0008 (one engine per window; revisit pooling with numbers from here), ADR 0010 (`multiWindow: true`) |
| Branch | `feat/d4a-windows-and-tabs` |
| PR title | `feat(desktop): add multiple windows, tabs and session restore` |
| Size | L (700–1,400 lines) |
| PR | — |

## Goal

Let power users work with many documents: each window holds tabs, a tab can be dragged out into a new window or onto another window's tab strip, File ▸ New Window opens an empty window, and on relaunch lamina restores the last session (windows, their geometry, tabs, and each tab's page and zoom). Requirement 5 of the [desktop plan](../../plans/desktop-app.md#functional). It also answers the plan's open question 3 (engine per window vs pool) with real memory numbers.

## Context

- **Design (Paper):** [S02 Window with document tabs and session restore](../../design/paper/desktop-shell.md#s02-window-with-document-tabs-and-session-restore). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [Desktop plan §4 req. 5](../../plans/desktop-app.md#functional) and §6 D4.
- Tauri: `WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))`, `app.webview_windows()`, `Emitter::emit_to(EventTarget::webview_window(label), …)`, `WindowEvent::CloseRequested { api }` (`api.prevent_close()`), capabilities targeting windows by glob (`"windows": ["main", "doc-*"]`), [`tauri-plugin-window-state`](https://v2.tauri.app/plugin/window-state/) for geometry (`StateFlags`).
- D2b: `EngineSupervisor` per window label, `DocumentRegistry` (trusted path sources); D3a: open routing; D3b: the "Remember recent files" privacy setting (session restore does **not** read `RecentFiles` entries — see Scope).
- `@laminapdf/ui` (web C4): tab strip doesn't exist on web (`multiWindow: false`); add a shared `Tabs` component rendered only when the `Platform` reports `multiWindow` — no `isTauri` checks.

## Scope

**In scope**

- Window model in Rust (`src-tauri/src/windows.rs`): `WindowRegistry` (label `doc-<n>` → tabs `[DocId]`, active tab), creation (`new_window(tabs?)`), focus tracking ("frontmost window" for open routing), close handling (closing the last tab closes the window on Windows/Linux; on macOS the app stays running with no windows and the Dock/Menu reopen a window on `RunEvent::Reopen`).
- Tabs: open routing from D3a now opens a **new tab** in the frontmost window (setting: "Open files in new tabs / new windows"); tab close, reorder, next/previous (`Ctrl+Tab`, `Ctrl+Shift+Tab`; `Cmd+Shift+]`/`[` on macOS — wired in D4b), duplicate detection (re-focus the existing tab when the same file is opened twice; setting to allow duplicates off by default).
- Engine affinity: documents move with their tab between windows; since engines are per window (ADR 0008), moving a tab re-opens the document in the target window's engine and transfers the view state (page, offset, zoom). **Encrypted documents**: a move keeps the same `DocId` and its registry entry, including the held `SessionPassword` (D2b) — it is a re-home, not a close. Order: (1) set the entry's owning window to the target; (2) send `OpenDocument` and, if a `SessionPassword` is held, `UnlockDocument` to the target engine; (3) only after `Unlocked` (or `Opened` for unencrypted files) send `CloseDocument` to the source engine, as an engine-only close that does **not** go through the registry's close path, so the password is not zeroised. If the target unlock fails, roll back (owner stays the source, tab stays) and show an error. The user is never re-prompted for a moved tab. Windows closing during a move wait until the move is done. Measure memory with 1, 5, 20 documents across 1 vs 4 windows; record in ADR 0008 and answer open question 3.
- Tab drag between windows: HTML5 drag inside a window for reorder; dragging outside the window bounds (`dragend` with screen coordinates outside) calls `detach_tab(doc, screen_x, screen_y)` → new window at that position; dropping onto another window's strip: the source sends `tab_drag_start(doc)`, the target window detects pointer entry (via `drag-over` events from Rust, which tracks the cursor using window positions) — implement the simplest reliable approach found and document it; fallback: "Move tab to window ▸" context menu (always available, accessible).
- Session restore: on quit (and every 30 s while running, debounced), persist `session.json` (schema-versioned, atomic write: temp + rename) in `app_config_dir()`; on launch, restore if the setting "Reopen last session" is on (default on); missing files restore as "missing" tabs with "Locate…".
  - Windows → geometry handled by `tauri-plugin-window-state`; `session.json` stores window order and labels, each window's tabs and active tab.
  - **Each tab stores its own document reference, independent of the recent list** (the user can clear or disable recent files at any time, and the recent list is capped at 20): `{ path, stamp: { len, mtime }, bookmark?, view: { page, offset, zoom } }`. `path` is the canonical absolute path. On macOS `bookmark` is a regular (not security-scoped — the app isn't sandboxed) `NSURL` bookmark (`bookmarkDataWithOptions:` via `objc2-foundation`) so a file moved or renamed between sessions is still found; it is resolved first and `path` is the fallback; a stale bookmark is refreshed. Windows/Linux store the path only. Restored paths enter `DocumentRegistry` as a trusted source (written by the main process, never by the webview).
  - **Privacy rule (decided)**: `session.json` is a record of document paths just like the recent list, so it follows the same switch. When "Remember recent files" (D3b) is **off**, lamina writes no session file and deletes an existing one; "Reopen last session" is shown disabled with the note "Needs 'Remember recent files'". Turning "Reopen last session" off also deletes `session.json`. "Clear recent files" clears the recent list only — it doesn't close or forget the documents open now, so the current session is still saved while remembering is on.
- Window titles: `"<doc title or file name> — Lamina"`; macOS `represented_filename`/proxy icon if Tauri exposes it (else skip), "edited" dot not used (reader).
- Capabilities: `capabilities/main.json` `windows` glob covers all `doc-*` labels; no window gets more permissions than another.

**Out of scope** (and which todo covers it)

- Native menus and accelerators (New Window, Close Tab, Move Tab to Window, Window menu list): [D4b](D4b-native-menus.md).
- Printing: [D4c](D4c-native-printing.md).
- Shared engine pool, if numbers argue for it: a follow-up todo (raise it in the PR if needed).

## Tasks

- [ ] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
- [ ] Add `tauri-plugin-window-state` (licence check); configure `StateFlags::SIZE | POSITION | MAXIMIZED` and skip restoring off-screen positions (monitor changes).
- [ ] `WindowRegistry` + commands (manifest + capability): `new_window`, `close_tab`, `activate_tab`, `move_tab(doc, target_window | new)`, `detach_tab(doc, x, y)`, `reorder_tabs(order)`, `session_state()`.
- [ ] Per-window engine lifecycle: spawn on window creation, shut down on close; document hand-over on moves (close in source engine after the target reports `Opened`).
- [ ] Shared `Tabs` component in `@laminapdf/ui` (keyboard accessible: roving tabindex, `role="tablist"`, close buttons with labels), desktop-only via `Platform.multiWindow`.
- [ ] Session persistence and restore (`src-tauri/src/session.rs`: `SessionFile`, `TabRef { path, stamp, bookmark, view }`, `fn load(dir) -> Result<Option<SessionFile>, SessionError>`, `fn save(dir, &SessionFile)`, `fn resolve(tab: &TabRef) -> Resolved { Found(PathBuf) | Moved(PathBuf) | Missing }`), the macOS bookmark helper, and the privacy rule above; `RunEvent::Reopen` on macOS; `RunEvent::ExitRequested` flushes the session.
- [ ] Memory measurements; ADR 0008 amendment answering open question 3.
- [ ] e2e scenarios: open 3 files → 3 tabs; move tab to new window (via context-menu command, deterministic); restart app → same windows/tabs/pages restored; close all tabs → behaviour per OS.
- [ ] Manual checklist: real drag-out and drag-between windows on each OS (Wayland quirks noted).

## Acceptance criteria

- [ ] Opening three PDFs results in three tabs in one window; opening an already-open PDF focuses its tab (e2e).
- [ ] "Move tab to new window" and "Move tab to window ▸ <name>" keep the page and zoom (e2e).
- [ ] Moving an unlocked **encrypted** document's tab to another window shows it rendered in the target window with no password prompt, and closing the source window afterwards does not lock it (e2e with an encrypted fixture; registry unit test that the `SessionPassword` survives the move and is zeroised only when the document is finally closed).
- [ ] Drag-out to a new window works on macOS, Windows and Linux X11 (manual check recorded); Wayland behaviour recorded (position may be compositor-controlled).
- [ ] Quit + relaunch restores windows (geometry), tabs and per-tab page/zoom (e2e); disabling "Reopen last session" starts empty.
- [ ] Session restore works after the recent list is cleared (e2e: open 3 files, "Clear recent files", quit, relaunch → the 3 tabs come back).
- [ ] With "Remember recent files" off, quitting leaves no `session.json` in `app_config_dir()` and relaunch starts empty (e2e + unit).
- [ ] On macOS, a file renamed between sessions is restored through its bookmark (manual check recorded; unit test of the resolve order with a fake resolver).
- [ ] Engine memory numbers for 1/5/20 documents recorded; ADR 0008 states the pooling decision.
- [ ] Tab strip passes keyboard-only navigation and axe checks in the shared component tests.

## Post-merge checks

None.

## Tests

- Rust unit tests: `WindowRegistry` operations (move — including the encrypted re-home order open → unlock → source close and its rollback on failed unlock —, detach, close last tab, duplicates), session serialisation round-trip and tolerance of missing files/corrupted JSON/unknown schema version; `resolve` order (bookmark → path → missing); the privacy rule (`should_persist_session(remember_recent, reopen_last)` table test; `save` is never called and the file is removed when either is off).
- Vitest + Testing Library for the `Tabs` component (keyboard, ARIA).
- WebdriverIO scenarios above (multi-window switching via `browser.switchWindow`/the tauri service's window handling).

## Docs to update

- `docs/plans/desktop-app.md` §9 D4: tick "Multi-window and tabs; drag a tab to a new window; session restore"; answer §8 Q3 with a link to ADR 0008.
- ADR 0008 amendment (engine pooling decision, numbers).
- `apps/desktop/README.md`: window/tab model.

## Risks and open questions

- **Cross-window tab drag** is not a first-class webview feature; the context-menu fallback guarantees functionality; the pointer-tracking approach may be imperfect on Wayland.
- **Memory** with one engine per window may exceed targets with many windows; numbers decide.
- **macOS app lifecycle** (no windows but running) interacts with single-instance and `RunEvent::Opened`; test cold/warm open with zero windows.
- **Restoring huge sessions** (20 tabs with 1 GB files) could slow startup: restore lazily (only active tabs' documents are opened; others open on activation).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
