# E1h — Desktop: the edit session, the Edit menu and the close guard

| Field | Value |
| --- | --- |
| ID | `E1h` |
| Status | `ready` |
| Plan | [E1 editing foundation §8](../../plans/editor/e1-editing-foundation.md#8-differences-between-the-shells) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [D4b](./D4b-native-menus.md), [E1d](../ongoing/E1d-edit-session-undo-ui.md) |
| ADRs | Follows ADR 0006, 0008 |
| Branch | `feat/e1h-desktop-edit-session` |
| PR title | `feat(desktop): the edit session, the Edit menu and the close guard` |
| Size | M (200–400) |
| PR | — |

## Goal

The desktop part of E1d: the edit session runs in the engine process as it runs in the web's document worker, Undo and Redo appear in the native Edit menu, and closing a window with unsaved changes asks first.

## Context

- [E1d](../ongoing/E1d-edit-session-undo-ui.md): the session's methods and events, the commands and the history panel this mirrors
- [D4b](./D4b-native-menus.md): menus generated from the command registry, so registering `edit.undo` and `edit.redo` puts them in the Edit menu
- Load the `/tauri-v2` skill before any work

## Scope

**In scope**

- Engine-process commands for the session (`apply`, `undo`, `redo`, `history`, the dirty event with touched pages), with the same types as the web worker's.
- Undo and Redo in the native Edit menu, enabled as the stacks allow.
- The window-close and quit guard (`CloseRequested`) for a dirty document.
- WebdriverIO e2e: rotate, undo, redo, the dirty indicator, the close prompt.

**Out of scope**

- Saving in place: [E1i](./E1i-desktop-save-in-place.md).

## Tasks

- [ ] Engine commands and their capability permissions.
- [ ] The Edit menu entries through D4b's registry export.
- [ ] The close guard.
- [ ] `apps/desktop/e2e/specs/edit-session.e2e.ts`.

## Acceptance criteria

- [ ] On macOS, Windows and Linux: rotate a page, undo and redo from the menu and the shortcuts, checking the page and the dirty indicator each step.
- [ ] Closing a dirty window prompts; closing a clean one doesn't.

## Post-merge checks

None.

## Tests

WebdriverIO e2e on three OSes; a Rust test of the commands' capability guard.

## Docs to update

`apps/desktop` README; the E1 plan's to-dos.

## Risks and open questions

None beyond E1d's.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
