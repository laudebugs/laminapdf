# E2d — Merge documents and split by count, ranges and bookmarks

| Field | Value |
| --- | --- |
| ID | `E2d` |
| Status | `ready` |
| Plan | [E2 page organisation §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B8b](../done/B8b-conformance-profiles-and-verapdf.md), [E2c](./E2c-cross-document-pages.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2d-merge-and-split` |
| PR title | `feat(ops): merge documents with outline, destination and field remapping; split` |
| Size | L (400–800): merge of the outline, the name trees, the AcroForm and the layers over [E2p](../done/E2p-source-documents-and-copier.md)'s copier, and split; the structure-tree merge is [E2o](E2o-structure-tree-merge.md)'s |
| PR | — |

## Goal

Merge whole documents with the hard parts done: outlines appended as subtrees, named destinations renamed on clash, AcroForm fields merged with the [E2 plan's clash rule](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions) (rename `<partial>#<n>` by default, or link same-named fields of the same type and value; [E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions)), OCGs merged; and split a document by page count, explicit ranges or top-level bookmarks. Merging structure trees is [E2o](E2o-structure-tree-merge.md); until it lands, merged pages lose their tags with `TagsDropped`.

## Context

- **Design (Paper):** [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents), [O08 Split document](../../design/paper/edit-session-organise.md#o08-split-document). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [05 §3.1 Merge and split](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)
- [03 §9 Tagged PDF](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149): ParentTree and MCIDs
- [04 §3 Forms](../../research/pdf/04-interactive-security.md#3-forms): field tree and fully qualified names
- [04 §1 Navigation](../../research/pdf/04-interactive-security.md#1-navigation): outlines and destinations

## Scope

**In scope**

- `merge(target, sources, options)` built on `InsertPagesFrom` plus document-level merges: outline, `/Names /Dests`, AcroForm (`/Fields`, `/DR`, `/DA`, `/NeedAppearances`), `/OCProperties`, page labels (concatenated with E2j's `PageLabels::remap` and number-tree writer; `/Names /Dests` through E2j's name-tree writer), and the catalog's `/Version` raised to the newest source's
- `split(doc, SplitBy::Count(n) | Ranges(..) | Bookmarks { level })` returning documents with their outline subsets, each written with only the objects it reaches; `merge` and `split` are jobs with run-to-completion forms (ADR 0028 F10)
- Merge report: renamed fields/destinations, dropped items, conformance claims lost (PDF/A, PDF/UA)

**Out of scope** (and which todo covers it)

- UI (multi-file merge, split dialog) → [E2m](E2m-document-dialogs.md); desktop folder output and Finder/Explorer selections → [E2n](E2n-desktop-organise.md)
- The CLI's page commands (`laminapdf pages delete`, `extract`, `reorder`, `insert`, `merge`, `split`) → [T6a](./T6a-cli-page-commands.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.06` merge multiple documents; `C.07` split by count, ranges, bookmarks
- **Crates and packages:** `laminapdf-model` (outline, names, AcroForm and layer merge helpers), `laminapdf-ops`
- **`Op` variants:** `MergeDocuments` op (composite; exactly one class per ADR 0006 §1: `PageStructure`, the most restrictive of the changes it makes, since it inserts pages and is forbidden at every DocMDP level; its document-level merges (outline, names, AcroForm, layers) are covered by that verdict); split is a pure function producing new documents
- **`SaveConstraint`:** class `PageStructure`; the verdicts per constraint are the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical (core); several files opened together from the OS offer a merge where `hasFileHandlers` ([E2m](E2m-document-dialogs.md), [E2n](E2n-desktop-organise.md))
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.06` → `round-trip` (core, pdf); `C.07` → `write` (core, pdf)

## Tasks

- [ ] Outline merge: append each source's outline as a child of a new item titled with the source's title/filename (option)
- [ ] Named destinations: rename on clash with a suffix and rewrite references in the imported pages
- [ ] AcroForm merge per the [E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions) rule; `/DR` font resources merged by hash
- [ ] OCG merge (`/OCGs`, `/D` arrays)
- [ ] Split implementations and tests incl. bookmarks pointing at named destinations

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Merging a tagged fixture reports `TagsDropped` and a PDF/UA `ConformanceWarning` (E2o keeps the tags)
- [ ] Merged forms: fields from both sources fillable in pdf.js and PDFium; clashing names handled per rule
- [ ] Split by bookmarks on a 3-chapter fixture yields 3 files whose outlines are the chapter subtrees
- [ ] Merge of 100 files of 10 pages each completes within the [E2 plan §10](../../plans/editor/e2-page-organisation.md#10-tests) budget, or the PR records what it measured and adjusts the budget in the plan

## Post-merge checks

None.

## Tests

- Integration tests with form, layer and chapter fixtures (`fixtures/pages/forms.pdf`, `layers.pdf`, `chapters.pdf`)
- qpdf `--check` and pdf.js open of every output
- Benchmark in `crates/laminapdf/benches/merge.rs`

## Docs to update

- API docs: merge and split
- Guide: merging tagged PDFs
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Struct-tree merge edge cases belong to E2o

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
