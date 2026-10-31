# E9a — Form authoring: feature plan

| Field | Value |
| --- | --- |
| ID | `E9a` |
| Status | `ready` |
| Plan | [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8b](./E8b-form-filling-feature-plan.md) |
| Blocks | [E9b](./E9b-field-creation-ops.md), [E10a](./E10a-adr-0020-signatures.md), [E10b](./E10b-signatures-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e9a-form-authoring-feature-plan` |
| PR title | `docs(editor): E9 feature plan (form authoring)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E9 ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)): a form-design mode to create, move, resize and configure every field type and signature fields, validation/format/calculation settings, tab-order editor, accessible labels, and optional automatic field detection on flat forms. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E9](../../plans/full-editor.md#e9-form-authoring): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e9-form-authoring.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E9 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01` text fields (single/multi-line); `E.02` checkboxes and radio buttons; `E.03` dropdowns and list boxes; `E.04` push buttons; `E.07` input validation; `E.08` automatic calculations; `E.09` tab order; `E.15` signature fields; `L.06` accessible form labels
- **Crates and packages:** (planned for the round) as E8 plus form-design mode
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) incremental on `Free`; authoring is class `Document`: under any certification a DocMDP violation, saved only after invalidating the named signatures; with approval signatures only `AppendOnly`, except changes to FieldMDP-locked fields
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e9-form-authoring.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`E.01`, `E.02`, `E.03`, `E.04`, `E.07`, `E.08`, `E.09`, `E.15`, `L.06`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `lamina` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E9"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E9b onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E9](../../plans/full-editor.md#e9-form-authoring)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E9"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Automatic field detection: heuristic only (lines/boxes) or an optional ML model (size, licence)?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
