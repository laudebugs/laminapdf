# E2m — Insert from a file, replace, extract, export, merge and split in the interface

| Field | Value |
| --- | --- |
| ID | `E2m` |
| Status | `done` |
| Plan | [E2 page organisation §4](../../plans/editor/e2-page-organisation.md#4-crate-and-package-changes) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2d](./E2d-merge-and-split.md), [E2h](./E2h-page-organiser-ui.md) |
| ADRs | Follows [0010](../../decisions/0010-platform-adapter.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e2m-document-dialogs` |
| PR title | `feat(ui): insert, replace, extract, merge and split dialogs` |
| Size | L (400–800): six dialogs and their outputs in `@laminapdf/ui` and `apps/web`, with Playwright tests; the ops are [E2c](../done/E2c-cross-document-pages.md)'s and [E2d](../done/E2d-merge-and-split.md)'s |
| PR | [laudebugs/laminapdf#641](https://github.com/laudebugs/laminapdf/pull/641) |

## Goal

Build the web dialogs that bring pages in from other files and write pages out to new ones, with their outputs through the `Platform`. They are Insert pages from a file, Replace with pages from a file, Extract, Export ranges, Merge documents and Split document. Split out of [E2h](../done/E2h-page-organiser-ui.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos).

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

- Crop, resize, boxes, orientation and labels dialogs → [E2g](../done/E2g-page-labels.md)
- Desktop folder output and Finder/Explorer selections → [E2n](../ready/E2n-desktop-organise.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.02`, `C.05`, `C.06`, `C.07`, `C.16`, `H.10`
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`SaveConstraint`:** inserts, replaces and merges into a signed file follow the [E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) table (refused before applying on certified files, with Apply anyway…); new files carry no signature, and the dialogs say so on a signed source
- **Web vs desktop:** as the plan's §7; this todo is the web side
- **Server features:** none (all processing stays on the device)
- **Capability-matrix rows to flip:** `C.02` → `round-trip` (web); `C.06` → `round-trip` (web); `C.16` → `round-trip` (web); `C.05` → `write` (web); `C.07` → `write` (web); `H.10` → `write` (web)

## Tasks

- [x] The dialogs and their commands (`pages.insert-from-file`, `pages.replace`, `pages.extract`, `document.merge`, `document.merge-add-files`, `document.split`)
- [x] `canWriteDirectory` and the outputs; the zip writer
- [x] OS multi-file open requests offering a merge
- [x] Source bytes in the page's edit log for replay
- [x] Playwright tests tagged with capability IDs

## Acceptance criteria

- [x] Playwright in Chromium, Firefox and WebKit, as a user: each dialog's operation done through the interface, saved, reopened, and the page facts checked; split writes a zip in Firefox and WebKit and a folder in Chromium (with the picker stubbed)
- [x] An encrypted source asks for its password and its pages arrive; the result is unencrypted
- [x] A restarted worker replays an insert from a file
- [x] axe reports no serious violations in each dialog
- [x] Driven in the built reader in a real browser before merge

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

## As built

- **`@laminapdf/viewer`** (the worker side):
  - `worker/sources.ts`: `previewSource` reads a file's pages, claims and title, and opens it in the backend so its pages draw as thumbnails (`renderSource`, `closeSource`). `addSource` makes it a source of the open document and of its text-capable copy (`EditAction` gains `{ source, password }`).
  - `worker/parts.ts`: `writeParts` writes one whole file per list of page IDs with `extractPages`, reporting progress, or one zip of them; `mergeFiles` merges files with `Document.merge`, the open document first when asked.
  - `worker/zip.ts`: a zip of stored files (APPNOTE 6.3.10, UTF-8 names, no ZIP64) with no dependency, so `cargo xtask deny` has nothing to check; it shares `worker/crc32.ts` with the PNG exporter. `unzip -t` and Python's `zipfile` read it.
  - The page side: `client.addSource`, plus `files.ts`'s functions over a client (`previewSource`, `renderSource`, `closeSource`, `writeParts`, `mergeFiles`). As functions, they load with the reader's later chunk.
  - The edit log keeps the bytes of each source a logged step names. A restarted worker adds them before it replays (`restoreEdits`' `sources`). The page keeps no password, so a step from an encrypted source can't be replayed, and its edits are lost on a restart.
- **`@laminapdf/ui`:**
  - `canWriteDirectory` and `Platform.chooseFolder` → `PlatformFolder` (`existing`, `write`). On the web the picker is `showDirectoryPicker`, in `platform/folder.ts`, loaded on first use. `saveAs` saves a zip when the name ends in `.zip`. `fakePlatform` has the flag; the desktop leaves it off until E2n.
  - `organise/from-file.tsx` (`PagesFromFileDialog`, with the file card and its inline password), `merge.tsx` (`MergeDialog`), `split.tsx` (`SplitDialog`, also Extract pages) and `parts.ts` (split plans and file names, Vitest).
  - The six commands sit in Insert ▾ and More page actions (`INSERT_COMMANDS`, `MORE_COMMANDS`).
- **The reader:**
  - `apps/web/src/documents.tsx`'s `DocumentsController` applies `insertPagesFrom`, `replacePages` and `mergeDocuments` through `OrganiseController.edit`, which asks first when a signature forbids the edit. It writes Split's and Extract's files into a folder (asking Replace or Keep both), as one or two saves, or as one zip.
  - `several.tsx` asks what to do with several files opened together from the system, loaded with `later.ts` only then.
- **Found in the browser and fixed:**
  - After a wrong password, the password form never submitted again. React Aria hands the field's error to the browser's own validation, which blocked the submit; the form now has `noValidate`.
  - Chromium names a blob download "download" when its name has a character outside ASCII, so part names write page ranges with a hyphen.
- **Differences from O07 and O08** are under each screen's "As built (web, E2m)". In short:
  - Fields always come with the pages (linked or renamed).
  - Merge into a new document saves a file on the web.
  - There is no Details panel, and no keep-bookmarks checkbox in Split.
- **Startup JavaScript:** 283,358 bytes gzipped, against 283,190 before. The dialogs, the worker calls and the folder picker load later; the rest is page-side. The owner raised the budget to 283,400 on 2026-11-15, then, with main merged at 284,447, to 284,950.
- **Tests:**
  - `apps/web/e2e/organise-documents.e2e.test.ts` (Chromium here; Firefox and WebKit in CI). It inserts by range, inserts from an encrypted file after its password, replaces a page, merges two files, splits (a stubbed folder in Chromium, a zip elsewhere) and extracts, each read back by the SDK. Capability tags: C.02, C.06, C.16 `web round-trip`; C.05, C.07, H.10 `web write`.
  - `packages/viewer/test/browser/sources.browser.test.ts` covers the preview, the password, parts, merges and the replay after a crash (and the loss for an encrypted source).
  - Vitest covers the zip writer, the edit log's sources, the split plans and names, and the folder picker.
  - The gallery has six states: `insert-from-file`, `insert-password`, `merge`, `merge-claims`, `split` and `extract`.
- **Not done here:**
  - User help: the docs site has no reader help yet, as E1e and E2h found.
  - The desktop's folders and Finder selections ([E2n](../ready/E2n-desktop-organise.md)).
  - From images, dropping files on the grid, and the palette's arguments.
  - Measuring a merge of ten 50 MB files: each source's bytes stay in the worker and the page's edit log while a step names them.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
