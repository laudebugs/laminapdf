# D4a — Multiple windows, tabs, tab drag between windows, session restore

| Field | Value |
| --- | --- |
| ID | `D4a` |
| Status | `ready` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D3b](./D3b-recent-files-and-file-watching.md) |
| Blocks | [D4b](./D4b-native-menus.md) |
| ADRs | Follows ADR 0008 (one engine per window; revisit pooling with numbers from here), ADR 0010 (`multiWindow: true`) |
| Branch | `feat/d4a-windows-and-tabs` |
| PR title | `feat(desktop): add multiple windows, tabs and session restore` |
| Size | L (700–1,400 lines) |
| PR | — |

## Goal

Let power users work with many documents: each window holds tabs, a tab can be dragged out into a new window or onto another window's tab strip, File ▸ New Window opens an empty window, and on relaunch lamina restores the last session (windows, their geometry, tabs, and each tab's page and zoom). Requirement 5 of the [desktop plan](../../plans/desktop-app.md#functional). It also answers the plan's open question 3 (engine per window vs pool) with real memory numbers.

## Context

- [Desktop plan §4 req. 5](../../plans/desktop-app.md#functional) and §6 D4.
- Tauri: `WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))`, `app.webview_windows()`, `Emitter::emit_to(EventTarget::webview_window(label), …)`, `WindowEvent::CloseRequested { api }` (`api.prevent_close()`), capabilities targeting windows by glob (`"windows": ["main", "doc-*"]`), [`tauri-plugin-window-state`](https://v2.tauri.app/plugin/window-state/) for geometry (`StateFlags`).
- D2b: `EngineSupervisor` per window label; D3a: open routing; D3b: `RecentFiles` view state.
- `@lamina/ui` (web C4): tab strip doesn't exist on web (`multiWindow: false`); add a shared `Tabs` component rendered only when the `Platform` reports `multiWindow` — no `isTauri` checks.

## Scope

**In scope**

- Window model in Rust (`src-tauri/src/windows.rs`): `WindowRegistry` (label `doc-<n>` → tabs `[DocId]`, active tab), creation (`new_window(tabs?)`), focus tracking ("frontmost window" for open routing), close handling (closing the last tab closes the window on Windows/Linux; on macOS the app stays running with no windows and the Dock/Menu reopen a window on `RunEvent::Reopen`).
- Tabs: open routing from D3a now opens a **new tab** in the frontmost window (setting: "Open files in new tabs / new windows"); tab close, reorder, next/previous (`Ctrl+Tab`, `Ctrl+Shift+Tab`; `Cmd+Shift+]`/`[` on macOS — wired in D4b), duplicate detection (re-focus the existing tab when the same file is opened twice; setting to allow duplicates off by default).
- Engine affinity: documents move with their tab between windows; since engines are per window (ADR 0008), moving a tab re-opens the document in the target window's engine and transfers the view state (page, offset, zoom). Measure memory with 1, 5, 20 documents across 1 vs 4 windows; record in ADR 0008 and answer open question 3.
- Tab drag between windows: HTML5 drag inside a window for reorder; dragging outside the window bounds (`dragend` with screen coordinates outside) calls `detach_tab(doc, screen_x, screen_y)` → new window at that position; dropping onto another window's strip: the source sends `tab_drag_start(doc)`, the target window detects pointer entry (via `drag-over` events from Rust, which tracks the cursor using window positions) — implement the simplest reliable approach found and document it; fallback: "Move tab to window ▸" context menu (always available, accessible).
- Session restore: on quit (and every 30 s while running, debounced), persist `session.json` in `app_config_dir()` (windows → geometry handled by `tauri-plugin-window-state`, tabs → document IDs referencing recent-file entries + view state); on launch, restore if the setting "Reopen last session" is on (default on); missing files restore as "missing" tabs with "Locate…".
- Window titles: `"<doc title or file name> — Lamina"`; macOS `represented_filename`/proxy icon if Tauri exposes it (else skip), "edited" dot not used (reader).
- Capabilities: `capabilities/main.json` `windows` glob covers all `doc-*` labels; no window gets more permissions than another.

**Out of scope** (and which todo covers it)

- Native menus and accelerators (New Window, Close Tab, Move Tab to Window, Window menu list): [D4b](D4b-native-menus.md).
- Printing: [D4c](D4c-native-printing.md).
- Shared engine pool, if numbers argue for it: a follow-up todo (raise it in the PR if needed).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2` and `rust-best-practices`.
- [ ] Add `tauri-plugin-window-state` (licence check); configure `StateFlags::SIZE | POSITION | MAXIMIZED` and skip restoring off-screen positions (monitor changes).
- [ ] `WindowRegistry` + commands (manifest + capability): `new_window`, `close_tab`, `activate_tab`, `move_tab(doc, target_window | new)`, `detach_tab(doc, x, y)`, `reorder_tabs(order)`, `session_state()`.
- [ ] Per-window engine lifecycle: spawn on window creation, shut down on close; document hand-over on moves (close in source engine after the target reports `Opened`).
- [ ] Shared `Tabs` component in `@lamina/ui` (keyboard accessible: roving tabindex, `role="tablist"`, close buttons with labels), desktop-only via `Platform.multiWindow`.
- [ ] Session persistence and restore; `RunEvent::Reopen` on macOS; `RunEvent::ExitRequested` flushes the session.
- [ ] Memory measurements; ADR 0008 amendment answering open question 3.
- [ ] e2e scenarios: open 3 files → 3 tabs; move tab to new window (via context-menu command, deterministic); restart app → same windows/tabs/pages restored; close all tabs → behaviour per OS.
- [ ] Manual checklist: real drag-out and drag-between windows on each OS (Wayland quirks noted).

## Acceptance criteria

- [ ] Opening three PDFs results in three tabs in one window; opening an already-open PDF focuses its tab (e2e).
- [ ] "Move tab to new window" and "Move tab to window ▸ <name>" keep the page and zoom (e2e).
- [ ] Drag-out to a new window works on macOS, Windows and Linux X11 (manual check recorded); Wayland behaviour recorded (position may be compositor-controlled).
- [ ] Quit + relaunch restores windows (geometry), tabs and per-tab page/zoom (e2e); disabling "Reopen last session" starts empty.
- [ ] Engine memory numbers for 1/5/20 documents recorded; ADR 0008 states the pooling decision.
- [ ] Tab strip passes keyboard-only navigation and axe checks in the shared component tests.

## Tests

- Rust unit tests: `WindowRegistry` operations (move, detach, close last tab, duplicates), session serialisation round-trip and tolerance of missing files/corrupted JSON.
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

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
