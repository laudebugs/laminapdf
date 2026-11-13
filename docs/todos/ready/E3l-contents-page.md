# E3l — The contents page

| Field | Value |
| --- | --- |
| ID | `E3l` |
| Status | `ready` |
| Plan | [E3 plan §2.2](../../plans/editor/e3-navigation-attachments-inspection.md#22-the-contents-page) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2c](./E2c-cross-document-pages.md), [E2o](./E2o-structure-tree-merge.md), [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md), [E3b](../ongoing/E3b-bookmarks-and-destinations.md), [E3c](./E3c-links.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0032](../../decisions/0032-flowing-layout-and-document-inputs.md) |
| Branch | `feat/e3l-contents-page` |
| PR title | `feat(ops): create a contents page from the bookmarks` |
| Size | M (200–400): one function composing existing ops, the page's layout and the dialog |
| PR | — |

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

- A contents page from a tagged file's headings → [E15b](E15b-structure-tree-editing.md)
- The desktop column → [E3k](E3k-desktop-navigation-and-export.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.05` clickable table of contents
- **Crates and packages:** `laminapdf` (the function), `laminapdf-build` (the page), `@laminapdf/ui`
- **`Op` variants:** none new: `InsertPagesFrom` (`PageStructure`), `AddLink` (`Annotation`), `AddOutlineItem` (`Document`)
- **`SaveConstraint`:** `Free` → incremental; approval signatures only → `AppendOnly`; any certification refuses it, since inserting a page is `PageStructure`
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `I.05` → `round-trip` (core, pdf, web)

## Tasks

- [ ] The page's layout over the builder's flowing layout
- [ ] The function as one transaction, with undo as one step
- [ ] Bindings and examples
- [ ] Dialog with preview

## Acceptance criteria

- [ ] Round-trip: open → contents page → save → reopen in lamina, pdf.js and PDFium; each entry's link lands on its bookmark's page; `qpdf --check` is clean
- [ ] Replaying the op log on a fresh copy gives byte-identical output
- [ ] veraPDF reports nothing new on `golden/tagged-report.pdf` with a contents page in an embedded font
- [ ] Undo removes the page, its links and its bookmark in one step

## Post-merge checks

None.

## Tests

- Unit tests of the entries, labels and leaders
- A golden of the contents page for `fixtures/navigation/outline.pdf`
- Playwright: create a contents page, click an entry, save, reopen

## Docs to update

- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
