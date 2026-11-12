# E2m — Insert from a file, replace, extract, export, merge and split in the interface

| Field | Value |
| --- | --- |
| ID | `E2m` |
| Status | `ready` |
| Plan | [E2 page organisation §4](../../plans/editor/e2-page-organisation.md#4-crate-and-package-changes) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2d](./E2d-merge-and-split.md), [E2h](./E2h-page-organiser-ui.md) |
| ADRs | Follows [0010](../../decisions/0010-platform-adapter.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e2m-document-dialogs` |
| PR title | `feat(ui): insert, replace, extract, merge and split dialogs` |
| Size | L (400–800): six dialogs and their outputs in `@laminapdf/ui` and `apps/web`, with Playwright tests; the ops are [E2c](E2c-cross-document-pages.md)'s and [E2d](E2d-merge-and-split.md)'s |
| PR | — |

## Goal

The dialogs that bring pages in from other files and write pages out to new ones, on the web: Insert pages from a file, Replace with pages from a file, Extract, Export ranges, Merge documents and Split document, with their outputs through the `Platform`. Split out of [E2h](E2h-page-organiser-ui.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos).

## Context

- **Design (Paper):** [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents), [O08 Split document](../../design/paper/edit-session-organise.md#o08-split-document), [O06](../../design/paper/edit-session-organise.md#o06-organise-mode)'s Extract and More page actions. Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) (source documents, functions, ranges) and [§7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells) (capability flags)
- `packages/ui/src/reader/print-pages.ts`: the range grammar the dialogs reuse

## Scope

**In scope**

- Insert from a file and Replace (O07): the source added with `addSource` (an encrypted source asks for its password inline, R03), page selection by range or thumbnails, position, Carry options
- Merge (O07): the ordered file list, result (add to this document or a new one), bookmarks and field-name options, the conformance notice, the merge report toast and Details
- Several files opened together from the OS where `hasFileHandlers` (the installed web app): offer "Merge into one document" or "Open each"
- Extract, Export ranges and Split (O08): previews, live validation, progress; outputs by `canSaveAsDialog` (one file) and the new `canWriteDirectory` flag (several files into a chosen folder; else one zip download, two files separately), with a small streaming zip writer whose licence `cargo xtask deny` accepts
- The `canWriteDirectory` flag in `Capabilities` (`showDirectoryPicker` on the web), with `fakePlatform` support
- The page's edit log keeps each source's bytes, so a restarted worker re-adds sources before replaying ops that name them

**Out of scope** (and which todo covers it)

- Crop, resize, boxes, orientation and labels dialogs → [E2g](E2g-page-labels.md)
- Desktop folder output and Finder/Explorer selections → [E2n](E2n-desktop-organise.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.02`, `C.05`, `C.06`, `C.07`, `C.16`, `H.10`
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`SaveConstraint`:** inserts, replaces and merges into a signed file follow the [E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) table (refused before applying on certified files, with Apply anyway…); new files carry no signature, and the dialogs say so on a signed source
- **Web vs desktop:** as the plan's §7; this todo is the web side
- **Server features:** none (all processing stays on the device)
- **Capability-matrix rows to flip:** `C.02` → `round-trip` (web); `C.06` → `round-trip` (web); `C.16` → `round-trip` (web); `C.05` → `write` (web); `C.07` → `write` (web); `H.10` → `write` (web)

## Tasks

- [ ] The dialogs and their commands (`pages.insert-from-file`, `pages.replace`, `pages.extract`, `document.merge`, `document.merge-add-files`, `document.split`)
- [ ] `canWriteDirectory` and the outputs; the zip writer
- [ ] OS multi-file open requests offering a merge
- [ ] Source bytes in the page's edit log for replay
- [ ] Playwright tests tagged with capability IDs

## Acceptance criteria

- [ ] Playwright in Chromium, Firefox and WebKit, as a user: each dialog's operation done through the interface, saved, reopened, and the page facts checked; split writes a zip in Firefox and WebKit and a folder in Chromium (with the picker stubbed)
- [ ] An encrypted source asks for its password and its pages arrive; the result is unencrypted
- [ ] A restarted worker replays an insert from a file
- [ ] axe reports no serious violations in each dialog
- [ ] Driven in the built reader in a real browser before merge

## Post-merge checks

None.

## Tests

- `apps/web/e2e/organise-documents.test.ts`; Vitest for the range validation and the zip writer

## Docs to update

- User help: merging and splitting
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and the [E2 plan §8](../../plans/editor/e2-page-organisation.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Memory: a merge of many large sources holds every source in memory for undo; measure with ten 50 MB files and say what happens past the budget

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
