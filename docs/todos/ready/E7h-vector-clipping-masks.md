# E7h — Vector path editing, fills, strokes, clipping paths and masks

| Field | Value |
| --- | --- |
| ID | `E7h` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E7d](./E7d-select-transform-existing.md) |
| Blocks | [E7j](./E7j-edit-corpus.md) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7h-vector-clipping-masks` |
| PR title | `feat(content): edit vector paths, clipping and masks` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Edit existing paths (points, fill/stroke colour, width, dash), edit or remove clipping paths and soft/image masks on objects.

## Context

- [02 §5 Paths](../../research/pdf/02-imaging-model.md#5-paths)
- [02 §7.2 Masking](../../research/pdf/02-imaging-model.md#72-masking)

## Scope

**In scope**

- Path point editing
- Style edits
- Clip and mask edits
- UI handles for Bézier points

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `B.07` edit vector paths, fills, strokes; `B.11` clipping paths and masks
- **Crates and packages:** `lamina-content`, `@lamina/ui`
- **`Op` variants:** `EditContent` variants `EditPath`, `SetClip`, `SetMask` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` violates every DocMDP level and approval signatures' integrity → blocked unless the user confirms invalidating the named signatures
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `B.07` → `round-trip` (core, pdf, web, desktop); `B.11` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] IR edits
- [ ] UI
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Render diffs confined to edited objects

## Tests

- Render diffs
- e2e

## Docs to update

- User help
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
