# E1i — Desktop: save in place and changes made outside

| Field | Value |
| --- | --- |
| ID | `E1i` |
| Status | `ready` |
| Plan | [E1 editing foundation §7](../../plans/editor/e1-editing-foundation.md#7-saving-and-the-save-constraint) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [D3b](./D3b-recent-files-and-file-watching.md), [E1e](../ongoing/E1e-save-and-constraint-banner.md), [E1h](./E1h-desktop-edit-session.md) |
| ADRs | Follows ADR 0007, 0008 |
| Branch | `feat/e1i-desktop-save-in-place` |
| PR title | `feat(desktop): save in place, with conflicts from outside changes` |
| Size | M (200–400) |
| PR | — |

## Goal

The desktop part of E1e. Save writes the open file in place: it appends an incremental update, or rewrites through a temporary file and a rename. The file watcher must not mistake the app's own write for an outside change. An outside change to a file with unsaved edits asks the user what to do.

## Context

- [E1e](../ongoing/E1e-save-and-constraint-banner.md): the save commands, the strip and the planner's options this uses
- [D3b](./D3b-recent-files-and-file-watching.md): the watcher and reload on outside change
- Load the `/tauri-v2` skill before any work

## Scope

**In scope**

- Rust commands to append an incremental update in place and to rewrite atomically (temporary file, then rename).
- A token that tells D3b's watcher to ignore the app's own write.
- Conflict detection by the base's size, modification time and hash; the dialog: reload, save as, or overwrite.

**Out of scope**

- The web's save: [E1e](../ongoing/E1e-save-and-constraint-banner.md).

## Tasks

- [ ] The two write commands and their capability permissions.
- [ ] Watcher suppression.
- [ ] The conflict dialog.
- [ ] `apps/desktop/e2e/specs/save.e2e.ts`.

## Acceptance criteria

- [ ] Edit, Save, reopen in the app and in pdf.js: the change is there, and an incremental save only grew the file by the appended revision.
- [ ] A file changed outside while dirty shows the conflict dialog; a clean one reloads keeping the page.
- [ ] A failed write (a read-only file) leaves the document dirty and the file as it was.
- [ ] On the desktop, editing each metadata field in the Properties dialog, saving and reopening shows the values in Info and XMP; these e2e tests flip the desktop column of J.01–J.03 and J.10.

## Post-merge checks

None.

## Tests

WebdriverIO e2e on three OSes; Rust tests of the atomic rewrite and the suppression token.

## Docs to update

`apps/desktop` README; the E1 plan's to-dos.

## Risks and open questions

- Windows file locking: D2b and D3b's handling (positioned reads, `FILE_SHARE_DELETE`) must hold while writing.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
