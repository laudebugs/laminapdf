# E3j — The export dialog: pages, text, images, links and bookmarks

| Field | Value |
| --- | --- |
| ID | `E3j` |
| Status | `done` |
| Plan | [E3 plan §7](../../plans/editor/e3-navigation-attachments-inspection.md#7-differences-between-the-shells) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2m](./E2m-document-dialogs.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md), [E3h](./E3h-images-to-pdf-and-page-export.md), [E3i](./E3i-extraction.md), [E3m](./E3m-web-image-export.md) |
| ADRs | Follows [0010](../../decisions/0010-platform-adapter.md), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) |
| Branch | `feat/e3j-export-dialog` |
| PR title | `feat(web): export pages, text, images, links and bookmarks` |
| Size | M (200–400): one dialog over E3h's and E3i's functions, the progress view and a store-only zip writer |
| PR | [#690](https://github.com/laudebugs/laminapdf/pull/690) |

## Goal

The interface for the round's exports: one dialog (O16) that exports pages as PNG, JPEG or TIFF, text as plain text, Markdown or JSON with positions, the file's embedded images, and its links and bookmarks as JSON or CSV. The todo also adds images → PDF to Insert in Organise (O07). On the web, several files stream into a zip one page at a time; with `canWriteDirectory` they go into a folder.

## Context

- **Design (Paper):** [O16 Export and extraction](../../design/paper/pages-navigation-extraction.md#o16-export-and-extraction), [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E3 plan §2.3 and §7](../../plans/editor/e3-navigation-attachments-inspection.md#23-functions-that-only-read-or-write-new-files)
- [E2m](../done/E2m-document-dialogs.md): `canWriteDirectory` and the folder output

## Scope

**In scope**

- The export dialog: kind, options per kind, the page range (the print dialog's range parser), the progress view with Cancel
- The web zip: entries stored, not compressed again, streamed into a `Blob` as each page finishes, within `Limits`; one file downloads directly
- Images → PDF from Organise's Insert menu, as a new document
- Reading positions, history and the edit session untouched: every export reads the document as it is now, edits included

**Out of scope** (and which todo covers it)

- The desktop's folder output through the engine → [E3k](../ready/E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): the web column of `H.02`, `H.06`, `H.11`, `H.07`, `H.08`, `B.03`, `A.10`
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer` (the worker's export calls), `apps/web`
- **`SaveConstraint`:** read-only; images → PDF makes a new document, saved by a full write
- **Web vs desktop:** this todo is the web side
- **Server features:** none (nothing is uploaded)
- **Capability-matrix rows to flip:** `H.02` → `write` (web); `H.06`, `H.11`, `H.07`, `H.08`, `B.03`, `A.10` → `read` (web)

## Tasks

- [x] Dialog and progress view, behind the lazily loaded chunk (`apps/web/src/later.ts`)
- [x] Store-only zip writer with CRC-32, streamed (`ZipWriter` in `@laminapdf/viewer`, sharing the zip format with Split's writer)
- [x] Images → PDF in Organise
- [x] Playwright specs tagged with capability IDs (`apps/web/e2e/export.e2e.test.ts`)

## Acceptance criteria

- [ ] Exporting 500 pages as PNG on the web streams into a zip without passing the memory budget, and Cancel stops it within one page. Partly: each page is drawn on its own and goes straight into the zip as `Blob` parts, so the zip never sits in one buffer, and an e2e test cancels a 40-page export with nothing downloaded; a 500-page run and its memory weren't measured.
- [x] Each export's output opens in a standard tool: the zip in the system's unzip (`ZipWriter`'s test runs `unzip -t`), the images in an image viewer (PNG signatures in the e2e test), the JSON against its committed schema (extract, links and outline, with Ajv)
- [x] The reader's startup size stays within `apps/web/size-budget.json` (raised to 286600 for the five commands, the More menu item and the dialog's state; the work, words and save code load lazily)

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

## As built

- **No Extract menu in Organise**, so "As images…" isn't there; Export pages as images is in the palette and Export… in the More menu.
- **Export links and bookmarks takes no page range**: links and bookmarks belong to the whole file.
- **From images makes a new PDF and saves it as a file of its own**, as Merge's new document does, rather than opening it in place of the document being edited; Insert's item reads "New PDF from images…". PNG and JPEG only, which a PDF holds as they are.
- **Several files on the web go into one zip**; where the browser can write a folder (Chromium), into a folder the person picks first, with ` (2)` added to a name already there.
- **Where the files go is settled before the work**: one file (a page, a TIFF of every page, or the text) is named then and its save dialog opens at once, while the click still lets the browser show it, and is written when made; a browser that refuses the dialog all the same downloads it. Pictures, whose number isn't known first, always go into a zip or a folder.
- **Pictures aren't counted when the dialog opens**: the SDK counts them only by decoding every one. The card stays open and its button reads "Export pictures".
- **A page named twice in a range is exported once.**
- **A page range with no pictures** says so ("These pages have no pictures to export") rather than saving an empty zip: pictures are counted only as they are read.
- **On Chromium, the file the save dialog made stays empty** when the export then fails or is cancelled: the browser creates it when the person chooses it, before the work. A refused folder picker falls back to a zip.
- **The desktop's open dialog ignores `openFiles`' new `accept: "images"`**: desktop has no From images yet, and [E3k](../ready/E3k-desktop-navigation-and-export.md) must give its dialog the pictures' filter before wiring it.
- **A TIFF too large for one file** (past the worker's 256 MB budget) fails with the export's message rather than becoming several files under one name.
- The palette-only commands have a home of their own, `export-dialog`, since every command needs one besides the palette (ADR 0024).

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
