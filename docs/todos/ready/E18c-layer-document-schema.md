# E18c — Layer-document schema and record ↔ op mapping

| Field | Value |
| --- | --- |
| ID | `E18c` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E2b](../ongoing/E2b-page-structure-ops.md), [E4b](./E4b-annotation-model-and-overlay.md), [E5b](./E5b-comment-threads-and-review.md), [E8c](./E8c-acroform-field-model.md), [E18b](./E18b-collaboration-feature-plan.md) |
| ADRs | Implements ADR 0012; follows ADR 0006 |
| Branch | `feat/e18c-layer-document-schema` |
| PR title | `feat(sync): layer-document records for annotations, field values and page order` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Define the synced records (annotation, field value, page-order entry `{pageId, srcRef, rotation, fracIndex}`) and convert between ops and record patches in Rust (shared by the DO and clients via wasm), with schema versioning, validation and fractional indexing.

## Context

- ADR 0012
- [editing notes §3](../../research/architecture/notes/editing-architecture.md#3-collaboration-annotation-layer-sync-crdt-vs-server-authoritative-conflict-semantics)

## Scope

**In scope**

- `laminapdf-sync` crate (core)
- Record schemas (serde + tsify)
- Fractional index implementation
- Apply remote patches as `Origin::Remote` transactions

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** new `laminapdf-sync`
- **`Op` variants:** maps E4/E5/E8/E2 ops to records (annotation ops from [E4b](E4b-annotation-model-and-overlay.md), reply/review-state ops from [E5b](E5b-comment-threads-and-review.md), field-value ops from [E8c](E8c-acroform-field-model.md), page-order ops from [E2b](../ongoing/E2b-page-structure-ops.md), all dependencies)
- **`SaveConstraint`:** no new save behaviour beyond the round's: base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Schemas
- [ ] Mapping
- [ ] Frac index
- [ ] Tests

## Acceptance criteria

- [ ] Property test: any interleaving of ops from two peers converges to the same records
- [ ] Unknown versions rejected (ADR 0006 §6)

## Post-merge checks

None.

## Tests

- Property tests

## Docs to update

- rustdoc
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
