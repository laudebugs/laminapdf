# E2d — Merge documents and split by count, ranges and bookmarks

| Field | Value |
| --- | --- |
| ID | `E2d` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2c](./E2c-cross-document-pages.md) |
| Blocks | [E2h](./E2h-page-organiser-ui.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2d-merge-and-split` |
| PR title | `feat(ops): merge documents with outline, field and structure remapping; split` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Merge whole documents with the hard parts done: outlines appended as subtrees, named destinations renamed on clash, AcroForm fields merged with the E2a clash rule, struct trees merged under one `/StructTreeRoot` with ParentTree renumbering, OCGs merged; and split a document by page count, explicit ranges or top-level bookmarks.

## Context

- [05 §3.1 Merge and split](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)
- [03 §9 Tagged PDF](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149): ParentTree and MCIDs
- [04 §3 Forms](../../research/pdf/04-interactive-security.md#3-forms): field tree and fully qualified names
- [04 §1 Navigation](../../research/pdf/04-interactive-security.md#1-navigation): outlines and destinations

## Scope

**In scope**

- `merge(target, sources, options)` built on `InsertPagesFrom` plus document-level merges: outline, `/Names /Dests`, AcroForm (`/Fields`, `/DR`, `/DA`, `/NeedAppearances`), StructTreeRoot (RoleMap, ClassMap, ParentTree, `/IDTree`), `/OCProperties`, page labels
- `split(doc, SplitBy::Count(n) | Ranges(..) | Bookmarks { level })` returning documents with their outline subsets
- Merge report: renamed fields/destinations, dropped items, conformance claims lost (PDF/A, PDF/UA)

**Out of scope** (and which todo covers it)

- UI (multi-file merge, split dialog) → [E2h](E2h-page-organiser-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.06` merge multiple documents; `C.07` split by count, ranges, bookmarks
- **Crates and packages:** `lamina-model` (outline, names, AcroForm, struct-tree merge helpers), `lamina-ops`
- **`Op` variants:** `MergeDocuments` op (composite, class `PageStructure` + `Document`); split is a pure function producing new documents
- **`SaveConstraint`:** `Free` → incremental. Signed: page insertion/deletion/reordering is class `PageStructure`. Under a certification signature it is a DocMDP violation at every level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked by default and allowed only by the explicit invalidation override (the warning names the signatures, ADR 0007 §3). With approval signatures only (no certification) it is `AppendOnly`: saved as an incremental update that keeps their `/ByteRange` intact (the signature panel, E10h, reports the later page change); it is a violation only when it changes a field locked by FieldMDP/`/Lock` (e.g. deleting or duplicating a page that carries a locked field's widget). Merging into a certified document is a violation at every DocMDP level; into an approval-only-signed document it is an incremental `AppendOnly` save. Split writes new documents (full write, `Free`)
- **Web vs desktop:** identical; desktop multi-select in Finder/Explorer merges directly (E2h)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.06` → `round-trip` (core, pdf); `C.07` → `write` (core, pdf)

## Tasks

- [ ] Outline merge: append each source's outline as a child of a new item titled with the source's title/filename (option)
- [ ] Named destinations: rename on clash with a suffix and rewrite references in the imported pages
- [ ] AcroForm merge per E2a rule; `/DR` font resources merged by hash
- [ ] Struct-tree merge: new parent tree keys, MCIDs untouched (per page), RoleMap conflicts reported
- [ ] OCG merge (`/OCGs`, `/D` arrays)
- [ ] Split implementations and tests incl. bookmarks pointing at named destinations

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Merging two tagged PDF/UA fixtures gives a tagged result whose struct tree validates (veraPDF structure checks, ignoring the PDF/UA claim which is reported as lost unless both conform)
- [ ] Merged forms: fields from both sources fillable in pdf.js and PDFium; clashing names handled per rule
- [ ] Split by bookmarks on a 3-chapter fixture yields 3 files whose outlines are the chapter subtrees
- [ ] Merge of 100 files of 10 pages each completes within a budget set in E2a (native and wasm)

## Post-merge checks

None.

## Tests

- Integration tests with tagged, form and OCG fixtures
- qpdf `--check` and pdf.js open of every output
- Benchmark in `crates/lamina/benches/merge.rs`

## Docs to update

- API docs: merge and split
- Guide: merging tagged PDFs
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Struct-tree merge edge cases (`/K` arrays with MCRs to other pages); fall back to dropping tags with a warning rather than producing an invalid tree

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
