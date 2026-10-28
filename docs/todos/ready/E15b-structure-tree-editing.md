# E15b — Structure-tree editing model, tag tree panel and reading-order overlay

| Field | Value |
| --- | --- |
| ID | `E15b` |
| Status | `ready` |
| Plan | [full-editor §E15](../../plans/full-editor.md#e15-accessibility) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E7i](./E7i-structure-and-conformance-maintenance.md), [E15a](./E15a-accessibility-feature-plan.md) |
| Blocks | [E15c](./E15c-tagging-tools.md), [E15d](./E15d-accessibility-checks.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e15b-structure-tree-editing` |
| PR title | `feat(model): editable structure tree with tag panel` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Edit the structure tree: create/retag/move/delete struct elements, attach content (MCIDs, OBJRs) to elements, reorder for logical reading order, with a tag-tree panel and an overlay that numbers content in reading order on the page.

## Context

- [03 §9.1 The model](../../research/pdf/03-text-and-fonts.md#91-the-model)
- [03 §9.2 Standard structure types](../../research/pdf/03-text-and-fonts.md#92-standard-structure-types)
- ISO 32000-2 §14.7–14.8

## Scope

**In scope**

- Ops: `CreateStructElem`, `UpdateStructElem` (type, attributes), `MoveStructElem`, `DeleteStructElem`, `TagContent { objs, elem }` (adds BDC/EMC via E7c IR)
- ParentTree maintenance
- Tag panel and reading-order overlay

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `L.01` add structure tags; `L.02` logical reading order; `I.07` define reading order; logical structure tags
- **Crates and packages:** `lamina-model` (struct tree write), `lamina-content`, `@lamina/ui`
- **`Op` variants:** the ops above (class `Document`, `TagContent` also `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Document` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations); ops noted as `Annotation` are allowed under P3 or when there is no certification. Violations are gated (override names the signatures). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock`
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `L.01` → `round-trip` (core, pdf, web, desktop); `L.02` → `round-trip` (core, pdf, web, desktop); `I.07` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Ops
- [ ] ParentTree
- [ ] Panel
- [ ] Overlay

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] veraPDF structure checks pass after edits on tagged fixtures
- [ ] Screen-reader order in NVDA follows the edited order (manual, recorded)

## Post-merge checks

None.

## Tests

- veraPDF nightly
- e2e

## Docs to update

- User help: tags
- [full-editor §E15](../../plans/full-editor.md#e15-accessibility) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
