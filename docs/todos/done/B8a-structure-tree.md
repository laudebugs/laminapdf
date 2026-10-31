# B8a — Tagged PDF: structure tree builder, artifacts, alt text, MCIDs and ParentTree

| Field | Value |
| --- | --- |
| ID | `B8a` |
| Status | `done` |
| Plan | [pdf-library §7 B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa), requirement [§4 F6](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M4 Library 0.2 |
| Depends on | [B5c](./B5c-lamina-facade-and-examples.md), [B6b](./B6b-lamina-pdf-package.md) |
| Blocks | [B8b](../ready/B8b-conformance-profiles-and-verapdf.md), [L3a](../ready/L3a-lamina-extract-reading-order.md), [U4b](../ready/U4b-reader-mode-from-structure-tree.md), [E2b](../ready/E2b-page-structure-ops.md), [E2d](../ready/E2d-merge-and-split.md), [E6c](../ready/E6c-add-text.md), [E6d](../ready/E6d-watermarks-headers-footers.md) |
| ADRs | Follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (lamina owns tagging; krilla's rule lists as reference), [0006](../../decisions/0006-op-log-and-undo.md) (no ops here: build-time only) |
| Branch | `feat/b8a-structure-tree` |
| PR title | `feat(build): tagged PDF with structure tree, artifacts, alt text, MCIDs and ParentTree` |
| Size | L (≈ 900–1,300 lines Rust + TS bindings) |
| PR | — |

## Goal

Make generated documents tagged: every piece of content is either inside a structure element (with marked-content IDs linked through the ParentTree) or marked as an artifact; figures carry alt text; links are `Link` structure elements containing their annotation via OBJR; headings, paragraphs, lists, tables and figures map to standard structure types. This is the structural half of B8; conformance profiles and validation are [B8b](../ready/B8b-conformance-profiles-and-verapdf.md).

## Context

- [Research 03 §9](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149): [§9.1 model](../../research/pdf/03-text-and-fonts.md#91-the-model), [§9.2 standard types](../../research/pdf/03-text-and-fonts.md#92-standard-structure-types), [§9.3 language, alt text, reading order](../../research/pdf/03-text-and-fonts.md#93-language-alt-text-reading-order), [§9.5 generating tagged PDF](../../research/pdf/03-text-and-fonts.md#95-generating-tagged-pdf-from-code).
- ISO 32000-2 §14.7–14.8 (structure, marked content, `StructParents`, `ParentTree`, `RoleMap`), §14.9.4 (alt text), §14.8.2.2 (artifacts).
- krilla's tagging module (reference only, MIT/Apache) and the [krilla-fit B1 finding](../../../spikes/krilla-fit/README.md) about `ParentTree` renumbering.
- [pdf-library §5 TS sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final): `tag: "H1"`, `alt`, `.artifact("Layout")`, `link: { uri, contents }`.

## Scope

**In scope**

- `StructTree` builder in `lamina-build`: `StructElem` kinds (`Document`, `Part`, `Sect`, `H1`–`H6`, `P`, `L`/`LI`/`Lbl`/`LBody`, `Table`/`TR`/`TH`/`TD`, `Figure`, `Caption`, `Link`, `Span`, `Quote`, `Note`, `Code`), nesting via `surface.tag(kind, |s| …)` or `.tag(kind)` on a run; attributes (`/Lang` per element, `/Alt`, `/ActualText`, `/E`, table `/Scope`).
- Marked content emission (`/<Tag> <</MCID n>> BDC … EMC`), per-page MCID counters, `/StructParents` on pages, `/StructParent` on annotations, `ParentTree` number tree (deterministic), `OBJR` for link annotations, `RoleMap` for custom tag names, `/MarkInfo << /Marked true >>`, `/StructTreeRoot`, `/K` ordering in reading order.
- Artifacts: `surface.artifact(ArtifactKind::{Layout, Page, Pagination, Background}, |s| …)` emitting `/Artifact BMC/BDC`.
- Images: `image(..., alt)` creates a `Figure` with `/Alt`; missing alt recorded for B8b's profile check.
- Builder bookkeeping exposing "untagged content" diagnostics (content outside any element or artifact) for B8b.
- TS bindings (`tag`, `artifact`, `alt`) in `@lamina/pdf` and `lamina-wasm`.
- Reading back: `lamina-model` gets a minimal read-only `StructTree` view (root, kids, kinds, MCIDs) used by tests.

**Out of scope** (and which todo covers it)

- Profiles (`PdfUa1`, `PdfA*`), errors at `finish()`, XMP identification, output intents, veraPDF: [B8b](../ready/B8b-conformance-profiles-and-verapdf.md).
- Editing structure of existing files, auto-tagging: full-editor E15.
- Merging structure trees when importing pages: full-editor E2.

## Tasks

- [x] Load `/rust-best-practices`; read 03 §9 and ISO 32000-2 §14.7–14.8
- [x] `lamina-build/src/tag/{mod,elem,mcid,parent_tree,artifact,rolemap}.rs`
- [x] Integrate with text runs, images, paths and links from B5a/B5b
- [x] `lamina-model::structure` read view
- [x] TS API and tests; update examples (`invoice` tagged)
- [x] Goldens: `tagged-report.pdf`; full-lane checks with pdf.js (structure tree via `getStructTree`) and PDFium

## Acceptance criteria

- [x] A tagged example's structure tree read by pdf.js `getStructTree()` and by `lamina-model` matches the builder input (kinds, order, alt text)
- [x] Every MCID on every page is referenced exactly once from the ParentTree; every `/StructParent(s)` resolves
- [x] Link annotations are inside `Link` elements via OBJR and have `/Contents`
- [x] Untagged-content diagnostics are empty for all tagged examples and non-empty for a deliberately untagged path
- [x] Byte-identical goldens; `qpdf --check` clean

## Outcome

- **API.** Rust: closures, as `path(|p| …)` and ADR 0016's `transact(tx => …)`: `surface.tag(tag, |s| …)`, `surface.artifact(kind, |s| …)` (the closure returns `lamina_build::Result`; the element ends even when it fails), `TextRun::tag`, `surface.figure(&image, rect, alt)`; `begin_tag`/`begin_artifact`/`end_tag` are the begin/end form the bindings use. TS: the same closures (`s.tag("H1", s => …)`, `s.artifact("layout", s => …)`, ended in a `finally`), a `tag` option on `text`/`textBox`, `image(image, rect, alt)`; WASM: `beginTag`/`beginArtifact`/`endTag` with flat arguments (no new serde records, for the size budget). `BuildOptions::with_tagged` (TS `tagged: true`) tags a document before any element; a page using one tags it either way.
- **Elements are per page** (one doesn't continue on the next page); appended pages (`Builder::on`) are never tagged (merging structure trees is E2). Tables: `TH` `/Scope` only; `/Headers`, `RowSpan`/`ColSpan`, `THead`/`TBody`/`TFoot` are not offered.
- **Diagnostics** are `Builder::tagging_diagnostics()` → `TagDiagnostic::{Untagged { page, content, count }, MissingAlt { page }}` (Rust and the façade only; no new error or warning codes: B8b decides which become errors). A clip path alone isn't counted as content.
- **Full-lane checks** follow B5b's precomputed-facts pattern: `fixtures/build/structure.json` (pdf.js `getStructTree`, PDFium `FPDF_StructTree_GetForPage`, `qpdf --check`) from `tools/fixtures/structure_facts.py`, checked by `tests/tagging.rs` in every lane.

## Post-merge checks

None.

## Tests

- Unit tests (MCID allocation, ParentTree writer); `crates/lamina-build/tests/tagging.rs`; TS tests in `packages/pdf/test/tagging.test.ts`.

## Docs to update

- [x] [pdf-library §10 B8](../../plans/pdf-library.md#10-to-dos): tick "Add structure-tree building"
- [ ] Docs site guide stub "Accessible PDFs" (completed in [B8b](../ready/B8b-conformance-profiles-and-verapdf.md)): not written, there is no docs site yet ([B7a](../ready/B7a-docs-site.md) is still `ready`); B8b writes the page

## Risks and open questions

- API ergonomics for nesting in TS (closures) vs explicit begin/end handles; pick one consistent with ADR 0016.
- Tables: header association (`/Headers`, `/Scope`) beyond simple cases is deferred; document limits.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
