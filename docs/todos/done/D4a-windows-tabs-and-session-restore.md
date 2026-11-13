# D4a — Multiple windows and tabs, and moving tabs between windows

| Field | Value |
| --- | --- |
| ID | `D4a` |
| Status | `done` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D3b](./D3b-recent-files-and-file-watching.md) |
| ADRs | Follows ADR 0008 (one engine per window; revisit pooling with numbers from here), ADR 0010 (`multiWindow: true`) |
| Branch | `feat/d4a-windows-and-tabs` |
| PR title | `feat(desktop): add multiple windows and tabs` |
| Size | L (700–1,400 lines) |
| PR | [#529](https://github.com/laudebugs/laminapdf/pull/529) |

## Goal

Let power users work with many documents: each window holds tabs, a tab moves to a new window or to another window ("Move tab to new window", "Move tab to window ▸") keeping its view, and File ▸ New Window opens an empty window. Requirement 5 of the [desktop plan](../../plans/desktop-app.md#functional). It also answers the plan's open question 3 (engine per window vs pool) with real memory numbers. On 2026-11-12 the owner split session restore into [D4d](../ready/D4d-session-restore.md) and dragging tabs between windows into [D4e](../ready/D4e-tab-drag-between-windows.md), both of which build on this.

## Context

- **Design (Paper):** [S02 Window with document tabs and session restore](../../design/paper/desktop-shell.md#s02-window-with-document-tabs-and-session-restore). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [Desktop plan §4 req. 5](../../plans/desktop-app.md#functional) and §6 D4.
- Tauri: `WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))`, `app.webview_windows()`, `Emitter::emit_to(EventTarget::webview_window(label), …)`, `WindowEvent::CloseRequested { api }` (`api.prevent_close()`), capabilities targeting windows by glob (`"windows": ["main", "doc-*"]`), [`tauri-plugin-window-state`](https://v2.tauri.app/plugin/window-state/) for geometry (`StateFlags`).
- D2b: `EngineSupervisor` per window label, `DocumentRegistry` (trusted path sources); D3a: open routing; D3b: the "Remember recent files" privacy setting ([D4d](../ready/D4d-session-restore.md)'s session restore follows it).
- `@laminapdf/ui` (web C4): tab strip doesn't exist on web (`multiWindow: false`); add a shared `Tabs` component rendered only when the `Platform` reports `multiWindow` — no `isTauri` checks.
- [T5b](../done/T5b-reopen-at-last-position.md) keeps each document's reading position in `Platform.positions` by fingerprint. Restored tabs keep their own view; a document opened outside a restored tab comes back at its position from `Platform.positions` (`Viewer.restore`).

## Scope

**In scope**

- Window model in Rust (`src-tauri/src/windows.rs`): `WindowRegistry` (label `doc-<n>` → tabs `[DocId]`, active tab), creation (`new_window(tabs?)`), focus tracking ("frontmost window" for open routing), close handling (closing the last tab closes the window on Windows/Linux; on macOS the app stays running with no windows and the Dock/Menu reopen a window on `RunEvent::Reopen`).
- Tabs: open routing from D3a now opens a **new tab** in the frontmost window (setting: "Open files in new tabs / new windows"); tab close, reorder, next/previous (`Ctrl+Tab`, `Ctrl+Shift+Tab`; `Cmd+Shift+]`/`[` on macOS — wired in D4b), duplicate detection (re-focus the existing tab when the same file is opened twice; setting to allow duplicates off by default).
- Engine affinity: documents move with their tab between windows; since engines are per window (ADR 0008), moving a tab re-opens the document in the target window's engine and transfers the view state (page, offset, zoom). **Encrypted documents**: a move keeps the same `DocId` and its registry entry, including the held `SessionPassword` (D2b) — it is a re-home, not a close. Order: (1) set the entry's owning window to the target; (2) send `OpenDocument` and, if a `SessionPassword` is held, `UnlockDocument` to the target engine; (3) only after `Unlocked` (or `Opened` for unencrypted files) send `CloseDocument` to the source engine, as an engine-only close that does **not** go through the registry's close path, so the password is not zeroised. If the target unlock fails, roll back (owner stays the source, tab stays) and show an error. The user is never re-prompted for a moved tab. Windows closing during a move wait until the move is done. Measure memory with 1, 5, 20 documents across 1 vs 4 windows; record in ADR 0008 and answer open question 3.
- Tab moves: HTML5 drag inside a window for reorder; "Move tab to new window" and "Move tab to window ▸ <name>" in the tab's context menu (always available, accessible). Dragging out of or between windows is D4e's.
- Window titles: `"<doc title or file name> — Lamina"`; macOS `represented_filename`/proxy icon if Tauri exposes it (else skip), "edited" dot not used (reader).
- Capabilities: `capabilities/main.json` `windows` glob covers all `doc-*` labels; no window gets more permissions than another.

**Out of scope** (and which todo covers it)

- Session restore (`session.json`, window geometry, macOS bookmarks, the privacy rule): [D4d](../ready/D4d-session-restore.md).
- Dragging a tab out of its window or onto another window's strip: [D4e](../ready/D4e-tab-drag-between-windows.md).

- Native menus and accelerators (New Window, Close Tab, Move Tab to Window, Window menu list): [D4b](../ongoing/D4b-native-menus.md).
- Printing: [D4c](../ready/D4c-native-printing.md).
- Shared engine pool, if numbers argue for it: a follow-up todo (raise it in the PR if needed).

## Tasks

- [x] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
- [x] `WindowRegistry` + commands (manifest + capability): `new_window`, `close_tab`, `activate_tab`, `move_tab(doc, target_window | new)`, `reorder_tabs(order)`, and `session_state()` for D4d to persist.
- [x] Per-window engine lifecycle: spawn on window creation, shut down on close; document hand-over on moves (close in source engine after the target reports `Opened`).
- [x] Shared `Tabs` component in `@laminapdf/ui` (keyboard accessible: roving tabindex, `role="tablist"`, close buttons with labels), desktop-only via `Platform.multiWindow`.
- [x] Memory measurements; ADR 0008 amendment answering open question 3.
- [x] e2e scenarios: open 3 files → 3 tabs; move tab to new window (via context-menu command, deterministic); close all tabs → behaviour per OS.

## Acceptance criteria

- [x] Opening three PDFs results in three tabs in one window; opening an already-open PDF focuses its tab (e2e).
- [x] "Move tab to new window" and "Move tab to window ▸ <name>" keep the page and zoom (e2e).
- [x] Moving an unlocked **encrypted** document's tab to another window shows it rendered in the target window with no password prompt, and closing the source window afterwards does not lock it (e2e with an encrypted fixture; registry unit test that the `SessionPassword` survives the move and is zeroised only when the document is finally closed).
- [x] Engine memory numbers for 1/5/20 documents recorded; ADR 0008 states the pooling decision.
- [x] Tab strip passes keyboard-only navigation and axe checks in the shared component tests.

## Decisions and deviations

- **Last tab.** Closing the last tab shows the start screen in that window on every OS, and the window stays open. The owner chose this on 2026-11-12. Closing the window itself follows the scope: macOS keeps the app running and the Dock makes a new window (`RunEvent::Reopen`); elsewhere closing the last window quits.
- **Where tabs live.** The webview keeps a window's tabs (their order, the shown one, each one's view, `apps/desktop/src/tabs/`). Rust keeps which window owns each document (`DocumentRegistry`) and the windows (`src-tauri/src/windows.rs`). So the commands are `new_window`, `move_tab`, `window_list`, `set_window_title` and `take_pending_tabs`; `close_tab`, `activate_tab` and `reorder_tabs` aren't needed, since closing a tab closes its document (`close_document`). `session_state()` is left to D4d, which knows what it persists.
- **Engines.** A window's engine still starts on first use (D2b) and stops when the window closes. A window that closes during a move waits for it.
- **Settings.** Settings › Files has "Open each file in a new window" (off: tabs) and "Open a file again in another tab" (off: its tab shows).
- **Titles.** "*name* — Lamina PDF", with the product's name. macOS's proxy icon is skipped: Tauri 2.12 doesn't expose `representedFilename`.
- **Design.** The strip's differences from S02 are in [desktop-shell.md's As built note](../../design/paper/desktop-shell.md#s02-window-with-document-tabs-and-session-restore).
- **Memory.** Each window's engine costs about 14 MiB; four windows with 20 documents use 80 MiB in all ([ADR 0008 §2](../../decisions/0008-desktop-shell.md#2-process-model-a-main-process-that-owns-files-and-a-sandboxed-engine-per-window)). One engine per window stays.

## Post-merge checks

None.

## Tests

- Rust unit tests: `WindowRegistry` operations (move — including the encrypted re-home order open → unlock → source close and its rollback on failed unlock —, close last tab, duplicates).
- Vitest + Testing Library for the `Tabs` component (keyboard, ARIA).
- WebdriverIO scenarios above (multi-window switching via `browser.switchWindow`/the tauri service's window handling).

## Docs to update

- `docs/plans/desktop-app.md` §9 D4: tick "Multi-window and tabs" (dragging and session restore are D4e's and D4d's); answer §8 Q3 with a link to ADR 0008.
- ADR 0008 amendment (engine pooling decision, numbers).
- `apps/desktop/README.md`: window/tab model.

## Risks and open questions

- **Memory** with one engine per window may exceed targets with many windows; numbers decide.
- **macOS app lifecycle** (no windows but running) interacts with single-instance and `RunEvent::Opened`; test cold/warm open with zero windows.
## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
