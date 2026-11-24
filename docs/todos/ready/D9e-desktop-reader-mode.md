# D9e — Desktop: reader mode

| Field | Value |
| --- | --- |
| ID | `D9e` |
| Status | `ready` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | [D9c](./D9c-desktop-find-in-document.md) |
| ADRs | Follows [ADR 0008](../../decisions/0008-desktop-shell.md) |
| Branch | `feat/d9e-desktop-reader-mode` |
| PR title | `feat(desktop): reader mode` |
| Size | M (200–400, if the web's reader mode moves to a shared package without change; L if it does not) |
| PR | — |

## Goal

Bring reader mode to the desktop: View ▸ Reader mode turns a document's text into a reflowed column with adjustable text size, line spacing and line length, for people who find the page's layout hard to read. The item and its settings are disabled today.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment.
- `packages/ui/src/commands/reader-commands.ts` (`readerModeCommands`): `view.reader-mode`, `view.reader-settings`, text smaller and larger, line spacing, line length and reset, all `delegated()`.
- `apps/web/src/reader-mode.tsx` (`useReaderMode`, the web's handlers) and `apps/web/src/reader-crop.ts`, `reader-start.ts`; `packages/ui/src/reader-mode.ts` and `packages/ui/src/reader/` hold the shared parts.
- Reader mode reflows the document's text, so it needs the page text the desktop engine does not serve yet (`desktop-engine-client.ts` header). [D9c](./D9c-desktop-find-in-document.md) adds that read; this todo reuses it and adds structure (reading order, headings) only if the web's reader mode needs more than runs and positions.
- The reader-mode settings may share D9a's persistence; the todo reuses it, not a second store.

## Scope

**In scope**

- `view.reader-mode` and the reader settings commands in the desktop, with the mode's column rendered by the shared code.
- Persistence of the reader-mode settings through the same place as the other desktop preferences.
- The menu items enabled and ticked.

**Out of scope**

- Engine-side text extraction: [D9c](./D9c-desktop-find-in-document.md).
- Reading aloud or any new feature the web's reader mode lacks.

## Tasks

- [ ] Fresh worktree; claim in the work PR. Confirm D9c is in `done/`.
- [ ] List what `reader-mode.tsx` needs from the web app; move the part that is not web-specific to `@laminapdf/ui` if it is not there yet.
- [ ] Register the handlers in `apps/desktop/src/app.tsx`; add the settings state.
- [ ] Tests, browser check, docs.

## Acceptance criteria

- [ ] In the real app on macOS, Windows and Linux, View ▸ Reader mode is enabled with a document that has text, shows the reflowed column, and leaves it again; the shortcut works.
- [ ] Text size, line spacing and line length change the column and survive a restart; Reset restores the defaults.
- [ ] A scanned document (no text) says reader mode has nothing to show, and does not leave a blank window.
- [ ] A test in the style of `use-native-menus.test.ts` shows `view.reader-mode` and the settings commands are available with the desktop's actions and a document open.
- [ ] A browser check from the user's side with real PDFs (a two-column paper, a book, a scanned file): read, resize the window, change each setting, and judge whether the text order is right. Record what was seen.
- [ ] No second copy of the web's reader mode code exists in `apps/desktop`.

## Post-merge checks

- [ ] None.

## Tests

- Unit: the handlers and the settings' storage; `use-native-menus.test.ts` for availability.
- The shared reader-mode tests keep passing in `@laminapdf/ui`.
- A desktop e2e scenario: enter reader mode on a fixture and read a heading.

## Docs to update

- `docs/plans/desktop-app.md`: tick the to-do under D9.
- `docs/capabilities/README.md`: no row for reader mode; no change.
- `apps/desktop/CHANGELOG.md`.

## Risks and open questions

- **Text order.** PDFium's reading order may differ from the library's; compare on the web's fixtures.
- **Size.** If reader mode's code is still inside `apps/web`, moving it makes this an L; split the move out.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
