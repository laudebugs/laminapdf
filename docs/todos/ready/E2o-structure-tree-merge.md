# E2o — Merge structure trees, for merged and imported pages

| Field | Value |
| --- | --- |
| ID | `E2o` |
| Status | `ready` |
| Plan | [E2 page organisation §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2d](./E2d-merge-and-split.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md) |
| Branch | `feat/e2o-structure-tree-merge` |
| PR title | `feat(model): merge structure trees with ParentTree renumbering` |
| Size | L (400–800): the structure-tree merge in `laminapdf-model`, `/ClassMap` and `/IDTree`, with veraPDF checks |
| PR | — |

## Goal

Keep tags when pages come from another document. Until this todo, `InsertPagesFrom`, `ReplacePages` and `MergeDocuments` drop the imported pages' structure and report `TagsDropped`; here the source's elements join the target's `/StructTreeRoot`. Most mergers drop tags, which destroys PDF/UA conformance ([05 §3.1](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)). Split out of [E2d](E2d-merge-and-split.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos).

## Context

- [05 §3.1 Merge and split](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split): concatenate `/K` children, renumber every `/StructParents` and `/StructParent`, merge `/RoleMap`, `/ClassMap` and namespaces
- [03 §9 Tagged PDF](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149)
- ISO 32000-2 §14.7 (logical structure), §14.7.4 (namespaces), §14.8 (Tagged PDF)
- `crates/laminapdf-model/src/structure.rs` (reads `/RoleMap`, `/ParentTree`, `/ParentTreeNextKey`; not `/ClassMap`, `/IDTree` or namespaces yet)

## Scope

**In scope**

- Read `/ClassMap`, `/IDTree` and `/Namespaces`
- For each source: its elements for the imported pages placed under the target's top element (a `Document` or `Part` per source on a merge), new `/StructParents` and `/StructParent` keys from `/ParentTreeNextKey` with `/ParentTree` entries through [E2j](../done/E2j-page-tree-and-tree-writers.md)'s number-tree writer, MCIDs untouched (they are per page)
- `/RoleMap` and `/ClassMap` merged; a clash (one name mapped two ways) renames the added one and reports it; `/IDTree` IDs that clash renamed
- Elements whose content is only partly on imported pages: kept with only the imported content items, or dropped with `TagsDropped` when that leaves an invalid element
- The target becomes tagged (`/MarkInfo /Marked true`) only if it was tagged or every source was
- Applied by `InsertPagesFrom`, `ReplacePages` and `MergeDocuments`

**Out of scope** (and which todo covers it)

- Editing the structure tree → [E15b](E15b-structure-tree-editing.md)

**Editor facts**

- **Operations shipped:** completes `C.02`, `C.06` and `C.16` for tagged files
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`
- **`Op` variants:** none new (the import and merge ops gain the step)
- **`SaveConstraint`:** unchanged: the ops stay `PageStructure`
- **Web vs desktop:** identical (core)
- **Server features:** none
- **Capability-matrix rows to flip:** none (verified by tests on the rows E2c and E2d flip)

## Tasks

- [ ] Read `/ClassMap`, `/IDTree` and namespaces in `StructTree`
- [ ] The merge step, with `/ParentTree` renumbering and map clashes
- [ ] Partial elements and the fallback
- [ ] Wire it into the three ops; remove `TagsDropped` where tags are now kept

## Acceptance criteria

- [ ] Merging two tagged PDF/UA fixtures gives a tagged result whose structure passes veraPDF's structure checks; the PDF/UA claim is reported as lost unless both sources claim it
- [ ] Inserting pages of `golden/tagged-report.pdf` into `golden/tagged-across-pages.pdf` keeps both trees: pdf.js `getStructTree` and PDFium's structure facts list every element on the pages they belong to
- [ ] No `/StructParents` key is shared between pages; every `/ParentTree` entry points at an element whose `/Pg` is that page
- [ ] Apply then undo leaves the overlay empty

## Post-merge checks

None.

## Tests

- `crates/laminapdf/tests/merge_structure.rs`; structure facts with `tools/fixtures/structure_facts.py`

## Docs to update

- Guide: merging tagged PDFs
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and the [E2 plan §8](../../plans/editor/e2-page-organisation.md#8-todos): point this todo's link at `done/`

## Risks and open questions

- `/K` arrays with content items on other pages; fall back to dropping tags with a warning rather than writing an invalid tree

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
