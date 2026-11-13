# E2o — Merge structure trees, for merged and imported pages

| Field | Value |
| --- | --- |
| ID | `E2o` |
| Status | `done` |
| Plan | [E2 page organisation §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2d](./E2d-merge-and-split.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md) |
| Branch | `feat/e2o-structure-tree-merge` |
| PR title | `feat(model): merge structure trees with ParentTree renumbering` |
| Size | L (400–800): the structure-tree merge in `laminapdf-model`, `/ClassMap` and `/IDTree`, with veraPDF checks |
| PR | [#572](https://github.com/laudebugs/laminapdf/pull/572) |

## Goal

Keep tags when pages come from another document. Until this todo, `InsertPagesFrom`, `ReplacePages` and `MergeDocuments` drop the imported pages' structure and report `TagsDropped`; here the source's elements join the target's `/StructTreeRoot`. Most mergers drop tags, which destroys PDF/UA conformance ([05 §3.1](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)). Split out of [E2d](../done/E2d-merge-and-split.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos).

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

- Editing the structure tree → [E15b](../ready/E15b-structure-tree-editing.md)

**Editor facts**

- **Operations shipped:** completes `C.02`, `C.06` and `C.16` for tagged files
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`
- **`Op` variants:** none new (the import and merge ops gain the step)
- **`SaveConstraint`:** unchanged: the ops stay `PageStructure`
- **Web vs desktop:** identical (core)
- **Server features:** none
- **Capability-matrix rows to flip:** none (verified by tests on the rows E2c and E2d flip)

## Tasks

- [x] Read `/ClassMap`, `/IDTree` and namespaces in `StructTree`
- [x] The merge step, with `/ParentTree` renumbering and map clashes
- [x] Partial elements and the fallback
- [x] Wire it into the three ops; remove `TagsDropped` where tags are now kept

## Acceptance criteria

- [x] Merging two tagged PDF/UA fixtures gives a tagged result whose structure passes veraPDF's structure checks; the PDF/UA claim is reported as lost unless both sources claim it
- [x] Inserting pages of `golden/tagged-report.pdf` into `golden/tagged-across-pages.pdf` keeps both trees: pdf.js `getStructTree` and PDFium's structure facts list every element on the pages they belong to
- [x] No `/StructParents` key is shared between pages; every `/ParentTree` entry points at an element whose `/Pg` is that page
- [x] Apply then undo leaves the overlay empty

## As built

- **Model:** `StructTree` reads `/ClassMap` (`class_map`, values as stored), `/IDTree` (`id_tree`) and `/Namespaces` (`namespaces`); `StructElem` gains `id` and `classes`.
- **The step:** `crates/laminapdf-ops/src/pages/import_tags.rs`, the import's last step before the sweep (the import no longer strips `/StructParents` and `/StructParent`). It copies the source elements that own content on the copies (from the source's `/ParentTree` entries for the copied pages, listed annotations and Form XObjects), with their ancestors, below the target's top `Document` element, or below a new `Part` per source on a merge, titled like its bookmark. A source's own top `Document` element isn't copied: its children take its place, in the source's order. Elements are appended after the target's existing ones.
- **Partial elements:** an element keeps only the kids that came along: MCIDs and `MCR`s on copied pages (and copied streams), `OBJR`s to annotations the copies list, and child elements that were copied. A `/ParentTree` entry whose elements weren't copied loses its key. No element is dropped as invalid: one that keeps no kids is an ancestor of a copied element, so it has at least one.
- **Keys and maps:** fresh `/StructParents` and `/StructParent` from `/ParentTreeNextKey`, the tree written again through `TreeWriter`. `/RoleMap` and `/ClassMap` entries join the target's; a name mapped another way becomes `<name>#<n>`, and so does an element `/ID` the target uses, all in `Applied::structure_renamed` (`StructureRename { kind: StructureName::{Role, Class, Id}, from, to }`, also on the façade's `Edited` and WASM's `structureRenamed`). Namespaces are matched by their URI, else copied and appended.
- **When tags are kept:** `Tags::kept`: the target is tagged, or it has no pages and every source is tagged (a new document from `Document::merge`, `extract_pages` and `split` of a tagged document). Otherwise the copies lose their keys and `tags_dropped` lists them, as before. A PDF/UA-1 target taking tagged pages from a source without the claim records `PendingChanges::pdfua_unchecked`, warned as the new `imported-pdfua-unchecked`.
- **Checks:** `crates/laminapdf/tests/merge_structure.rs` (both trees kept, distinct keys, each page's entry naming elements on that page, role and ID renames, undo); `crates/laminapdf-ops/tests/import.rs` and `merge.rs` (kept in a tagged file, dropped in an untagged one, a `Part` per file, an untagged input leaving a new document untagged). veraPDF 1.30.3 passes `tagged-across-pages.pdf` with its own pages inserted against PDF/UA-1 (106 rules, none failed). A new document merged from two copies of it fails PDF/UA-1 only on what a new document doesn't carry (the XMP metadata stream, `/Lang`, `/DisplayDocTitle`), no structure rule. pdf.js 6.4.299 (`getStructTree`) and PDFium (`FPDF_StructTree_GetForPage`) read each copied page with the same per-page structure as its original; on the merged file, with one more level, the `Part`. qpdf `--check` is clean on all three. These tool runs used `tools/fixtures/structure_facts.py`'s functions by hand; no committed facts file changed.

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

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
