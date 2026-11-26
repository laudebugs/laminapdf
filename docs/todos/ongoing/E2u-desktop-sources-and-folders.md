# E2u — Desktop: pages from other files, merging files opened together, writing parts to a folder

| Field | Value |
| --- | --- |
| ID | `E2u` |
| Status | `ongoing` |
| Plan | [E2 page organisation §7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [D3a](../done/D3a-open-from-os.md), [E2m](../done/E2m-document-dialogs.md), [E2n](../done/E2n-desktop-organise.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e2u-desktop-sources-and-folders` |
| PR title | `feat(desktop): insert, replace and merge from other files; split to a folder` |
| Size | L (800–1,200): a source-document protocol, folder output in the main process, the multi-file open offer |
| PR | [#812](https://github.com/laudebugs/laminapdf/pull/812) |

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
- [ ] Insert from a file, replace and merge over them; merge into a new file
- [ ] Folder output and `canWriteDirectory`
- [ ] The merge offer on multi-file open requests
- [ ] WebdriverIO specs, tagged with capability IDs

## Acceptance criteria

- [ ] WebdriverIO on three systems: insert from a file, replace, merge, split to a folder and extract done through the interface; saved files reopened and checked
- [ ] Opening three PDFs together from the OS offers the merge, and the merged file has every page and outline
- [ ] An existing name in the folder asks Replace or Keep both

## Post-merge checks

None.

## Tests

- `apps/desktop/e2e/specs/organise-documents.e2e.ts`; Rust tests for the protocol's source messages

## Docs to update

- [E2 plan §2 and §8](../../plans/editor/e2-page-organisation.md#8-todos) and [full-editor §E2](../../plans/full-editor.md#e2-page-organisation): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- A large source read whole into the engine: E2c's sources live in the edit session's blob store; check the engine's memory limit against the 1 GB fixture.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
