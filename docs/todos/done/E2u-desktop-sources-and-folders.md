# E2u — Desktop: pages from other files, merging files opened together, writing parts to a folder

| Field | Value |
| --- | --- |
| ID | `E2u` |
| Status | `done` |
| Plan | [E2 page organisation §7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [D3a](./D3a-open-from-os.md), [E2m](./E2m-document-dialogs.md), [E2n](./E2n-desktop-organise.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e2u-desktop-sources-and-folders` |
| PR title | `feat(desktop): insert, replace and merge from other files; split to a folder` |
| Size | L (800–1,200): a source-document protocol, folder output in the main process, the multi-file open offer |
| PR | [#812](https://github.com/laudebugs/laminapdf/pull/812), [#813](https://github.com/laudebugs/laminapdf/pull/813), [#814](https://github.com/laudebugs/laminapdf/pull/814), [#815](https://github.com/laudebugs/laminapdf/pull/815), [#816](https://github.com/laudebugs/laminapdf/pull/816), [#817](https://github.com/laudebugs/laminapdf/pull/817), [#818](https://github.com/laudebugs/laminapdf/pull/818) (one per step, owner's request) |

## Goal

The second half of the desktop's E2 column, split from [E2n](../done/E2n-desktop-organise.md) when E2n turned out to need the whole Organise mode first (decided 2026-11-16). E2n brings the Organise mode, its page-setup dialogs and its signature checks to the desktop. This todo adds what reads or writes other files: inserting, replacing and merging pages from another PDF, merging several PDFs opened together from Finder or Explorer, and split and extract writing their parts into a folder.

## Context

- **Design (Paper):** [O07](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents) (desktop: "Merge into one document"), [O08](../../design/paper/edit-session-organise.md#o08-split-document) (Save to a folder, Show in Finder)
- [E2m](../done/E2m-document-dialogs.md): the dialogs and `DocumentsController` (`apps/web/src/documents.tsx`), which this todo runs over the engine
- [D3a](../done/D3a-open-from-os.md): open requests, which today split a request into one `open://document` per path before the webview sees it

## Scope

**In scope**

- Source documents in the engine's edit session: a protocol message that adds a source (the façade's `add_source`), read through the main process's file access from a path the native dialog chose, never a path from the webview (ADR 0008); source page previews; `InsertPagesFrom`, `ReplacePages` and `MergeDocuments` over it
- Merging into a new file: the merged document saved through the native save dialog
- The certification banner's Save a copy and Remove signature… on the desktop ([E2t](../done/E2t-signed-file-choices.md)'s surfaces): the engine writes a copy with the refused change and without the signatures it breaks (`copyWithEdit`, which the desktop client refuses today), passed to the shared Organise mode as its `copies`
- `canWriteDirectory` true on the desktop, through the dialog plugin's folder picker; split, extract and export write into the chosen folder from the main process, with Replace / Keep both for existing names and Show in Finder or Explorer
- An open request with several PDFs offers "Merge into one document" or "Open each" (the web's `askAboutSeveral`), so the request's grouping reaches the webview
- WebdriverIO suites on macOS, Windows and Linux

**Out of scope**

- The Organise mode, page setup, labels and signature checks: [E2n](../done/E2n-desktop-organise.md)

**Editor facts**

- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-engine`, `laminapdf-desktop-protocol`, `@laminapdf/ui`
- **Capability-matrix rows to flip:** `C.02`, `C.06`, `C.16` → `round-trip` (desktop); `C.05`, `C.07`, `H.10` → `write` (desktop)

## Tasks

- [x] The engine's source documents and their previews (PR 1 of 5, [#812](https://github.com/laudebugs/laminapdf/pull/812))
- [x] Insert from a file and replace over them (PR 2 of 6)
- [x] Merge into this document (PR 3 of 7; no PDF/A or PDF/UA notice yet: the engine reads no conformance claims)
- [x] Merge into a new file, saved through the native save dialog (PR 4 of 7)
- [x] Save a copy and Remove signature… on the certification banner (`copyWithEdit`) (PR 5 of 7)
- [x] Folder output and `canWriteDirectory` (PR 6 of 7; no split at top-level bookmarks yet: the desktop reads no outline)
- [x] The merge offer on multi-file open requests (PR 7 of 7: Merge into one document, or Open each)
- [x] WebdriverIO specs, tagged with capability IDs: C.02 and C.16 with insert and replace, C.06 with merge, C.05, C.07 and H.10 with extract and split (`organise-documents.e2e.ts`, `open-several.e2e.ts`)

## Acceptance criteria

- [x] WebdriverIO on three systems: insert from a file, replace, merge, split to a folder and extract done through the interface; saved files reopened and checked
- [x] Opening three PDFs together from the OS offers the merge, and the merged file has every page and outline
- [x] An existing name in the folder asks Replace or Keep both

## Post-merge checks

None.

## Tests

- `apps/desktop/e2e/specs/organise-documents.e2e.ts` and `open-several.e2e.ts`; Rust tests for the protocol's source, merge, copy and part messages and for the main process's folders

## Docs to update

- [E2 plan §2 and §8](../../plans/editor/e2-page-organisation.md#8-todos) and [full-editor §E2](../../plans/full-editor.md#e2-page-organisation): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- A large source read whole into the engine: E2c's sources live in the edit session's blob store. **Not measured:** the engine reads each source (and each merged file) whole, with no total cap across files, and was not run against the 1 GB fixture. A file too large to hold fails as `limit`.
- Left out: a PDF/A or PDF/UA notice in the merge dialog (the engine reads no conformance claims), and splitting at top-level bookmarks (the desktop reads no outline).

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
