# C8b — The Close button beside the file name, `file.close` and Back to files

| Field | Value |
| --- | --- |
| ID | `C8b` |
| Status | `ongoing` |
| Plan | [Close a document §4](../../plans/close-document.md#4-decisions) |
| Round / milestone | Round C (web reader): C8, closing a document |
| Depends on | [C8a](./C8a-every-close-asks-first.md) |
| ADRs | [ADR 0024](../../decisions/0024-app-shell-ux.md) (one command registry; every command with a visible home), [ADR 0010](../../decisions/0010-platform-adapter.md) (no platform checks in UI code) |
| Branch | `feat/c8b-close-button-and-command` |
| PR title | `feat(web): close a document with the × beside its name and go back to the home screen` |
| Size | M |
| PR | — |

## Goal

The web reader has no visible way to leave a document: More's "Choose a file" closes it under a label that says it opens one. This todo builds [Reader A R19](../../design/paper/reader-a.md#r19-close-a-document): an × beside the file name, the `file.close` command in the palette and More, More's "Open another file…" opening the picker, and the phone's and tablet's designed Back to files. Each closes through C8a's one guarded path and lands on the home screen with focus on its heading.

## Context

- [Close a document plan](../../plans/close-document.md) and [Reader A R19](../../design/paper/reader-a.md#r19-close-a-document), with its boards in `docs/design/paper/reader-a/project/R19-*.dc.html`.
- [Reader B R17 and R18](../../design/paper/reader-b.md#r18-phone-outline-find-more): Back to files on tablet and phone.
- [Edit session O03](../../design/paper/edit-session-organise.md): the confirmation the guard shows.
- `packages/ui/src/commands/reader-commands.ts` (`file.open`), `packages/ui/src/reader/top-bar.tsx`, `packages/ui/src/messages.ts`, `apps/web/src/app.tsx` (More's items), `apps/desktop/src/app.tsx` (uses `ReaderTopBar`: its tabs own closing).
- Design language [§8.8](../../design/paper/design-language.md#88-keyboard-and-shortcuts): no `Mod+W` on the web.

## Scope

**In scope**

- `file.close` ("Close document"; group `file`; icon `close`; no shortcut; enabled in every phase but `empty`; runs `requestClose()`).
- `ReaderTopBar`: the 24 px × after the file name at desktop width, named "Close {name}", tooltip "Close document", shown in every phase but `empty`, only when the shell passes it (the desktop app doesn't).
- `ReaderTopBar` at phone and tablet widths: Back to files at the top left, running `file.close`.
- More: "Choose a file" becomes "Open another file…" (`file.open`, `Ctrl O` chip), and "Close document" follows it directly.
- After a close: focus on the home heading (`tabindex="-1"`), a polite announcement "Closed {name}", the window title "lamina".

**Out of scope** (and which todo covers it)

- The fragment and Try another routes: [C8a](C8a-every-close-asks-first.md).
- A Reopen toast: the owner decided against it (2026-11-13).
- The desktop app's tab close: Desktop shell S02, already built.
- R01's board, which still draws a linked file in Recent: a design fix of its own.

## Tasks

- [ ] Add `file.close` to the registry, with its messages, and to the shortcut help's list (no key).
- [ ] Add the × to `ReaderTopBar` behind an optional `onClose`-style prop or the command id, with the file name cut short before it.
- [ ] Add Back to files to the phone and tablet top bar.
- [ ] Rewrite More's file items in `apps/web/src/app.tsx`.
- [ ] Move focus and announce after a close; check the window title.
- [ ] Keep the desktop app's top bar without the ×.
- [ ] Update the gallery's states for the top bar (`packages/ui/test/states.tsx`) and its screenshots (`cargo xtask gallery --update`, which the owner runs in Docker).

## Acceptance criteria

- [ ] The ×, More's Close document, the palette and Back to files each close to the home screen, with focus on the heading and the announcement.
- [ ] With unsaved changes, each shows O03; Cancel keeps the document and returns focus to the control that asked.
- [ ] Open another file… opens the picker; picking a file with unsaved changes open shows O03 first.
- [ ] The desktop app shows no ×.
- [ ] axe finds nothing new on the top bar in three themes and both directions.

## Post-merge checks

None.

## Tests

- `packages/ui/test/`: the command's enabled state per phase; the top bar's × (name, tooltip, absent on `empty` and without the prop); Back to files at phone width (browser tests in Chromium, Firefox, WebKit).
- `apps/web/e2e/`: closing by each route, focus, announcement, title, Recent's first row; O03 on each route with an unsaved edit.
- The gallery's visual and axe tests for the top bar's new state.

## Docs to update

The plan's to-dos and status, [Reader A R19](../../design/paper/reader-a.md#r19-close-a-document) (an As built note if anything differs), the shortcut help if it lists commands without keys, `apps/web/README.md` if it lists the More menu.

## Risks and open questions

- A narrow desktop window where the file area is tight: the file name must shrink before the × moves.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
