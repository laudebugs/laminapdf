# E7i — Struct-tree and MCID maintenance; PDF/A and PDF/UA re-checks on save

| Field | Value |
| --- | --- |
| ID | `E7i` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B8b](../done/B8b-conformance-profiles-and-verapdf.md), [E7d](./E7d-select-transform-existing.md), [E7e](./E7e-text-runs-and-inline-edit.md) |
| Dependency notes | [E7d](E7d-select-transform-existing.md) and [E7e](E7e-text-runs-and-inline-edit.md) (the hooks sit in `EditContent` and `EditText`; `SetTextStyle` and `ReplaceText` reach them through `EditText`), `B8` ([pdf-library §B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa): tagged PDF, PDF/UA, PDF/A) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7i-structure-and-conformance-maintenance` |
| PR title | `feat(content): keep tagging valid across content edits` |
| Size | M (200–600) |
| PR | — |

## Goal

Keep tagged documents valid across E6/E7 edits: update MCIDs, ParentTree, `/ActualText` and `/Alt` when runs change or are deleted; re-check PDF/A and PDF/UA claims on save and warn.

## Context

- **Design (Paper):** [A07 Structure and conformance warning](../../design/paper/add-edit-content.md#a07-structure-and-conformance-warning). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [03 §9 Tagged PDF](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149)
- [library B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa) profiles

## Scope

**In scope**

- MCID/ParentTree maintenance hooks in `EditContent`/`EditText`
- Conformance checks reuse B8 profiles
- Warnings in the save banner
- `RemoveConformanceClaims { pdfa, pdfua }` (class `Metadata`), the only op that may remove `pdfaid:` and `pdfuaid:`; A07's Save without the claim applies it ([E7 plan](../../plans/editor/e7-editing-existing-content.md) §3, §10)
- A07's strip, claim badges and Check before saving dialog

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `laminapdf-model` (struct tree), `laminapdf-content`, `laminapdf-ops`, `laminapdf`, `@laminapdf/sdk`, `@laminapdf/ui`
- **`Op` variants:** `RemoveConformanceClaims` (class `Metadata`)
- **`SaveConstraint`:** as E7
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none new; protects E7 rows' `round-trip` on tagged fixtures

## Tasks

- [ ] Hooks
- [ ] Checks
- [ ] Tests

## Acceptance criteria

- [ ] veraPDF structure checks pass after edits on PDF/UA fixtures
- [ ] Deleting a tagged run removes its struct element or MCR
- [ ] `pending_conformance_warnings` covers content edits (an added font not embedded, a figure without alternative text, broken tags)
- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): `RemoveConformanceClaims` is exposed in `laminapdf` and `@laminapdf/sdk` with docs, an example and a serde sample frozen in `fixtures/ops/v1/`
- [ ] `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) asserts `RemoveConformanceClaims`'s verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2)

## Post-merge checks

None.

## Tests

- veraPDF nightly
- Unit

## Docs to update

- Guide: editing tagged PDFs
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
