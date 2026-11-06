# E4b — Annotation model with `/NM` IDs, annotation ops and overlay rendering

| Field | Value |
| --- | --- |
| ID | `E4b` |
| Status | `ready` |
| Plan | [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E1d](./E1d-edit-session-undo-ui.md), [E3c](./E3c-links.md), [E4a](./E4a-markup-and-drawing-annotations-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4b-annotation-model-and-overlay` |
| PR title | `feat(model): typed annotations, annotation ops and overlay layer` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Typed annotation views in `laminapdf-model` (common dictionary, markup dictionary, Popup), `AnnotId` with `/NM` hint index, the generic ops `AddAnnot`/`UpdateAnnot`/`DeleteAnnot`, and the viewer's editable annotation layer that renders lamina-created or edited annotations from the model until save, hiding PDFium's stale rendering of those annotations.

## Context

- [04 §2.1 The common dictionary](../../research/pdf/04-interactive-security.md#21-the-common-dictionary)
- [04 §2.3 Markup annotations, replies, review state](../../research/pdf/04-interactive-security.md#23-markup-annotations-replies-review-state-12562)
- [ADR 0005](../../decisions/0005-document-model.md) §5: `/NM` is a hint, never the ID
- [architecture §6](../../plans/architecture.md#6-risks): two-parsers risk and overlay rendering
- [web C3](../../plans/web-app.md#c3-viewer-core): viewer layers

## Scope

**In scope**

- `Annotation` view (subtype enum incl. unknown passthrough), `MarkupAnnotation` (`/T`, `/Contents`, `/CreationDate`, `/Subj`, `/IRT`, `/RT`, `/CA`, `/RC`), `Popup` pairing
- Ops: `AddAnnot { page, spec: AnnotSpec }`, `UpdateAnnot { id, patch }`, `DeleteAnnot { id }` (deletes Popup and, per E4a, replies), each with `/NM` derived from the ID
- Relation to [E3c](E3c-links.md)'s `AddLink`/`UpdateLink`/`DeleteLink`: they stay as ops (v1 wire names, ADR 0006 §6) and are re-implemented here as thin wrappers over the same annotation core. `AnnotSpec` has no `Link` variant (links are created only by `AddLink`); `UpdateAnnot` and `DeleteAnnot` accept a link's ID for the common properties (`/F`, `/C`, border, delete) and produce the same COS changes as `UpdateLink`/`DeleteLink`; a test asserts the two paths are byte-identical
- `/P` back-reference, `/F` flags (Print default for markup), `/M` modification date from the payload
- Viewer: annotation editing layer (SVG/canvas) drawing from a JSON projection of the model; per-annotation hide mask for the PDFium raster; selection handles; hit-testing
- Appearance generator trait (`AppearanceBuilder`) skeleton in `laminapdf-build`, implemented per subtype in E4c–E4e

**Out of scope** (and which todo covers it)

- Appearances for each subtype → [E4c](E4c-appearances-markup-notes-caret.md)
- Tools UI → [E4f](E4f-annotation-tools-ui.md)

**Editor facts**

- **Crates and packages:** `laminapdf-model` (annotations), `laminapdf-ops`, `laminapdf-build` (appearance trait), `@laminapdf/viewer` (edit layer)
- **`Op` variants:** `AddAnnot`, `UpdateAnnot`, `DeleteAnnot` (class `Annotation`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Annotation` is allowed only under DocMDP P3 or when there is no certification signature (approval signatures alone allow annotations); otherwise a violation → warn and gate. Note the Evil Annotation Attack: annotations added after certification are shown as such in the signature panel (E10)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none flipped (subtypes flip in E4c–E4e)

## Tasks

- [ ] Views and unknown-key passthrough (never drop keys)
- [ ] Ops with undo and coalescing for drags
- [ ] `/NM` index with duplicate warnings
- [ ] Viewer edit layer and PDFium hide mask (render with `FPDF_ANNOT` flag per annotation or re-render page without the annotation)
- [ ] Bindings

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Unknown annotation keys survive an update (round-trip test)
- [ ] Editing an existing annotation shows the change immediately without a page re-parse; after save the PDFium render matches the overlay (diff below threshold)

## Post-merge checks

None.

## Tests

- Unit tests for views over corpus annotations
- Viewer tests (Vitest + Playwright screenshot)
- Fuzz target for annotation view parsing

## Docs to update

- API docs: annotations (generic)
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- PDFium's per-annotation hiding may need re-rendering the page without the annot; measure cost (feeds E6f budget)

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
