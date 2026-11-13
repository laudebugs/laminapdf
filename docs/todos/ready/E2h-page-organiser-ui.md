# E2h — Page organiser UI: the Organise mode, thumbnails drag-and-drop and multi-select

| Field | Value |
| --- | --- |
| ID | `E2h` |
| Status | `ready` |
| Plan | [E2 page organisation §4](../../plans/editor/e2-page-organisation.md#4-crate-and-package-changes) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1e](../done/E1e-save-and-constraint-banner.md), [E2k](../done/E2k-duplicate-pages.md), [E2l](./E2l-viewer-across-page-edits.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2h-page-organiser-ui` |
| PR title | `feat(ui): page organiser with drag-and-drop thumbnails` |
| Size | L (400–800): the Organise mode's page grid, selection, drag and its commands in `@laminapdf/ui`, with Playwright tests; the viewer's part is [E2l](E2l-viewer-across-page-edits.md)'s and the dialogs [E2g](E2g-page-labels.md)'s and [E2m](E2m-document-dialogs.md)'s |
| PR | — |

## Goal

The Organise mode on the web: a page grid with multi-select thumbnails, drag-and-drop reordering with an insertion bar, Move to page…, context menus and the single-document commands (insert blank, delete, duplicate, rotate, reverse), keyboard-accessible. The [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos) moved the documents dialogs (insert from a file, replace, extract, merge, split) to [E2m](E2m-document-dialogs.md), the page-setup dialogs to [E2g](E2g-page-labels.md), and the desktop part to [E2n](E2n-desktop-organise.md).

## Context

- **Design (Paper):** [O06 Organise mode](../../design/paper/edit-session-organise.md#o06-organise-mode), [O17 Phone: organise pages](../../design/paper/pages-navigation-extraction.md#o17-phone-organise-pages). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.7](../../plans/ui-ux.md#57-trust-surfaces-r9r13) — a multi-select grid with an insertion bar (no live swapping), a "Move to page…" dialog as the single-pointer alternative to dragging (WCAG 2.2 SC 2.5.7), and one undo step per batch move (ADR 0006); the Organise mode registers through [U1c](../done/U1c-inverted-l-shell-and-modes.md).
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) web vs desktop
- [web C6](../../plans/web-app.md#c6-navigation-print-and-properties): the thumbnail sidebar this grows from
- [desktop D3](../../plans/desktop-app.md#d3-files): drag-drop onto window and icon

## Scope

**In scope**

- The Organise mode (`mode.organise`, ADR 0024 §2) with its toolbar, rail and inspector (O06); the page grid over the viewer's thumbnails; multi-select (shift/cmd), drag reorder (`MovePages`, one transaction), Move to page… (the single-pointer alternative, WCAG 2.2 SC 2.5.7)
- Commands: `pages.insert-blank` (Blank pages dialog), `pages.delete`, `pages.duplicate`, `pages.rotate-left`/`-right` (`RotatePages { mode: turn }`), `pages.reverse`, `pages.move-to`, `pages.select`
- Signed files: ops refused before applying by `would_violate`, with O06's banner on certified files and the named-signature dialog for a signed widget's page ([E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint)); delete reports ("1 bookmark now points to page 4") as toasts with Undo
- Keyboard: arrow navigation, Cmd/Ctrl+Shift+arrows to move pages, Delete key; the ARIA pattern O06 recommends (a multiselectable `listbox`) or ADR 0024's grid, decided here and recorded in the area document

**Out of scope** (and which todo covers it)

- The documents dialogs and their outputs → [E2m](E2m-document-dialogs.md)
- Page setup (crop, resize, boxes, orientation, labels) → [E2g](E2g-page-labels.md)
- The desktop column and OS merge → [E2n](E2n-desktop-organise.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.01` insert blank pages; `C.03` delete and duplicate pages; `C.04` reorder pages; `C.08` rotate pages 90/180/270; `C.13` reverse page order
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer` (thumbnail drag layer), `apps/web`
- **`SaveConstraint`:** banner from E1e shows violations as the user drags on signed files; drops are refused before applying when `would_violate`
- **Web vs desktop:** the shared interface; the desktop column is [E2n](E2n-desktop-organise.md)'s
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.01`, `C.03`, `C.04`, `C.08`, `C.13` → `round-trip` (web)

## Tasks

- [ ] The Organise mode in the mode registry
- [ ] Drag-and-drop with auto-scroll and insertion indicator
- [ ] Commands, the Blank pages dialog, Move to page… and the context menu
- [ ] Signed-file refusals and reference toasts
- [ ] e2e tests tagged with capability IDs

## Acceptance criteria

- [ ] Playwright in Chromium, Firefox and WebKit for every op here: perform via the UI as a user, save, reopen, verify page facts
- [ ] Drag reorder of 200 thumbnails stays responsive (no long task > 50 ms in the C3 harness)
- [ ] Keyboard-only reorder works; axe zero serious violations
- [ ] On a signed fixture, a drag that would violate DocMDP shows the named signature before applying

## Post-merge checks

None.

## Tests

- Playwright `apps/web/e2e/organise.test.ts`
- Vitest for selection model

## Docs to update

- User help: organising pages
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Thumbnail rendering cost on huge documents during drag: reuse C3 canvas pool and low-res thumbs

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
