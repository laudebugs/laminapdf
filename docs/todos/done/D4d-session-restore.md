# D4d — Session restore: windows, tabs and each tab's view after a relaunch

| Field | Value |
| --- | --- |
| ID | `D4d` |
| Status | `done` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D4a](./D4a-windows-tabs-and-session-restore.md) |
| ADRs | Follows ADR 0008, ADR 0010 |
| Branch | `feat/d4d-session-restore` |
| PR title | `feat(desktop): restore the last session's windows and tabs on launch` |
| Size | L (400–800): the session file and its resolver, window geometry, the macOS bookmark helper, the privacy rule and their tests |
| PR | [#573](https://github.com/laudebugs/laminapdf/pull/573) |

## Goal

On relaunch, Lamina PDF brings back the last session: its windows with their size and position, their tabs, and each tab's page and zoom. Split out of D4a on 2026-11-12 (the owner's choice), which delivers the windows and tabs this restores.

## Context

- **Design (Paper):** [S02 Window with document tabs and session restore](../../design/paper/desktop-shell.md#s02-window-with-document-tabs-and-session-restore).
- D4a's `WindowRegistry`, its tabs and its `session_state()`; D2b's `DocumentRegistry` (trusted path sources); D3b's "Remember recent files" setting.
- [`tauri-plugin-window-state`](https://v2.tauri.app/plugin/window-state/) for geometry (`StateFlags`).
- [T5b](../done/T5b-reopen-at-last-position.md): a document opened outside a restored tab comes back at its position from `Platform.positions`.

## Scope

**In scope**

- `tauri-plugin-window-state` (licence check), with `StateFlags::SIZE | POSITION | MAXIMIZED`; off-screen positions (a monitor that went away) are not restored.
- `session.json` in `app_config_dir()`, schema-versioned, written atomically (temp + rename) on quit and every 30 s while running (debounced); it stores window order and labels, each window's tabs and active tab.
- **Each tab stores its own document reference, independent of the recent list** (which can be cleared, turned off, and holds 20): `TabRef { path, stamp: { len, mtime }, bookmark?, view: { page, offset, zoom } }`. `path` is canonical and absolute. On macOS, `bookmark` is a regular `NSURL` bookmark (`bookmarkDataWithOptions:` through `objc2-foundation`; the app isn't sandboxed), resolved first with `path` as the fallback and refreshed when stale, so a file moved or renamed between sessions is still found. Windows and Linux store the path only. Restored paths enter `DocumentRegistry` as a trusted source, written by the main process, never by the webview.
- **Privacy rule (decided):** `session.json` records document paths just like the recent list, so it follows the same switch. With "Remember recent files" off, no session file is written and an existing one is deleted; "Reopen last session" (default on) shows disabled with "Needs 'Remember recent files'". Turning "Reopen last session" off also deletes `session.json`. "Clear recent files" clears the recent list only.
- Restore on launch when "Reopen last session" is on: missing files come back as "missing" tabs with "Locate…"; only each window's active tab opens its document, the others open when activated, so a session of 20 large files starts fast.
- `RunEvent::ExitRequested` flushes the session.

**Out of scope** (and which todo covers it)

- Windows, tabs and moves → [D4a](../done/D4a-windows-tabs-and-session-restore.md)
- Dragging tabs between windows → [D4e](../done/D4e-tab-drag-between-windows.md)

## Tasks

- [x] `src-tauri/src/session.rs`: `SessionFile`, `TabRef`, `fn load(dir) -> Result<Option<SessionFile>, SessionError>`, `fn save(dir, &SessionFile)`, `fn resolve(tab: &TabRef) -> Resolved { Found(PathBuf) | Moved(PathBuf) | Missing }`
- [x] The macOS bookmark helper
- [x] The window-state plugin and its off-screen check
- [x] The privacy rule, the two settings and their link
- [x] Lazy restore, and the "missing" tab with "Locate…"

## Acceptance criteria

- [x] Quit + relaunch restores windows (geometry), tabs and per-tab page and zoom (e2e); disabling "Reopen last session" starts empty.
- [x] Session restore works after the recent list is cleared (e2e: open 3 files, "Clear recent files", quit, relaunch → the 3 tabs come back).
- [x] With "Remember recent files" off, quitting leaves no `session.json` in `app_config_dir()` and relaunch starts empty (e2e + unit).
- [x] On macOS, a file renamed between sessions is restored through its bookmark (manual check recorded; unit test of the resolve order with a fake resolver): `laminapdf-desktop-os`'s `a_bookmark_should_find_its_file_after_a_rename` renames a real file on macOS and finds it again, and `session::resolve`'s order has fake-resolver tests.

## Post-merge checks

None.

## Tests

- Rust unit tests: the session file's round trip and its tolerance of missing files, corrupted JSON and an unknown schema version; `resolve`'s order (bookmark → path → missing); the privacy rule (`should_persist_session(remember_recent, reopen_last)` table test; `save` is never called and the file is removed when either is off).
- WebdriverIO: the scenarios above.

## Docs to update

- `docs/plans/desktop-app.md` §9 D4: tick session restore.
- `apps/desktop/README.md`: the session file.

## Risks and open questions

- Restoring huge sessions: lazy restore keeps startup to the active tabs.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
