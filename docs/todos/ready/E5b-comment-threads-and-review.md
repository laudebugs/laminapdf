# E5b — Comment panel: replies, review state, filter and search

| Field | Value |
| --- | --- |
| ID | `E5b` |
| Status | `ready` |
| Plan | [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4f](./E4f-annotation-tools-ui.md), [E5a](./E5a-review-stamps-interchange-feature-plan.md) |
| Blocks | [E5d](./E5d-xfdf-fdf.md), [E5e](./E5e-flatten-and-summary.md), [E11d](./E11d-sanitiser.md), [E18b](./E18b-collaboration-feature-plan.md), [E18c](./E18c-layer-document-schema.md), [E18e](./E18e-client-sync-and-presence.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5b-comment-threads-and-review` |
| PR title | `feat(annots): replies, review states and comment panel` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Reply threads (`/IRT`), state-model replies (`/StateModel /Review` Accepted/Rejected/Cancelled/Completed/None and `/Marked`), a comment panel with filtering (author, type, page, state, date) and search, and delete (single/bulk).

## Context

- **Design (Paper):** [C04 Comments panel](../../design/paper/comment-review.md#c04-comments-panel). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §2.3](../../research/pdf/04-interactive-security.md#23-markup-annotations-replies-review-state-12562)
- ISO 32000-2 §12.5.6.2 (markup annotations), Table 172 (state models)

## Scope

**In scope**

- Ops: `AddReply { parent, text, author, at }`, `SetReviewState { parent, model, state, author, at }`, bulk `DeleteAnnots`
- Thread view in `laminapdf-model` (tolerant of cycles and missing parents)
- Comment panel with filters, search, jump-to; author name from settings

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.10` replies to comments; `D.11` status and review properties; `D.12` filter and search comments; `D.15` delete or summarise annotations
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `@laminapdf/ui`
- **`Op` variants:** `AddReply`, `SetReviewState`, `DeleteAnnots` (class `Annotation`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Annotation` is allowed only under DocMDP P3 or when there is no certification signature (approval signatures alone allow annotations); otherwise a violation → warn and gate. Note the Evil Annotation Attack: annotations added after certification are shown as such in the signature panel (E10)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.10` → `round-trip` (core, pdf, web, desktop); `D.11` → `round-trip` (core, pdf, web, desktop); `D.12` → `read` (core, pdf, web, desktop); `D.15` not flipped here: this PR ships only its delete half (`DeleteAnnots`); the row flips once, to `round-trip`, in [E5e](E5e-flatten-and-summary.md), which adds the summary and completes it

## Tasks

- [ ] Thread model
- [ ] Ops
- [ ] Panel
- [ ] Tests incl. Acrobat-made threads

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Threads created in lamina show as threads in Acrobat Reader (manual check recorded) and pdf.js (if supported) — XFDF export identical

## Post-merge checks

None.

## Tests

- Unit tests for cycles
- e2e

## Docs to update

- User help: reviewing
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
