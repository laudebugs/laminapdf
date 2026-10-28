# E5e — Flatten annotations into content and comment summary

| Field | Value |
| --- | --- |
| ID | `E5e` |
| Status | `ready` |
| Plan | [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E2e](./E2e-content-append-api.md), [E5b](./E5b-comment-threads-and-review.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5e-flatten-and-summary` |
| PR title | `feat(ops): flatten annotations and export a comment summary` |
| Size | M (200–600) |
| PR | — |

## Goal

Flatten selected or all annotations: each appearance becomes a Form XObject drawn by an appended `q <cm> /FmN Do Q` stream (E2 append API) with the matrix from §12.5.5 algorithm, then the annotation is removed; and generate a comment summary as a new PDF.

## Context

- [04 §2.5 Flattening](../../research/pdf/04-interactive-security.md#25-flattening)
- [06 §10](../../research/pdf/06-feature-inventory.md#10-implications-for-lamina): "flattening without appearances" trap
- ISO 32000-2 §12.5.5 (appearance streams, Algorithm 8.1 equivalent)

## Scope

**In scope**

- `FlattenAnnots { ids | all, include_hidden: false }` op; generate missing appearances first (never flatten without `/AP`)
- Respect `/F` Hidden/NoView/Print flags and OCG membership
- Comment summary builder (list of comments with page thumbnails) as a new document

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.14` flatten annotations into content; `D.15` delete or summarise annotations
- **Crates and packages:** `lamina-content` (append API), `lamina-build`, `lamina-ops`
- **`Op` variants:** `FlattenAnnots` (classes `Content` + `Annotation`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` violates every DocMDP level and approval signatures' integrity → blocked unless the user confirms invalidating the named signatures. Note: flattened annotation objects remain in earlier revisions; the dialog offers a full rewrite
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.14` → `round-trip` (core, pdf, web, desktop); `D.15` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Matrix algorithm
- [ ] Missing-appearance generation
- [ ] Op and undo
- [ ] Summary builder
- [ ] UI actions

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Flattened pages render pixel-identical (below threshold) to the annotated pages in PDFium and pdf.js
- [ ] A test proves no annotation without `/AP` is flattened to nothing

## Tests

- Render diffs
- Trap test
- e2e

## Docs to update

- User help: flattening
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
