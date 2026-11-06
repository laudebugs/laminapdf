# E12b — Mark and review: Redact annotations and preview

| Field | Value |
| --- | --- |
| ID | `E12b` |
| Status | `ready` |
| Plan | [full-editor §E12](../../plans/full-editor.md#e12-redaction) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E4b](./E4b-annotation-model-and-overlay.md), [E12a](./E12a-redaction-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e12b-mark-and-review` |
| PR title | `feat(redact): mark redactions and preview them` |
| Size | M (200–600) |
| PR | — |

## Goal

`redact` mode: mark areas or text selections as Redact annotations (`/QuadPoints`, `/IC`, `/OverlayText`, `/RO`), review list with accept/reject, and a preview that renders the page as it will look after applying.

## Context

- **Design (Paper):** [G08 Redact mode: mark and review](../../design/paper/sign-protect-redact.md#g08-redact-mode-mark-and-review). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §10.2 What true redaction must do](../../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do)

## Scope

**In scope**

- Redact annotation spec and appearance
- Mode and review panel
- Preview render

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.06` permanent redaction
- **Crates and packages:** `laminapdf-model`, `laminapdf-build`, `@laminapdf/ui`
- **`Op` variants:** `MarkRedaction` (class `Annotation`)
- **`SaveConstraint`:** marks alone are annotations (incremental); the dialog warns that marks do not remove anything
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none. Marking alone is not permanent redaction, so `G.06` stays `unsupported` until it flips once, to `round-trip`, in [E12d](E12d-scrub-and-rewrite.md), when `ApplyRedactions` is complete and ungated. This PR's UI says "marked for redaction" and offers no apply action

## Tasks

- [ ] Spec
- [ ] Mode
- [ ] Panel
- [ ] Preview

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug. Expected for `MarkRedaction` (`Annotation`): allowed on P3 and approval-only, violation on P1 and P2
- [ ] Marks round-trip and show in Acrobat as redaction marks (manual)

## Post-merge checks

None.

## Tests

- Round-trip
- e2e

## Docs to update

- User help: redaction
- [full-editor §E12](../../plans/full-editor.md#e12-redaction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
