# E2b — Page-structure ops: insert blank, delete, duplicate, reorder, rotate, reverse

| Field | Value |
| --- | --- |
| ID | `E2b` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B8a](./B8a-structure-tree.md), [B8b](./B8b-conformance-profiles-and-verapdf.md), [E1c](./E1c-metadata-ops.md), [E2a](./E2a-page-organisation-feature-plan.md) |
| Blocks | [E2c](./E2c-cross-document-pages.md), [E2f](./E2f-page-geometry.md), [E2g](./E2g-page-labels.md), [E3b](./E3b-bookmarks-and-destinations.md), [E18b](./E18b-collaboration-feature-plan.md), [E18c](./E18c-layer-document-schema.md), [E18e](./E18e-client-sync-and-presence.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2b-page-structure-ops` |
| PR title | `feat(ops): page structure ops with reference cleanup` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Implement the single-document page ops as view-level mapping of `PageId`s (no re-parse of content): insert blank pages, delete, duplicate, move (reorder), rotate (batch, generalising B3's `RotatePage`) and reverse, with cleanup of references to deleted pages (outline items, link destinations, named destinations, struct-tree `/Pg`, `/OpenAction`).

## Context

- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation)
- [ADR 0005](../../decisions/0005-document-model.md) §5: `MovePages` keeps every `PageId`; new pages get new IDs
- [06 §3 group C](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms): mechanisms (`/Kids`, `/Count`, inherited attributes)
- [01 §10 Document logical structure](../../research/pdf/01-file-structure.md#10-document-logical-structure-77): page tree and inheritance
- ISO 32000-2 §7.7.3 (page tree, inheritable attributes), §12.3.2 (destinations)

## Scope

**In scope**

- Ops: `InsertBlankPages { at, count, size: PageSize, rotate }`, `DeletePages { pages }`, `DuplicatePages { pages, at }`, `MovePages { pages, to }`, `RotatePages { pages, rotate }`, `ReversePages { range }`
- Page-tree editing that materialises inherited attributes (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`) before moving a page out of its parent
- Rebalancing: keep the tree shape for small edits; flatten-and-rebuild only when depth or fan-out passes a threshold
- **Tree writers** in `lamina-model`, shared by every later op that edits a number or name tree: a number-tree writer (`/PageLabels`, `/ParentTree`; balanced, sorted keys, `/Nums` leaf limit, `/Limits` maintained) and a name-tree writer (`/Dests`, `/EmbeddedFiles`, …; balanced, keys sorted by byte order, `/Names` leaf limit, `/Limits` maintained). [E2c](E2c-cross-document-pages.md), [E2d](E2d-merge-and-split.md), [E2g](E2g-page-labels.md), [E3b](E3b-bookmarks-and-destinations.md) and [E3f](E3f-attachments-and-associated-files.md) build on them rather than writing their own
- **Page-label maintenance** for every op here: `PageLabels::remap(old_index → new_index)` rebuilds `/PageLabels` ranges from `PageId`s so each surviving page keeps its label after insert/delete/duplicate/move/reverse (inserted blank pages continue the range they land in); E2c and E2d call the same function for `InsertPagesFrom`, `ReplacePages`, extract and merge
- Reference cleanup on delete according to the E2a decision (named destinations removed or retargeted through the name-tree writer), with an `OpReport` listing affected outline items/links/destinations
- Duplicate copies the page dict and annotations (new `/NM`, new IDs), sharing content streams and resources, and never leaves shared structure or field links behind:
  - **Tagged files:** the copy gets a fresh `/StructParents` key (from `/ParentTreeNextKey`, which is advanced), and every copied annotation with `/StructParent` gets a fresh key too. Struct elements whose marked content lies only on the source page are cloned as siblings after the originals with `/Pg` set to the duplicate, and the new ParentTree entries point at the clones (the shared content stream keeps the same MCIDs, so the mapping is one-to-one). MCIDs owned by elements that span pages are mapped to null, and the `OpReport` carries `DuplicateTagsIncomplete`; if the file claims PDF/UA, the planner adds a `ConformanceWarning`. Never reuse the source page's key
  - **Widgets:** a duplicated widget becomes a new field, never a second widget silently attached to the original. A widget merged with its field (no `/Parent`) is copied with a unique `/T` (`<name>#<n>`, pattern fixed in E2a); a kid widget is copied with a new terminal field dictionary (field-level keys `/FT`, `/V`, `/DV`, `/Ff`, `/DA`, `/Opt` copied) named `<partial>#<n>` and added to the original parent's `/Kids` (or to `/AcroForm /Fields` when the field is top-level). Signature-field widgets (`/FT /Sig`) are not duplicated; the report lists them

**Out of scope** (and which todo covers it)

- Pages from other documents, replace, extract → [E2c](E2c-cross-document-pages.md)
- Merge and split → [E2d](E2d-merge-and-split.md)
- Thumbnail UI → [E2h](E2h-page-organiser-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.01` insert blank pages; `C.03` delete and duplicate pages; `C.04` reorder pages; `C.08` rotate pages 90/180/270; `C.13` reverse page order
- **Crates and packages:** `lamina-model` (page tree mutation helpers), `lamina-ops`, bindings
- **`Op` variants:** `InsertBlankPages`, `DeletePages`, `DuplicatePages`, `MovePages`, `RotatePages`, `ReversePages` (class `PageStructure`)
- **`SaveConstraint`:** `Free` → incremental. Signed: page insertion/deletion/reordering is class `PageStructure`. Under a certification signature it is a DocMDP violation at every level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked by default and allowed only by the explicit invalidation override (the warning names the signatures, ADR 0007 §3). With approval signatures only (no certification) it is `AppendOnly`: saved as an incremental update that keeps their `/ByteRange` intact (the signature panel, E10h, reports the later page change); it is a violation when it changes a field locked by FieldMDP/`/Lock` (e.g. deleting or duplicating a page that carries a locked field's widget), and **always a violation when it touches a signed signature field's widget**: deleting the page that holds the widget of a signed `/FT /Sig` field removes the signature's visible appearance and leaves the widget's `/P` (and the field's `/Kids`) pointing at a page that is gone, so `DeletePages` (and `ReplacePages` in E2c, which reuses this check) reports `Violation::SignedWidgetPage { field, signature }` and the save is `Conflict`, never plain `AppendOnly`. `MovePages`, `RotatePages` and `ReversePages` on such a page keep `/P` valid but change how the signed appearance is presented (`/Rotate` turns it), so `RotatePages` on a page holding a signed widget is also a violation; moving/reversing is allowed (`AppendOnly`) and reported. Pages holding only unsigned signature fields follow the normal rule
- **Web vs desktop:** identical (core); UI in E2h
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.01` → `round-trip` (core, pdf); `C.03` → `round-trip` (core, pdf); `C.04` → `round-trip` (core, pdf); `C.08` → `round-trip` (core, pdf); `C.13` → `round-trip` (core, pdf)

## Tasks

- [ ] Implement page-tree mutation in `lamina-model` with inherited-attribute materialisation and `/Count` maintenance
- [ ] Implement each op with undo records; `MovePages` must keep `PageId`s and object numbers
- [ ] Reference index: find outline items, link annotations, named destinations, struct elements and open action pointing at a page
- [ ] Number-tree and name-tree writers (balanced, sorted keys, leaf limits, `/Limits`) with property tests (random insert/delete → valid tree that qpdf and pdf.js read back identically)
- [ ] `PageLabels::remap` used by every op here, public for E2c/E2d
- [ ] Cleanup policy per E2a, reported in `Applied`
- [ ] Signed-widget check: `would_violate` finds pages holding widgets of signed `/FT /Sig` fields and reports `SignedWidgetPage` for delete and rotate
- [ ] Bindings (`doc.pages.insertBlank()`, `.delete()`, `.duplicate()`, `.move()`, `.rotate()`, `.reverse()`)
- [ ] Examples in Rust and TS

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Page count, order and rotation after each op match pypdf and pdf.js on 20 corpus files (incl. deep/unbalanced page trees and inherited attributes)
- [ ] On an approval-signed fixture, `DeletePages` and `RotatePages` on the page holding the signed widget give `Violation::SignedWidgetPage` and `constraint.kind() == Conflict` (save refused without the override); the same ops on another page are `AppendOnly` and the signature still validates after the incremental save
- [ ] Page labels survive every op: on a fixture with roman + arabic + prefixed ranges, each surviving page's label is unchanged after delete/duplicate/move/reverse (read back in pdf.js and PDFium)
- [ ] Deleting a page that an outline item targets follows the E2a policy and the report lists it
- [ ] Duplicating a page of a tagged fixture gives the copy its own `/StructParents` key; the ParentTree maps the copy's MCIDs to cloned struct elements whose `/Pg` is the copy (or to null with `DuplicateTagsIncomplete` for cross-page elements); no key is shared between the two pages; veraPDF's PDF/UA result on the fixture is no worse than before
- [ ] Duplicating a page of a form fixture leaves every widget in its parent's `/Kids` (or in `/Fields`), gives each duplicated field a distinct fully qualified name and its own value (filling one in pdf.js and PDFium leaves the other unchanged), and skips signature widgets with a report entry
- [ ] Determinism: saving the same state twice gives identical bytes on native and wasm32 (ADR 0007 §6)

## Post-merge checks

None.

## Tests

- Unit/property tests (random sequences of page ops then undo all → empty overlay)
- Corpus differential vs pypdf (page order facts) and pdf.js (render hashes per page)
- Fixtures: inherited `/Resources` and `/MediaBox`, 3-level page tree, page referenced by outline + named dest + link, a tagged page (struct elements on one page and one spanning two pages, a tagged link annotation), and an AcroForm page with a merged field/widget, a kid widget and a signature field

## Docs to update

- API docs: pages
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Shared page dicts (same object referenced twice in `/Kids`) are invalid but occur; duplicate them on first mutation and warn

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
