# E17b — Action editor: JavaScript authoring, buttons, event-triggered and open actions

| Field | Value |
| --- | --- |
| ID | `E17b` |
| Status | `ready` |
| Plan | [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E3c](./E3c-links.md), [E9c](./E9c-form-design-mode.md), [E17a](./E17a-advanced-interactions-feature-plan.md), [E17c](./E17c-layers.md) |
| Blocks | [E17f](./E17f-3d-detection.md) |
| ADRs | Follows ADR 0021; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e17b-action-editor` |
| PR title | `feat(actions): action editor for links, buttons, events and open actions` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Author actions (GoTo, URI, Named, ResetForm, Hide, SetOCGState, JavaScript as text only — never executed outside ADR 0021) on links, buttons, fields, pages and the document (`/AA`, `/OpenAction`), with a full inventory of existing actions.

## Context

- **Design (Paper):** [X11 Layers and actions](../../design/paper/convert-compare-produce.md#x11-layers-and-actions). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- ADR 0021
- [04 §1.3 Actions](../../research/pdf/04-interactive-security.md#13-actions-126)
- [04 §4](../../research/pdf/04-interactive-security.md#4-javascript-in-pdf)

## Scope

**In scope**

- Action model and ops (`SetActions { target, trigger, actions }`)
- Editor UI
- JS editor with warning banner

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `M.01` add/edit JavaScript actions; `M.02` interactive buttons; `M.03` event-triggered actions; `M.10` document open actions
- **Crates and packages:** `lamina-model`, `lamina-ops`, `@laminapdf/ui`
- **`Op` variants:** `SetActions` (class `Document`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Document` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations); ops noted as `Annotation` are allowed under P3 or when there is no certification. Violations are gated (override names the signatures). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock`
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `M.01` → `write` (core, pdf, web, desktop); `M.02` → `round-trip` (core, pdf, web, desktop); `M.03` → `round-trip` (core, pdf, web, desktop); `M.10` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Model
- [ ] Ops
- [ ] UI

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean

## Post-merge checks

None.

## Tests

- Round-trip
- e2e

## Docs to update

- User help
- [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
