# D4f — The restore notice, and missing files marked in the tab strip

| Field | Value |
| --- | --- |
| ID | `D4f` |
| Status | `ready` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D4d](../done/D4d-session-restore.md) |
| ADRs | Follows ADR 0008, ADR 0024 |
| Branch | `feat/d4f-restore-notice-and-missing-marks` |
| PR title | `feat(desktop): say what the last session brought back, and mark missing files in the tab strip` |
| Size | M (200–400): the notice, the strip's marks and menu entries in `@laminapdf/ui`, the "looks different" question, their gallery states |
| PR | — |

## Goal

Finish S02's session restore as drawn. D4d brings the last session back and gives a restored tab whose file is gone a notice with Locate… and Close; it leaves out the parts of S02 that change the shared tab strip and the canvas around it.

## Context

- **Design (Paper):** [S02 Window with document tabs and session restore](../../design/paper/desktop-shell.md#s02-window-with-document-tabs-and-session-restore), its "As built" notes, and the `S02-tab-missing` artboard.
- D4d's `session_open` fails with `io` for a file that is gone; the webview knows a tab is the last session's while it hasn't opened (`restoredToken`).

## Scope

**In scope**

- The restore notice: a `role="status"` strip after a restore, "Restored your last session. N tabs, each at the page you were reading. N files were not found.", with Show missing file and Dismiss; it goes on Dismiss or the first navigation.
- A missing file marked in the strip: the tab's warning glyph, and "Not found" in the strip's menu, where each tab also shows its page.
- Locate… asks "This file looks different from the one you had open. Open it anyway?" when the chosen file's size and date differ from the session's stamp.
- The gallery's states for the strip's new marks.

**Out of scope**

- The missing tab's canvas naming the folder the file was in: the webview never sees paths (ADR 0008 §6), so the notice names only the file.

## Acceptance criteria

- [ ] After a restore with a missing file, the notice says so and Show missing file activates that tab (e2e).
- [ ] The missing tab shows its glyph and "Not found" in the strip's menu until it is located or closed.
- [ ] Locating a file of another size asks first.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
