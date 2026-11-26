# E3l — The contents page

| Field | Value |
| --- | --- |
| ID | `E3l` |
| Status | `done` |
| Plan | [E3 plan §2.2](../../plans/editor/e3-navigation-attachments-inspection.md#22-the-contents-page) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2c](./E2c-cross-document-pages.md), [E2o](./E2o-structure-tree-merge.md), [E3a](./E3a-navigation-attachments-inspection-feature-plan.md), [E3b](./E3b-bookmarks-and-destinations.md), [E3c](./E3c-links.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0032](../../decisions/0032-flowing-layout-and-document-inputs.md) |
| Branch | `feat/e3l-contents-page` |
| PR title | `feat(ops): create a contents page from the bookmarks` |
| Size | M (200–400): one function composing existing ops, the page's layout and the dialog |
| PR | [#645](https://github.com/laudebugs/laminapdf/pull/645) |

## Goal

Create a clickable contents page from the bookmarks, one or two levels deep, inserted before or after a page or at the end, as one undo step. `create_contents_page(&mut doc, &ContentsOptions)` builds the page as a new document, adds it as a source, inserts it with `InsertPagesFrom` and links each entry with `AddLink`, all in one transaction; no new op is needed ([E3 plan §2.2](../../plans/editor/e3-navigation-attachments-inspection.md#22-the-contents-page)).

## Context

- **Design (Paper):** [O11 Bookmarks editor](../../design/paper/pages-navigation-extraction.md#o11-bookmarks-editor) (the contents page dialog). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E3 plan §2.2](../../plans/editor/e3-navigation-attachments-inspection.md#22-the-contents-page) and [§11](../../plans/editor/e3-navigation-attachments-inspection.md#11-decisions-on-the-open-questions): bookmarks only, no guessed headings
- The builder's flowing layout and its `Contents` block (T4h): leaders and page numbers

## Scope

**In scope**

- `create_contents_page` in the façade and `doc.createContentsPage` in `@laminapdf/sdk`, with an example in each language
- The entries show the page labels readers will see after the insert (`PageLabels::remap`, E2j)
- Tagged files: the page is built tagged (`TOC`, `TOCI`) and joins the structure tree through E2o
- `ContentsOptions` names the font; the library's default, Helvetica, raises `font-not-embedded` on a PDF/A or PDF/UA-1 file; the app passes its bundled font
- The dialog: Insert, Levels, Page title, "Add a Contents bookmark", the preview and the consequence sentence

**Out of scope** (and which todo covers it)

- A contents page from a tagged file's headings → [E15b](../ready/E15b-structure-tree-editing.md)
- The desktop column → [E3k](../ongoing/E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.05` clickable table of contents
- **Crates and packages:** `laminapdf` (the function), `laminapdf-build` (the page), `@laminapdf/ui`
- **`Op` variants:** none new: `InsertPagesFrom` (`PageStructure`), `AddLink` (`Annotation`), `AddOutlineItem` (`Document`)
- **`SaveConstraint`:** `Free` → incremental; approval signatures only → `AppendOnly`; any certification refuses it, since inserting a page is `PageStructure`
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `I.05` → `round-trip` (core, pdf, web)

## Tasks

- [x] The page's layout over the builder's flowing layout
- [x] The function as one transaction, with undo as one step
- [x] Bindings and examples
- [x] Dialog with preview

## Acceptance criteria

- [x] Round-trip: open → contents page → save → reopen in lamina, pdf.js and PDFium; each entry's link lands on its bookmark's page; `qpdf --check` is clean (the round-trip suite's `a_contents_pages_links_land_on_their_bookmarks_pages_after_a_save`, with `golden/examples/create_contents_page.pdf` among the suite's files, whose saves the full lane checks with qpdf, pdf.js and PDFium; the reader's e2e test follows an entry, saves and reads the links back)
- [x] Replaying the op log on a fresh copy gives byte-identical output (`creating_a_contents_page_twice_on_fresh_copies_should_save_the_same_bytes`: the contents page is a façade function over ops, not an op of its own)
- [x] veraPDF reports nothing new on a tagged file with a contents page in an embedded font: `golden/examples/create_contents_page-pdfua.pdf`, made from `golden/flow-table.pdf`, passes PDF/UA-1 as the file it came from does. `golden/tagged-report.pdf` claims no profile, so veraPDF has nothing to check on it
- [x] Undo removes the page, its links and its bookmark in one step

## Post-merge checks

None.

## Tests

- Unit tests of the entries, labels and leaders
- A golden of the contents page for `golden/links-and-outlines.pdf` (`fixtures/navigation/outline.pdf` doesn't exist; that golden holds the bookmarks two levels deep, a named destination and labels), and one for `golden/flow-table.pdf` under PDF/UA-1
- Playwright: create a contents page, click an entry, save, read the links back

## Docs to update

- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Inserting before a first page labelled 1 (or "i") gave the new page that label too. The bug [page-label-repeated-before-the-first-page](../../bugs/page-label-repeated-before-the-first-page/detail.md) fixed it on `main` as O11 asks: the contents page joins the first range, so it is "i" and the title page "ii". This PR's tests and golden expect that

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
