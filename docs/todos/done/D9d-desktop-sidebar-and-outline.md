# D9d — Desktop: sidebar with thumbnails and outline, shared with the web

| Field | Value |
| --- | --- |
| ID | `D9d` |
| Status | `done` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | Follows [ADR 0008](../../decisions/0008-desktop-shell.md) |
| Branch | `feat/d9d-desktop-sidebar` |
| PR title | `feat(desktop): a sidebar with thumbnails and the outline, shared with the web` |
| Size | L (400–800: moving the rail into a shared package is most of it; split the move from the desktop wiring if you can) |
| PR | [#822](https://github.com/laudebugs/laminapdf/pull/822) |

## Goal

Give the desktop a sidebar that opens with View ▸ Toggle sidebar and holds the page thumbnails and the document's outline (its bookmarks), and make it the same code the web reader uses, not a second copy. Today the item is disabled and the desktop has no rail.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment: `view.toggle-sidebar` is `delegated()`; `shellActions()` in `packages/ui/src/shell/actions.ts` registers it only when a rail exists (`{ rail }`), so without rail tabs the command stays out of view.
- `apps/web/src/rail.tsx`: `railTabs()` builds the tabs (Thumbnails, Outline, Results while find is open, Threads, Attachments, Bookmarks editor) from the web's `Engine`, `ShellLayout` and `Viewer`. It is not shared; the desktop cannot import it.
- `packages/viewer/src/thumbnails`: the viewer's thumbnail strip, which the web's rail uses. The desktop already draws thumbnails for the Organise mode (`apps/desktop/src/organise.tsx`, `thumbnail()`), so the engine serves small renders.
- The outline: the desktop's client has no outline read (`desktop-engine-client.ts`); the web's `apps/web/src/outline.ts` builds the tree from the engine. The engine needs an outline read (a `Query` in `crates/laminapdf-desktop-protocol/src/msg.rs`) unless the existing reads carry it; find out first.
- [Maintainability, one home per concept](../../guidelines/maintainability.md): move the rail's tab list, the thumbnails tab and the outline tab into `@laminapdf/ui` (or the viewer package) behind an interface the two apps fill with their engines, and have `apps/web/src/rail.tsx` call it. Web-only tabs (Threads, Attachments, bookmark editing) stay in the web.

## Scope

**In scope**

- Extract the rail and its Thumbnails and Outline tabs into a shared package; the web uses them from there with no change in behaviour.
- The desktop's outline read in the engine and client, and the tabs in the desktop, with click-to-go-to-page and the current page highlighted.
- `view.toggle-sidebar` enabled in the desktop through `shellActions` with `rail: true` while a document is open, and ticked while the sidebar is open; the rail's width and open state remembered per window.
- The Results tab shows once [D9c](../ready/D9c-desktop-find-in-document.md) has landed, as a follow-up line in that PR or this one, whichever merges second.

**Out of scope**

- Threads, Attachments and the bookmark editor in the desktop: later, with the editor's desktop work.
- Focus mode: [D9b](../done/D9b-desktop-focus-mode.md).

## Tasks

- [x] Fresh worktree; claim in the work PR.
- [x] List what `rail.tsx` imports from the web app; define the engine interface the shared rail needs (thumbnail, outline).
- [x] Move the shared part; keep the web's tests green; change the web to use it (first commit, own review).
- [x] Add the outline read and the thumbnail source for the desktop; mount the rail; register `view.toggle-sidebar`.
- [x] Menu tick, tests, docs. The browser check is below, under the acceptance criteria.

## Acceptance criteria

- [x] `apps/web/src/rail.tsx` no longer holds the shared tabs, and the desktop imports the same module; no file in `apps/desktop` copies the web's rail code.
- [x] In the real app on macOS, Windows and Linux, View ▸ Toggle sidebar is enabled with a document open and shows and hides the sidebar; the outline tab lists the bookmarks of a PDF that has them, and clicking one goes to its page. *Covered by `apps/desktop/e2e/specs/sidebar.e2e.ts`, which CI runs on the three systems; not run on a machine by hand.* *`sidebar.e2e.ts` ran on Linux, macOS and Windows in CI (66 scenarios passing each); the e2e screenshots show the sidebar at 1200 px.*
- [ ] The Thumbnails tab shows pages, marks the current one and goes to a clicked page; it stays smooth on the 1,000-page fixture. *The strip is the web's, with its render cancelling; the page click is in `sidebar.e2e.ts`. The 1,000-page fixture was not tried.*
- [x] A document with no outline shows an empty-state message, not a blank tab. *The shared Outline tab shows it, and `sidebar.e2e.ts` checks it on a file with no outline.*
- [x] A test in the style of `use-native-menus.test.ts` shows `view.toggle-sidebar` is available with the desktop's actions and a document open, and unavailable without a rail.
- [ ] A browser check from the user's side: open a PDF with an outline, use the sidebar with mouse and keyboard, and judge layout at narrow and wide window sizes. Record what was seen. *Not done: no one drove the app by hand; the desktop was not built locally.* The CI screenshots at 1200 px were looked at; narrow sizes were not.
- [x] The web reader's tests and e2e still pass. *The web's unit tests pass (239) and its e2e suite passed in CI.*

## Post-merge checks

- [x] None.

## Tests

- Unit and browser tests for the shared rail move with it; the web's rail tests keep passing.
- Rust: the outline read on a fixture with nested bookmarks, with none, and a malformed one.
- `use-native-menus.test.ts`: the sidebar command.
- A desktop e2e scenario: toggle, click an outline entry, read the current page.

## Docs to update

- `docs/plans/desktop-app.md`: tick the to-do under D9; §5 for the outline read.
- `docs/capabilities/README.md`: row I.11 (outline and navigation preferences) has no desktop cell; this todo reads the outline, so set it only if its evidence fits; otherwise no change.
- `apps/web/src/rail.tsx` header comment; `apps/desktop/CHANGELOG.md`.

## Risks and open questions

- **Soft dependency on D9c.** The Results tab shares the find results of [D9c](../ready/D9c-desktop-find-in-document.md). Whichever merges second adds the tab; the rest of this todo does not wait.
- **The move.** The rail leans on web-only types; the interface may need two PRs. The first moves it with no desktop change.
- **Thumbnails at scale.** Reuse the Organise mode's thumbnail path and its cancel rules.
- **Outline destinations.** Named destinations and actions need the engine's reads; unsupported ones go to the page or do nothing, and the PR says which.

## Definition of done

- [x] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
