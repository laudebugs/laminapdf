# D4e — Drag a tab out to a new window, or onto another window's tab strip

| Field | Value |
| --- | --- |
| ID | `D4e` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D4a](../done/D4a-windows-tabs-and-session-restore.md) |
| ADRs | Follows ADR 0008 |
| Branch | `feat/d4e-tab-drag-between-windows` |
| PR title | `feat(desktop): drag tabs out of and between windows` |
| Size | M (200–400): the drag detection, the cross-window drop, and the manual checks |
| PR | [#574](https://github.com/laudebugs/laminapdf/pull/574) |

## Goal

Move a tab by dragging it: out of its window to open a new window where it is dropped, or onto another window's tab strip. D4a's "Move tab to new window" and "Move tab to window ▸" commands already move tabs, keeping the view and an encrypted document's password; this adds the pointer gesture on top of them. Split out of D4a on 2026-11-12 (the owner's choice).

## Context

- D4a's `move_tab(doc, target_window | new)` and the reorder within a tab strip (HTML5 drag).
- Cross-window drag isn't a first-class webview feature; the context-menu commands stay the always-available, accessible way.

## Scope

**In scope**

- Dragging outside the window's bounds (`dragend` with screen coordinates outside it) calls `detach_tab(doc, screen_x, screen_y)`, which opens a new window at that position with the tab.
- Dropping onto another window's strip: the source sends `tab_drag_start(doc)`, and the target detects the pointer's entry through `drag-over` events from Rust, which tracks the cursor against window positions. Implement the simplest approach that works reliably, and document it.

**Out of scope** (and which todo covers it)

- Session restore → [D4d](../ready/D4d-session-restore.md)

## Tasks

- [ ] `detach_tab` and the drop target
- [ ] Manual checks on each OS, recorded in the PR

## Acceptance criteria

- [ ] Drag-out to a new window works on macOS, Windows and Linux X11 (manual check recorded); Wayland behaviour recorded (position may be compositor-controlled).
- [ ] Dropping a tab on another window's strip moves it there, keeping its page and zoom (manual check recorded; the move itself is D4a's tested `move_tab`).

## Post-merge checks

None.

## Tests

- Rust unit tests for the cursor-to-window hit test; the manual checks above.

## Docs to update

- `docs/plans/desktop-app.md` §9 D4: tick dragging a tab to a new window.

## Risks and open questions

- Wayland may not report window positions, so a drop onto another window may not be detectable there; the context menu remains.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
