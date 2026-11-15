# U7g — Drag tabs by pointer events, so it works on Windows

| Field | Value |
| --- | --- |
| ID | `U7g` |
| Status | `ready` |
| Plan | [Motion and interaction states](../../plans/motion-and-states.md) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | None |
| ADRs | Follows 0024 (tokens in layers), 0010 (capability flags) |
| Branch | `feat/u7g-pointer-tab-drag` |
| PR title | `fix(desktop): drag tabs by pointer events on every OS` |
| Size | M (200–600) |
| PR | — |

## Goal

Tab reordering and dragging a tab out of its window use HTML5 drag and drop (`packages/ui/src/reader/document-tabs.tsx:289-293`), which Tauri turns off on Windows while its file-drop handler is on (`windows.rs:159-176`; Tauri's own note at `webview_window.rs:1096`), so they may not work there at all; on macOS and Linux the same mechanism shows a "not allowed" cursor over a window that says "drop here". Confirm the fault on Windows, then drive the drag with pointer events, keeping D4a's `move_tab` and D4e's `tab_drag_start`/`tab_drag_end`.

## Context

- [Plan §2.3](../../plans/motion-and-states.md#23-motion), the desktop research
- [D4e](../done/D4e-tab-drag-between-windows.md) (drag between windows) and its `apps/desktop/src-tauri/src/tab_drag.rs`
- Load the `tauri-v2` skill

## Scope

**In scope**

- A Windows check first: record whether reorder and drag-out work today (the result goes in `docs/bugs/` if they don't)
- Pointer-driven reorder with an insertion marker and the `grabbing` cursor
- Drag-out and drag-between through D4e's Rust side
- Keyboard and menu routes unchanged

**Out of scope**

- Tab motion beyond the drag → U7f

## Tasks

- [ ] Confirm on Windows
- [ ] Pointer-driven drag
- [ ] Feedback while dragging
- [ ] e2e

## Acceptance criteria

- [ ] On Windows, macOS and Linux a tab reorders within its strip and moves to another window or a new one
- [ ] The cursor is `grabbing` while dragging, with no "not allowed" cursor over a valid target

## Post-merge checks

- [ ] Drag tabs with a real mouse on each OS

## Tests

- A WebdriverIO scenario per OS: reorder, drag to another window, drag out

## Docs to update

- `apps/desktop/README.md`
- `docs/bugs/` if Windows fails today
- Plan §7: point this todo at `done/`

## Risks and open questions

- WebdriverIO's pointer actions may not cross windows; D4e's manual checks used OS-level events, which the e2e may need too

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
