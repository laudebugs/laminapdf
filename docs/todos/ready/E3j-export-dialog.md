# E3j — The export dialog: pages, text, images, links and bookmarks

| Field | Value |
| --- | --- |
| ID | `E3j` |
| Status | `ready` |
| Plan | [E3 plan §7](../../plans/editor/e3-navigation-attachments-inspection.md#7-differences-between-the-shells) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2m](./E2m-document-dialogs.md), [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md), [E3h](./E3h-images-to-pdf-and-page-export.md), [E3i](./E3i-extraction.md) |
| ADRs | Follows [0010](../../decisions/0010-platform-adapter.md), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) |
| Branch | `feat/e3j-export-dialog` |
| PR title | `feat(web): export pages, text, images, links and bookmarks` |
| Size | M (200–400): one dialog over E3h's and E3i's functions, the progress view and a store-only zip writer |
| PR | — |

## Goal

The interface for the round's exports: one dialog (O16) that exports pages as PNG, JPEG or TIFF, text as plain text, Markdown or JSON with positions, the file's embedded images, and its links and bookmarks as JSON or CSV. The todo also adds images → PDF to Insert in Organise (O07). On the web, several files stream into a zip one page at a time; with `canWriteDirectory` they go into a folder.

## Context

- **Design (Paper):** [O16 Export and extraction](../../design/paper/pages-navigation-extraction.md#o16-export-and-extraction), [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E3 plan §2.3 and §7](../../plans/editor/e3-navigation-attachments-inspection.md#23-functions-that-only-read-or-write-new-files)
- [E2m](E2m-document-dialogs.md): `canWriteDirectory` and the folder output

## Scope

**In scope**

- The export dialog: kind, options per kind, the page range (the print dialog's range parser), the progress view with Cancel
- The web zip: entries stored, not compressed again, streamed into a `Blob` as each page finishes, within `Limits`; one file downloads directly
- Images → PDF from Organise's Insert menu, as a new document
- Reading positions, history and the edit session untouched: every export reads the document as it is now, edits included

**Out of scope** (and which todo covers it)

- The desktop's folder output through the engine → [E3k](E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): the web column of `H.02`, `H.06`, `H.11`, `H.07`, `H.08`, `B.03`, `A.10`
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer` (the worker's export calls), `apps/web`
- **`SaveConstraint`:** read-only; images → PDF makes a new document, saved by a full write
- **Web vs desktop:** this todo is the web side
- **Server features:** none (nothing is uploaded)
- **Capability-matrix rows to flip:** `H.02` → `write` (web); `H.06`, `H.11`, `H.07`, `H.08`, `B.03`, `A.10` → `read` (web)

## Tasks

- [ ] Dialog and progress view, behind the lazily loaded chunk (`apps/web/src/later.ts`)
- [ ] Store-only zip writer with CRC-32, streamed
- [ ] Images → PDF in Organise
- [ ] Playwright specs tagged with capability IDs

## Acceptance criteria

- [ ] Exporting 500 pages as PNG on the web streams into a zip without passing the memory budget, and Cancel stops it within one page
- [ ] Each export's output opens in a standard tool: the zip in the system's unzip, the images in an image viewer, the JSON against its committed schema
- [ ] The reader's startup size stays within `apps/web/size-budget.json`

## Post-merge checks

None.

## Tests

- Playwright in Chromium, Firefox and WebKit: each export kind driven as a user, its output checked
- Unit tests of the zip writer against `unzip -t`

## Docs to update

- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
