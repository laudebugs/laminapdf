# E2n — Desktop: Organise in the engine, merging files opened together, writing parts to a folder

| Field | Value |
| --- | --- |
| ID | `E2n` |
| Status | `ready` |
| Plan | [E2 page organisation §7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [D3a](../done/D3a-open-from-os.md), [E1i](./E1i-desktop-save-in-place.md), [E2g](./E2g-page-labels.md), [E2m](./E2m-document-dialogs.md) |
| ADRs | Follows [0008](../../decisions/0008-desktop-shell.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e2n-desktop-organise` |
| PR title | `feat(desktop): organise pages, merge from Finder and Explorer, split to a folder` |
| Size | M (200–400): Organise in the engine, the OS merge and folder outputs, over the web's dialogs |
| PR | — |

## Goal

The desktop part of round E2 ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): an editor round's desktop part is its own todo). The Organise mode and every E2 dialog run over the engine's edit session; several PDFs opened together from Finder or Explorer offer a merge; split and export write into a folder. This completes the desktop column of every E2 row.

## Context

- **Design (Paper):** [O06](../../design/paper/edit-session-organise.md#o06-organise-mode), [O07](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents) (desktop: "Merge into one document"), [O08](../../design/paper/edit-session-organise.md#o08-split-document) (Save to a folder, Show in Finder)
- [E2 plan §5 and §7](../../plans/editor/e2-page-organisation.md#7-differences-between-the-shells)
- [E1h](E1h-desktop-edit-session.md) and [E1i](E1i-desktop-save-in-place.md): the engine's edit session and saving in place; [D3a](../done/D3a-open-from-os.md): open requests with several paths

## Scope

**In scope**

- The engine's edit session reports the new page order as the worker does ([E2l](../ongoing/E2l-viewer-across-page-edits.md)), so the shared viewer follows page-list edits on the desktop
- `canWriteDirectory` true on the desktop, through the dialog plugin; split and export write into the chosen folder, with Replace / Keep both for existing names and Show in Finder or Explorer
- An open request with several PDFs offers "Merge into one document" or "Open each"
- Source documents read through the engine's file access, never a path from the webview (ADR 0008)
- WebdriverIO suites on macOS, Windows and Linux

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): every E2 row's desktop column
- **Crates and packages:** `apps/desktop`, `laminapdf-desktop-engine`, `laminapdf-desktop-protocol`
- **`SaveConstraint`:** as the web; the banner and the named-signature dialog checked on the desktop here
- **Web vs desktop:** this todo is the desktop side
- **Server features:** none
- **Capability-matrix rows to flip:** `C.01`, `C.02`, `C.03`, `C.04`, `C.06`, `C.08`, `C.09`, `C.10`, `C.11`, `C.12`, `C.13`, `C.16`, `I.10` → `round-trip` (desktop); `C.05`, `C.07`, `H.10` → `write` (desktop)

## Tasks

- [ ] Page order from the engine's session
- [ ] Folder output and `canWriteDirectory`
- [ ] The merge offer on multi-file open requests
- [ ] WebdriverIO specs `apps/desktop/e2e/specs/organise.e2e.ts`, tagged with capability IDs

## Acceptance criteria

- [ ] WebdriverIO on three systems: reorder, delete, insert from a file, merge, split to a folder and a page-label edit done through the interface, saved in place, reopened and checked
- [ ] Opening three PDFs together from the OS offers the merge, and the merged file has every page and outline
- [ ] On the approval-signed fixture, deleting the signature's page shows the named-signature dialog; on a certified one, a drag is refused with the banner

## Post-merge checks

None.

## Tests

- `apps/desktop/e2e/specs/organise.e2e.ts`

## Docs to update

- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and the [E2 plan §8](../../plans/editor/e2-page-organisation.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
